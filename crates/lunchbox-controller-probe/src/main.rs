use anyhow::{Context, Result, ensure};
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    about = "Inspect a trusted target runtime's SDL3 controllers and optional resolved bindings"
)]
struct Args {
    /// Inventory SDL2 for BizHawk instead of using the SDL3 adapter.
    #[arg(long)]
    sdl2_inventory: bool,
    /// Open this exact SDL2 device only to inspect its control counts.
    #[arg(long, requires = "sdl2_inventory")]
    sdl2_controls_for_path: Vec<String>,
    /// Load the target frontend's mapping database after SDL2 initialization.
    #[arg(long, requires = "sdl2_inventory")]
    sdl2_mapping_db: Option<PathBuf>,
    #[arg(long)]
    sdl_library: Option<PathBuf>,
    #[arg(long)]
    mapping_db: Option<PathBuf>,
    #[arg(long, value_name = "SDL_NAME=value")]
    hint: Vec<String>,
    /// Require exactly one device at each requested runtime path. Never matches names.
    #[arg(long)]
    match_path: Vec<String>,
    /// Open this exact gamepad path to query SDL's resolved input bindings.
    #[arg(long)]
    bindings_for_path: Vec<String>,
    /// Open ALL SDL devices in event order to project this DuckStation revision's player IDs.
    #[arg(long, value_name = "REVISION")]
    duckstation_player_probe: Option<String>,
    /// Project PCSX2 SDL player IDs using the explicitly supported contract.
    #[arg(long, conflicts_with = "duckstation_player_probe")]
    pcsx2_player_probe: bool,
    /// Trusted target-runtime dependency to load before SDL, in dependency order.
    #[arg(long)]
    runtime_library: Vec<PathBuf>,
    /// Dump raw evdev capabilities and sysfs identity for these /dev/input/eventN
    /// nodes instead of inspecting SDL. Read-only; no SDL library is needed.
    #[arg(long)]
    evdev_catalog: Vec<PathBuf>,
}

fn run() -> Result<()> {
    let args = Args::parse();
    #[cfg(target_os = "linux")]
    if !args.evdev_catalog.is_empty() {
        ensure!(
            !args.sdl2_inventory
                && args.sdl2_controls_for_path.is_empty()
                && args.sdl2_mapping_db.is_none()
                && args.sdl_library.is_none()
                && args.mapping_db.is_none()
                && args.hint.is_empty()
                && args.match_path.is_empty()
                && args.bindings_for_path.is_empty()
                && args.duckstation_player_probe.is_none()
                && !args.pcsx2_player_probe
                && args.runtime_library.is_empty(),
            "Evdev catalog mode shares no options with the SDL probes"
        );
        let catalog = lunchbox_controller_probe::evdev_catalog::catalog(&args.evdev_catalog)?;
        println!("{}", serde_json::to_string_pretty(&catalog)?);
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    ensure!(
        args.evdev_catalog.is_empty(),
        "Evdev catalog mode requires Linux"
    );
    if args.sdl2_inventory {
        ensure!(
            args.mapping_db.is_none()
                && args.hint.is_empty()
                && args.bindings_for_path.is_empty()
                && args.duckstation_player_probe.is_none()
                && !args.pcsx2_player_probe
                && args.runtime_library.is_empty(),
            "SDL2 inventory uses its inherited runtime environment; SDL3 probe options are not supported"
        );
        let sdl_library = args
            .sdl_library
            .as_deref()
            .context("SDL2 inventory needs --sdl-library");
        let snapshot = lunchbox_controller_probe::sdl2::inspect_with_mapping_database(
            sdl_library?,
            &args.sdl2_controls_for_path,
            args.sdl2_mapping_db.as_deref(),
        )?;
        for path in &args.match_path {
            snapshot
                .device_at_path(path)
                .with_context(|| format!("Matching {path}"))?;
        }
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
        return Ok(());
    }
    let mut hints = BTreeMap::new();
    for text in &args.hint {
        let (name, value) = lunchbox_controller_probe::parse_hint(text)?;
        ensure!(hints.insert(name, value).is_none(), "Duplicate SDL hint");
    }
    let sdl_library = args
        .sdl_library
        .as_deref()
        .context("SDL inspection needs --sdl-library");
    let snapshot = lunchbox_controller_probe::inspect_target_runtime(
        sdl_library?,
        args.mapping_db.as_deref(),
        hints,
        &args.bindings_for_path,
        if args.pcsx2_player_probe {
            Some(lunchbox_controller_probe::players::PCSX2_CONTRACT)
        } else {
            args.duckstation_player_probe.as_deref()
        },
        &args.runtime_library,
    )?;
    for path in &args.match_path {
        snapshot
            .device_at_path(path)
            .with_context(|| format!("Matching {path}"))?;
    }
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
