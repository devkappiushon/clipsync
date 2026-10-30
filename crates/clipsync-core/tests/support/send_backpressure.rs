use super::*;
use clipsync_proto::{ClipOffer, MIME_TEXT_UTF8};
struct Pair {
    server: Endpoint,
    client: Endpoint,
    send: Connection,
    receive: Connection,
}
impl Pair {
    async fn new() -> Self {
        let a = Identity::generate().unwrap();
        let b = Identity::generate().unwrap();
        let mut config =
            tls::server_config(&b, PinVerifier::new(VerifyMode::Expect(a.id())), ALPN_MESH)
                .unwrap();
        let mut receive = quinn::TransportConfig::default();
        receive
            .stream_receive_window(32u32.into())
            .receive_window(32u32.into());
        config.transport_config(Arc::new(receive));
        let server = Endpoint::server(config, "127.0.0.1:0".parse().unwrap()).unwrap();
        let client = Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        let mut config =
            tls::client_config(&a, PinVerifier::new(VerifyMode::Expect(b.id())), ALPN_MESH)
                .unwrap();
        let mut send = quinn::TransportConfig::default();
        send.send_window(64);
        config.transport_config(Arc::new(send));
        let outgoing = client
            .connect_with(config, server.local_addr().unwrap(), crate::SERVER_NAME)
            .unwrap();
        let (send, receive) =
            tokio::join!(outgoing, async { server.accept().await.unwrap().await });
        Self {
            server,
            client,
            send: send.unwrap(),
            receive: receive.unwrap(),
        }
    }
    fn frame() -> Frame {
        Frame::Module(ModuleMsg::Clip(ClipMsg::Offer(ClipOffer {
            id: EventId::random(),
            lamport: 1,
            mime: MIME_TEXT_UTF8.into(),
            text: "x".repeat(65536),
            created_unix_ms: 1,
            ttl_ms: 1000,
        })))
    }
    fn stop(self) {
        self.server.close(0u32.into(), b"test");
        self.client.close(0u32.into(), b"test");
    }
}
#[tokio::test]
async fn partial_write_timeout_resets_the_stream_instead_of_finishing_prefix() {
    let pair = Pair::new().await;
    let conn = pair.send.clone();
    let task = tokio::spawn(async move { send_uni(&conn, &Pair::frame()).await });
    let mut recv = tokio::time::timeout(Duration::from_secs(1), pair.receive.accept_uni())
        .await
        .unwrap()
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err(SendError::Transport(_))));
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), recv.read_to_end(100_000))
            .await
            .unwrap(),
        Err(quinn::ReadToEndError::Read(quinn::ReadError::Reset(_)))
    ));
    pair.stop();
}
#[tokio::test]
async fn external_cancellation_after_prefix_resets_and_releases_credit() {
    let pair = Pair::new().await;
    let conn = pair.send.clone();
    let task = tokio::spawn(async move { send_uni(&conn, &Pair::frame()).await });
    let mut recv = tokio::time::timeout(Duration::from_secs(1), pair.receive.accept_uni())
        .await
        .unwrap()
        .unwrap();
    let mut prefix = [0; 4];
    recv.read_exact(&mut prefix).await.unwrap();
    assert!(u32::from_be_bytes(prefix) > 1000);
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(1), recv.read_to_end(100_000))
            .await
            .unwrap(),
        Err(quinn::ReadToEndError::Read(quinn::ReadError::Reset(_)))
    ));
    // Cancellation leaves the connection usable for a complete following control frame.
    let conn = pair.send.clone();
    let send = tokio::spawn(async move { send_uni(&conn, &Frame::Ping { nonce: 4 }).await });
    let mut next = pair.receive.accept_uni().await.unwrap();
    assert!(matches!(
        read_frame(&mut next, MAX_FRAME_BYTES).await.unwrap(),
        Frame::Ping { nonce: 4 }
    ));
    send.await.unwrap().unwrap();
    pair.stop();
}
