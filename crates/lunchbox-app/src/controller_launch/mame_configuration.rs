//! One source configuration for MAME inspection, persistence and launch.
use super::*;
use std::path::PathBuf;

pub(crate) struct Snapshot {
    pub(super) base_path: PathBuf,
    pub(super) base: String,
    base_target: PathBuf,
    pub(super) library: String,
    pub(super) content: PathBuf,
    pub(super) options: String,
    options_source: Option<(PathBuf, PathBuf)>,
    automatic: bool,
    directories: Vec<(PathBuf, PathBuf)>,
    native_configs: Vec<(PathBuf, Option<String>)>,
}

impl Snapshot {
    pub(super) fn read(
        executable: &EmulatorExecutable,
        library: &str,
        content: &Path,
        automatic: bool,
    ) -> Result<Self> {
        ensure!(
            matches!(executable, EmulatorExecutable::Native(_)),
            "MAME configuration requires a resolved native RetroArch runtime"
        );
        let dirs = directories::BaseDirs::new().context("Finding MAME frontend configuration")?;
        let base_path = dirs.config_dir().join("retroarch");
        // At 69a4f0ea configuration.c:3642-3676, a missing primary file can
        // select the legacy home config or a build-specific system skeleton.
        // Do not interpret that unresolved source as an empty configuration.
        let base_file = read_configuration_file(
            &base_path.join("retroarch.cfg"),
            "MAME frontend configuration",
        )?
        .with_context(|| {
            format!(
                "MAME needs an explicit frontend configuration at {}; initialize native RetroArch configuration before inspection instead of relying on legacy/system fallbacks",
                base_path.join("retroarch.cfg").display()
            )
        })?;
        let base = base_file.text;
        ensure!(
            !base
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "Resolve included frontend configuration before MAME inspection"
        );
        if automatic {
            ensure_no_frontend_overrides(&base_path, &base, library, content)?;
        }
        let options = effective_options(&base_path, &base, library, content)?;
        ensure!(
            !options
                .text
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "Resolve included MAME core options before inspection"
        );
        Ok(Self {
            base_path,
            base,
            base_target: base_file.canonical,
            library: library.to_owned(),
            content: content.to_path_buf(),
            options: options.text,
            options_source: options.source,
            automatic,
            directories: Vec::new(),
            native_configs: Vec::new(),
        })
    }

    pub(super) fn verify(&self) -> Result<()> {
        let base = read_configuration_file(
            &self.base_path.join("retroarch.cfg"),
            "MAME frontend configuration",
        )?
        .context("MAME frontend configuration disappeared during preparation; prepare again")?;
        ensure!(
            base.canonical == self.base_target && base.text == self.base,
            "MAME frontend configuration file or contents changed during preparation; prepare again"
        );
        if self.automatic {
            ensure_no_frontend_overrides(
                &self.base_path,
                &self.base,
                &self.library,
                &self.content,
            )?;
        }
        let options = effective_options(&self.base_path, &self.base, &self.library, &self.content)?;
        ensure!(
            options.source == self.options_source && options.text == self.options,
            "MAME effective core-options file or contents changed during preparation; inspect again"
        );
        for (path, canonical) in &self.directories {
            ensure!(
                path.is_dir() && path.canonicalize()? == *canonical,
                "MAME persistent path changed during preparation: {}",
                path.display()
            );
        }
        for (path, expected) in &self.native_configs {
            ensure!(
                native_config_hash(path)? == *expected,
                "MAME native configuration changed during preparation: {}",
                path.display()
            );
        }
        Ok(())
    }

    pub(super) fn create_persistent_directory(&mut self, path: &Path) -> Result<PathBuf> {
        ensure!(path.is_absolute(), "MAME persistence must be absolute");
        std::fs::create_dir_all(path)?;
        let canonical = path.canonicalize()?;
        self.directories
            .push((path.to_path_buf(), canonical.clone()));
        Ok(canonical)
    }

    /// Retain absence as well as existing bytes: a new default/game cfg after
    /// discovery must not silently be omitted from the private launch tree.
    pub(super) fn track_native_config(&mut self, path: &Path) -> Result<bool> {
        let hash = native_config_hash(path)?;
        let present = hash.is_some();
        self.native_configs.push((path.to_path_buf(), hash));
        Ok(present)
    }
}

struct EffectiveOptions {
    // Keep the selected lexical path as well as its symlink-resolved target.
    // A new higher-priority file matters even when its bytes are identical.
    source: Option<(PathBuf, PathBuf)>,
    text: String,
}

