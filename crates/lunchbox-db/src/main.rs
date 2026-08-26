mod audit;
mod compress;
mod database;
mod emulators;
mod ids;
mod inspect;
mod libretro;
mod source;

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "lunchbox-db")]
#[command(about = "Build, inspect, validate, and package the Lunchbox canonical database")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Rebuild a canonical database atomically from declared source files.
    Build {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        emulators: PathBuf,
        #[arg(long)]
        libretro_manifest: PathBuf,
        #[arg(long)]
        libretro_rdb_dir: PathBuf,
        #[arg(long, default_value = "1970-01-01T00:00:00Z")]
        build_timestamp: String,
    },
    /// Create an empty canonical database schema.
    Init {
        #[arg(long)]
        database: PathBuf,
    },
    /// Import or refresh the normalized emulator reference catalog.
    ImportEmulators {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[arg(long, default_value = "1970-01-01T00:00:00Z")]
        import_timestamp: String,
    },
    /// Download, verify, and extract the exact pinned Libretro source snapshot.
    PrepareLibretro {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        cache: PathBuf,
    },
    /// Import a verified Libretro RDB snapshot into an initialized database.
    ImportLibretro {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        rdb_dir: PathBuf,
        #[arg(long, default_value = "1970-01-01T00:00:00Z")]
        import_timestamp: String,
    },
    /// Run integrity, identity, relationship, and data-quality checks.
    Audit {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        strict: bool,
    },
    /// Produce a read-only machine-readable audit of the preserved databases.
    InspectExisting {
        #[arg(long)]
        legacy_catalog: PathBuf,
        #[arg(long)]
        openvgdb: PathBuf,
        #[arg(long)]
        emulators: PathBuf,
        #[arg(long)]
        minerva: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Create a maximally compressed, tested 7z archive and checksum manifest.
    Compress {
        #[arg(long)]
        database: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 100_000_000)]
        max_bytes: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            database,
            emulators,
            libretro_manifest,
            libretro_rdb_dir,
            build_timestamp,
        } => {
            let result = database::build(
                &database,
                &emulators,
                &libretro_manifest,
                &libretro_rdb_dir,
                &build_timestamp,
            )?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Init { database } => {
            database::initialize_path(&database)?;
            let report = audit::audit_path(&database)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::ImportEmulators {
            database,
            source,
            import_timestamp,
        } => {
            let mut connection = database::open_existing(&database)?;
            let stats = emulators::import(&mut connection, &source, &import_timestamp)?;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        Command::PrepareLibretro { manifest, cache } => {
            let prepared = source::prepare_libretro(&manifest, &cache)?;
            println!("{}", serde_json::to_string_pretty(&prepared)?);
        }
        Command::ImportLibretro {
            database,
            manifest,
            rdb_dir,
            import_timestamp,
        } => {
            let mut connection = database::open_existing(&database)?;
            let stats = libretro::import(&mut connection, &manifest, &rdb_dir, &import_timestamp)?;
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        Command::Audit { database, strict } => {
            let report = audit::audit_path(&database)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            if strict && !report.valid {
                bail!("strict database audit failed");
            }
        }
        Command::InspectExisting {
            legacy_catalog,
            openvgdb,
            emulators,
            minerva,
            output,
        } => {
            let report =
                inspect::inspect_existing(&legacy_catalog, &openvgdb, &emulators, &minerva)?;
            inspect::write_report(&output, &report)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Compress {
            database,
            output,
            max_bytes,
        } => {
            let result = compress::compress(&database, &output, max_bytes)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
