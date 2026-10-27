//! `clipsync probe clipboard`: prints mime, size and timing. Never prints clipboard content.
use crate::adapter::{ClipError, ClipboardAdapter, X11Clipboard};
use crate::ledger::{WriteLedger, hash_text};
use crate::msg;
use std::time::{Duration, Instant};

#[derive(clap::Args, Debug)]
pub struct ProbeArgs {
    #[arg(long, value_parser = humantime::parse_duration)]
    pub watch: Option<Duration>,
    #[arg(long)]
    pub write: bool,
}

fn open() -> anyhow::Result<X11Clipboard> {
    X11Clipboard::new().map_err(|e| match e {
        ClipError::NoDisplay => anyhow::anyhow!(msg::NO_DISPLAY),
        other => anyhow::anyhow!(other),
    })
}

pub async fn run(args: ProbeArgs) -> anyhow::Result<()> {
    if args.watch.is_none() && !args.write {
        anyhow::bail!(msg::NOTHING_TO_DO);
    }
    tokio::task::spawn_blocking(move || {
        let mut ad = open()?;
        if let Some(d) = args.watch {
            watch(&mut ad, d)?;
        }
        if args.write {
            write(&mut ad)?;
        }
        Ok(())
    })
    .await?
}

fn watch(ad: &mut dyn ClipboardAdapter, total: Duration) -> anyhow::Result<()> {
    println!("{}", msg::WATCH_HEADER);
    let end = Instant::now() + total;
    let mut current = ad.read_text()?.map(|s| hash_text(&s.text));
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        if ad
            .wait_change(left.min(Duration::from_millis(200)))?
            .is_none()
        {
            continue;
        }
        let t = Instant::now();
        let Some(snap) = ad.read_text()? else {
            println!("{}", msg::EMPTY_IGNORED);
            continue;
        };
        let h = hash_text(&snap.text);
        if current == Some(h) {
            continue; // unchanged content (e.g. the compositor taking ownership): not a new copy
        }
        current = Some(h);
        println!(
            "{}",
            msg::watch_line(&snap.mime, snap.text.len(), t.elapsed().as_millis())
        );
    }
    Ok(())
}

fn write(ad: &mut dyn ClipboardAdapter) -> anyhow::Result<()> {
    let text = format!("clipsync-probe-sintetico-ção-{}", clipsync_proto::mono_us());
    let mut ledger = WriteLedger::default();
    let h = hash_text(&text);
    ledger.register(h, Instant::now());
    let wrote = ad.write_text(&text).is_ok();
    let (mut echo_dropped, mut new_copy) = (false, false);
    let end = Instant::now() + Duration::from_millis(500);
    while let Some(left) = end.checked_duration_since(Instant::now()) {
        if ad.wait_change(left)?.is_none() {
            continue;
        }
        match ad.read_text()? {
            Some(s) if ledger.is_echo(&hash_text(&s.text), Instant::now()) => echo_dropped = true,
            Some(_) => new_copy = true,
            None => {}
        }
    }
    let identical = ad.read_text()?.is_some_and(|s| s.text == text);
    println!(
        "{}",
        msg::write_line(wrote, identical, echo_dropped, new_copy)
    );
    if wrote && identical && echo_dropped && !new_copy {
        Ok(())
    } else {
        anyhow::bail!("probe de escrita falhou")
    }
}
