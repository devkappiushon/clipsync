//! Dev harness for `probe::run` until the CLI wires `probe clipboard` (not shipped).
use clap::Parser;
use clipsync_clipboard::probe::{ProbeArgs, run};

#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: ProbeArgs,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run(Cli::parse().args).await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
