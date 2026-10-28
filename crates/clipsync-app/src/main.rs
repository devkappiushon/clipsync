//! ClipSync desktop: casca Tauri 2 com o agente no mesmo processo (contrato §12.7). Fina de
//! propósito: a lógica das telas fica no `clipsync-ui`; aqui só janela, bandeja, eventos,
//! notificação e sinais. Só compila no CI (esta máquina não tem os headers do WebKitGTK).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod events;
mod lifecycle;
mod msg;
mod notify_prefs;
mod notify_text;
mod signals;
mod state;
mod tray;
mod window;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, RunEvent, WindowEvent};

// A superfície da §12.3 (`Ui`, `UiOptions`, `UiError`, `UiEvent`) vem do crate do Rivet; os
// módulos da casca a alcançam por `crate::ui`.
use clipsync_ui as ui;

use crate::state::AppState;
use crate::ui::UiOptions;

/// Linha de comando: `--profile DIR` (como a CLI) e `--hidden` (autostart, §12.7 regra 6).
#[derive(Debug, PartialEq, Eq, Default)]
struct Args {
    profile: Option<PathBuf>,
    hidden: bool,
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Args {
    let mut parsed = Args::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            parsed.profile = args.next().map(PathBuf::from);
        } else if arg == "--hidden" {
            parsed.hidden = true;
        }
    }
    parsed
}

/// Opções do app real (§12.3): consulta de 1 s, adaptadores reais, caminhos padrão da plataforma.
fn ui_options(profile: Option<PathBuf>) -> UiOptions {
    UiOptions {
        profile,
        poll: Duration::from_secs(1),
        listen: None,
        fake_adapters: false,
        autostart_dir: None,
        downloads_dir: None,
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_env("CLIPSYNC_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

fn main() {
    init_logging();
    let args = parse_args(std::env::args_os().skip(1));
    let options = ui_options(args.profile);

    let built = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::new(args.hidden))
        .invoke_handler(tauri::generate_handler![commands::clipsync])
        // Fechar só esconde; "Sair" (ou um sinal) encerra.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        // A janela nasce invisível e aparece quando o front terminou de carregar.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                window::reveal(webview.app_handle());
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            window::create(&handle)?;
            tray::build(&handle)?;
            signals::spawn(handle.clone());
            tauri::async_runtime::spawn(lifecycle::boot(handle, options));
            Ok(())
        })
        .build(tauri::generate_context!());

    let app = match built {
        Ok(app) => app,
        Err(error) => {
            eprintln!("{}: {error}", msg::APP_FAILED);
            std::process::exit(1);
        }
    };

    app.run(|_app, event| {
        // Sem código = pedido do sistema ou da última janela, que aqui só se esconde: o app segue
        // na bandeja. `lifecycle::quit` sai com `Some(0)` depois do Bye.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Args {
        parse_args(list.iter().map(|s| OsString::from(*s)))
    }

    #[test]
    fn parses_profile_and_hidden_in_any_order() {
        assert_eq!(args(&[]), Args::default());
        assert_eq!(
            args(&["--hidden", "--profile", "/p"]),
            Args {
                profile: Some(PathBuf::from("/p")),
                hidden: true
            }
        );
        assert_eq!(
            args(&["--profile", "/p"]),
            Args {
                profile: Some(PathBuf::from("/p")),
                hidden: false
            }
        );
    }

    #[test]
    fn unknown_flags_are_ignored() {
        assert!(args(&["--verbose", "--hidden"]).hidden);
        assert_eq!(args(&["--profile"]).profile, None);
    }

    #[test]
    fn app_options_follow_the_contract_defaults() {
        let options = ui_options(None);
        assert_eq!(options.poll, Duration::from_secs(1));
        assert!(!options.fake_adapters);
        assert!(options.listen.is_none() && options.autostart_dir.is_none());
    }
}
