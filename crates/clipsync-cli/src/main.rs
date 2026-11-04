//! `clipsync` command line (contract §6). Exit codes: 0 ok, 1 error, 2 usage, 3 refused, 4 agent stopped.
mod agent;
mod cmds;
mod fail;
mod init;
mod msg;
mod pair;
mod prompt;
mod text;

use clap::{Parser, Subcommand};
use clipsync_core::{Profile, diag};
use fail::Fail;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "clipsync",
    version,
    about = "Teclado, mouse e clipboard entre os seus computadores"
)]
struct Cli {
    /// Pasta do perfil (padrão: $CLIPSYNC_PROFILE, depois ~/.config/clipsync)
    #[arg(long, global = true, value_name = "DIR")]
    profile: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Cria a identidade e a configuração deste dispositivo
    Init {
        #[arg(long)]
        name: Option<String>,
        /// Módulos ligados: input, clipboard ou input,clipboard
        #[arg(long)]
        modules: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        /// Adaptadores falsos, só para teste
        #[arg(long)]
        fake_adapters: bool,
    },
    /// Mostra o agente, os pares e os módulos
    Status,
    /// Localiza o arquivo de log deste perfil
    Log {
        #[command(subcommand)]
        what: LogKind,
    },
    /// Pareia este dispositivo com outro
    Pair {
        #[command(subcommand)]
        side: PairSide,
    },
    /// Sobe o agente em primeiro plano (Ctrl-C encerra)
    Run,
    /// Pausa o envio e a aplicação
    Pause,
    /// Retoma sem reenviar o que foi copiado na pausa
    Resume,
    /// Envia o texto da entrada padrão aos pares autorizados
    Send {
        #[arg(long, required = true)]
        stdin: bool,
        #[arg(long, value_name = "PAR")]
        to: Option<String>,
    },
    /// Lista a caixa de recebidos do clipboard
    Inbox,
    /// Verifica sistema, clipboard e input
    Diagnose,
    /// Provas técnicas
    Probe {
        #[command(subcommand)]
        what: ProbeKind,
    },
}

#[derive(Subcommand)]
enum LogKind {
    /// Mostra o caminho previsto, sem criar arquivo
    Caminho,
}

#[derive(Subcommand)]
enum PairSide {
    /// Aceita um pareamento por tempo limitado
    Host {
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long, default_value = "120s", value_parser = humantime::parse_duration)]
        timeout: Duration,
        /// clipboard-in,clipboard-out,input-in
        #[arg(long)]
        allow: Option<String>,
    },
    /// Entra no pareamento de um host
    Join {
        addr: String,
        #[arg(long)]
        allow: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProbeKind {
    #[cfg(target_os = "linux")]
    Clipboard(clipsync_clipboard::probe::ProbeArgs),
    /// Sem prova de clipboard fora do Linux; o comando existe só para dar a mensagem
    #[cfg(not(target_os = "linux"))]
    Clipboard,
    Input(clipsync_input::probe::ProbeArgs),
}

async fn diagnose(profile: &Profile) -> Result<(), Fail> {
    let status = match clipsync_core::control::request(
        profile,
        clipsync_proto::ControlRequest::Status,
    )
    .await
    {
        Ok(clipsync_proto::ControlResponse::Status(s)) => Some(s),
        _ => None,
    };
    let mut items = diag::system().await;
    items.extend(clipsync_clipboard::diagnose().await);
    items.extend(
        clipsync_input::diagnose_for(&profile.dir, status.as_ref().map(|s| s.id), status.as_ref())
            .await,
    );
    for i in items {
        println!(
            "{}",
            msg::diag_line(text::diag_status(i.status), &i.area, &i.name, &i.detail)
        );
    }
    Ok(())
}

async fn probe(profile: &Profile, what: ProbeKind) -> Result<(), Fail> {
    match what {
        #[cfg(target_os = "linux")]
        ProbeKind::Clipboard(args) => clipsync_clipboard::probe::run(args)
            .await
            .map_err(|e| Fail::error(e.to_string())),
        #[cfg(not(target_os = "linux"))]
        ProbeKind::Clipboard => Err(Fail::refused(msg::PROBE_CLIPBOARD_LINUX_ONLY)),
        ProbeKind::Input(args) if args.loopback => {
            let to = args
                .to
                .as_deref()
                .ok_or_else(|| Fail::error(msg::PROBE_PEER_REQUIRED))?;
            cmds::loopback(profile, to, args.seconds).await
        }
        ProbeKind::Input(args) => clipsync_input::probe::run(args).await.map_err(|e| {
            match e.downcast_ref::<clipsync_input::probe::Unavailable>() {
                Some(unavailable) => Fail::refused(unavailable.to_string()),
                None => Fail::error(e.to_string()),
            }
        }),
    }
}

async fn dispatch(cli: Cli) -> Result<(), Fail> {
    let profile = Profile::resolve(cli.profile)?;
    match cli.cmd {
        Cmd::Init {
            name,
            modules,
            port,
            fake_adapters,
        } => {
            let stdin = std::io::stdin();
            let mut ui = prompt::Prompter::new(stdin.lock(), std::io::stdout());
            init::run(
                &profile,
                init::InitArgs {
                    name,
                    modules,
                    port,
                    fake_adapters,
                },
                &mut ui,
            )
        }
        Cmd::Status => cmds::status(&profile).await,
        Cmd::Log {
            what: LogKind::Caminho,
        } => {
            println!(
                "{}",
                clipsync_core::logging::log_path(&profile.dir).display()
            );
            Ok(())
        }
        Cmd::Pair {
            side:
                PairSide::Host {
                    port,
                    timeout,
                    allow,
                },
        } => pair::host(&profile, port, timeout, allow).await,
        Cmd::Pair {
            side: PairSide::Join { addr, allow },
        } => pair::join(&profile, &addr, allow).await,
        Cmd::Run => agent::run(&profile).await,
        Cmd::Pause => cmds::pause(&profile, true).await,
        Cmd::Resume => cmds::pause(&profile, false).await,
        Cmd::Send { to, .. } => cmds::send(&profile, to).await,
        Cmd::Inbox => cmds::inbox(&profile).await,
        Cmd::Diagnose => diagnose(&profile).await,
        Cmd::Probe { what } => probe(&profile, what).await,
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse(); // usage errors exit 2 here
    clipsync_core::logging::init();
    match dispatch(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(f) => {
            if !f.message.is_empty() {
                eprintln!("{}: {}", msg::ERROR_PREFIX, f.message);
            }
            ExitCode::from(f.code)
        }
    }
}
