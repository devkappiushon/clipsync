//! `clipsync probe input`: capture and emulation checks on the real adapters. Never prints keys or key codes.
use crate::msg;
use clipsync_proto::LoopbackStats;

#[derive(clap::Args, Debug)]
pub struct ProbeArgs {
    #[arg(long)]
    pub capture: bool,
    #[arg(long)]
    pub emulate: bool,
    #[arg(long)]
    pub loopback: bool,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub seconds: u32,
    #[arg(long)]
    pub yes: bool,
}

/// The platform has no capture/emulation adapter; the CLI maps this to exit code 3.
#[derive(thiserror::Error, Debug)]
#[error("{}", msg::UNAVAILABLE_ON_PLATFORM)]
pub struct Unavailable;

pub async fn run(args: ProbeArgs) -> anyhow::Result<()> {
    if args.loopback && !args.capture && !args.emulate {
        anyhow::bail!(msg::LOOPBACK_IS_CLI);
    }
    if !args.capture && !args.emulate {
        anyhow::bail!(msg::NOTHING_TO_DO);
    }
    if args.capture {
        capture(args.seconds.clamp(1, 600)).await?;
    }
    if args.emulate {
        emulate(args.yes).await?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", windows))]
mod run;

#[cfg(not(any(target_os = "linux", windows)))]
async fn capture(_: u32) -> anyhow::Result<()> {
    Err(Unavailable.into())
}

#[cfg(not(any(target_os = "linux", windows)))]
async fn emulate(_: bool) -> anyhow::Result<()> {
    Err(Unavailable.into())
}

#[cfg(any(target_os = "linux", windows))]
use run::{capture, emulate};

/// Text for the loopback result, in pt-BR.
pub fn print_loopback(stats: &LoopbackStats) -> String {
    let ms = |us: u64| us as f64 / 1000.0;
    format!(
        "eventos recebidos: {} · datagramas perdidos: {} · fora de ordem: {} · p50: {:.2} ms · p95: {:.2} ms · máx: {:.2} ms",
        stats.received,
        stats.lost,
        stats.out_of_order,
        ms(stats.p50_us),
        ms(stats.p95_us),
        ms(stats.max_us)
    )
}
