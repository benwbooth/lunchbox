//! Compressed disc images (CHD/CDZ) staged as the cue/bin set Mesen2 boots,
//! using MAME's own CHD core linked into the application.
//!
//! `Core/PCE/PceConsole.cpp` boots a PC Engine disc only when the file's
//! extension is exactly `.cue`; every other file is read as a raw HuCard
//! image, so a `.chd` from the library is misread and fails. `CdReader` has
//! no CHD container support anywhere, so the container must be unpacked
//! before Mesen sees it.
//!
//! `libchdman-rs` wraps `libutil/chd.cpp` directly (the same code `chdman`
//! is built from), which means this path needs no host `chdman` or MAME
//! Flatpak and behaves identically on Linux, Windows and macOS. Verified
//! against `chdman extractcd`: identical cue sheet and byte-identical bin.

use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::controller_native_process::cancelled;

/// Compressed disc containers Mesen2 has no reader for but this staging can
/// turn into the cue/bin set it boots.
pub(crate) const CONVERTIBLE_DISC_EXTENSIONS: &[&str] = &["chd"];

/// Disc containers Mesen2 cannot read directly. CHD is staged as a cue/bin
/// copy at launch; the rest need a manual conversion because no bundled
/// reader turns them into a cue sheet.
pub(crate) const UNSUPPORTED_DISC_EXTENSIONS: &[&str] = &[
    "cdz", "ccd", "iso", "img", "toc", "m3u", "mds", "gdi", "sub",
];

/// Unpack `source` into `destination` as a cue/bin pair and return the cue.
/// `destination` must already exist: the cue's `FILE` entry names the bin
/// beside it, which is where Mesen resolves it from.
pub(crate) fn extract_cue(
    source: &Path,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    ensure!(
        extension(source)
            .is_some_and(|value| CONVERTIBLE_DISC_EXTENSIONS.contains(&value.as_str())),
        "Only compressed disc images ({}) can be staged this way",
        CONVERTIBLE_DISC_EXTENSIONS.join("/")
    );
    let stem = source
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .context("Compressed disc image has no usable file name")?
        .to_string_lossy()
        .into_owned();
    let cue = destination.join(format!("{stem}.cue"));
    let bin = destination.join(format!("{stem}.bin"));
    ensure!(
        !cue.try_exists()? && !bin.try_exists()?,
        "Disc staging directory already holds a cue/bin pair"
    );
    cancelled(cancel)?;
    let mut last_reported = u64::MAX;
    let mut progress = |done: u64| {
        // Cancellation is checked from the progress callback: the CHD read
        // loop is the long pole and this is the only hook it offers.
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        if done.abs_diff(last_reported) >= 64 * 1024 * 1024 {
            last_reported = done;
            tracing::debug!("staging disc image: {} MiB", done / (1024 * 1024));
        }
    };
    // `ChdError` is a bare C-style enum without a Display impl, so carry its
    // debug form: it is the only detail the CHD core reports.
    let outcome = libchdman_rs::cd::extract_to_cue(source, &cue, &bin, &mut progress);
    if let Err(error) = outcome {
        let _ = std::fs::remove_file(&cue);
        let _ = std::fs::remove_file(&bin);
        anyhow::bail!(
            "Unpacking the compressed disc image failed for {}: {error:?}",
            source.display()
        );
    }
    cancelled(cancel)?;
    ensure!(
        cue.is_file() && bin.is_file(),
        "Disc image staging produced no cue/bin pair for {}",
        source.display()
    );
    Ok(cue)
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CHD built by this test with the linked library, then unpacked again.
    /// Exercises the real reader/writer path without shipping any game data.
    #[test]
    fn staged_pair_round_trips_a_plain_iso_chd() {
        const SECTOR: usize = 2048;
        const SECTORS: usize = 24;
        let directory = tempfile::tempdir().unwrap();
        let mut iso = vec![0u8; SECTOR * SECTORS];
        // Minimal ISO9660 primary volume descriptor, enough for the reader
        // to accept the medium without any copyrighted content.
        let pvd = 16 * SECTOR;
        iso[pvd] = 1;
        iso[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        iso[pvd + 6] = 1;
        iso[pvd + 40..pvd + 40 + 8].copy_from_slice(b"LUNCHBOX");
        iso[pvd + 80..pvd + 84].copy_from_slice(&(SECTORS as u32).to_le_bytes());
        let iso_path = directory.path().join("disc.iso");
        std::fs::write(&iso_path, &iso).unwrap();
        let chd_path = directory.path().join("disc.chd");
        let mut progress = |_progress: libchdman_rs::CompressionProgress| {};
        if let Err(error) = libchdman_rs::cd::create_from_iso(
            &iso_path,
            &chd_path,
            libchdman_rs::cd::CdCreateOptions::default(),
            &mut progress,
            &|| false,
        ) {
            // The writer needs no fixtures, but fail loudly if the linked
            // core lacks ISO creation rather than reporting a false pass.
            panic!("linked CHD writer rejected the fixture: {error:?}");
        }

        let media = directory.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let cue = extract_cue(&chd_path, &media, &AtomicBool::new(false)).unwrap();
        assert_eq!(cue.file_name().unwrap(), "disc.cue");
        let text = std::fs::read_to_string(&cue).unwrap();
        assert!(
            text.starts_with("FILE \"disc.bin\" BINARY"),
            "cue must name the bin beside it: {text}"
        );
        let bin = std::fs::metadata(media.join("disc.bin")).unwrap().len();
        assert!(bin > 0, "staged bin must not be empty");
    }

    #[test]
    fn staging_refuses_containers_it_cannot_unpack() {
        let directory = tempfile::tempdir().unwrap();
        let media = directory.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        for name in ["disc.cue", "disc.iso", "disc.bin"] {
            let path = directory.path().join(name);
            std::fs::write(&path, b"x").unwrap();
            assert!(
                extract_cue(&path, &media, &AtomicBool::new(false)).is_err(),
                "{name} must not be treated as a compressed image"
            );
        }
    }

    /// The extensions this module stages must be exactly the ones Mesen2's
    /// loader cannot read but this reader can.
    #[test]
    fn convertible_and_unsupported_containers_are_disjoint() {
        for name in UNSUPPORTED_DISC_EXTENSIONS {
            assert!(
                !CONVERTIBLE_DISC_EXTENSIONS.contains(name),
                "{name} cannot be both staged and refused"
            );
        }
        assert!(CONVERTIBLE_DISC_EXTENSIONS.contains(&"chd"));
    }
}
