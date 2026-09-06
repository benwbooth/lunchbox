//! Per-port libretro mode selection. Physical layouts never choose a cheaper
//! emulated device when the user's selected mode requires missing controls.
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::path::Path;

/// Confirm the effective core and content before using content-named options.
/// Subsystems, content substitution and unresolved custom flags are not guessed.
pub fn validate_arguments(
    arguments: &[OsString],
    prepared: &crate::emulator::PreparedRetroarchContent,
) -> Result<()> {
    ensure!(
        prepared.core.is_absolute() && prepared.content.is_absolute(),
        "RetroArch launch identity requires absolute prepared paths"
    );
    let mut core = false;
    let mut content = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.to_str() {
            Some("--verbose" | "-v" | "--fullscreen" | "-f") => {}
            Some("-L" | "--libretro") => {
                index += 1;
                ensure!(
                    !core
                        && arguments
                            .get(index)
                            .is_some_and(|path| path == prepared.core.as_os_str()),
                    "Custom command selects a different or duplicate RetroArch core"
                );
                core = true;
            }
            Some("--device" | "-d" | "--nodevice" | "-N" | "--dualanalog" | "-A") => {
                index += 1;
                ensure!(index < arguments.len(), "Missing controller-mode argument");
            }
            Some("--") => {
                ensure!(
                    !content
                        && arguments.len() == index + 2
                        && arguments[index + 1] == prepared.content.as_os_str(),
                    "Custom command changes RetroArch content"
                );
                content = true;
                break;
            }
            Some(text) if text.starts_with("--libretro=") || text.starts_with("-L") => {
                let path = text
                    .strip_prefix("--libretro=")
                    .or_else(|| text.strip_prefix("-L"))
                    .unwrap();
                ensure!(
                    !core && Path::new(path) == prepared.core,
                    "Custom command selects a different or duplicate RetroArch core"
                );
                core = true;
            }
            Some(text)
                if [
                    "--device=",
                    "--nodevice=",
                    "--dualanalog=",
                    "-d",
                    "-N",
                    "-A",
                ]
                .iter()
                .any(|prefix| text.starts_with(prefix) && text.len() > prefix.len()) => {}
            _ => {
                ensure!(
                    !content && argument == prepared.content.as_os_str(),
                    "Custom RetroArch arguments need content-identity resolution before calibrated launch"
                );
                content = true;
            }
        }
        index += 1;
    }
    ensure!(
        core && content,
        "Custom command omits the prepared RetroArch core or content"
    );
    Ok(())
}

pub fn configured_modes(config: &str, arguments: &[OsString], ports: usize) -> Result<Vec<u32>> {
    ensure!((1..=16).contains(&ports), "Invalid controller port count");
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configurations need effective controller-mode resolution"
    );
    let mut modes = vec![1; ports];
    for (index, mode) in modes.iter_mut().enumerate() {
        let key = format!("input_libretro_device_p{}", index + 1);
        let mut found = false;
        for line in config.lines().map(str::trim) {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() != key {
                continue;
            }
            ensure!(
                name.ends_with(|c: char| c.is_ascii_whitespace()),
                "RetroArch requires whitespace before '=' in {key}"
            );
            ensure!(
                !found,
                "Duplicate {key}; resolve the ambiguous controller mode first"
            );
            found = true;
            let value = value.trim();
            let value = if let Some(value) = value.strip_prefix('"') {
                let (number, rest) = value
                    .split_once('"')
                    .context("Unterminated controller mode")?;
                ensure!(
                    rest.trim().is_empty() || rest.trim_start().starts_with('#'),
                    "Invalid controller-mode suffix"
                );
                number
            } else {
                value.split('#').next().unwrap().trim()
            };
            *mode = number(value)?;
        }
    }
    let mut index = 0;
    while index < arguments.len() {
        let Some(argument) = arguments[index].to_str() else {
            index += 1;
            continue;
        };
        if argument == "--" {
            break;
        }
        let matched = [
            ("--device", "-d", None),
            ("--nodevice", "-N", Some(0)),
            ("--dualanalog", "-A", Some(5)),
        ]
        .iter()
        .find_map(|(long, short, fixed)| {
            if argument == *long || argument == *short {
                Some((*fixed, None))
            } else if let Some(value) = argument.strip_prefix(&format!("{long}=")) {
                Some((*fixed, Some(value)))
            } else {
                argument
                    .strip_prefix(short)
                    .filter(|v| !v.is_empty())
                    .map(|v| (*fixed, Some(v)))
            }
        });
        if let Some((fixed, attached)) = matched {
            let value = match attached {
                Some(value) => value,
                None => {
                    index += 1;
                    arguments
                        .get(index)
                        .and_then(|arg| arg.to_str())
                        .context("Missing controller-mode argument")?
                }
            };
            let (port, mode) = if let Some(mode) = fixed {
                (value, mode)
            } else {
                let (port, mode) = value.split_once(':').context("Use PORT:ID for --device")?;
                (port, number(mode)?)
            };
            let port = number(port)? as usize;
            ensure!(
                (1..=ports).contains(&port),
                "Controller mode selects a port outside this verified mode"
            );
            modes[port - 1] = mode;
        }
        index += 1;
    }
    Ok(modes)
}

fn number(value: &str) -> Result<u32> {
    // RetroArch's config integer reader uses base-0 conversion.
    let (digits, radix) = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (hex, 16)
    } else if value.starts_with('0') && value.len() > 1 {
        (&value[1..], 8)
    } else {
        (value, 10)
    };
    ensure!(
        !digits.is_empty() && !digits.starts_with(['+', '-']),
        "Invalid controller mode"
    );
    Ok(u32::from_str_radix(digits, radix)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }
    #[test]
    fn keeps_mixed_port_modes_and_cli_precedence() {
        let config = "input_libretro_device_p1 = \"261\"\ninput_libretro_device_p2 = 1 # Digital\n";
        assert_eq!(configured_modes(config, &[], 2).unwrap(), [261, 1]);
        assert_eq!(
            configured_modes("", &args(&["--device=0x1:0x105", "-d2:0405"]), 2).unwrap(),
            [261, 261]
        );
        assert_eq!(
            configured_modes(config, &args(&["--device=2:261", "-N1", "-d", "1:1"]), 2).unwrap(),
            [1, 261]
        );
        assert_eq!(
            configured_modes("", &args(&["--dualanalog", "1", "--", "--device=1:1"]), 2).unwrap(),
            [5, 1]
        );
        assert_eq!(
            configured_modes(
                "input_libretro_device_p1 = 0x105\ninput_libretro_device_p2 = 0405",
                &[],
                2
            )
            .unwrap(),
            [261, 261]
        );
    }
    #[test]
    fn rejects_ambiguous_or_unresolved_modes() {
        for config in [
            "input_libretro_device_p1=261",
            "#include \"other.cfg\"",
            "input_libretro_device_p1=1\ninput_libretro_device_p1=261",
            "input_libretro_device_p1=\"261\"oops",
            "input_libretro_device_p1=-1",
        ] {
            assert!(configured_modes(config, &[], 2).is_err(), "{config}");
        }
        for arguments in [
            args(&["--device"]),
            args(&["--device=3:261"]),
            args(&["--device=1:no"]),
            args(&["--nodevice=0"]),
        ] {
            assert!(configured_modes("", &arguments, 2).is_err());
        }
    }
}
