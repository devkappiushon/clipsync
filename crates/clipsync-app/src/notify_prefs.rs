//! Quais notificações do sistema o usuário quer (`Prefs.notifications`, §12.5). A casca confere isto
//! antes de mostrar qualquer `notification`, mesmo que o crate já tenha filtrado: a preferência é
//! do usuário e a casca é quem fala com o sistema. Sem dado, tudo desligado (falha fechada).
//! Módulo puro (só `serde_json`), por isso testável sem o Tauri.

use serde_json::Value;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NotifyPrefs {
    pub item_received: bool,
    pub device_connected: bool,
    pub block: bool,
}

impl NotifyPrefs {
    /// Lê `notifications` de um objeto `Prefs` (resposta de `prefs_get` ou `data.prefs` do evento
    /// `prefs_changed`). Forma errada ou campo faltando dá `None`: quem chama mantém o que tinha.
    pub fn from_prefs(prefs: &Value) -> Option<Self> {
        let n = prefs.get("notifications")?;
        let flag = |name: &str| n.get(name).and_then(Value::as_bool);
        Some(Self {
            item_received: flag("item_received")?,
            device_connected: flag("device_connected")?,
            block: flag("block")?,
        })
    }

    /// `kind` é o `kind` do evento `notification`. Tipo desconhecido não passa.
    pub fn allows(&self, kind: &str) -> bool {
        match kind {
            "item_received" => self.item_received,
            "device_connected" => self.device_connected,
            "block" => self.block,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn prefs(item: bool, device: bool, block: bool) -> Value {
        json!({
            "autostart": false,
            "notifications": { "item_received": item, "device_connected": device, "block": block },
            "appearance": { "theme": "system", "reduce_motion": false },
        })
    }

    #[test]
    fn nothing_is_allowed_until_the_preferences_are_known() {
        let unknown = NotifyPrefs::default();
        for kind in ["item_received", "device_connected", "block", "outro"] {
            assert!(!unknown.allows(kind), "{kind}");
        }
    }

    #[test]
    fn each_kind_follows_its_own_switch() {
        let p = NotifyPrefs::from_prefs(&prefs(true, false, true)).expect("valid prefs");
        assert!(p.allows("item_received"));
        assert!(!p.allows("device_connected"));
        assert!(p.allows("block"));
        let off = NotifyPrefs::from_prefs(&prefs(false, true, false)).expect("valid prefs");
        assert!(
            !off.allows("item_received") && off.allows("device_connected") && !off.allows("block")
        );
    }

    #[test]
    fn an_unknown_kind_is_never_allowed() {
        let all = NotifyPrefs::from_prefs(&prefs(true, true, true)).expect("valid prefs");
        assert!(!all.allows("clipboard_content"));
        assert!(!all.allows(""));
    }

    #[test]
    fn a_bad_shape_gives_none_and_keeps_the_caller_where_it_was() {
        assert_eq!(NotifyPrefs::from_prefs(&json!({})), None);
        assert_eq!(NotifyPrefs::from_prefs(&json!(null)), None);
        assert_eq!(
            NotifyPrefs::from_prefs(&json!({ "notifications": { "block": true } })),
            None
        );
        assert_eq!(
            NotifyPrefs::from_prefs(&json!({ "notifications": {
                "item_received": "sim", "device_connected": false, "block": false } })),
            None
        );
    }
}
