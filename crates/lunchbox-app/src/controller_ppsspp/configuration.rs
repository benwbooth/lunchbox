//! Effective global/per-game PPSSPP input configuration preparation.
//! Pin: e49c0bd8836a8a8f678565357773386f1174d3f5/Core/Config.cpp.
//! Routing these private files into the emulator is the native launch adapter's
//! responsibility. --appendconfig does not load KeyMap and must not be used.
use super::{SdlInput, render_controls};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputLayer {
    Global,
    Game,
}

/// Retains both present and absent source documents. Creating a new game INI
/// after preparation changes the effective layer and invalidates the session.
pub(crate) struct PreparedConfiguration {
    directory: tempfile::TempDir,
    originals: BTreeMap<PathBuf, Option<Vec<u8>>>,
    pub(crate) layer: InputLayer,
    game_filename: String,
    source_system: PathBuf,
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Reading PPSSPP config {}", path.display()));
        }
    };
    ensure!(
        file.metadata()?.is_file(),
        "PPSSPP config path is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_CONFIG_BYTES,
        "PPSSPP configuration exceeds size limit"
    );
    Ok(Some(bytes))
}

impl PreparedConfiguration {
    /// `source_system` is the explicitly resolved Config::searchPath_, normally
    /// PSP/SYSTEM. `game_id` is native game identity, never a title match. This
    /// contract excludes VR's separate ppssppvr/controlsvr filenames.
    pub(crate) fn prepare(
        source_system: &Path,
        game_id: &str,
        sdl_device_index: u8,
        controls: &BTreeMap<String, SdlInput>,
    ) -> Result<Self> {
        ensure!(
            source_system.is_absolute() && source_system.is_dir(),
            "Resolve the native PPSSPP system configuration directory first"
        );
        ensure!(
            !game_id.is_empty()
                && game_id.len() <= 128
                && game_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
            "PPSSPP needs an exact filename-safe game ID"
        );
        let game_filename = format!("{game_id}_ppsspp.ini");
        let mut originals = BTreeMap::new();
        for name in ["ppsspp.ini", "controls.ini", game_filename.as_str()] {
            let path = source_system.join(name);
            originals.insert(path.clone(), read_optional(&path)?);
        }
        let global = originals[&source_system.join("ppsspp.ini")]
            .as_ref()
            .context(
                "PPSSPP global settings are absent; resolve an initialized native configuration",
            )?;
        let ordinary = originals[&source_system.join("controls.ini")].as_ref()
            .context("PPSSPP controls.ini is absent; its platform defaults must be resolved before remapping")?;
        let game = originals[&source_system.join(&game_filename)].as_ref();
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-ppsspp-config-")
            .tempdir()?;
        // Keep auxiliary SYSTEM files visible when the complete directory is
        // overlaid. Do not follow symlinks out into unrelated trees.
        let mut copied = 0_u64;
        let mut entries_seen = 0_usize;
        copy_system_tree(
            source_system,
            directory.path(),
            0,
            &mut copied,
            &mut entries_seen,
        )?;
        std::fs::write(
            directory.path().join("ppsspp.ini"),
            startup_logging(std::str::from_utf8(global).context("PPSSPP settings are not UTF-8")?),
        )?;
        let ordinary = std::str::from_utf8(ordinary).context("PPSSPP controls.ini is not UTF-8")?;
        std::fs::write(
            directory.path().join("controls.ini"),
            render_controls(ordinary, sdl_device_index, controls)?,
        )?;
        let layer = if let Some(game) = game {
            let game =
                std::str::from_utf8(game).context("PPSSPP game configuration is not UTF-8")?;
            std::fs::write(
                directory.path().join(&game_filename),
                render_controls(game, sdl_device_index, controls)?,
            )?;
            InputLayer::Game
        } else {
            InputLayer::Global
        };
        let prepared = Self {
            directory,
            originals,
            layer,
            game_filename,
            source_system: source_system.canonicalize()?,
        };
        prepared.verify_sources()?;
        Ok(prepared)
    }

    pub(crate) fn system_directory(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) fn effective_input_path(&self) -> PathBuf {
        self.directory.path().join(match self.layer {
            InputLayer::Global => "controls.ini",
            InputLayer::Game => &self.game_filename,
        })
    }

