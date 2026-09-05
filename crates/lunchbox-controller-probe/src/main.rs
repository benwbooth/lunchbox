use anyhow::{Context, Result, ensure};
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    about = "Inspect a trusted target runtime's SDL3 controllers and optional resolved bindings"
)]
struct Args {
    #[arg(long)]
    sdl_library: PathBuf,
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
}

fn run() -> Result<()> {
    let args = Args::parse();
    let mut hints = BTreeMap::new();
    for text in &args.hint {
        let (name, value) = lunchbox_controller_probe::parse_hint(text)?;
        ensure!(hints.insert(name, value).is_none(), "Duplicate SDL hint");
    }
    let snapshot = lunchbox_controller_probe::inspect_with_bindings(
        &args.sdl_library,
        args.mapping_db.as_deref(),
        hints,
        &args.bindings_for_path,
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
