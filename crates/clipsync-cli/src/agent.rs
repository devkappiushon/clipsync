//! `clipsync run`: builds the enabled modules and hands them to the core agent.
use crate::fail::Fail;
use crate::{msg, text};
use clipsync_core::{CoreError, Profile, agent};
use clipsync_proto::{Module, ModuleSet};

/// What `run` builds: the configured modules minus what the platform cannot provide.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub clipboard: bool,
    pub input: bool,
    /// Configured but unavailable here: announced, not built.
    pub input_skipped: bool,
}

/// An agent without any module would serve nothing (contract §11 rule 8): that is an error.
pub fn plan(set: ModuleSet, input_supported: bool) -> Result<Plan, Fail> {
    let plan = Plan {
        clipboard: set.clipboard,
        input: set.input && input_supported,
        input_skipped: set.input && !input_supported,
    };
    if plan.clipboard || plan.input {
        Ok(plan)
    } else {
        Err(Fail::error(msg::RUN_NO_MODULE))
    }
}

fn module_err(e: clipsync_proto::ModuleError) -> Fail {
    Fail::error(e.to_string())
}

pub async fn run(profile: &Profile) -> Result<(), Fail> {
    let cfg = profile.load_config()?;
    let plan = plan(cfg.modules, clipsync_input::SUPPORTED)?;
    // a disabled module is not built: no adapter loaded, no permission asked, not announced
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    if plan.input_skipped {
        println!("{}", msg::INPUT_UNAVAILABLE);
    }
    if plan.clipboard {
        let c = clipsync_clipboard::ClipboardConfig::from_table(cfg.clipboard.as_ref())
            .map_err(module_err)?;
        modules.push(clipsync_clipboard::build(c, &profile.dir).map_err(module_err)?);
    }
    if plan.input {
        let c = clipsync_input::InputConfig::from_table(cfg.input.as_ref()).map_err(module_err)?;
        modules.push(clipsync_input::build(c, &profile.dir).map_err(module_err)?);
    }
    println!(
        "{}",
        msg::run_started(
            &cfg.device_name,
            &cfg.listen.to_string(),
            &text::modules(cfg.modules)
        )
    );
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    match agent::run(profile, modules, shutdown).await {
        Ok(()) => {
            println!("{}", msg::RUN_STOPPED);
            Ok(())
        }
        Err(CoreError::AgentAlreadyRunning) => Err(Fail::error(msg::ALREADY_RUNNING)),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(clipboard: bool, input: bool) -> ModuleSet {
        ModuleSet { clipboard, input }
    }

    #[test]
    fn plan_without_input_support_and_clipboard_off_is_an_error() {
        let e = plan(set(false, true), false).unwrap_err();
        assert_eq!((e.code, e.message.as_str()), (1, msg::RUN_NO_MODULE));
    }

    #[test]
    fn plan_without_input_support_skips_input_and_keeps_clipboard() {
        let p = plan(set(true, true), false).unwrap();
        assert_eq!(
            p,
            Plan {
                clipboard: true,
                input: false,
                input_skipped: true
            }
        );
    }

    #[test]
    fn plan_with_input_support_builds_what_is_enabled() {
        let p = plan(set(false, true), true).unwrap();
        assert_eq!(
            p,
            Plan {
                clipboard: false,
                input: true,
                input_skipped: false
            }
        );
        assert!(plan(set(false, false), true).is_err());
    }

    #[test]
    fn plan_for_this_platform_matches_its_support() {
        assert_eq!(
            plan(set(false, true), clipsync_input::SUPPORTED).is_ok(),
            clipsync_input::SUPPORTED
        );
    }
}
