//! PUAE save-directory configuration inputs, without changing user files.
//! Source: libretro/libretro-uae 6536174a80d74e6c325aaa5390ff091fac8761d0,
//! libretro-core.c retro_set_paths, emu_config and retro_load_game configuration.
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) struct InputSnapshot {
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        for (path, expected) in &self.files {
            ensure!(
                read_configuration(path)? == *expected,
                "PUAE configuration changed during launch preparation: {}",
                path.display()
            );
        }
        Ok(())
    }
}

fn read_configuration(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Inspecting PUAE configuration {}", path.display()));
        }
        Ok(metadata) => ensure!(
            metadata.is_file() && metadata.len() <= 64 * 1024,
            "PUAE configuration requires a bounded regular file: {}",
            path.display()
        ),
    }
    Ok(Some(std::fs::read(path).with_context(|| {
        format!("Reading PUAE configuration {}", path.display())
    })?))
}

/// The caller must supply RetroArch's effective GET_SAVE_DIRECTORY value, not
/// merely its base savefile_directory setting. Sorted/content save paths and
/// Flatpak namespace translation must be resolved before calling this function.
pub(crate) fn prepare(save_directory: &Path, content: &Path, model: &str) -> Result<InputSnapshot> {
    ensure!(
        save_directory.is_absolute(),
        "PUAE requires an absolute effective save directory"
    );
    ensure!(
        matches!(
            model,
            "A500"
                | "A500OG"
                | "A500PLUS"
                | "A600"
                | "A1200"
                | "A1200OG"
                | "A2000"
                | "A2000OG"
                | "A4030"
                | "A4040"
                | "CDTV"
                | "CD32"
                | "CD32FR"
        ),
        "PUAE requires an explicit reviewed machine model"
    );
    let basename = content
        .file_stem()
        .and_then(|name| name.to_str())
        .context("PUAE requires an exact UTF-8 content basename")?;
    ensure!(
        !basename.is_empty() && basename.is_ascii() && !basename.chars().any(char::is_control),
        "PUAE content basename requires native-encoding resolution"
    );

    // retro_set_paths removes a trailing slash, then strips a one-byte final
    // directory component to share WHDLoad images across alphabetical folders.
    let mut directory = save_directory.to_path_buf();
    if directory
        .file_name()
        .is_some_and(|name| name.as_encoded_bytes().len() == 1)
    {
        directory = directory
            .parent()
            .context("PUAE save-directory normalization has no parent")?
            .to_path_buf();
    }
    let paths = [
        directory.join(format!("puae_libretro_{model}.uae")),
        directory.join("puae_libretro_global.uae"),
        directory.join(format!("{basename}.uae")),
    ];
    let mut files = Vec::new();
    for path in paths {
        // PUAE truncates native paths into RETRO_PATH_MAX buffers. Do not check
        // a different untruncated file and then infer that the core sees it.
        ensure!(
            path.as_os_str().as_encoded_bytes().len() < 512,
            "PUAE custom configuration path exceeds the reviewed bound"
        );
        let bytes = read_configuration(&path)?;
        if let Some(bytes) = &bytes {
            let text =
                std::str::from_utf8(bytes).context("PUAE custom configuration is not UTF-8")?;
            ensure!(
                text.lines().all(|line| {
                    let line = line.trim();
                    line.is_empty() || line.starts_with(';')
                }),
                "PUAE custom configuration needs effective input resolution before calibrated launch: {}. The file has not been changed.",
                path.display()
            );
        }
        files.push((path, bytes));
    }
    Ok(InputSnapshot { files })
}
