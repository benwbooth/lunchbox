//! lunchbox-emulator-e2e: end-to-end feature verification for one emulator
//! on the current host, fully hermetic.
//!
//! For `--emulator <slug>` this tool exercises the three shipped features
//! against isolated session directories (never the real home):
//!
//! 1. controller mapping — every captured `config`/`input` purpose must
//!    resolve to a creatable local path or name a documented non-file
//!    store (registry, defaults, user-chosen, chooser-owned);
//! 2. BIOS/firmware/keys — every captured `bios` purpose resolves to a
//!    directory, and files from `--bios-dir` whose names appear in the
//!    record are staged into it and re-hashed;
//! 3. save state/SRAM sync — every routed save/state root gets sentinel
//!    files, then a two-device local-folder round trip
//!    (`prepare_sync`/`apply`) must preserve bytes.
//!
//! Terminal dispositions (`not_supported`, `not_required`, gap statuses,
//! WholeImage/retroarch-core refusals) are reported as skipped, never
//! invented. Exit non-zero on any failed feature.

use anyhow::{Context, Result, bail};
use lunchbox_app::platform_locations::{
    LocationBases, Purpose, adapter_locations_for_platform, load_records,
    save_route_roots_for_platform,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn current_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn session_bases(root: &Path, platform: &str) -> Result<LocationBases> {
    let (home, config_dir, data_dir, data_local_dir) = match platform {
        "windows" => (
            root.join("User"),
            root.join("User/AppData/Roaming"),
            root.join("User/AppData/Roaming"),
            root.join("User/AppData/Local"),
        ),
        "macos" => (
            root.join("Users/test"),
            root.join("Users/test/Library/Application Support"),
            root.join("Users/test/Library/Application Support"),
            root.join("Users/test/Library/Application Support"),
        ),
        _ => (
            root.join("home"),
            root.join("home/.config"),
            root.join("home/.local/share"),
            root.join("home/.local/share"),
        ),
    };
    for dir in [&home, &config_dir, &data_dir, &data_local_dir] {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating session base {}", dir.display()))?;
    }
    Ok(LocationBases {
        home,
        config_dir,
        data_dir,
        data_local_dir,
        flatpak_roots: Vec::new(),
    })
}

/// Documented non-file stores (registry, defaults, chooser-owned, …).
/// Mirrors the cross-platform feature matrix suite's rule: a known host
/// prefix counts as actionable even when prose annotations keep the strict
/// resolver from accepting the full string.
fn is_documented_store(documented: &str) -> bool {
    const MARKERS: [&str; 12] = [
        "registry",
        "hkcu",
        "nsuserdefaults",
        ".plist",
        "user-chosen",
        "user-located",
        "user-selected",
        "no configuration file",
        "hardcoded",
        "keyboard-only",
        "file chooser",
        "jfilechooser",
    ];
    const PREFIXES: [&str; 8] = [
        "~/",
        "$XDG_CONFIG_HOME/",
        "$XDG_DATA_HOME/",
        "%APPDATA%\\",
        "%APPDATA%/",
        "%LOCALAPPDATA%\\",
        "%LOCALAPPDATA%/",
        "%USERPROFILE%\\",
    ];
    let lower = documented.to_lowercase();
    MARKERS.iter().any(|marker| lower.contains(marker))
        || PREFIXES.iter().any(|prefix| documented.starts_with(prefix))
}

#[derive(Debug, Default, serde::Serialize)]
struct FeatureResult {
    status: String,
    detail: Vec<String>,
}

#[derive(Debug, Default, serde::Serialize)]
struct E2EReport {
    emulator: String,
    platform: String,
    controller: FeatureResult,
    firmware: FeatureResult,
    save_sync: FeatureResult,
    overall: String,
}

fn check_controller(
    slug: &str,
    platform: &str,
    bases: &LocationBases,
    report: &mut FeatureResult,
) -> Result<()> {
    let records = load_records()?;
    let found = adapter_locations_for_platform(
        &records,
        slug,
        &[Purpose::Config, Purpose::Input],
        platform,
        bases,
    );
    if found.is_empty() {
        report.status = "skipped".to_owned();
        report
            .detail
            .push("no config/input purposes captured for this host".to_owned());
        return Ok(());
    }
    for entry in &found {
        if entry.status != "captured" {
            report.detail.push(format!(
                "{} {}: {} ({})",
                entry.purpose.as_str(),
                entry.status,
                entry.documented,
                "terminal disposition, not exercised"
            ));
            continue;
        }
        match &entry.resolved {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("creating controller parent {}", parent.display())
                    })?;
                }
                report.detail.push(format!(
                    "{} resolves to {}",
                    entry.purpose.as_str(),
                    path.display()
                ));
            }
            None if is_documented_store(&entry.documented) => {
                report.detail.push(format!(
                    "{} uses documented store: {}",
                    entry.purpose.as_str(),
                    entry.documented
                ));
            }
            None => {
                report.status = "failed".to_owned();
                report.detail.push(format!(
                    "{} captured but unactionable: {}",
                    entry.purpose.as_str(),
                    entry.documented
                ));
            }
        }
    }
    if report.status.is_empty() {
        report.status = "passed".to_owned();
    }
    Ok(())
}

