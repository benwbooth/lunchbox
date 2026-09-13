use anyhow::Result;
use clap::Parser;
use lunchbox_controller_probe::libretro_input::{Diagnostic, NesTopology, SnesTopology};
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Run an original input diagnostic in an explicitly trusted libretro core")]
struct Args {
    #[arg(long)]
    core: PathBuf,
    #[arg(long)]
    sha256: String,
    /// Trusted dependency to load before the core; repeat in dependency order.
    #[arg(long)]
    runtime_library: Vec<PathBuf>,
    #[arg(long)]
    bitmask: bool,
    /// New file that receives the JSON report; native core stdout remains separate.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Only call retro_api_version and retro_get_system_info; do not initialize the core.
    #[arg(long)]
    identity_only: bool,
    /// Directory containing scph5500.bin, scph5501.bin, and scph5502.bin.
    #[arg(long)]
    bios_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "gba")]
    system: Diagnostic,
    /// NES controller topology; rejected for non-NES diagnostics unless left at its default.
    #[arg(long, value_enum, default_value = "two-player")]
    nes_topology: NesTopology,
    /// SNES controller topology; rejected for non-SNES diagnostics unless left at its default.
    #[arg(long, value_enum, default_value = "two-player")]
    snes_topology: SnesTopology,
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=120))]
    timeout_seconds: u64,
}

fn emit_json(value: &impl Serialize, output: Option<&PathBuf>) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    if let Some(path) = output {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    } else {
        println!("{}", String::from_utf8(bytes)?);
    }
    Ok(())
}

fn run() -> Result<()> {
    let args = Args::parse();
    if args.identity_only {
        anyhow::ensure!(
            args.runtime_library.is_empty(),
            "--runtime-library is not supported with --identity-only"
        );
        let identity =
            lunchbox_controller_probe::libretro_input::core_identity(&args.core, &args.sha256)?;
        emit_json(&identity, args.output.as_ref())?;
        return Ok(());
    }
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
    let result = lunchbox_controller_probe::libretro_input::inspect_with_runtime_options(
        &args.core,
        &args.sha256,
        args.bitmask,
        args.system,
        args.bios_dir.as_deref(),
        args.nes_topology,
        args.snes_topology,
        &args.runtime_library,
    );
    let _ = done.send(());
    let _ = watchdog.join();
    emit_json(&result?, args.output.as_ref())?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
