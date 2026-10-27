//! pt-BR rendering of protocol values.
use clipsync_proto::{
    DeliveryState, DiagStatus, ModuleKind, ModuleSet, PeerPerms, PeerState, RejectReason,
};

/// Anything that came from a peer is data, not terminal commands: controls (C0, DEL, C1) and Unicode
/// formatting characters become `?`; line breaks and tabs become a space.
pub fn clean(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\n' | '\r' | '\t' => ' ',
            c if c.is_control()
                || matches!(c, '\u{061C}' | '\u{200B}'..='\u{200F}' | '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}') => '?',
            c => c,
        })
        .collect()
}

pub fn delivery(peer: &str, state: DeliveryState) -> String {
    let peer = clean(peer);
    let peer = peer.as_str();
    let text = match state {
        DeliveryState::Applied => "aplicado".to_owned(),
        DeliveryState::InInbox => "na caixa de recebidos".to_owned(),
        DeliveryState::Received => "recebido".to_owned(),
        DeliveryState::Expired => "expirado".to_owned(),
        DeliveryState::Unreachable => "não alcançado".to_owned(),
        DeliveryState::Rejected(RejectReason::ModuleDisabled) => {
            format!("recusado: módulo clipboard desligado em {peer}")
        }
        DeliveryState::Rejected(RejectReason::Paused) => "recusado: pausado".to_owned(),
        DeliveryState::Rejected(r) => format!("recusado: {}", r.pt()),
    };
    crate::msg::delivery(peer, &text)
}

/// Applied and InInbox are the only outcomes that count as delivered.
pub fn delivered(state: DeliveryState) -> bool {
    matches!(state, DeliveryState::Applied | DeliveryState::InInbox)
}

pub fn modules(m: ModuleSet) -> String {
    let mut v = Vec::new();
    if m.clipboard {
        v.push("clipboard");
    }
    if m.input {
        v.push("input");
    }
    if v.is_empty() {
        crate::msg::NONE.to_owned()
    } else {
        v.join(", ")
    }
}

pub fn perms(p: PeerPerms) -> String {
    let mut v = Vec::new();
    if p.clipboard_in {
        v.push("clipboard-in");
    }
    if p.clipboard_out {
        v.push("clipboard-out");
    }
    if p.input_in {
        v.push("input-in");
    }
    if v.is_empty() {
        crate::msg::NONE.to_owned()
    } else {
        v.join(", ")
    }
}

pub fn peer_state(s: &PeerState) -> String {
    match s {
        PeerState::Connected => crate::msg::CONNECTED.to_owned(),
        PeerState::Connecting => crate::msg::CONNECTING.to_owned(),
        PeerState::Disconnected => crate::msg::DISCONNECTED.to_owned(),
        PeerState::Blocked { reason } => crate::msg::blocked(reason),
    }
}

pub fn kind(k: ModuleKind) -> &'static str {
    match k {
        ModuleKind::Clipboard => "clipboard",
        ModuleKind::Input => "input",
    }
}

pub fn diag_status(s: DiagStatus) -> &'static str {
    match s {
        DiagStatus::Ok => "ok",
        DiagStatus::Warn => "aviso",
        DiagStatus::Missing => "ausente",
        DiagStatus::Error => "erro",
    }
}

/// `HH:MM:SS UTC` of a unix time in ms (local zones need unsafe libc calls, forbidden here).
pub fn clock_utc(unix_ms: u64) -> String {
    let day = (unix_ms / 1000) % 86_400;
    format!(
        "{:02}:{:02}:{:02} UTC",
        day / 3600,
        (day / 60) % 60,
        day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_replaces_controls_c1_and_bidi_and_keeps_text() {
        assert_eq!(clean("ev\u{1b}[2Jil"), "ev?[2Jil");
        assert_eq!(clean("a\u{9b}b"), "a?b");
        assert_eq!(clean("a\u{202e}b\u{200b}c\u{feff}"), "a?b?c?");
        assert_eq!(clean("linha1\nlinha2\tx"), "linha1 linha2 x");
        assert_eq!(clean("olá ção ✓"), "olá ção ✓");
    }

    #[test]
    fn a_hostile_peer_name_never_reaches_the_delivery_line_raw() {
        let line = delivery("x\u{1b}[31m", DeliveryState::Applied);
        assert!(!line.contains('\u{1b}'), "{line:?}");
    }
}
