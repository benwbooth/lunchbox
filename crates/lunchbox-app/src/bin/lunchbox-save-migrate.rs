//! One-time, explicitly requested migration of a local-folder save archive.

use anyhow::{Context, Result, ensure};
use lunchbox_app::save_cloud::{CloudProfile, CloudStore};
use lunchbox_app::save_sync::SyncScope;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().context(
        "usage: lunchbox-save-migrate ROOT EMULATOR_SLUG RUNTIME_PLATFORM DEVICE_ID (--prepare|--remove-legacy-blobs)",
    )?);
    let emulator_slug = args
        .next()
        .context("missing emulator slug")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("emulator slug is not UTF-8"))?;
    let runtime_platform = args
        .next()
        .context("missing runtime platform")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("runtime platform is not UTF-8"))?;
    let device_id = args
        .next()
        .context("missing device ID")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("device ID is not UTF-8"))?;
    let phase = args
        .next()
        .context("missing --prepare or --remove-legacy-blobs")?;
    let remove_blobs = match phase.to_str() {
        Some("--prepare") => false,
        Some("--remove-legacy-blobs") => true,
        _ => anyhow::bail!("expected --prepare or --remove-legacy-blobs"),
    };
    ensure!(
        args.next().is_none(),
        "unexpected extra migration arguments"
    );
    let scope = SyncScope::new(emulator_slug, runtime_platform)?;
    let profile = CloudProfile::new_local_folder(&root, &device_id, false)?;
    let store = CloudStore::connect(profile.provider, &profile.root, &profile.auth)?;
    let report = store.migrate_legacy_scope(&scope, &device_id, remove_blobs)?;
    println!(
        "Migrated {} named versions and {} current files; removed {} verified legacy blobs from {}",
        report.named_versions,
        report.current_files,
        report.removed_blobs,
        scope.remote_prefix()
    );
    Ok(())
}
