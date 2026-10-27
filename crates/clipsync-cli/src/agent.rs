//! `clipsync run`: builds the enabled modules and hands them to the core agent.
use crate::fail::Fail;
use crate::{msg, text};
use clipsync_core::{CoreError, Profile, agent};
use clipsync_proto::Module;

fn module_err(e: clipsync_proto::ModuleError) -> Fail {
    Fail::error(e.to_string())
}

pub async fn run(profile: &Profile) -> Result<(), Fail> {
    let cfg = profile.load_config()?;
    // a disabled module is not built: no adapter loaded, no permission asked, not announced
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    if cfg.modules.clipboard {
        let c = clipsync_clipboard::ClipboardConfig::from_table(cfg.clipboard.as_ref())
            .map_err(module_err)?;
        modules.push(clipsync_clipboard::build(c, &profile.dir).map_err(module_err)?);
    }
    if cfg.modules.input {
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
