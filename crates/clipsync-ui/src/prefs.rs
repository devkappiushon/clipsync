//! 7 · Preferências (§12.5): `prefs_get` and `prefs_set`, kept in `DIR/ui.toml`. Owned by Rivet.
//!
//! `ui.toml` keeps notifications and appearance; autostart lives in the system (`autostart.rs`)
//! and is read from there, so a change made outside the app shows up as it is.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::api::{PrefsSetArgs, code};
use crate::autostart::fsio;
use crate::events::{PrefsChanged, UiEvent};
use crate::{Ctx, UiError, autostart, lock, msg};

const FILE: &str = "ui.toml";
const VERSION: u32 = 1;

/// Fixed text in this version. On Windows with input on they work (feature `global_shortcuts`,
/// §13.9); on Linux they are only shown.
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

/// The license shown in "Sobre". Until the owner picks one the real legal state applies: all
/// rights reserved. Changing the license later is this one line (and the three fixtures).
pub const LICENSE: &str = "Todos os direitos reservados";

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
            license: LICENSE.into(),
        }
    }
}

/// What `ui.toml` holds.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stored {
    version: u32,
    #[serde(default)]
    pub(crate) notifications: Notifications,
    #[serde(default)]
    pub(crate) appearance: Appearance,
}

impl Default for Stored {
    fn default() -> Self {
        Self {
            version: VERSION,
            notifications: Notifications::default(),
            appearance: Appearance::default(),
        }
    }
}

/// A missing or unreadable `ui.toml` gives the defaults: preferences never keep the app from
/// opening, and the next `prefs_set` writes a good file.
pub(crate) fn load(dir: &Path) -> Stored {
    let text = match fs::read_to_string(dir.join(FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Stored::default(),
        Err(e) => {
            tracing::warn!(error = %e, "ui.toml unreadable, using defaults");
            return Stored::default();
        }
    };
    match toml::from_str::<Stored>(&text) {
        Ok(s) if s.version == VERSION => s,
        Ok(s) => {
            tracing::warn!(
                version = s.version,
                "ui.toml version not supported, using defaults"
            );
            Stored::default()
        }
        Err(e) => {
            tracing::warn!(error = %e, "ui.toml invalid, using defaults");
            Stored::default()
        }
    }
}

/// New temp file (0600) + rename (§12.2 rule 5). The profile directory may not exist before the
/// onboarding.
fn save(dir: &Path, s: &Stored) -> Result<(), UiError> {
    let text = toml::to_string(s).map_err(|e| UiError::internal(&e.to_string()))?;
    create_dir(dir)
        .and_then(|()| fsio::write_atomic(&dir.join(FILE), text.as_bytes(), 0o600))
        .map_err(|e| UiError::new(code::IO, msg::io(&e.to_string())))
}

/// 0700 like `Profile::init`, which would tighten it anyway.
fn create_dir(dir: &Path) -> io::Result<()> {
    let mut b = fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
    b.create(dir)
}

fn current(ctx: &Ctx, stored: Stored) -> Prefs {
    Prefs {
        autostart: autostart::is_enabled(ctx.opts.autostart_dir.as_deref()),
        notifications: stored.notifications,
        appearance: stored.appearance,
        ..Prefs::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrefsReply {
    pub prefs: Prefs,
}

pub(crate) fn get(ctx: &Ctx) -> Prefs {
    let stored = *lock(&ctx.prefs);
    current(ctx, stored)
}

/// Autostart first: when the system refuses it, `ui.toml` is left as it was. When `ui.toml` then
/// fails to save, the autostart goes back to what it was, so an error never leaves the login entry
/// changed (it is the explicit-consent item, §12.5).
pub(crate) fn set(ctx: &Ctx, args: PrefsSetArgs) -> Result<PrefsReply, UiError> {
    let prefs = {
        let mut stored = lock(&ctx.prefs);
        let dir = ctx.opts.autostart_dir.as_deref();
        let was_on = autostart::is_enabled(dir);
        if let Some(on) = args.autostart {
            autostart::set(on, dir)
                .map_err(|why| UiError::new(code::IO, why).with_field("autostart"))?;
        }
        let mut next = *stored;
        if let Some(n) = args.notifications {
            next.notifications = n;
        }
        if let Some(a) = args.appearance {
            next.appearance = a;
        }
        if next != *stored {
            if let Err(error) = save(&ctx.profile.dir, &next) {
                return Err(undo_autostart(args.autostart, was_on, dir, error));
            }
            *stored = next;
        }
        current(ctx, *stored)
    };
    ctx.emit(UiEvent::PrefsChanged(PrefsChanged {
        prefs: prefs.clone(),
    }));
    Ok(PrefsReply { prefs })
}

/// Puts the autostart back after a failed save. `asked` is what `prefs_set` sent. If even that
/// fails the error says so, and the entry stays as the failed attempt left it.
fn undo_autostart(
    asked: Option<bool>,
    was_on: bool,
    dir: Option<&Path>,
    error: UiError,
) -> UiError {
    if asked.is_none() {
        return error;
    }
    match autostart::set(was_on, dir) {
        Ok(()) => error,
        Err(why) => {
            tracing::warn!("autostart could not be put back after a failed save");
            error.with_details(serde_json::json!({ "autostart_restored": false, "reason": why }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_round_trips_and_bad_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()), Stored::default());
        let s = Stored {
            notifications: Notifications {
                item_received: false,
                device_connected: true,
                block: false,
            },
            appearance: Appearance {
                theme: Theme::Dark,
                reduce_motion: true,
            },
            ..Stored::default()
        };
        save(dir.path(), &s).unwrap();
        assert_eq!(load(dir.path()), s);
        fs::write(dir.path().join(FILE), "version = 9\n").unwrap();
        assert_eq!(load(dir.path()), Stored::default());
        fs::write(dir.path().join(FILE), "não é toml").unwrap();
        assert_eq!(load(dir.path()), Stored::default());
    }
}
