//! TLS 1.3 mutual auth with self-signed certificates and key pinning (contract §5 rules 1, 2, 9).
//! Owned by Rivet.
//!
//! Rejections only carry fixed rustls errors (`ApplicationVerificationFailure` for an unknown or
//! changed key, `UnknownIssuer` for a broken X.509): quinn copies the rustls error text into the
//! CONNECTION_CLOSE reason, so nothing internal may be in it. Details go to the local log.

use std::collections::HashSet;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Duration;

use clipsync_proto::PeerId;
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::{ParsedCertificate, WebPkiClientVerifier};
use rustls::{
    CertificateError, DigitallySignedStruct, DistinguishedName, RootCertStore, SignatureScheme,
};

use crate::{CoreError, Identity};

pub const KEEP_ALIVE: Duration = Duration::from_secs(5);
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);

/// The ONLY function that turns a certificate into a PeerId (contract §5, boundary rule 2):
/// SHA-256 of the SubjectPublicKeyInfo DER, via `ParsedCertificate`, as in the spike.
pub fn pin_of(cert: &CertificateDer<'_>) -> Result<PeerId, CoreError> {
    let parsed =
        ParsedCertificate::try_from(cert).map_err(|e| CoreError::Identity(e.to_string()))?;
    Ok(PeerId::from_spki_der(
        parsed.subject_public_key_info().as_ref(),
    ))
}

/// PeerId of the first certificate a connected peer presented.
pub fn peer_id_of(conn: &quinn::Connection) -> Option<PeerId> {
    let certs = conn
        .peer_identity()?
        .downcast::<Vec<CertificateDer<'static>>>()
        .ok()?;
    pin_of(certs.first()?).ok()
}

/// Pinned keys of the mesh, shared with the agent so `ReloadPeers` takes effect at once.
#[derive(Debug, Default)]
pub struct PinSet(RwLock<HashSet<PeerId>>);

impl PinSet {
    pub fn new(ids: impl IntoIterator<Item = PeerId>) -> Self {
        Self(RwLock::new(ids.into_iter().collect()))
    }

    pub fn replace(&self, ids: impl IntoIterator<Item = PeerId>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = ids.into_iter().collect();
    }

    pub fn contains(&self, id: &PeerId) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(id)
    }
}

#[derive(Debug)]
pub enum VerifyMode {
    /// Mesh server: the client's key must be in peers.toml.
    Mesh(Arc<PinSet>),
    /// Dialing a known peer: the server must present exactly this key.
    Expect(PeerId),
    /// Pairing (ALPN_PAIR): any key, but possession is still proven by the TLS 1.3 signature.
    Pairing,
}

/// The single verifier (contract §5 rule 1): implements both `ClientCertVerifier` and
/// `ServerCertVerifier`. Nothing is switched off: the X.509 is validated with the certificate
/// itself as the anchor, the TLS 1.3 handshake signature is verified with ring, and outside
/// pairing the SHA-256 of the SPKI must be pinned.
pub struct PinVerifier {
    mode: VerifyMode,
    provider: Arc<CryptoProvider>,
    calls: AtomicUsize,
    rejected: Mutex<Option<PeerId>>,
}

impl fmt::Debug for PinVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PinVerifier")
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn rejected(e: CertificateError) -> rustls::Error {
    rustls::Error::InvalidCertificate(e)
}

impl PinVerifier {
    pub fn new(mode: VerifyMode) -> Arc<Self> {
        Arc::new(Self {
            mode,
            provider: provider(),
            calls: AtomicUsize::new(0),
            rejected: Mutex::new(None),
        })
    }

    /// How many certificates this verifier has checked. Proves that resumption is off:
    /// every connection, including reconnections, runs it.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }

