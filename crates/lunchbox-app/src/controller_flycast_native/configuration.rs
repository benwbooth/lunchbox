//! Private emu.cfg preparation; selecting this directory at launch is separate.
use super::isolation::PreparedMappings;
use anyhow::{Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

const LIMIT: usize = 16 * 1024 * 1024;

fn read_config(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "Flycast source config is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= LIMIT, "Flycast config exceeds size limit");
    Ok(bytes)
}

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    source: PathBuf,
    canonical_source: PathBuf,
    original: Vec<u8>,
    generated: Vec<u8>,
}

impl PreparedConfig {
    pub(crate) fn create(
        source: &Path,
        mappings: &PreparedMappings,
        observed_native_instances: &[i32],
        players: &[super::routing::PlayerPort],
    ) -> Result<Self> {
        ensure!(
            source.is_absolute(),
            "Flycast source config must be absolute"
        );
        let canonical_source = source.canonicalize()?;
        let original = read_config(source)?;
        let text = std::str::from_utf8(&original)?;
        ensure!(!text.contains('\0'), "Flycast config contains NUL");
        mappings.verify_before_launch()?;
        let value = mappings.mapping_directory_value()?;
        // IniFile::load removes the first and last quotes before Option's
        // directory-list parser sees the value. Keep both quoting layers.
        // Repeated sections merge and the last assignment wins in native INI.
        let assignments = super::routing::assignments(observed_native_instances, players)?;
        let mut generated =
            format!("{text}\n[config]\nDreamcast.MappingsPath = \"{value}\"\n[input]\n");
        for (key, port) in assignments {
            generated.push_str(&format!("{key} = {port}\n"));
        }
        generated.push_str("[log]\nLogToConsole = yes\nINPUT = yes\nVerbosity = 3\n");
        let generated = generated.into_bytes();
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-flycast-config-")
            .tempdir()?;
        std::fs::create_dir(directory.path().join("flycast"))?;
        std::fs::write(directory.path().join("flycast/emu.cfg"), &generated)?;
        let prepared = Self {
            directory,
            source: source.to_owned(),
            canonical_source,
            original,
            generated,
        };
        prepared.verify_before_launch()?;
        Ok(prepared)
    }

    pub(crate) fn directory(&self) -> &Path {
        self.directory.path()
    }

    /// Native Linux startup appends /flycast to XDG_CONFIG_HOME. Do not change
    /// HOME, data directories, or cwd: those also affect saves and relative ROMs.
    /// This selects emu.cfg but does not isolate fallback mapping discovery.
    #[cfg(target_os = "linux")]
    pub(crate) fn select_for_launch(&self, plan: &mut crate::emulator::LaunchPlan) -> Result<()> {
        self.verify_before_launch()?;
        ensure!(
            plan.program.is_absolute() && plan.current_directory.is_absolute(),
            "Flycast native launch paths must be absolute"
        );
        ensure!(
            plan.retroarch_content.is_none(),
            "Cannot apply standalone Flycast config to RetroArch"
        );
        let key = std::ffi::OsStr::new("XDG_CONFIG_HOME");
        // A plan may already select the user's ordinary config directory; the
        // session intentionally replaces that selection with the private copy.
        plan.environment.retain(|(name, _)| name != key);
        plan.environment
            .push((key.to_owned(), self.directory().as_os_str().to_owned()));
        Ok(())
    }

    pub(crate) fn verify_source(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical_source
                && read_config(&self.source)? == self.original,
            "Flycast source config changed during preparation"
        );
        Ok(())
    }

    /// Native auto-save may legitimately rewrite the private file after startup.
    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        self.verify_source()?;
        let path = self.directory().join("flycast/emu.cfg");
        let metadata = std::fs::symlink_metadata(&path)?;
        ensure!(
            metadata.is_file()
                && metadata.len() == self.generated.len() as u64
                && path.canonicalize()? == self.directory().canonicalize()?.join("flycast/emu.cfg")
                && std::fs::read(&path)? == self.generated,
            "Flycast private config changed before launch"
        );
        Ok(())
    }
}
