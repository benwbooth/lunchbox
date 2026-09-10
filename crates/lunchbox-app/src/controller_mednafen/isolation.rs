//! Private native config-layer files; keep the user's base directory and saves.
use super::{Input, discovery::Discovered, profiles::Gamepad};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    snapshot: Discovered,
    files: Vec<(PathBuf, PathBuf, String)>,
}

impl PreparedConfig {
    pub(crate) fn prepare(
        snapshot: Discovered,
        gamepad: Gamepad,
        native_id: &str,
        controls: &BTreeMap<String, Input>,
    ) -> Result<Self> {
        let assignments = gamepad.assignments(native_id, controls)?;
        Self::prepare_assignments(snapshot, &assignments)
    }

    /// Own one immutable layer set for all selected native controller ports.
    pub(crate) fn prepare_assignments(
        snapshot: Discovered,
        assignments: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let replacements = snapshot.replacements(assignments)?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mednafen-config-")
            .tempdir()?;
        let mut files = Vec::new();
        for (index, (target, text)) in replacements.into_iter().enumerate() {
            let private = directory.path().join(format!("layer-{index}.cfg"));
            // The native global settings file is opened read/write and locked.
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
                "Mednafen private configuration changed during preparation"
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
                "Mednafen child does not see a private config layer"
            );
        }
        Ok(())
    }

    /// Append to a bubblewrap command after its root bind and before launch.
    /// This only supplies config mounts: the caller must also
    /// resolve native base/override paths and validate device/ROM routing.
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
