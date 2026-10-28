//! 1 · Onboarding (§12.5): `onboarding_start`, `onboarding_check_name`, `onboarding_create` and
//! `onboarding_self_test`. Owned by Rivet.
//!
//! The identity is made in memory by `onboarding_start`, so step 1 shows its fingerprint before
//! anything is written; `onboarding_create` writes that same identity (`Profile::init_with`).

use std::net::{Ipv4Addr, SocketAddr};

use clipsync_clipboard::{AdapterChoice, ClipboardConfig, SelfTestOutcome};
use clipsync_core::{Identity, InitOptions};
use clipsync_proto::{ControlRequest, ControlResponse, ModuleSet};
use serde::{Deserialize, Serialize};

use crate::agent::{self, INPUT_SUPPORTED};
use crate::api::{OnboardingCreateArgs, code};
use crate::status::{DeviceSelf, ModuleName, Modules};
use crate::{Ctx, UiError, clean, lock, msg, status};

/// When the hostname is missing or is not a valid device name.
const FALLBACK_NAME: &str = "clipsync";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OnboardingStart {
    /// The hostname, when it is a valid device name.
    pub default_name: String,
    /// Of the identity made now; `onboarding_create` writes this same one.
    pub fingerprint_short: String,
    pub input_supported: bool,
    pub already_initialized: bool,
    /// Step 2: "Teclado e mouse" on where it works, "Área de transferência" off.
    pub defaults: Modules,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NameOk {
    pub ok: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Created {
    pub device: DeviceSelf,
    /// "IP:PORT" the agent listens on.
    pub listen: String,
    pub modules: Modules,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Validated,
    Experimental,
    Unavailable,
    NotTested,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TestResult {
    pub module: ModuleName,
    pub status: TestStatus,
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelfTest {
    pub results: Vec<TestResult>,
}

/// The CLI's default (`clipsync init`), kept only when it passes the device-name rule.
fn default_name() -> String {
    #[cfg(windows)]
    let host = std::env::var("COMPUTERNAME").ok();
    #[cfg(not(windows))]
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname").ok();
    host.map(|h| h.trim().to_owned())
        .filter(|h| clipsync_proto::validate_device_name(h).is_ok())
        .unwrap_or_else(|| FALLBACK_NAME.to_owned())
}

fn check(name: &str) -> Result<(), UiError> {
    clipsync_proto::validate_device_name(name)
        .map_err(|_| UiError::new(code::INVALID_NAME, msg::INVALID_NAME).with_field("name"))
}

/// Repeated calls keep the same identity until `onboarding_create` writes it.
pub(crate) fn start(ctx: &Ctx) -> Result<OnboardingStart, UiError> {
    let mut pending = lock(&ctx.pending_identity);
    let id = match &*pending {
        Some(i) => i.id(),
        None => {
            let i = Identity::generate()?;
            let id = i.id();
            *pending = Some(i);
            id
        }
    };
    Ok(OnboardingStart {
        default_name: default_name(),
        fingerprint_short: id.short(),
        input_supported: INPUT_SUPPORTED,
        already_initialized: ctx.profile.is_initialized(),
        defaults: Modules {
            input: INPUT_SUPPORTED,
            clipboard: false,
        },
    })
}

pub(crate) fn check_name(name: &str) -> Result<NameOk, UiError> {
    check(name).map(|()| NameOk { ok: true })
}

/// Writes the identity of `onboarding_start` and starts the agent. An existing profile asks for
/// `recreate: true` first (`needs_confirmation`): a new identity means the peers pair again.
pub(crate) async fn create(ctx: &Ctx, args: OnboardingCreateArgs) -> Result<Created, UiError> {
    check(&args.name)?;
    let set = ModuleSet {
        clipboard: args.modules.clipboard,
        input: args.modules.input,
    };
    if set.is_empty() {
        return Err(UiError::new(code::NO_MODULE, msg::NO_MODULE).with_field("modules"));
    }
    agent::plan(set, INPUT_SUPPORTED)?;
    if args.port == 0 && ctx.opts.listen.is_none() {
        return Err(UiError::bad_request("port: 1..65535").with_field("port"));
    }
    if ctx.profile.is_initialized() && !args.recreate {
        return Err(UiError::new(
            code::NEEDS_CONFIRMATION,
            msg::RECREATE_CONFIRM,
        ));
    }
    // The running agent holds the old identity: an in_app one stops, an external one is not
    // ours (`agent_external`).
    agent::stop(ctx).await?;

    let taken = lock(&ctx.pending_identity).take();
    let identity = match taken {
        Some(i) => i,
        None => Identity::generate()?,
    };
    let listen = ctx
        .opts
        .listen
        .unwrap_or_else(|| SocketAddr::from((Ipv4Addr::UNSPECIFIED, args.port)));
    let kept = identity.clone();
    let written = ctx.profile.init_with(
        InitOptions {
            device_name: args.name.clone(),
            modules: set,
            listen,
            fake_adapters: ctx.opts.fake_adapters,
        },
        identity,
    );
    let identity = match written {
        Ok(i) => i,
        Err(e) => {
            // The fingerprint on screen stays valid for the next try.
            *lock(&ctx.pending_identity) = Some(kept);
            return Err(e.into());
        }
    };
    // The profile exists now; a failed start shows in app_state and agent_state.
    if let Err(e) = agent::start(ctx).await {
        tracing::warn!(code = %e.code, "{}", e.message);
    }
    Ok(Created {
        device: DeviceSelf::new(identity.id(), &args.name),
        listen: listen.to_string(),
        modules: set.into(),
    })
}

/// A module that was not called says why.
fn untested(module: ModuleName, enabled: bool, supported: bool) -> TestResult {
    let (status, detail) = match (supported, enabled) {
        (false, _) => (TestStatus::Unavailable, msg::SELF_TEST_UNSUPPORTED),
        (true, false) => (TestStatus::NotTested, msg::SELF_TEST_MODULE_OFF),
        (true, true) => (TestStatus::NotTested, msg::SELF_TEST_MISSING),
    };
    TestResult {
        module,
        status,
        detail: Some(detail.to_string()),
    }
}

fn tested(module: ModuleName, outcome: SelfTestOutcome) -> TestResult {
    let (status, detail) = match &outcome {
        SelfTestOutcome::Validated(d) => (TestStatus::Validated, d),
        SelfTestOutcome::Experimental(d) => (TestStatus::Experimental, d),
        SelfTestOutcome::Unavailable(d) => (TestStatus::Unavailable, d),
    };
    TestResult {
        module,
        status,
        detail: Some(clean(detail)),
    }
}

/// The test opens its own adapter, so a running agent would see the synthetic text and offer
/// it to the peers: the agent is paused first (fails closed) and resumed after. `true` = we
/// paused it.
async fn pause_for_test(ctx: &Ctx) -> Result<bool, UiError> {
    let Some(s) = status::agent_status(ctx).await else {
        return Ok(false);
    };
    if s.paused {
        return Ok(false);
    }
    match ctx.request(ControlRequest::Pause).await? {
        ControlResponse::Ok => Ok(true),
        // Paused in memory, only state.toml failed: still safe to test.
        ControlResponse::Error { message } if message == clipsync_core::msg::STATE_NOT_SAVED => {
            Ok(true)
        }
        ControlResponse::Error { message } => Err(UiError::internal(&clean(&message))),
        _ => Err(UiError::internal(msg::UNEXPECTED_REPLY)),
    }
}

/// Clipboard: `clipsync_clipboard::self_test` (a fake one when the profile uses fake adapters).
/// Input: not called until its `self_test` exists (Vector, L7).
pub(crate) async fn self_test(ctx: &Ctx) -> Result<SelfTest, UiError> {
    ctx.require_profile()?;
    let cfg = ctx.profile.load_config()?;
    let clipboard = if cfg.modules.clipboard {
        let fake = ClipboardConfig::from_table(cfg.clipboard.as_ref())
            .is_ok_and(|c| c.adapter == AdapterChoice::Fake);
        let paused = pause_for_test(ctx).await?;
        let outcome = if fake {
            tokio::task::spawn_blocking(|| {
                let (mut adapter, _handle) = clipsync_clipboard::FakeClipboard::new();
                clipsync_clipboard::self_test_with(&mut adapter)
            })
            .await
            .unwrap_or_else(|e| SelfTestOutcome::Unavailable(e.to_string()))
        } else {
            clipsync_clipboard::self_test().await
        };
        if paused && let Err(e) = ctx.request(ControlRequest::Resume).await {
            tracing::warn!(code = %e.code, "agent not resumed after the self test");
        }
        tested(ModuleName::Clipboard, outcome)
    } else {
        untested(ModuleName::Clipboard, false, true)
    };
    Ok(SelfTest {
        results: vec![
            clipboard,
            untested(ModuleName::Input, cfg.modules.input, INPUT_SUPPORTED),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_name_is_always_a_valid_device_name() {
        assert!(clipsync_proto::validate_device_name(&default_name()).is_ok());
    }

    #[test]
    fn names_follow_the_device_name_rule() {
        assert!(check_name("pc-pessoal").is_ok());
        for bad in ["", "a\u{1b}b", "a\u{202e}b", &"x".repeat(65)] {
            let e = check_name(bad).unwrap_err();
            assert_eq!(
                (e.code.as_str(), e.field.as_deref()),
                (code::INVALID_NAME, Some("name"))
            );
        }
    }
}
