//! `clipsync pair host|join`.
use crate::fail::Fail;
use crate::prompt::Prompter;
use crate::{msg, text};
use clipsync_core::pairing::{self, PairDecision, PairOutcome, PairPrompt};
use clipsync_core::{CoreError, Profile, control};
use clipsync_proto::{ControlRequest, ModuleSet, PeerPerms};
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::time::Duration;

/// `--allow clipboard-in,clipboard-out,input-in`: answers the permission questions in advance.
pub fn parse_allow(list: &str) -> Result<PeerPerms, Fail> {
    let mut p = PeerPerms::default();
    for part in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match part {
            "clipboard-in" => p.clipboard_in = true,
            "clipboard-out" => p.clipboard_out = true,
            "input-in" => p.input_in = true,
            _ => return Err(Fail::error(msg::BAD_ALLOW)),
        }
    }
    Ok(p)
}

/// The decision flow, over any reader/writer. The CODE is always confirmed by the user; permissions
/// are asked only for the modules this device runs, unless `--allow` already answered them.
pub fn decide<R: BufRead, W: Write>(
    ui: &mut Prompter<R, W>,
    prompt: &PairPrompt,
    local: ModuleSet,
    allow: Option<PeerPerms>,
) -> std::io::Result<PairDecision> {
    ui.say(&msg::pair_peer(
        &text::clean(&prompt.peer_name),
        &prompt.peer_id.short(),
        &text::modules(prompt.peer_modules),
    ))?;
    ui.say(&msg::pair_code(&prompt.code))?;
    let declined = PairDecision {
        accept: false,
        grants: PeerPerms::default(),
    };
    if !ui.yes_no(msg::ASK_CODE, false)? {
        return Ok(declined);
    }
    let grants = match allow {
        Some(p) => p,
        None => PeerPerms {
            clipboard_in: local.clipboard && ui.yes_no(msg::ASK_CLIPBOARD_IN, false)?,
            clipboard_out: local.clipboard && ui.yes_no(msg::ASK_CLIPBOARD_OUT, false)?,
            input_in: local.input && ui.yes_no(msg::ASK_INPUT_IN, false)?,
        },
    };
    Ok(PairDecision {
        accept: true,
        grants,
    })
}

fn confirm_fn(
    local: ModuleSet,
    allow: Option<PeerPerms>,
) -> impl FnOnce(&PairPrompt) -> PairDecision + Send + 'static {
    move |prompt| {
        let stdin = std::io::stdin();
        let mut ui = Prompter::new(stdin.lock(), std::io::stdout());
        // a broken stdin is a "no": nothing is granted without an explicit answer
        decide(&mut ui, prompt, local, allow).unwrap_or(PairDecision {
            accept: false,
            grants: PeerPerms::default(),
        })
    }
}

async fn finish(profile: &Profile, outcome: Result<PairOutcome, CoreError>) -> Result<(), Fail> {
    match outcome {
        Ok(PairOutcome::Paired(entry)) => {
            println!(
                "{}",
                msg::paired(
                    &text::clean(&entry.name),
                    &entry.id.short(),
                    &text::perms(entry.perms)
                )
            );
            // a running agent reads peers.toml again; no agent is fine
            let _ = control::request(profile, ControlRequest::ReloadPeers).await;
            Ok(())
        }
        Ok(PairOutcome::Declined { by_peer }) => Err(Fail::refused(if by_peer {
            msg::PAIR_DECLINED_PEER
        } else {
            msg::PAIR_DECLINED_LOCAL
        })),
        Err(CoreError::Pairing(d)) => Err(Fail::refused(msg::pair_failed(&text::clean(&d)))),
        Err(e) => Err(e.into()),
    }
}

pub async fn host(
    profile: &Profile,
    port: u16,
    window: Duration,
    allow: Option<String>,
) -> Result<(), Fail> {
    let allow = allow.as_deref().map(parse_allow).transpose()?;
    let local = profile.load_config()?.modules;
    let secs = window.as_secs();
    let outcome = pairing::host(
        profile,
        port,
        window,
        move |addr| println!("{}", msg::pair_waiting(&addr.to_string(), secs)),
        confirm_fn(local, allow),
    )
    .await;
    finish(profile, outcome).await
}

pub async fn join(profile: &Profile, addr: &str, allow: Option<String>) -> Result<(), Fail> {
    let addr: SocketAddr = addr.parse().map_err(|_| Fail::error(msg::BAD_ADDR))?;
    let allow = allow.as_deref().map(parse_allow).transpose()?;
    let local = profile.load_config()?.modules;
    let outcome = pairing::join(profile, addr, confirm_fn(local, allow)).await;
    finish(profile, outcome).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipsync_proto::PeerId;

    #[test]
    fn hostile_name_is_cleaned_and_the_code_comes_right_before_the_question() {
        let prompt = PairPrompt {
            code: "123 456".into(),
            peer_name: "evil\u{1b}[2J\nCódigo: 000 000".into(),
            peer_id: PeerId([9; 32]),
            peer_modules: ModuleSet {
                clipboard: true,
                input: false,
            },
        };
        let mut out = Vec::new();
        let decision = {
            let mut ui = Prompter::new(&b"n\n"[..], &mut out);
            decide(&mut ui, &prompt, ModuleSet::default(), None).unwrap()
        };
        assert!(!decision.accept);
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains('\u{1b}'), "{text:?}");
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[0].starts_with("Par: evil?[2J Código: 000 000"),
            "{lines:?}"
        );
        assert_eq!(lines[1], "Código: 123 456");
        assert!(
            lines[2].contains("[s/N]"),
            "the question follows the code: {lines:?}"
        );
    }
}
