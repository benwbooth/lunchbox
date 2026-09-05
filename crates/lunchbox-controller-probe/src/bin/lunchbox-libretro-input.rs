use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    about = "Run an original GBA input diagnostic in an explicitly trusted mGBA libretro core"
)]
struct Args {
    #[arg(long)]
    core: PathBuf,
    #[arg(long)]
    sha256: String,
    #[arg(long)]
    bitmask: bool,
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=120))]
    timeout_seconds: u64,
}
fn run() -> Result<()> {
    let args = Args::parse();
    let (done, receiver) = std::sync::mpsc::channel();
    let watchdog = std::thread::spawn(move || {
        if receiver
            .recv_timeout(std::time::Duration::from_secs(args.timeout_seconds))
            .is_err()
        {
            eprintln!("Trusted core diagnostic exceeded its wall-time limit");
            std::process::exit(124);
        }
    });
    let result = lunchbox_controller_probe::libretro_input::inspect_mgba(
        &args.core,
        &args.sha256,
        args.bitmask,
    );
    let _ = done.send(());
    let _ = watchdog.join();
    println!("{}", serde_json::to_string_pretty(&result?)?);
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