    pub(crate) fn verify_sources(&self) -> Result<()> {
        for (path, original) in &self.originals {
            ensure!(
                read_optional(path)? == *original,
                "PPSSPP source config changed during preparation: {}",
                path.display()
            );
        }
        Ok(())
    }

    /// Mount only SYSTEM over itself inside the child's namespace. No XDG/HOME
    /// override is used: SAVEDATA, SAVESTATE, GAME and other memory-stick paths
    /// retain their native persistent locations. Caller resolves trusted bwrap
    /// and verifies PPSSPP actually uses this exact system directory.
    #[cfg(target_os = "linux")]
    pub(crate) fn overlay_arguments(
        &self,
        executable: &Path,
        arguments: &[std::ffi::OsString],
        working_directory: &Path,
    ) -> Result<Vec<std::ffi::OsString>> {
        self.verify_sources()?;
        ensure!(
            executable.is_absolute() && working_directory.is_absolute(),
            "PPSSPP overlay requires absolute executable and working directory"
        );
        let mut result = vec![
            "--die-with-parent".into(),
            "--bind".into(),
            "/".into(),
            "/".into(),
            "--bind".into(),
            self.directory.path().as_os_str().to_owned(),
            self.source_system.as_os_str().to_owned(),
            "--chdir".into(),
            working_directory.as_os_str().to_owned(),
            "--".into(),
            executable.as_os_str().to_owned(),
        ];
        result.extend_from_slice(arguments);
        Ok(result)
    }
}

// Only the private global configuration changes. Preserve all unrelated keys,
// comments and section ordering, including explicitly disabled user logging.
fn startup_logging(original: &str) -> String {
    let mut text = original.to_owned();
    for (section, key, value) in [
        ("General", "Enable Logging", "True"),
        ("General", "FileLogging", "False"),
        ("Log", "SYSTEMEnabled", "True"),
        ("Log", "SYSTEMLevel", "4"),
        ("LogDebug", "SYSTEMEnabled", "True"),
        ("LogDebug", "SYSTEMLevel", "4"),
    ] {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut result = String::new();
        let mut active = false;
        let mut inserted = false;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                active = trimmed[1..trimmed.len() - 1].eq_ignore_ascii_case(section);
                result.push_str(line);
                if active && !inserted {
                    if !result.ends_with('\n') {
                        result.push_str(newline);
                    }
                    result.push_str(&format!("{key} = {value}{newline}"));
                    inserted = true;
                }
            } else if !(active
                && trimmed
                    .split_once('=')
                    .is_some_and(|(candidate, _)| candidate.trim().eq_ignore_ascii_case(key)))
            {
                result.push_str(line);
            }
        }
        if !inserted {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push_str(newline);
            }
            result.push_str(&format!("[{section}]{newline}{key} = {value}{newline}"));
        }
        text = result;
    }
    text
}

fn copy_system_tree(
    source: &Path,
    destination: &Path,
    depth: usize,
    total: &mut u64,
    entries_seen: &mut usize,
) -> Result<()> {
    ensure!(
        depth <= 16,
        "PPSSPP SYSTEM directory nesting exceeds the session limit"
    );
    let mut entries = std::fs::read_dir(source)?.collect::<std::io::Result<Vec<_>>>()?;
    ensure!(
        entries.len() <= 4096,
        "PPSSPP SYSTEM directory has too many entries"
    );
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        *entries_seen += 1;
        ensure!(
            *entries_seen <= 4096,
            "PPSSPP SYSTEM snapshot has too many total entries"
        );
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            std::fs::create_dir(&target)?;
            copy_system_tree(&entry.path(), &target, depth + 1, total, entries_seen)?;
        } else {
            ensure!(
                kind.is_file(),
                "Resolve PPSSPP SYSTEM symlinks or special files before private routing"
            );
            let bytes = read_optional(&entry.path())?
                .context("PPSSPP SYSTEM file disappeared during staging")?;
            *total += bytes.len() as u64;
            ensure!(
                *total <= 64 * 1024 * 1024,
                "PPSSPP SYSTEM snapshot exceeds 64 MiB"
            );
            std::fs::write(target, bytes)?;
        }
    }
    Ok(())
}
