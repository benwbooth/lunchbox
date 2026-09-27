//! Integrity checks for the opt-in SimCoupe controller core.
//! A matching sidecar is a local package declaration, not a trusted signature
//! or proof that this core has passed runtime validation.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u32,
    contract: String,
    source_revision: String,
    library_name: String,
    library_version: String,
    core_sha256: String,
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    canonical: PathBuf,
    digest: [u8; 32],
}

fn inspect(path: &Path, maximum: u64, retain: bool) -> Result<(FileStamp, Vec<u8>)> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("Resolving SimCoupe artifact {}", path.display()))?;
    let mut file = std::fs::File::open(&canonical)?;
    let length = file.metadata()?;
    ensure!(
        length.is_file() && length.len() > 0 && length.len() <= maximum,
        "SimCoupe artifact must be a nonempty bounded regular file: {}",
        path.display()
    );
    let mut digest = Sha256::new();
    let mut bytes = Vec::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure!(
            total <= maximum,
            "SimCoupe artifact grew beyond its size limit"
        );
        digest.update(&buffer[..count]);
        if retain {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    ensure!(
        total == length.len() && path.canonicalize()? == canonical,
        "SimCoupe artifact changed during preparation"
    );
    Ok((
        FileStamp {
            canonical,
            digest: digest.finalize().into(),
        },
        bytes,
    ))
}

pub(crate) struct ArtifactSnapshot {
    core_path: PathBuf,
    receipt_path: PathBuf,
    core: FileStamp,
    receipt: FileStamp,
}

#[derive(Clone, Copy)]
pub(crate) enum JoystickInterface {
    SamOne = 1,
    SamTwo = 2,
    Kempston = 3,
}

pub(crate) struct ConfigSnapshot {
    home: PathBuf,
    canonical_home: PathBuf,
    path: PathBuf,
    stamp: FileStamp,
}

pub(crate) struct PreparedInput {
    artifact: ArtifactSnapshot,
    config: ConfigSnapshot,
}

impl PreparedInput {
    pub(crate) fn verify(&self) -> Result<()> {
        self.artifact.verify()?;
        self.config.verify()
    }
}

pub(crate) fn prepare_input(
    core: &Path,
    home: &Path,
    interface: JoystickInterface,
) -> Result<PreparedInput> {
    let prepared = PreparedInput {
        artifact: prepare(core)?,
        config: prepare_config(home, interface)?,
    };
    prepared.verify()?;
    Ok(prepared)
}

impl ConfigSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.home.is_dir() && self.home.canonicalize()? == self.canonical_home,
            "SimCoupe HOME changed during controller preparation"
        );
        ensure!(
            inspect(&self.path, 1024 * 1024, false)?.0 == self.stamp,
            "SimCoupe native configuration changed during controller preparation"
        );
        Ok(())
    }
}

pub(crate) fn prepare_config(home: &Path, interface: JoystickInterface) -> Result<ConfigSnapshot> {
    ensure!(
        home.is_absolute() && home.is_dir(),
        "SimCoupe requires an existing absolute HOME"
    );
    let path = home.join(".simcoupe/SimCoupe.cfg");
    let (stamp, bytes) = inspect(&path, 1024 * 1024, true)
        .context("Initialize SimCoupe using native setup before applying controller mappings")?;
    let text =
        std::str::from_utf8(&bytes).context("SimCoupe native configuration must be UTF-8")?;
    ensure!(
        !text.starts_with('\u{feff}') && !text.contains('\0'),
        "Ambiguous SimCoupe native configuration"
    );
    let mut version = None;
    let mut first = None;
    let mut second = None;
    let mut drive = None;
    for line in text.split_inclusive('\n') {
        ensure!(
            line.len() < 255,
            "SimCoupe config line exceeds the core parser's supported size"
        );
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim_matches(|c: char| c.is_ascii_whitespace());
        ensure!(
            !key.is_empty() && !key.chars().any(|c| c.is_ascii_whitespace()),
            "Ambiguous SimCoupe configuration key"
        );
        let target = if key.eq_ignore_ascii_case("CfgVersion") {
            &mut version
        } else if key.eq_ignore_ascii_case("JoyType1") {
            &mut first
        } else if key.eq_ignore_ascii_case("JoyType2") {
            &mut second
        } else if key.eq_ignore_ascii_case("Drive1") {
            &mut drive
        } else {
            continue;
        };
        let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
        let parsed: i32 = value
            .parse()
            .context("SimCoupe input settings require plain integers")?;
        ensure!(
            target.is_none(),
            "Repeated SimCoupe input setting requires native configuration cleanup"
        );
        *target = Some(parsed);
    }
    ensure!(
        version == Some(4),
        "SimCoupe requires CfgVersion = 4; other versions reset native settings"
    );
    ensure!(
        drive == Some(1),
        "SimCoupe requires Drive1 = 1 (floppy) for this disk contract"
    );
    ensure!(
        first == Some(interface as i32) && second == Some(0),
        "SimCoupe JoyType1 must match the selected interface and JoyType2 must be 0; native settings were not changed"
    );
    let snapshot = ConfigSnapshot {
        home: home.to_owned(),
        canonical_home: home.canonicalize()?,
        path,
        stamp,
    };
    snapshot.verify()?;
    Ok(snapshot)
}

impl ArtifactSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            inspect(&self.core_path, 256 * 1024 * 1024, false)?.0 == self.core,
            "SimCoupe core changed after controller preparation"
        );
        ensure!(
            inspect(&self.receipt_path, 16 * 1024, false)?.0 == self.receipt,
            "SimCoupe controller contract metadata changed after preparation"
        );
        Ok(())
    }
}

pub(crate) fn prepare(core_path: &Path) -> Result<ArtifactSnapshot> {
    ensure!(
        core_path.is_absolute(),
        "SimCoupe core path must be absolute"
    );
    let mut receipt_name = core_path.as_os_str().to_owned();
    receipt_name.push(".controller.json");
    let receipt_path = PathBuf::from(receipt_name);
    let (receipt, bytes) = inspect(&receipt_path, 16 * 1024, true)
        .context("This mode requires the opt-in lunchbox-simcp-controller1 package; the upstream core lacks fire-button forwarding")?;
    let parsed: Receipt =
        serde_json::from_slice(&bytes).context("Invalid SimCoupe controller contract metadata")?;
    ensure!(
        parsed.schema_version == 1
            && parsed.contract == "lunchbox-simcp-controller1"
            && parsed.source_revision == "c28046241ac6a4d79e55326b6e354dc02f92fa34"
            && parsed.library_name == "SimCoupe"
            && parsed.library_version == "v1-lunchbox-controller1",
        "Unsupported SimCoupe controller artifact contract"
    );
    ensure!(
        parsed.core_sha256.len() == 64
            && parsed
                .core_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "SimCoupe artifact hash must be a lowercase SHA-256 digest"
    );
    let (core, _) = inspect(core_path, 256 * 1024 * 1024, false)?;
    let actual = core
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ensure!(
        actual == parsed.core_sha256,
        "SimCoupe library does not match its controller package metadata"
    );
    let snapshot = ArtifactSnapshot {
        core_path: core_path.to_owned(),
        receipt_path,
        core,
        receipt,
    };
    snapshot.verify()?;
    Ok(snapshot)
}