fn effective_options(
    base_path: &Path,
    config: &str,
    library: &str,
    content: &Path,
) -> Result<EffectiveOptions> {
    // RetroArch 69a4f0ea runloop_init_core_options_path selects one source:
    // game -> folder -> core -> global. Never merge these option layers.
    let directory = config_directory(base_path, config)?.join(library);
    if config_bool(config, "game_specific_options", true)? {
        let game = content
            .file_stem()
            .context("MAME content has no game basename")?;
        let folder = content
            .parent()
            .and_then(Path::file_name)
            .context("MAME content has no folder basename")?;
        for name in [game, folder] {
            let mut filename = name.to_os_string();
            filename.push(".opt");
            if let Some(options) = read_options_file(&directory.join(filename))? {
                return Ok(options);
            }
        }
    }
    if !config_bool(config, "global_core_options", false)?
        && let Some(options) = read_options_file(&directory.join(format!("{library}.opt")))?
    {
        return Ok(options);
    }
    let global = match cfg_value(config, "core_options_path")?.filter(|value| !value.is_empty()) {
        Some(value) => configured_path(&value)?,
        None => base_path.join("retroarch-core-options.cfg"),
    };
    Ok(read_options_file(&global)?.unwrap_or(EffectiveOptions {
        source: None,
        text: String::new(),
    }))
}

fn read_options_file(path: &Path) -> Result<Option<EffectiveOptions>> {
    Ok(
        read_configuration_file(path, "MAME core options")?.map(|file| EffectiveOptions {
            source: Some((path.to_path_buf(), file.canonical)),
            text: file.text,
        }),
    )
}

struct ConfigurationFile {
    canonical: PathBuf,
    text: String,
}

/// Resolve a selected file without collapsing broken links or read failures
/// into absence. Both frontend and options sources use this bounded reader.
fn read_configuration_file(path: &Path, label: &str) -> Result<Option<ConfigurationFile>> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Checking {label} {}", path.display()));
        }
    }
    // Configurations managed by Nix/Home Manager may use valid symlinks. Keep
    // their resolved identities, but do not fall back past a broken link,
    // unreadable file, directory or special file as if it did not exist.
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Resolving {label} {}", path.display()))?;
    let metadata = std::fs::metadata(&canonical)?;
    ensure!(
        metadata.is_file(),
        "{label} must resolve to a regular file: {}",
        path.display()
    );
    ensure!(
        metadata.len() <= 8 * 1024 * 1024,
        "{label} exceeds 8 MiB: {}",
        path.display()
    );
    let mut text = String::new();
    std::fs::File::open(&canonical)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_string(&mut text)
        .with_context(|| format!("Reading {label} {}", path.display()))?;
    ensure!(
        text.len() <= 8 * 1024 * 1024,
        "{label} exceeds 8 MiB: {}",
        path.display()
    );
    ensure!(
        path.canonicalize()? == canonical,
        "{label} target changed while reading: {}",
        path.display()
    );
    Ok(Some(ConfigurationFile { canonical, text }))
}

pub(super) fn config_directory(base_path: &Path, config: &str) -> Result<PathBuf> {
    // RetroArch platform_unix.c supplies config/ when omitted. configuration.c
    // clears an explicit "default"; file_path_special.c then falls back to the
    // current configuration file's directory, as it does for an explicit "".
    match cfg_value(config, "rgui_config_directory")? {
        None => Ok(base_path.join("config")),
        Some(value) if value.is_empty() || value == "default" => Ok(base_path.to_path_buf()),
        Some(value) => configured_path(&value),
    }
}

fn ensure_no_frontend_overrides(
    base_path: &Path,
    config: &str,
    library: &str,
    content: &Path,
) -> Result<()> {
    if !config_bool(config, "auto_overrides_enable", true)? {
        return Ok(());
    }
    // config_load_override stacks core, folder and game .cfg files. Reject
    // unresolved applicable overrides before deriving any persistent paths.
    let directory = config_directory(base_path, config)?.join(library);
    let game = content
        .file_stem()
        .context("MAME content has no game basename")?;
    let folder = content
        .parent()
        .and_then(Path::file_name)
        .context("MAME content has no folder basename")?;
    for name in [std::ffi::OsStr::new(library), folder, game] {
        let mut filename = name.to_os_string();
        filename.push(".cfg");
        let path = directory.join(filename);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => bail!(
                "Automatic MAME setup cannot preserve the applicable RetroArch override {}. Resolve its effective configuration and save paths in a reviewed per-game setup first",
                path.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Checking applicable MAME override {}", path.display())
                });
            }
        }
    }
    Ok(())
}

fn native_config_hash(path: &Path) -> Result<Option<String>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "MAME configuration needs an explicit regular-file location: {}",
                path.display()
            );
            Ok(Some(lunchbox_controller_probe::file_hash(path)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("Reading MAME configuration {}", path.display()))
        }
    }
}