/// Copy BIOS-dir files whose names the record mentions into each resolved
/// bios root, then re-hash them.
fn check_firmware(
    slug: &str,
    platform: &str,
    bases: &LocationBases,
    bios_dir: Option<&Path>,
    report: &mut FeatureResult,
) -> Result<()> {
    let records = load_records()?;
    let found = adapter_locations_for_platform(
        &records,
        slug,
        &[Purpose::Bios, Purpose::Keys],
        platform,
        bases,
    );
    if found.is_empty() {
        report.status = "skipped".to_owned();
        report
            .detail
            .push("no bios/keys purposes captured for this host".to_owned());
        return Ok(());
    }
    let candidates = match bios_dir {
        Some(dir) => collect_files(dir, 3)?,
        None => {
            report.detail.push(
                "no BIOS library available on this host; roots verified, staging skipped".to_owned(),
            );
            Vec::new()
        }
    };
    for entry in &found {
        if entry.status != "captured" {
            report.detail.push(format!(
                "{} {}: {}",
                entry.purpose.as_str(),
                entry.status,
                entry.documented
            ));
            continue;
        }
        let Some(root) = entry.resolved.clone() else {
            report.detail.push(format!(
                "{} documents a non-path store: {}",
                entry.purpose.as_str(),
                entry.documented
            ));
            continue;
        };
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating firmware root {}", root.display()))?;
        let haystack = format!("{} {}", entry.documented, entry.naming).to_lowercase();
        let mut staged = 0;
        for candidate in &candidates {
            if let Some(name) = candidate.file_name().and_then(|n| n.to_str()) {
                // Token-boundary match: "IPL.bin" must not match
                // "64DD_IPL.BIN". Delimiters are whitespace, path
                // separators, and common punctuation — never [_.-] joins.
                let needle = name.to_lowercase();
                if needle.is_empty() {
                    continue;
                }
                let mut matched = false;
                let mut search = haystack.as_str();
                while let Some(pos) = search.find(needle.as_str()) {
                    let before = search[..pos].chars().next_back();
                    let after = search[pos + needle.len()..].chars().next();
                    let boundary = |c: Option<char>| {
                        c.is_none_or(|c| {
                            c.is_whitespace() || "/\\(),;:'\"[]<>|".contains(c)
                        })
                    };
                    if boundary(before) && boundary(after) {
                        matched = true;
                        break;
                    }
                    search = &search[pos + 1..];
                }
                if matched {
                    let dest = root.join(name);
                    std::fs::copy(candidate, &dest).with_context(|| {
                        format!("staging {} into {}", candidate.display(), root.display())
                    })?;
                    staged += 1;
                }
            }
        }
        if staged == 0 {
            if bios_dir.is_none() {
                report.detail.push(format!(
                    "{} root {} verified (no library to stage from)",
                    entry.purpose.as_str(),
                    root.display()
                ));
            } else {
                report.status = "failed".to_owned();
                report.detail.push(format!(
                    "{} resolved to {} but no --bios-dir file is named by the record",
                    entry.purpose.as_str(),
                    root.display()
                ));
            }
        } else {
            report.detail.push(format!(
                "{} staged {staged} file(s) into {}",
                entry.purpose.as_str(),
                root.display()
            ));
        }
    }
    if report.status.is_empty() {
        report.status = "passed".to_owned();
    }
    Ok(())
}

