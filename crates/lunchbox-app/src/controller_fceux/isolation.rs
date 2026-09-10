//! Private native config-layer files; keep the user's base directory and saves.
use super::{configuration::Selection, layers::Snapshot};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    snapshot: Snapshot,
    files: Vec<(PathBuf, PathBuf, String)>,
}

impl PreparedConfig {
    pub(crate) fn prepare(root: &Path, selections: &[Selection]) -> Result<Self> {
        let snapshot = Snapshot::capture(root)?;
        Self::from_snapshot(snapshot, selections)
    }

    pub(crate) fn from_snapshot(snapshot: Snapshot, selections: &[Selection]) -> Result<Self> {
        let replacements = snapshot.replacements(selections)?;
        let targets = snapshot.canonical_targets();
        // Two source layers may alias one file, but their distinct rewritten
        // contents cannot both be mounted on the same canonical target.
        ensure!(
            targets.iter().collect::<BTreeSet<_>>().len() == targets.len(),
            "FCEUX config layers alias the same file"
        );
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-fceux-config-")
            .tempdir()?;
        let mut files = Vec::new();
        for (index, ((_, text), target)) in replacements.into_iter().zip(targets).enumerate() {
            let private = directory.path().join(format!("layer-{index}.cfg"));
            // Native configSys opens layers for both reading and writing.
            std::fs::write(&private, &text)?;
            files.push((private, target, text));
        }
        let prepared = Self {
            directory,
            snapshot,
            files,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.snapshot.verify()?;
        for (private, _, text) in &self.files {
            ensure!(
                private.parent() == Some(self.directory.path())
                    && std::fs::symlink_metadata(private)?.file_type().is_file()
                    && std::fs::read(private)? == text.as_bytes(),
                "FCEUX private configuration changed during preparation"
            );
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn verify_child_mounts(&self, pid: u32) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        for (private, target, _) in &self.files {
            let mounted =
                PathBuf::from(format!("/proc/{pid}/root")).join(target.strip_prefix("/")?);
            let actual = std::fs::metadata(mounted)?;
            let expected = std::fs::metadata(private)?;
            ensure!(
                actual.dev() == expected.dev() && actual.ino() == expected.ino(),
                "FCEUX child does not see a private config layer"
            );
        }
        Ok(())
    }

    /// Append to a bubblewrap command after its root bind and before launch.
    /// This only supplies config mounts: the caller must also mount generated
    /// profiles, resolve native FCEUX_HOME, and validate device/ROM routing.
    pub(crate) fn append_mounts(&self, arguments: &mut Vec<OsString>) -> Result<()> {
        self.verify()?;
        for (private, target, _) in &self.files {
            arguments.extend([
                OsString::from("--bind"),
                private.as_os_str().to_owned(),
                target.as_os_str().to_owned(),
            ]);
        }
        Ok(())
    }
}
