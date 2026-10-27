//! Commands that talk to a running agent through the control socket.
use crate::fail::Fail;
use crate::{msg, text};
use clipsync_core::{CoreError, Profile, control};
use clipsync_proto::{
    ControlRequest, ControlResponse, InboxItem, MAX_CLIP_TEXT_BYTES, ModuleCommand, ModuleKind,
    ModuleReply, PeerId,
};
use std::io::Read;

async fn ask(profile: &Profile, req: ControlRequest) -> Result<ControlResponse, Fail> {
    match control::request(profile, req).await {
        Ok(ControlResponse::Error { message }) => Err(Fail::error(message)),
        Ok(other) => Ok(other),
        Err(CoreError::AgentNotRunning) => Err(Fail::not_running()),
        Err(e) => Err(e.into()),
    }
}

async fn ask_module(
    profile: &Profile,
    kind: ModuleKind,
    cmd: ModuleCommand,
) -> Result<ModuleReply, Fail> {
    match ask(profile, ControlRequest::Module { kind, cmd }).await? {
        ControlResponse::Module(ModuleReply::Error(m)) => Err(Fail::error(m)),
        ControlResponse::Module(r) => Ok(r),
        other => Err(Fail::error(msg::unexpected_reply(&format!("{other:?}")))),
    }
}

fn resolve_peer(profile: &Profile, key: &str) -> Result<PeerId, Fail> {
    let peers = profile.load_peers()?;
    peers
        .find(key)
        .map(|p| p.id)
        .ok_or_else(|| Fail::error(msg::UNKNOWN_PEER))
}

pub async fn status(profile: &Profile) -> Result<(), Fail> {
    match control::request(profile, ControlRequest::Status).await {
        Ok(ControlResponse::Status(s)) => {
            println!(
                "{}",
                msg::status_head(
                    &text::clean(&s.device_name),
                    &s.id.short(),
                    &s.listen.to_string(),
                    s.paused
                )
            );
            println!("{}", msg::PEERS_HEADER);
            if s.peers.is_empty() {
                println!("  {}", msg::NO_PEERS);
            }
            for p in &s.peers {
                let addr = p.addr.map_or_else(|| "?".to_owned(), |a| a.to_string());
                println!(
                    "{}",
                    msg::peer_line(
                        &text::clean(&p.name),
                        &p.id.short(),
                        &text::peer_state(&p.state),
                        &addr,
                        &text::modules(p.remote_modules),
                        &text::perms(p.perms)
                    )
                );
            }
            println!("{}", msg::MODULES_HEADER);
            for m in &s.modules {
                println!(
                    "{}",
                    msg::module_line(
                        text::kind(m.kind),
                        m.enabled,
                        m.adapter.as_deref().unwrap_or("-"),
                        &text::clean(m.last_event.as_deref().unwrap_or(msg::NONE)),
                        &text::clean(m.last_block.as_deref().unwrap_or(msg::NONE))
                    )
                );
            }
            Ok(())
        }
        Ok(ControlResponse::Error { message }) => Err(Fail::error(message)),
        Ok(other) => Err(Fail::error(msg::unexpected_reply(&format!("{other:?}")))),
        Err(CoreError::AgentNotRunning) => {
            // not running: say what the profile holds, then exit 4
            let cfg = profile
                .load_config()
                .map_err(|_| Fail::error(msg::NOT_INITIALIZED))?;
            println!(
                "{}",
                msg::stopped_head(
                    &cfg.device_name,
                    &cfg.listen.to_string(),
                    cfg.modules.input,
                    cfg.modules.clipboard
                )
            );
            let peers = profile.load_peers()?;
            println!("{}", msg::PEERS_HEADER);
            if peers.peer.is_empty() {
                println!("  {}", msg::NO_PEERS);
            }
            for p in &peers.peer {
                println!(
                    "  {} ({}) · {}",
                    text::clean(&p.name),
                    p.id.short(),
                    text::perms(p.perms)
                );
            }
            Err(Fail {
                code: 4,
                message: String::new(),
            })
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn pause(profile: &Profile, pause: bool) -> Result<(), Fail> {
    let req = if pause {
        ControlRequest::Pause
    } else {
        ControlRequest::Resume
    };
    ask(profile, req).await?;
    println!("{}", if pause { msg::PAUSED } else { msg::RESUMED });
    Ok(())
}

/// Reads at most the clipboard limit + 1 bytes so an oversized input is detected without slurping it all.
fn read_stdin_text() -> Result<String, Fail> {
    let mut buf = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_CLIP_TEXT_BYTES as u64 + 1)
        .read_to_end(&mut buf)?;
    if buf.len() > MAX_CLIP_TEXT_BYTES {
        return Err(Fail::error(msg::STDIN_TOO_BIG));
    }
    if buf.is_empty() {
        return Err(Fail::error(msg::EMPTY_STDIN));
    }
    String::from_utf8(buf).map_err(|_| Fail::error(msg::STDIN_NOT_UTF8))
}

pub async fn send(profile: &Profile, to: Option<String>) -> Result<(), Fail> {
    let to = to.map(|k| resolve_peer(profile, &k)).transpose()?;
    let body = read_stdin_text()?;
    match ask_module(
        profile,
        ModuleKind::Clipboard,
        ModuleCommand::ClipSend { text: body, to },
    )
    .await?
    {
        ModuleReply::ClipSent(outcomes) => {
            let mut all_delivered = true;
            for o in &outcomes {
                println!("{}", text::delivery(&o.peer, o.state));
                all_delivered &= text::delivered(o.state);
            }
            if all_delivered {
                Ok(())
            } else {
                Err(Fail {
                    code: 3,
                    message: String::new(),
                })
            }
        }
        other => Err(Fail::error(msg::unexpected_reply(&format!("{other:?}")))),
    }
}

pub async fn inbox(profile: &Profile) -> Result<(), Fail> {
    match ask_module(profile, ModuleKind::Clipboard, ModuleCommand::ClipInbox).await? {
        ModuleReply::ClipInbox(items) => {
            if items.is_empty() {
                println!("{}", msg::INBOX_EMPTY);
            }
            for i in &items {
                println!("{}", inbox_line(i));
            }
            Ok(())
        }
        other => Err(Fail::error(msg::unexpected_reply(&format!("{other:?}")))),
    }
}

fn inbox_line(i: &InboxItem) -> String {
    let from = text::clean(&i.from);
    let state = text::delivery(&from, i.state);
    let state = state
        .strip_prefix(&format!("{from}: "))
        .unwrap_or(&state)
        .to_owned();
    msg::inbox_line(
        &i.id.short(),
        &from,
        &text::clock_utc(i.received_unix_ms),
        i.bytes,
        &state,
        &text::clean(&i.preview),
    )
}

pub async fn loopback(profile: &Profile, to: &str, seconds: u32) -> Result<(), Fail> {
    let to = resolve_peer(profile, to)?;
    match ask_module(
        profile,
        ModuleKind::Input,
        ModuleCommand::InputLoopback { to, seconds },
    )
    .await?
    {
        ModuleReply::InputLoopback(stats) => {
            println!("{}", clipsync_input::probe::print_loopback(&stats));
            Ok(())
        }
        other => Err(Fail::error(msg::unexpected_reply(&format!("{other:?}")))),
    }
}
