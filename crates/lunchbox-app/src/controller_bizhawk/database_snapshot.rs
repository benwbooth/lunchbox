//! Ordered database includes for the pinned BizHawk cartridge identity path.
use super::cartridge_identity::{DatabaseInput, DatabaseMatch, DatabaseRecord, RecordIndex};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Component, Path, PathBuf},
};

/// Pins selected database roots and every consumed or optional missing file.
/// Does not establish that a running emulator used these roots or this build.
pub(crate) struct DatabaseSnapshot {
    bundled: PathBuf,
    bundled_target: PathBuf,
    user: PathBuf,
    user_target: PathBuf,
    absent: BTreeSet<PathBuf>,
    artifacts: Vec<super::RuntimeArtifact>,
    index: RecordIndex,
    bytes_read: usize,
    visits: usize,
}

impl DatabaseSnapshot {
    /// Use the explicit environment emitted by the direct-Mono invocation.
    /// PathUtils uses existing BIZHAWK_HOME / BIZHAWK_DATA_HOME directories;
    /// GameDBHelper appends gamedb to each. This entry point deliberately does
    /// not consult the host process environment or reproduce wrapper behavior.
    #[cfg(target_os = "linux")]
    pub(crate) fn prepare_for_environment(
        exe_directory: &Path,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<Self> {
        fn explicit_directory(
            environment: &[(std::ffi::OsString, std::ffi::OsString)],
            name: &str,
        ) -> Result<PathBuf> {
            let mut values = environment
                .iter()
                .filter(|(key, _)| key == std::ffi::OsStr::new(name));
            let (_, value) = values.next().with_context(|| {
                format!("Database preparation requires explicit {name} in the launch environment")
            })?;
            ensure!(
                values.next().is_none(),
                "Duplicate launch environment key {name}"
            );
            let text = value
                .to_str()
                .with_context(|| format!("{name} must be UTF-8"))?;
            ensure!(
                !text.is_empty() && !text.chars().any(char::is_control),
                "Invalid launch directory {name}"
            );
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute() && path.is_dir(),
                "{name} must name an existing absolute directory; runtime fallback is not accepted"
            );
            Ok(path)
        }
        ensure!(
            exe_directory.is_absolute() && exe_directory.is_dir(),
            "Database preparation requires the selected absolute installation directory"
        );
        let home = explicit_directory(environment, "BIZHAWK_HOME")?;
        let data = explicit_directory(environment, "BIZHAWK_DATA_HOME")?;
        ensure!(
            home.canonicalize()? == exe_directory.canonicalize()?,
            "Launch BIZHAWK_HOME differs from the selected installation"
        );
        // Keep requested paths, not just canonical targets: snapshot verify
        // must detect a symlink being retargeted after preparation.
        Self::prepare(&home.join("gamedb"), &data.join("gamedb"))
    }

    pub(crate) fn prepare(bundled: &Path, requested_user: &Path) -> Result<Self> {
        ensure!(
            bundled.is_absolute() && requested_user.is_absolute() && bundled.is_dir(),
            "Database snapshot requires absolute roots and an existing bundled directory"
        );
        let mut absent = BTreeSet::new();
        let user = match std::fs::symlink_metadata(requested_user) {
            Ok(_) => {
                ensure!(
                    requested_user.is_dir(),
                    "Selected user database root is not a directory"
                );
                requested_user.to_owned()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // InitializeDatabase falls back to bundledRoot in this case.
                absent.insert(requested_user.to_owned());
                bundled.to_owned()
            }
            Err(error) => return Err(error).context("Inspecting user database root"),
        };
        let mut snapshot = Self {
            bundled: bundled.to_owned(),
            bundled_target: bundled.canonicalize()?,
            user_target: user.canonicalize()?,
            user,
            absent,
            artifacts: Vec::new(),
            index: RecordIndex::default(),
            bytes_read: 0,
            visits: 0,
        };
        snapshot.load(
            &bundled.join("gamedb.txt"),
            false,
            false,
            &mut BTreeSet::new(),
        )?;
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.bundled.is_dir()
                && self.user.is_dir()
                && self.bundled.canonicalize()? == self.bundled_target
                && self.user.canonicalize()? == self.user_target,
            "BizHawk database root changed during preparation"
        );
        for path in &self.absent {
            match std::fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("Rechecking missing database input"),
                Ok(_) => anyhow::bail!(
                    "Previously absent database input appeared: {}",
                    path.display()
                ),
            }
        }
        for artifact in &self.artifacts {
            artifact.verify()?;
        }
        Ok(())
    }

    pub(crate) fn lookup(&self, input: &DatabaseInput) -> Result<Option<DatabaseMatch<'_>>> {
        self.verify()?;
        Ok(self.index.lookup(input))
    }

    fn load(
        &mut self,
        path: &Path,
        in_user: bool,
        optional: bool,
        stack: &mut BTreeSet<PathBuf>,
    ) -> Result<()> {
        self.visits += 1;
        ensure!(
            self.visits <= 1024 && stack.len() < 32,
            "Database include count or depth exceeded"
        );
        match std::fs::symlink_metadata(path) {
            Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => {
                self.absent.insert(path.to_owned());
                return Ok(());
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Reading database include {}", path.display()));
            }
            Ok(_) => {}
        }
        let artifact = super::RuntimeArtifact::capture(path)?;
        ensure!(
            stack.insert(artifact.resolved.clone()),
            "Database include cycle at {}",
            path.display()
        );
        const MAX_BYTES: usize = 128 * 1024 * 1024;
        let remaining = MAX_BYTES - self.bytes_read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(remaining as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= remaining,
            "Database include expansion exceeds 128 MiB"
        );
        self.bytes_read += bytes.len();
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        ensure!(
            digest == artifact.sha256,
            "Database bytes changed while parsing"
        );
        artifact.verify()?;
        // StreamReader recognizes a UTF-8 BOM. Other encodings fail explicitly
        // until their loader semantics are implemented, rather than lossy decode.
        let text = std::str::from_utf8(&bytes).context("Database input must be UTF-8")?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (line_number, line) in text.split(['\r', '\n']).enumerate() {
            ensure!(line.len() <= 1024 * 1024, "Database line exceeds 1 MiB");
            if line.starts_with(';') || line.trim().is_empty() {
                continue;
            }
            if line.starts_with('#') {
                let lower = line.to_ascii_lowercase();
                let user_include = lower.starts_with("#includeuser");
                if !user_include && !lower.starts_with("#include") {
                    continue;
                }
                // Pinned source skips the directive and one following byte.
                let offset = if user_include { 12 } else { 8 };
                let name = line
                    .get(offset..)
                    .context("Malformed database include directive")?
                    .trim_start();
                let relative = Path::new(name);
                ensure!(
                    !name.is_empty()
                        && !name.chars().any(char::is_control)
                        && !name.contains('\\')
                        && relative
                            .components()
                            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
                    "Unsupported database include path at {}:{}",
                    path.display(),
                    line_number + 1
                );
                let search_user = in_user || user_include;
                let root = if search_user {
                    &self.user
                } else {
                    &self.bundled
                };
                let included = root.join(relative);
                // Includes are rooted at the configured root, not this file's
                // parent. Missing includes are optional inside user databases.
                self.load(&included, search_user, in_user || user_include, stack)?;
            } else {
                let record = DatabaseRecord::parse(line).with_context(|| {
                    format!("Database record {}:{}", path.display(), line_number + 1)
                })?;
                self.index.insert(record)?;
            }
        }
        stack.remove(&artifact.resolved);
        self.artifacts.push(artifact);
        Ok(())
    }
}
