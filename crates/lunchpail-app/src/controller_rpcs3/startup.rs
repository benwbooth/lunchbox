//! Pinned SDL handler startup records. Input must belong to this child launch.
use anyhow::{Context, Result, ensure};

/// Read the log actually held open by the child, not a guessed cache path or
/// a previous launch's file. The caller first verifies the PID's executable.
#[cfg(target_os = "linux")]
pub(crate) fn child_log(pid: u32) -> Result<Option<String>> {
    use std::{io::Read, path::Path};
    let mut selected = None;
    for entry in std::fs::read_dir(format!("/proc/{pid}/fd"))? {
        let entry = entry?;
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        if target.file_name() != Some(std::ffi::OsStr::new("RPCS3.log")) {
            continue;
        }
        ensure!(
            selected.is_none(),
            "RPCS3 has multiple candidate startup logs open"
        );
        let file = std::fs::File::open(entry.path())?;
        ensure!(
            file.metadata()?.is_file() && Path::new(&target).is_absolute(),
            "RPCS3 startup log is not a regular absolute file"
        );
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 4 * 1024 * 1024,
            "RPCS3 startup log exceeds limit"
        );
        let end = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        selected = Some(std::str::from_utf8(&bytes[..end])?.to_owned());
    }
    Ok(selected)
}

/// The unique generated path binds confirmation to this preparation. Later
/// profile switches must not silently retain a successful earlier confirmation.
pub(crate) fn loaded_profile(log: &str, expected: &std::path::Path) -> Result<bool> {
    ensure!(
        log.len() <= 4 * 1024 * 1024,
        "RPCS3 startup log exceeds limit"
    );
    let expected = expected
        .to_str()
        .context("RPCS3 profile path is not UTF-8")?;
    let mut selected = false;
    for line in log
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        let Some((_, value)) = line.split_once("Loading input configuration: '") else {
            continue;
        };
        let value = value.trim_end_matches(['\r', '\n']);
        let value = value
            .strip_suffix('\'')
            .context("Malformed RPCS3 profile log record")?;
        ensure!(
            !selected || value == expected,
            "RPCS3 switched away from the generated input profile"
        );
        selected = value == expected;
    }
    Ok(selected)
}

pub(crate) struct Opened {
    pub instance: u32,
    pub name: String,
    pub path: String,
}

pub(crate) fn opened_gamepads(log: &str) -> Result<Vec<Opened>> {
    ensure!(
        log.len() <= 4 * 1024 * 1024,
        "RPCS3 startup log exceeds limit"
    );
    let mut opened = Vec::new();
    for line in log
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        let Some((_, record)) = line.split_once("Found game pad ") else {
            continue;
        };
        let (instance, record) = record
            .split_once(": type=")
            .context("Malformed RPCS3 gamepad record")?;
        let instance: u32 = instance.parse()?;
        ensure!(instance != 0, "Invalid RPCS3 native instance");
        let (_, record) = record
            .split_once(", name='")
            .context("RPCS3 gamepad name missing")?;
        let (name, record) = record
            .split_once("', guid='")
            .context("RPCS3 gamepad GUID missing")?;
        let (guid, record) = record
            .split_once("', path='")
            .context("RPCS3 gamepad path missing")?;
        ensure!(
            guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Malformed RPCS3 gamepad GUID"
        );
        let (path, _) = record
            .split_once("', serial='")
            .context("RPCS3 gamepad serial field missing")?;
        ensure!(
            !path.is_empty()
                && !name.chars().any(char::is_control)
                && !path.chars().any(char::is_control),
            "Invalid RPCS3 gamepad metadata"
        );
        ensure!(opened.len() < 256, "Too many RPCS3 startup gamepads");
        opened.push(Opened {
            instance,
            name: name.to_owned(),
            path: path.to_owned(),
        });
    }
    Ok(opened)
}

/// Returns false while the complete expected enumeration has not arrived.
/// Compare by physical path and native name; process-local instance IDs need
/// not match those from the helper process.
pub(crate) fn confirm_routing(log: &str, expected: &[super::routing::Assignment]) -> Result<bool> {
    let opened = opened_gamepads(log)?;
    ensure!(
        opened.len() <= expected.len(),
        "RPCS3 opened unexpected or repeated gamepads"
    );
    let devices = opened
        .iter()
        .map(|device| super::routing::Device {
            instance: device.instance,
            name: &device.name,
            path: &device.path,
        })
        .collect::<Vec<_>>();
    let actual = super::routing::project(&devices)?;
    for assignment in &actual {
        ensure!(
            expected
                .iter()
                .any(|candidate| candidate.path == assignment.path
                    && candidate.native_device == assignment.native_device),
            "RPCS3 native gamepad naming differs from prepared mappings"
        );
    }
    Ok(actual.len() == expected.len())
}