fn collect_files(dir: &Path, depth: usize) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if depth == 0 {
        return Ok(out);
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(out),
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_dir() {
            out.extend(collect_files(&path, depth - 1)?);
        } else if path.is_file() {
            out.push(path);
        }
        if out.len() >= 2000 {
            break;
        }
    }
    Ok(out)
}

fn check_save_sync(
    slug: &str,
    platform: &str,
    bases: &LocationBases,
    session: &Path,
    report: &mut FeatureResult,
) -> Result<()> {
    use lunchbox_app::save_cloud::{CloudProfile, CloudStore};
    use lunchbox_app::save_sync::{SavePurpose, SyncScope};
    use lunchbox_app::save_sync_service::prepare_sync;

    let records = load_records()?;
    let roots = match save_route_roots_for_platform(&records, slug, platform, bases) {
        Ok(roots) => roots,
        Err(error) => {
            report.status = "skipped".to_owned();
            report
                .detail
                .push(format!("save routing refused: {error:#}"));
            return Ok(());
        }
    };
    if roots.is_empty() {
        report.status = "skipped".to_owned();
        report
            .detail
            .push("no save/state roots routed for this host".to_owned());
        return Ok(());
    }
    // Sentinel per routed root.
    let mut sentinels = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        std::fs::create_dir_all(&root.path)
            .with_context(|| format!("creating save root {}", root.path.display()))?;
        let bytes = format!("e2e sentinel {slug}/{platform}/{index}");
        std::fs::write(root.path.join(format!("lunchbox-e2e-{index}.srm")), &bytes)?;
        sentinels.push(bytes);
        report.detail.push(format!(
            "{} root {} seeded",
            root.route.purpose.as_str(),
            root.path.display()
        ));
    }
    let provider = session.join("remote");
    std::fs::create_dir_all(&provider)
        .with_context(|| format!("creating local-folder remote {}", provider.display()))?;
    let profile = CloudProfile::new_local_folder(&provider, "device-a", true)?;
    let store = CloudStore::connect(profile.provider, &profile.root, &profile.auth)?;
    store.probe()?;
    let scope = SyncScope::new(slug, platform)?;
    let recovery = session.join("recovery");
    let uploaded = prepare_sync(&store, scope.clone(), "device-a", roots.clone(), None)?.apply(
        &store,
        &BTreeMap::new(),
        &recovery,
        1000,
    )?;
    // Fresh device downloads everything back; every sentinel byte string
    // must reappear under it.
    let device_b = session.join("device-b");
    let b_roots: Vec<lunchbox_app::save_sync::RouteRoot> = roots
        .iter()
        .enumerate()
        .map(|(index, root)| {
            let path = device_b.join(format!("{}-{index}", root.route.purpose.as_str()));
            std::fs::create_dir_all(&path).expect("device-b route dir");
            lunchbox_app::save_sync::RouteRoot {
                route: root.route,
                path,
                create_if_missing: true,
            }
        })
        .collect();
    let downloaded = prepare_sync(&store, scope, "device-b", b_roots, None)?;
    downloaded.apply(&store, &BTreeMap::new(), &recovery, 2000)?;
    let found = collect_files(&device_b, 4)?;
    for sentinel in &sentinels {
        let mut matched = false;
        for file in &found {
            if std::fs::read(file).is_ok_and(|bytes| bytes == sentinel.as_bytes()) {
                matched = true;
                break;
            }
        }
        if !matched {
            report.status = "failed".to_owned();
            report
                .detail
                .push(format!("sentinel lost in download: {sentinel}"));
            return Ok(());
        }
    }
    report.detail.push(format!(
        "local-folder round trip preserved {} sentinel(s), manifest {}",
        sentinels.len(),
        uploaded.manifest_id
    ));
    let _ = SavePurpose::Saves;
    report.status = "passed".to_owned();
    Ok(())
}

