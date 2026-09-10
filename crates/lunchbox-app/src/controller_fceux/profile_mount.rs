//! Preserve native input profiles in a private directory overlay.
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    path::{Path, PathBuf},
};

type Tree = BTreeMap<PathBuf, Option<Vec<u8>>>;

fn capture(root: &Path) -> Result<Tree> {
    let mut tree = Tree::new();
    let mut pending = vec![(PathBuf::new(), 0)];
    let mut total = 0;
    while let Some((relative, depth)) = pending.pop() {
        ensure!(depth <= 16, "FCEUX input directory nesting exceeds limit");
        for entry in std::fs::read_dir(root.join(&relative))? {
            let entry = entry?;
            let path = relative.join(entry.file_name());
            let kind = entry.file_type()?;
            ensure!(
                kind.is_dir() || kind.is_file(),
                "FCEUX input overlay requires regular files and directories, not symlinks"
            );
            if kind.is_dir() {
                pending.push((path.clone(), depth + 1));
                tree.insert(path, None);
            } else {
                let mut bytes = Vec::new();
                std::fs::File::open(entry.path())?
                    .take(2 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                total += bytes.len();
                ensure!(
                    bytes.len() <= 2 * 1024 * 1024 && total <= 32 * 1024 * 1024,
                    "FCEUX input profiles exceed size limit"
                );
                tree.insert(path, Some(bytes));
            }
            ensure!(
                tree.len() <= 4096,
                "FCEUX input profiles exceed entry limit"
            );
        }
    }
    Ok(tree)
}

pub(crate) struct PreparedProfiles {
    directory: tempfile::TempDir,
    source: PathBuf,
    canonical: PathBuf,
    original: Tree,
    generated: BTreeMap<PathBuf, String>,
}

impl PreparedProfiles {
    /// Profile text must come from the calibrated native profile renderer.
    pub(crate) fn prepare(
        root: &Path,
        profiles: &[(super::configuration::Selection, String)],
    ) -> Result<Self> {
        ensure!(
            root.is_absolute() && !profiles.is_empty() && profiles.len() <= 4,
            "FCEUX needs an absolute base and one to four profiles"
        );
        let source = root.join("input");
        let canonical = source.canonicalize()?;
        let original = capture(&source)?;
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-fceux-input-")
            .tempdir()?;
        for (relative, bytes) in &original {
            let path = directory.path().join(relative);
            if let Some(bytes) = bytes {
                std::fs::create_dir_all(path.parent().unwrap())?;
                std::fs::write(path, bytes)?;
            } else {
                std::fs::create_dir_all(path)?;
            }
        }
        let mut generated = BTreeMap::new();
        let mut players = std::collections::BTreeSet::new();
        for (selection, text) in profiles {
            // Reuse config validation for GUID and safe basename boundaries.
            super::configuration::render("", std::slice::from_ref(selection))?;
            ensure!(
                players.insert(selection.player),
                "Duplicate FCEUX profile player"
            );
            ensure!(
                !text.contains('\0')
                    && text.lines().count() == 4
                    && text.split_inclusive('\n').all(|line| line.len() <= 255),
                "Invalid generated FCEUX profile records"
            );
            let relative =
                PathBuf::from(&selection.guid).join(format!("{}.txt", selection.profile));
            ensure!(
                !original.contains_key(&relative) && !generated.contains_key(&relative),
                "FCEUX session profile must have a unique private basename"
            );
            let path = directory.path().join(&relative);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, text)?;
            generated.insert(relative, text.clone());
        }
        let prepared = Self {
            directory,
            source,
            canonical,
            original,
            generated,
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.source.canonicalize()? == self.canonical
                && capture(&self.source)? == self.original,
            "FCEUX input profiles changed during preparation"
        );
        for (relative, expected) in &self.generated {
            ensure!(
                std::fs::read(self.directory.path().join(relative))? == expected.as_bytes(),
                "FCEUX generated profile changed during preparation"
            );
        }
        Ok(())
    }

    pub(crate) fn append_mounts(&self, arguments: &mut Vec<OsString>) -> Result<()> {
        self.verify()?;
        arguments.extend([
            OsString::from("--bind"),
            self.directory.path().as_os_str().to_owned(),
            self.canonical.as_os_str().to_owned(),
        ]);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn verify_child_mounts(&self, pid: u32) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        let root =
            PathBuf::from(format!("/proc/{pid}/root")).join(self.canonical.strip_prefix("/")?);
        for relative in
            std::iter::once(Path::new("")).chain(self.generated.keys().map(PathBuf::as_path))
        {
            let actual = std::fs::metadata(root.join(relative))?;
            let expected = std::fs::metadata(self.directory.path().join(relative))?;
            ensure!(
                actual.dev() == expected.dev() && actual.ino() == expected.ino(),
                "FCEUX child does not see its private input profiles"
            );
        }
        Ok(())
    }
}
