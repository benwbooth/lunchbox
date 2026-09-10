//! ep128emu default input prerequisites; never rewrite native configuration.
use anyhow::{Context, Result, ensure};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(crate) struct InputSnapshot {
    content: PathBuf,
    header: [u8; 64],
    absent: Vec<PathBuf>,
    system: PathBuf,
}

fn header(path: &Path) -> Result<[u8; 64]> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Inspecting ep128emu media: {}", path.display()))?;
    ensure!(
        metadata.is_file() && metadata.len() >= 64,
        "ep128emu input resolution requires regular media containing at least 64 header bytes: {}",
        path.display()
    );
    let mut bytes = [0; 64];
    std::fs::File::open(path)?
        .read_exact(&mut bytes)
        .with_context(|| format!("Reading ep128emu media header: {}", path.display()))?;
    Ok(bytes)
}

fn require_absent(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("Inspecting {}", path.display())),
        Ok(_) => anyhow::bail!(
            "ep128emu default controller mode requires configuration resolution for {}; use native setup until custom configuration mapping is supported",
            path.display()
        ),
    }
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            header(&self.content)? == self.header,
            "ep128emu content header changed during launch preparation"
        );
        ensure!(
            self.system.is_dir(),
            "ep128emu system directory disappeared"
        );
        for path in &self.absent {
            require_absent(path)?;
        }
        Ok(())
    }

    pub(crate) fn append_config(&self) -> Result<String> {
        let path = self
            .system
            .to_str()
            .context("ep128emu system directory must be UTF-8")?;
        ensure!(
            !path.chars().any(|c| c.is_control() || "\"\\".contains(c)),
            "ep128emu system directory cannot be represented in RetroArch configuration"
        );
        Ok(format!("system_directory = \"{path}\"\n"))
    }
}

pub(crate) fn prepare_cpc(system: &Path, content: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir() && content.is_absolute(),
        "ep128emu needs existing absolute system and content paths"
    );
    let bytes = header(content)?;
    let extension = content
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // main.cpp checks CPC disk signatures before TZX. TZX is CPC only with
    // the CDT extension; the same bytes named TZX would select ZX Spectrum.
    let cpc_disk = extension == "dsk"
        && (bytes.starts_with(b"MV - CPCEMU") || bytes.starts_with(b"EXTENDED CPC DSK File"));
    let cpc_tape = extension == "cdt" && bytes.starts_with(b"ZXTape!\x1a\x01");
    ensure!(
        cpc_disk || cpc_tape,
        "ep128emu CPC joystick mode requires a CPC DSK header or a CDT tape with TZX v1 signature"
    );
    snapshot(system, content, bytes, "cpc")
}

pub(crate) fn prepare_enterprise(system: &Path, content: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir() && content.is_absolute(),
        "ep128emu needs existing absolute system and content paths"
    );
    let bytes = header(content)?;
    ensure!(
        content
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tap"))
            && bytes.starts_with(b"\x02\x75\xcd\x72\x1c\x44\x51\x26"),
        "ep128emu Enterprise default mode requires a native ep128emu TAP signature"
    );
    snapshot(system, content, bytes, "enterprise")
}

pub(crate) fn prepare_zx(system: &Path, content: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir() && content.is_absolute(),
        "ep128emu needs existing absolute system and content paths"
    );
    let bytes = header(content)?;
    let extension = content
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let tzx = extension == "tzx" && bytes.starts_with(b"ZXTape!\x1a\x01");
    // Reproduce zx_header_match rather than accepting all TAP files: the core
    // sends nonmatching TAP content to Enterprise tape mode.
    let tap = extension == "tap"
        && bytes[0] > 0x0e
        && bytes[0] < 0x22
        && bytes[1] == 0
        && matches!(bytes[2], 0 | 0xff);
    ensure!(
        tzx || tap,
        "ep128emu ZX default mode requires TZX v1 content or a TAP header matching the core's ZX detection"
    );
    snapshot(system, content, bytes, "zx")
}

pub(crate) fn prepare_tvc(system: &Path, content: &Path) -> Result<InputSnapshot> {
    ensure!(
        system.is_absolute() && system.is_dir() && content.is_absolute(),
        "ep128emu needs existing absolute system and content paths"
    );
    let bytes = header(content)?;
    let extension = content
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let tape = extension == "tvcwav" && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE";
    let cartridge = extension == "crt" && bytes.starts_with(b"MOPS");
    ensure!(
        tape || cartridge,
        "ep128emu TVC default mode requires RIFF/WAVE TVCWAV tape or MOPS CRT cartridge content"
    );
    snapshot(system, content, bytes, "tvc")
}

fn snapshot(system: &Path, content: &Path, bytes: [u8; 64], family: &str) -> Result<InputSnapshot> {
    let parent = content
        .parent()
        .context("ep128emu content directory is missing")?;
    let mut absent = vec![
        content.with_extension("ep128cfg"),
        system.join(format!("ep128emu/config/{family}.ep128cfg")),
    ];
    // All four markers can change VM selection, even when not ultimately loaded.
    for name in ["tvc", "cpc", "zx", "enterprise"] {
        absent.push(parent.join(format!("{name}.ep128cfg")));
    }
    for path in &absent {
        require_absent(path)?;
    }
    let snapshot = InputSnapshot {
        content: content.to_path_buf(),
        header: bytes,
        absent,
        system: system.to_path_buf(),
    };
    snapshot.append_config()?;
    Ok(snapshot)
}
