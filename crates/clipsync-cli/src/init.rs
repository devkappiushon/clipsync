//! `clipsync init`.
use crate::fail::Fail;
use crate::msg;
use crate::prompt::Prompter;
use clipsync_core::{InitOptions, Profile};
use clipsync_proto::ModuleSet;
use std::io::{BufRead, Write};
use std::net::{Ipv4Addr, SocketAddr};

pub const DEFAULT_PORT: u16 = 47800;

pub struct InitArgs {
    pub name: Option<String>,
    /// `Some("input,clipboard")` skips the module questions.
    pub modules: Option<String>,
    pub port: Option<u16>,
    pub fake_adapters: bool,
}

fn parse_modules(list: &str) -> Result<ModuleSet, Fail> {
    let mut set = ModuleSet::default();
    for part in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part {
            "input" => set.input = true,
            "clipboard" => set.clipboard = true,
            _ => return Err(Fail::error(msg::BAD_MODULE)),
        }
    }
    if set.is_empty() {
        return Err(Fail::error(msg::NO_MODULE));
    }
    Ok(set)
}

/// A profile whose every module is unavailable on this platform would never run (contract §11 rule 8).
fn check_usable(set: ModuleSet, input_supported: bool) -> Result<(), Fail> {
    if set.clipboard || (set.input && input_supported) {
        Ok(())
    } else {
        Err(Fail::refused(msg::INIT_NO_USABLE_MODULE))
    }
}

fn default_name() -> String {
    #[cfg(windows)]
    let host = std::env::var("COMPUTERNAME").ok();
    #[cfg(not(windows))]
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname").ok();
    host.map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "clipsync".to_owned())
}

pub fn run<R: BufRead, W: Write>(
    profile: &Profile,
    args: InitArgs,
    ui: &mut Prompter<R, W>,
) -> Result<(), Fail> {
    let modules_arg = args.modules.as_deref().map(parse_modules).transpose()?;
    if profile.is_initialized() && !ui.yes_no(msg::ASK_RECREATE, false)? {
        return Err(Fail::refused(msg::INIT_NOT_CONFIRMED));
    }
    let name = match args.name {
        Some(n) => n,
        None => {
            ui.say(msg::INIT_INTRO)?;
            let fallback = default_name();
            ui.line(&format!("{} [{fallback}]:", msg::ASK_NAME))?
                .filter(|n| !n.is_empty())
                .unwrap_or(fallback)
        }
    };
    let modules = match modules_arg {
        Some(m) => m,
        // input is on by default where an adapter exists (Linux); the clipboard question defaults to NO
        None => ModuleSet {
            input: clipsync_input::SUPPORTED,
            clipboard: ui.yes_no(msg::ASK_CLIPBOARD, false)?,
        },
    };
    check_usable(modules, clipsync_input::SUPPORTED)?;
    let listen = SocketAddr::from((Ipv4Addr::UNSPECIFIED, args.port.unwrap_or(DEFAULT_PORT)));
    let identity = profile.init(InitOptions {
        device_name: name.clone(),
        modules,
        listen,
        fake_adapters: args.fake_adapters,
    })?;
    ui.say(&msg::init_done(
        &profile.dir.display().to_string(),
        &identity.id().short(),
        &name,
        &listen.to_string(),
        modules.input,
        modules.clipboard,
        args.fake_adapters,
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(clipboard: bool, input: bool) -> ModuleSet {
        ModuleSet { clipboard, input }
    }

    #[test]
    fn init_without_input_support_refuses_clipboard_off() {
        for input in [false, true] {
            let e = check_usable(set(false, input), false).unwrap_err();
            assert_eq!(
                (e.code, e.message.as_str()),
                (3, msg::INIT_NO_USABLE_MODULE)
            );
        }
    }

    #[test]
    fn init_without_input_support_accepts_clipboard() {
        assert!(check_usable(set(true, false), false).is_ok());
        assert!(check_usable(set(true, true), false).is_ok());
    }

    #[test]
    fn init_with_input_support_accepts_input_alone_but_not_nothing() {
        assert!(check_usable(set(false, true), true).is_ok());
        assert!(check_usable(set(false, false), true).is_err());
    }
}
