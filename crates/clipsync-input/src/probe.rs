//! `clipsync probe input`: portal capture and emulation checks. Never prints keys or key codes.
use crate::adapter::{
    BarrierSpec, CaptureAdapter, CaptureEvent, EmulateAdapter, ZoneRef, barrier_segment,
};
#[cfg(target_os = "linux")]
use crate::adapter::{PortalCapture, PortalEmulate, TokenStore};
use crate::msg;
use clipsync_proto::{Edge, InputEvent, LoopbackStats, mono_us};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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

const KEY_ESC: u32 = 1;
// ABNT2 evdev codes for "ção": ç, dead tilde, a, o.
const TYPE_ABNT2: [u32; 5] = [39, 40, 30, 24, 0];

fn profile_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("CLIPSYNC_PROFILE") {
        return PathBuf::from(p);
    }
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(x).join("clipsync");
    }
    std::env::var_os("HOME").map_or_else(
        || PathBuf::from(".clipsync"),
        |h| PathBuf::from(h).join(".config/clipsync"),
    )
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

#[derive(Default)]
struct Counts {
    motion: u64,
    button: u64,
    scroll: u64,
    key: u64,
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[(sorted.len() * p / 100).min(sorted.len() - 1)]
    }
}

#[cfg(not(target_os = "linux"))]
async fn capture(_: u32) -> anyhow::Result<()> {
    Err(Unavailable.into())
}

#[cfg(not(target_os = "linux"))]
async fn emulate(_: bool) -> anyhow::Result<()> {
    Err(Unavailable.into())
}

#[cfg(target_os = "linux")]
async fn capture(seconds: u32) -> anyhow::Result<()> {
    let mut cap = PortalCapture::new();
    let zones = cap.zones().await?;
    if zones.is_empty() {
        anyhow::bail!(msg::NO_ZONE);
    }
    for z in &zones {
        println!(
            "{}",
            msg::zone_line(
                z.index,
                z.name.as_deref().unwrap_or("(sem nome)"),
                z.width,
                z.height,
                z.x,
                z.y
            )
        );
    }
    let seg = barrier_segment(&zones[0], Edge::Right, 0.0, 0.5);
    println!("{}", msg::barrier_line(seg.x1, seg.y1, seg.x2, seg.y2));
    let spec = BarrierSpec {
        zone: ZoneRef::Index(zones[0].index),
        edge: Edge::Right,
        from: 0.0,
        to: 0.5,
    };
    let mut rx = cap.start(&[spec]).await?;
    println!("{} ({seconds} s)", msg::CAPTURE_HINT);

    let deadline = Instant::now() + Duration::from_secs(u64::from(seconds));
    let mut counts = Counts::default();
    let mut lat: Vec<u64> = Vec::new();
    let mut activated_at: Option<Instant> = None;
    let mut first_event: Option<Duration> = None;
    loop {
        let ev = tokio::select! {
            e = rx.recv() => match e { Some(e) => e, None => break },
            () = tokio::time::sleep_until(deadline.into()) => break,
        };
        match ev {
            CaptureEvent::Activated { barrier, pos, .. } => {
                activated_at = Some(Instant::now());
                println!("Activated: barreira {barrier}, posição {pos:.2}");
            }
            CaptureEvent::Input { t_mono_us, ev } => {
                lat.push(mono_us().saturating_sub(t_mono_us));
                if first_event.is_none() {
                    first_event = activated_at.map(|a| a.elapsed());
                }
                match ev {
                    InputEvent::Motion { .. } => counts.motion += 1,
                    InputEvent::Button { .. } => counts.button += 1,
                    InputEvent::Scroll { .. } | InputEvent::ScrollDiscrete { .. } => {
                        counts.scroll += 1;
                    }
                    InputEvent::Key { code, pressed } => {
                        counts.key += 1;
                        // Esc releases; the code is compared, never printed.
                        if code == KEY_ESC && pressed {
                            break;
                        }
                    }
                }
            }
            CaptureEvent::Deactivated => println!("Deactivated"),
            CaptureEvent::Error(e) => {
                println!("erro do adaptador: {e}");
                break;
            }
        }
    }
    let released = cap.release().await;
    cap.stop().await?;
    lat.sort_unstable();
    println!(
        "eventos: ponteiro={} botão={} rolagem={} teclado={}",
        counts.motion, counts.button, counts.scroll, counts.key
    );
    if let Some(f) = first_event {
        println!("primeiro evento {} ms após Activated", f.as_millis());
    }
    println!(
        "latência adaptador→probe (µs): p50={} p95={} max={} n={}",
        percentile(&lat, 50),
        percentile(&lat, 95),
        lat.last().copied().unwrap_or(0),
        lat.len()
    );
    println!(
        "Release: {}",
        if released.is_ok() { "ok" } else { "falhou" }
    );
    released?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn emulate(yes: bool) -> anyhow::Result<()> {
    if !yes {
        print!("{}", msg::CONFIRM_EMULATE);
        std::io::stdout().flush()?;
        let answer = tokio::task::spawn_blocking(|| {
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).map(|_| s)
        })
        .await??;
        if !matches!(answer.trim().to_lowercase().as_str(), "s" | "sim") {
            anyhow::bail!(msg::ABORTED);
        }
    }
    let mut emu = PortalEmulate::new(TokenStore::new(&profile_dir()));
    emu.start().await?;
    println!("{}", msg::FOCUS_NOW);
    tokio::time::sleep(Duration::from_secs(5)).await;
    let result = square_and_type(&mut emu).await;
    let _ = emu.release_all().await;
    emu.stop().await?;
    result
}

async fn square_and_type(emu: &mut dyn EmulateAdapter) -> anyhow::Result<()> {
    let mut moves = 0u32;
    for (dx, dy) in [(4.0f32, 0.0f32), (0.0, 4.0), (-4.0, 0.0), (0.0, -4.0)] {
        for _ in 0..30 {
            emu.emit(&[InputEvent::Motion { dx, dy }]).await?;
            moves += 1;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    println!("quadrado concluído: {moves} movimentos");
    let mut typed = 0u32;
    for code in TYPE_ABNT2 {
        if code == 0 {
            continue;
        }
        emu.emit(&[InputEvent::Key {
            code,
            pressed: true,
        }])
        .await?;
        tokio::time::sleep(Duration::from_millis(30)).await;
        emu.emit(&[InputEvent::Key {
            code,
            pressed: false,
        }])
        .await?;
        tokio::time::sleep(Duration::from_millis(60)).await;
        typed += 1;
    }
    println!("digitação sintética enviada: {typed} teclas (layout ABNT2)");
    Ok(())
}

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
