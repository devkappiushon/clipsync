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

fn default_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "clipsync".to_owned())
}

pub fn run<R: BufRead, W: Write>(profile: &Profile, args: InitArgs, ui: &mut Prompter<R, W>) -> Result<(), Fail> {
    let modules_arg = args.modules.as_deref().map(parse_modules).transpose()?;
    if profile.is_initialized() && !ui.yes_no(msg::ASK_RECREATE, false)? {
        return Err(Fail::refused(msg::INIT_NOT_CONFIRMED));
    }
    let name = match args.name {
        Some(n) => n,
        None => {
            ui.say(msg::INIT_INTRO)?;
            let fallback = default_name();
            ui.line(&format!("{} [{fallback}]:", msg::ASK_NAME))?.filter(|n| !n.is_empty()).unwrap_or(fallback)
        }
    };
    let modules = match modules_arg {
        Some(m) => m,
        // input is on by default; the clipboard question defaults to NO
        None => ModuleSet { input: true, clipboard: ui.yes_no(msg::ASK_CLIPBOARD, false)? },
    };
    let listen = SocketAddr::from((Ipv4Addr::UNSPECIFIED, args.port.unwrap_or(DEFAULT_PORT)));
    let identity = profile.init(InitOptions { device_name: name.clone(), modules, listen, fake_adapters: args.fake_adapters })?;
    ui.say(&msg::init_done(
        &profile.dir.display().to_string(),
        &identity.id.short(),
        &name,
        &listen.to_string(),
        modules.input,
        modules.clipboard,
        args.fake_adapters,
    ))?;
    Ok(())
}
