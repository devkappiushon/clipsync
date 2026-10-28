//! 7 · Preferências (§12.5): `prefs_get` and `prefs_set`, kept in `DIR/ui.toml`. Owned by Rivet.

use serde::{Deserialize, Serialize};

/// Shortcuts are text only in this version (feature `global_shortcuts` false).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Shortcuts {
    pub open: String,
    pub pause_all: String,
    pub send_once: String,
    pub lock_screen: String,
    pub go_to_device: Vec<String>,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            open: "Win+Alt+C".into(),
            pause_all: "Win+Alt+P".into(),
            send_once: "Win+Alt+V".into(),
            lock_screen: "Win+Alt+L".into(),
            go_to_device: vec!["Win+Alt+1".into(), "Win+Alt+2".into(), "Win+Alt+3".into()],
        }
    }
}

/// Which notifications reach the system (never with clipboard content, §12.2 rule 7).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Notifications {
    pub item_received: bool,
    pub device_connected: bool,
    pub block: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self {
            item_received: true,
            device_connected: false,
            block: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    pub theme: Theme,
    pub reduce_motion: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Prefs {
    pub shortcuts: Shortcuts,
    /// Off by default: explicit consent.
    pub autostart: bool,
    pub notifications: Notifications,
    pub appearance: Appearance,
    pub language: String,
    pub version: String,
    pub license: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            shortcuts: Shortcuts::default(),
            autostart: false,
            notifications: Notifications::default(),
            appearance: Appearance::default(),
            language: "pt-BR".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "a definir".into(),
        }
    }
}
