//! Linux topology guard for index-bound native SDL2 controller sessions.
//! This observes kernel-node changes; it does not replace a same-runtime SDL
//! inventory check or promise zero latency between hotplug and the next poll.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
struct Node {
    kernel_device: PathBuf,
    rdev: u64,
    filesystem: u64,
    inode: u64,
}

pub(crate) struct InputTopology {
    nodes: BTreeMap<PathBuf, Node>,
}

fn snapshot() -> Result<BTreeMap<PathBuf, Node>> {
    let root = Path::new("/dev/input");
    let mut nodes = BTreeMap::new();
    for entry in std::fs::read_dir(root).context("Reading native controller topology")? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(index) = name
            .strip_prefix("event")
            .or_else(|| name.strip_prefix("js"))
        else {
            continue;
        };
        if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        ensure!(
            nodes.len() < 4096,
            "Native input topology exceeds the supported bound"
        );
        let metadata = entry.metadata()?;
        ensure!(
            metadata.file_type().is_char_device(),
            "Native input node is not a character device"
        );
        let sys = Path::new("/sys/class/input").join(name);
        let identity = std::fs::canonicalize(sys.join("device"))?;
        let expected = std::fs::read_to_string(sys.join("dev"))?;
        ensure!(
            expected.trim()
                == format!(
                    "{}:{}",
                    libc::major(metadata.rdev()),
                    libc::minor(metadata.rdev())
                ),
            "Native input node changed while recording topology"
        );
        nodes.insert(
            entry.path(),
            Node {
                kernel_device: identity,
                rdev: metadata.rdev(),
                filesystem: metadata.dev(),
                inode: metadata.ino(),
            },
        );
    }
    Ok(nodes)
}

impl InputTopology {
    /// Match SDL-reported nodes by kernel input identity, never numeric suffix.
    pub(crate) fn resolve_runtime_path<'a>(
        &self,
        selected: &Path,
        runtime_paths: impl IntoIterator<Item = &'a str>,
    ) -> Result<String> {
        self.verify()?;
        let selected_node = self
            .nodes
            .get(selected)
            .context("Selected controller is absent from captured topology")?;
        let mut matches = std::collections::BTreeSet::new();
        for path in runtime_paths {
            if let Some(node) = self.nodes.get(Path::new(path))
                && node.kernel_device == selected_node.kernel_device
            {
                matches.insert(path);
            }
        }
        ensure!(
            matches.len() == 1,
            "Selected controller must match exactly one SDL2 runtime node; found {} (HID backends need separate identity translation)",
            matches.len()
        );
        let resolved = matches
            .into_iter()
            .next()
            .context("No matching SDL2 controller")?
            .to_owned();
        self.verify()?;
        Ok(resolved)
    }

    /// Capture before probing SDL, then verify again after the probe. Observe
    /// all input nodes: an unselected pad's removal can renumber selected pads.
    pub(crate) fn capture(selected_paths: &[PathBuf]) -> Result<Self> {
        ensure!(
            !selected_paths.is_empty(),
            "No native controller paths selected"
        );
        let nodes = snapshot()?;
        for path in selected_paths {
            ensure!(
                nodes.contains_key(path),
                "Selected native input path is not in kernel topology: {}",
                path.display()
            );
        }
        let guard = Self { nodes };
        guard.verify()?;
        Ok(guard)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            snapshot()? == self.nodes,
            "Native input devices changed; SDL2 controller numbering must be resolved again"
        );
        Ok(())
    }
}