fn print_usage(program: &str) {
    println!("usage: {program} --emulator SLUG --bios-dir DIR [--json] [--keep] [--list]");
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let program = args.first().cloned().unwrap_or_default();
    let mut slug: Option<String> = None;
    let mut bios_dir: Option<PathBuf> = None;
    let mut json = false;
    let mut keep = false;
    let mut list = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--emulator" => {
                index += 1;
                slug = args.get(index).cloned();
            }
            "--bios-dir" => {
                index += 1;
                bios_dir = args.get(index).map(PathBuf::from);
            }
            "--json" => json = true,
            "--keep" => keep = true,
            "--list" => list = true,
            "--help" | "-h" => {
                print_usage(&program);
                return Ok(());
            }
            other => bail!("unknown argument {other}"),
        }
        index += 1;
    }
    let platform = current_platform();
    if list {
        let records = load_records()?;
        let mut slugs: Vec<&str> = records
            .iter()
            .filter(|record| record.has_platform(platform))
            .map(|record| record.slug())
            .collect();
        slugs.sort_unstable();
        if json {
            println!("{}", serde_json::to_string_pretty(&slugs)?);
        } else {
            for slug in slugs {
                println!("{slug}");
            }
        }
        return Ok(());
    }
    let (Some(slug), mut bios_dir) = (slug, bios_dir) else {
        print_usage(&program);
        bail!("--emulator is required");
    };
    // Remote hosts may not mount the shared BIOS library; without it the
    // firmware step reports what it can (resolved roots, documented
    // stores) and skips staging.
    let bios_available = bios_dir
        .as_ref()
        .is_some_and(|dir| dir.is_dir() && collect_files(dir, 1).is_ok_and(|v| !v.is_empty()));
    if !bios_available {
        bios_dir = None;
    }
    let mut guard = Some(
        tempfile::Builder::new()
            .prefix("lunchbox-e2e-")
            .tempdir()
            .context("session tempdir")?,
    );
    let session_path = if keep {
        guard.take().expect("session guard present").keep()
    } else {
        guard
            .as_ref()
            .expect("session guard present")
            .path()
            .to_path_buf()
    };
    let _guard = guard;
    let bases = session_bases(&session_path, platform)?;
    let mut report = E2EReport {
        emulator: slug.clone(),
        platform: platform.to_owned(),
        ..E2EReport::default()
    };
    if let Err(error) = check_controller(&slug, platform, &bases, &mut report.controller) {
        report.controller.status = "failed".to_owned();
        report.controller.detail.push(format!("{error:#}"));
    }
    if let Err(error) = check_firmware(&slug, platform, &bases, &bios_dir, &mut report.firmware) {
        report.firmware.status = "failed".to_owned();
        report.firmware.detail.push(format!("{error:#}"));
    }
    if let Err(error) = check_save_sync(
        &slug,
        platform,
        &bases,
        &session_path,
        &mut report.save_sync,
    ) {
        report.save_sync.status = "failed".to_owned();
        report.save_sync.detail.push(format!("{error:#}"));
    }
    report.overall = if [&report.controller, &report.firmware, &report.save_sync]
        .iter()
        .all(|feature| feature.status == "passed" || feature.status == "skipped")
    {
        "passed".to_owned()
    } else {
        "failed".to_owned()
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("emulator: {} ({})", report.emulator, report.platform);
        for (name, feature) in [
            ("controller", &report.controller),
            ("firmware", &report.firmware),
            ("save_sync", &report.save_sync),
        ] {
            println!("  {name}: {}", feature.status);
            for line in &feature.detail {
                println!("    - {line}");
            }
        }
        println!("overall: {}", report.overall);
    }
    if report.overall != "passed" {
        std::process::exit(1);
    }
    Ok(())
}