    /// Key of the last certificate refused by the pin (unknown peer, or changed key in Expect).
    pub fn last_rejected(&self) -> Option<PeerId> {
        *self.rejected.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn check(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<PeerId, rustls::Error> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if !intermediates.is_empty() {
            tracing::warn!("certificate chain refused: peers present only their own certificate");
            return Err(rejected(CertificateError::UnknownIssuer));
        }
        self.check_x509(end_entity, now).map_err(|e| {
            tracing::warn!(error = %e, "invalid X.509 refused");
            rejected(CertificateError::UnknownIssuer)
        })?;
        let id = pin_of(end_entity).map_err(|_| rejected(CertificateError::UnknownIssuer))?;
        let allowed = match &self.mode {
            VerifyMode::Mesh(pins) => pins.contains(&id),
            VerifyMode::Expect(want) => *want == id,
            VerifyMode::Pairing => true,
        };
        if allowed {
            return Ok(id);
        }
        *self.rejected.lock().unwrap_or_else(PoisonError::into_inner) = Some(id);
        match &self.mode {
            VerifyMode::Expect(want) => {
                tracing::warn!(expected = %want, got = %id, "{}", crate::msg::KEY_CHANGED_LOG)
            }
            _ => tracing::warn!("{} {id}", crate::msg::UNKNOWN_PEER_LOG),
        }
        Err(rejected(CertificateError::ApplicationVerificationFailure))
    }

    /// Self-signature, validity period and EKU, with the certificate as its own trust anchor.
    fn check_x509(&self, cert: &CertificateDer<'_>, now: UnixTime) -> Result<(), rustls::Error> {
        let mut roots = RootCertStore::empty();
        roots.add(cert.clone().into_owned())?;
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots), self.provider.clone())
            .build()
            .map_err(|e| rustls::Error::General(e.to_string()))?
            .verify_client_cert(cert, &[], now)
            .map(|_| ())
    }

    fn tls13(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn no_tls12() -> rustls::Error {
    rustls::Error::PeerIncompatible(rustls::PeerIncompatible::Tls13RequiredForQuic)
}

impl ClientCertVerifier for PinVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        self.check(end_entity, intermediates, now)
            .map(|_| ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(no_tls12())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        self.check(end_entity, intermediates, now)
            .map(|_| ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _: &[u8],
        _: &CertificateDer<'_>,
        _: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Err(no_tls12())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.tls13(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

/// Keep-alive 5 s, idle 15 s, datagrams on (quinn default receive buffer).
pub fn transport_config() -> Arc<quinn::TransportConfig> {
    let mut t = quinn::TransportConfig::default();
    t.keep_alive_interval(Some(KEEP_ALIVE));
    if let Ok(idle) = quinn::IdleTimeout::try_from(IDLE_TIMEOUT) {
        t.max_idle_timeout(Some(idle));
    }
    Arc::new(t)
}

fn tls_err(e: impl fmt::Display) -> CoreError {
    CoreError::Transport(format!("tls config: {e}"))
}

/// Mutual TLS 1.3 server. Resumption is OFF (rule 2): with a session cache or tickets the
/// verifier would not run on reconnection.
pub fn server_config(
    id: &Identity,
    verifier: Arc<PinVerifier>,
    alpn: &[u8],
) -> Result<quinn::ServerConfig, CoreError> {
    let mut tls = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(tls_err)?
        .with_client_cert_verifier(verifier)
        .with_single_cert(vec![id.cert_der()], id.key_der())
        .map_err(tls_err)?;
    tls.alpn_protocols = vec![alpn.to_vec()];
    tls.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
    tls.send_tls13_tickets = 0;
    let crypto = QuicServerConfig::try_from(tls).map_err(tls_err)?;
    let mut cfg = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    cfg.transport_config(transport_config());
    Ok(cfg)
}

/// Mutual TLS 1.3 client, resumption OFF (rule 2).
pub fn client_config(
    id: &Identity,
    verifier: Arc<PinVerifier>,
    alpn: &[u8],
) -> Result<quinn::ClientConfig, CoreError> {
    let mut tls = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(tls_err)?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(vec![id.cert_der()], id.key_der())
        .map_err(tls_err)?;
    tls.alpn_protocols = vec![alpn.to_vec()];
    tls.resumption = rustls::client::Resumption::disabled();
    let crypto = QuicClientConfig::try_from(tls).map_err(tls_err)?;
    let mut cfg = quinn::ClientConfig::new(Arc::new(crypto));
    cfg.transport_config(transport_config());
    Ok(cfg)
}

/// Fixed rustls texts that quinn may put in a handshake CONNECTION_CLOSE. Anything else in a
/// reason would be a leak (rule 9).
pub const HANDSHAKE_REJECT_REASONS: [&str; 2] = [
    "invalid peer certificate: ApplicationVerificationFailure",
    "invalid peer certificate: UnknownIssuer",
];

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use clipsync_proto::{ALPN_MESH, CloseCode};
    use quinn::{ConnectionError, Endpoint, VarInt};

    use super::*;

    fn ident(_name: &str) -> Identity {
        Identity::generate().unwrap()
    }

    fn server(id: &Identity, v: Arc<PinVerifier>) -> Endpoint {
        let cfg = server_config(id, v, ALPN_MESH).unwrap();
        Endpoint::server(cfg, "127.0.0.1:0".parse().unwrap()).unwrap()
    }

    fn client() -> Endpoint {
        Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap()
    }

    /// Accepts one connection; Ok(conn) only if the server side completed the handshake.
    async fn accept_one(ep: &Endpoint) -> Result<quinn::Connection, ConnectionError> {
        ep.accept().await.unwrap().await
    }

    async fn dial(
        ep: &Endpoint,
        id: &Identity,
        v: Arc<PinVerifier>,
        to: SocketAddr,
    ) -> Result<quinn::Connection, ConnectionError> {
        let cfg = client_config(id, v, ALPN_MESH).unwrap();
        ep.connect_with(cfg, to, crate::SERVER_NAME).unwrap().await
    }

    fn reason_of(e: &ConnectionError) -> Option<String> {
        match e {
            ConnectionError::ConnectionClosed(c) => Some(String::from_utf8_lossy(&c.reason).into()),
            ConnectionError::ApplicationClosed(c) => {
                Some(String::from_utf8_lossy(&c.reason).into())
            }
            ConnectionError::TransportError(t) => Some(t.reason.clone()),
            _ => None,
        }
    }

    fn allowed_reason(r: &str) -> bool {
        CloseCode::ALL.iter().any(|c| c.reason() == r.as_bytes())
            || HANDSHAKE_REJECT_REASONS.contains(&r)
    }

    #[tokio::test]
    async fn tls_verifier_runs_on_every_connection_resumption_is_off() {
        let (a, b) = (ident("a"), ident("b"));
        let pins = Arc::new(PinSet::new([b.id()]));
        let sv = PinVerifier::new(VerifyMode::Mesh(pins));
        let srv = server(&a, sv.clone());
        let addr = srv.local_addr().unwrap();
        let ep = client();
        for round in 1..=3usize {
            let cv = PinVerifier::new(VerifyMode::Expect(a.id()));
            let (c, s) = tokio::join!(dial(&ep, &b, cv.clone(), addr), accept_one(&srv));
            let (c, s) = (c.unwrap(), s.unwrap());
            assert_eq!(peer_id_of(&s), Some(b.id()));
            assert_eq!(peer_id_of(&c), Some(a.id()));
            assert_eq!(
                sv.calls(),
                round,
                "server verifier ran on connection {round}"
            );
            assert_eq!(cv.calls(), 1, "client verifier ran on connection {round}");
            c.close(
                VarInt::from_u32(CloseCode::Normal.code()),
                CloseCode::Normal.reason(),
            );
            assert!(matches!(
                s.closed().await,
                ConnectionError::ApplicationClosed(_)
            ));
        }
    }

    #[tokio::test]
    async fn tls_unknown_client_refused_with_fixed_reason() {
        let (a, c) = (ident("a"), ident("intruder"));
        let sv = PinVerifier::new(VerifyMode::Mesh(Arc::new(PinSet::default())));
        let srv = server(&a, sv.clone());
        let addr = srv.local_addr().unwrap();
        // The intruder would accept any server: the refusal must come from A.
        let ep = client();
        let (cres, sres) = tokio::join!(
            dial(&ep, &c, PinVerifier::new(VerifyMode::Pairing), addr),
            accept_one(&srv)
        );
        let server_err = sres.expect_err("server must refuse");
        assert_eq!(sv.last_rejected(), Some(c.id()));
        // TLS 1.3: connect() may return Ok before the server checks the client cert.
        let client_err = match cres {
            Ok(conn) => conn.closed().await,
            Err(e) => e,
        };
        for e in [&server_err, &client_err] {
            let r = reason_of(e).unwrap_or_default();
            assert!(allowed_reason(&r), "leaked reason: {r:?}");
            assert!(
                !r.contains(&c.id().to_hex()[..8]) && !r.contains("Other"),
                "{r:?}"
            );
        }
        assert_eq!(
            reason_of(&client_err).as_deref(),
            Some(HANDSHAKE_REJECT_REASONS[0])
        );
    }

    #[tokio::test]
    async fn tls_changed_server_key_detected_by_dialer() {
        let (a, a2, b) = (ident("a"), ident("a"), ident("b"));
        let pins = Arc::new(PinSet::new([b.id()]));
        let srv = server(&a2, PinVerifier::new(VerifyMode::Mesh(pins)));
        let addr = srv.local_addr().unwrap();
        let ep = client();
        let cv = PinVerifier::new(VerifyMode::Expect(a.id()));
        let (cres, _) = tokio::join!(dial(&ep, &b, cv.clone(), addr), async {
            let _ = accept_one(&srv).await;
        });
        let e = cres.expect_err("dialer must refuse a changed key");
        assert_eq!(cv.last_rejected(), Some(a2.id()));
        assert!(matches!(e, ConnectionError::TransportError(_)), "{e:?}");
    }

    #[tokio::test]
    async fn tls_pairing_mode_accepts_any_key_but_mesh_does_not() {
        let (a, b) = (ident("a"), ident("b"));
        let srv = server(&a, PinVerifier::new(VerifyMode::Pairing));
        let addr = srv.local_addr().unwrap();
        let ep = client();
        let (c, s) = tokio::join!(
            dial(&ep, &b, PinVerifier::new(VerifyMode::Pairing), addr),
            accept_one(&srv)
        );
        assert_eq!(peer_id_of(&s.unwrap()), Some(b.id()));
        assert_eq!(peer_id_of(&c.unwrap()), Some(a.id()));
    }

    #[test]
    fn tls_pin_set_replace() {
        let set = PinSet::new([PeerId([1; 32])]);
        assert!(set.contains(&PeerId([1; 32])));
        set.replace([PeerId([2; 32])]);
        assert!(!set.contains(&PeerId([1; 32])) && set.contains(&PeerId([2; 32])));
    }
}
