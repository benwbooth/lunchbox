//! Private directory mounts keep native Dolphin's user/save paths unchanged.
use super::configuration::PreparedConfiguration;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

type Tree = BTreeMap<PathBuf, Option<Vec<u8>>>;

fn capture(root: &Path) -> Result<Tree> {
    ensure!(
        root.is_dir(),
        "Dolphin Config and GameSettings directories must already exist"
    );
    let mut result = Tree::new();
    let mut pending = vec![(PathBuf::new(), 0usize)];
    let mut size = 0usize;
    while let Some((relative, depth)) = pending.pop() {
        ensure!(
            depth <= 32,
            "Dolphin settings directory nesting exceeds limit"
        );
        for entry in std::fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            let kind = entry.file_type()?;
            ensure!(
                !kind.is_symlink(),
                "Dolphin private settings currently require non-symlink entries: {}",
                entry.path().display()
            );
            if kind.is_dir() {
                result.insert(path.clone(), None);
                pending.push((path, depth + 1));
            } else {
                ensure!(
                    kind.is_file(),
                    "Dolphin settings contain a non-regular file"
                );
                let mut bytes = Vec::new();
                std::fs::File::open(entry.path())?
                    .take(8 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                ensure!(
                    bytes.len() <= 8 * 1024 * 1024,
                    "Dolphin settings file exceeds limit"
                );
                size += bytes.len();
                ensure!(
                    size <= 64 * 1024 * 1024,
                    "Dolphin settings tree exceeds limit"
                );
                result.insert(path, Some(bytes));
            }
            ensure!(
                result.len() <= 10000,
                "Dolphin settings tree has too many entries"
            );
        }
    }
    Ok(result)
}

pub(crate) struct PrivateConfiguration {
    directory: tempfile::TempDir,
    prepared: PreparedConfiguration,
    trees: Vec<(PathBuf, PathBuf, Tree)>,
}

impl PrivateConfiguration {
    pub(crate) fn materialize(user: &Path, prepared: PreparedConfiguration) -> Result<Self> {
        prepared.verify()?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-dolphin-")
            .tempdir()?;
        let mut trees = Vec::new();
        for name in ["Config", "GameSettings"] {
            let original = user.join(name);
            let canonical = original.canonicalize()?;
            let tree = capture(&original)?;
            let private = directory.path().join(name);
            std::fs::create_dir(&private)?;
            for (relative, bytes) in &tree {
                let target = private.join(relative);
                if let Some(bytes) = bytes {
                    std::fs::create_dir_all(
                        target
                            .parent()
                            .context("Dolphin settings parent is absent")?,
                    )?;
                    std::fs::write(target, bytes)?;
                } else {
                    std::fs::create_dir_all(target)?;
                }
            }
            trees.push((original, canonical, tree));
        }
        ensure!(
            !trees[0].1.starts_with(&trees[1].1) && !trees[1].1.starts_with(&trees[0].1),
            "Dolphin Config and GameSettings roots must not overlap"
        );
        for (target, bytes) in &prepared.replacements {
            let relative = target
                .strip_prefix(user)
                .context("Dolphin replacement is outside user settings")?;
            ensure!(
                relative
                    .components()
                    .all(|part| matches!(part, std::path::Component::Normal(_)))
                    && ["Config", "GameSettings"]
                        .iter()
                        .any(|name| relative.starts_with(name)),
                "Dolphin replacement is outside private settings roots"
            );
            let target = directory.path().join(relative);
            std::fs::create_dir_all(
                target
                    .parent()
                    .context("Dolphin replacement parent is absent")?,
            )?;
            std::fs::write(target, bytes)?;
        }
        let result = Self {
            directory,
            prepared,
            trees,
        };
        result.verify()?;
        Ok(result)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.prepared.verify()?;
        for (original, canonical, expected) in &self.trees {
            ensure!(
                original.canonicalize()? == *canonical && capture(original)? == *expected,
                "Dolphin settings changed during launch preparation"
            );
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn verify_child_mounts(&self, pid: u32) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        for (original, canonical, _) in &self.trees {
            let private = self.directory.path().join(
                original
                    .file_name()
                    .context("Missing settings directory name")?,
            );
            let mounted =
                PathBuf::from(format!("/proc/{pid}/root")).join(canonical.strip_prefix("/")?);
            let expected = std::fs::metadata(private)?;
            let actual = std::fs::metadata(mounted)?;
            ensure!(
                actual.dev() == expected.dev() && actual.ino() == expected.ino(),
                "Dolphin child does not see its private settings directory"
            );
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn overlay_arguments(
        &self,
        executable: &Path,
        arguments: &[OsString],
        cwd: &Path,
    ) -> Result<Vec<OsString>> {
        self.verify()?;
        ensure!(
            executable.is_absolute() && cwd.is_absolute(),
            "Dolphin needs absolute launch paths"
        );
        let mut result = vec![
            "--die-with-parent".into(),
            "--bind".into(),
            "/".into(),
            "/".into(),
        ];
        for (original, canonical, _) in &self.trees {
            let name = original
                .file_name()
                .context("Dolphin settings directory name is absent")?;
            result.extend([
                OsString::from("--bind"),
                self.directory.path().join(name).into_os_string(),
                canonical.as_os_str().to_owned(),
            ]);
        }
        result.extend([
            OsString::from("--chdir"),
            cwd.as_os_str().to_owned(),
            OsString::from("--"),
            executable.as_os_str().to_owned(),
        ]);
        result.extend_from_slice(arguments);
        Ok(result)
    }
}
