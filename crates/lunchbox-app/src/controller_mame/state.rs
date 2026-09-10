//! Enumerate declared persistent roots without following nested symlinks.
use anyhow::{Result, ensure};
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn persistent_inputs(
    paths: &super::PersistentPaths,
    cancel: &AtomicBool,
) -> Result<Vec<super::InspectionInput>> {
    let mut inputs = Vec::new();
    for (category, root) in [("nvram", &paths.nvram), ("diff", &paths.diff)] {
        ensure!(
            root.is_absolute() && root.is_dir() && root.canonicalize()? == *root,
            "Resolve the MAME persistent root before discovering state"
        );
        let mut pending = vec![(root.clone(), 0usize)];
        let mut entries_seen = 0usize;
        while let Some((directory, depth)) = pending.pop() {
            ensure!(
                !cancel.load(Ordering::Relaxed),
                "MAME state discovery cancelled"
            );
            ensure!(
                depth <= 64,
                "MAME state directory nesting exceeds its limit"
            );
            for entry in std::fs::read_dir(&directory)? {
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "MAME state discovery cancelled"
                );
                entries_seen += 1;
                ensure!(
                    entries_seen <= 16384,
                    "MAME state directory entry limit exceeded"
                );
                let entry = entry?;
                let path = entry.path();
                let kind = entry.file_type()?;
                ensure!(
                    !kind.is_symlink(),
                    "Nested MAME state symlink needs explicit resolution: {}",
                    path.display()
                );
                if kind.is_dir() {
                    pending.push((path, depth + 1));
                } else {
                    ensure!(
                        kind.is_file(),
                        "MAME state contains a non-regular file: {}",
                        path.display()
                    );
                    let relative = path.strip_prefix(root)?;
                    inputs.push(super::InspectionInput {
                        source: path.clone(),
                        destination: std::path::Path::new(category).join(relative),
                    });
                    ensure!(
                        inputs.len() <= 4096,
                        "MAME persistent state file limit exceeded"
                    );
                }
            }
        }
    }
    inputs.sort_by(|a, b| a.destination.cmp(&b.destination));
    Ok(inputs)
}
