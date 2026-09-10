//! Standalone Flycast mapping format, not the libretro core.
//! Pin fb286f777ce690ef8acf3359a75ab84b61566ad9/core/input/mapping.cpp.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod arcade;
pub(crate) mod configuration;
pub(crate) mod discovery;
pub(crate) mod isolation;
#[cfg(target_os = "linux")]
pub(crate) mod launch;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod paths;
pub(crate) mod physical;
pub(crate) mod prepared;
pub(crate) mod routing;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;
#[cfg(target_os = "linux")]
pub(crate) mod startup_log;
pub(crate) mod triggers;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Input {
    Button(u32),
    AxisHalf { code: u32, positive: bool },
}

pub(crate) const OUTPUTS: &[&str] = &[
    "btn_a",
    "btn_b",
    "btn_c",
    "btn_d",
    "btn_x",
    "btn_y",
    "btn_z",
    "btn_start",
    "btn_dpad1_left",
    "btn_dpad1_right",
    "btn_dpad1_up",
    "btn_dpad1_down",
    "btn_dpad2_left",
    "btn_dpad2_right",
    "axis_trigger_left",
    "axis_trigger_right",
];

pub(crate) struct Port<'a> {
    /// Native mapping port index 0–3, independent of SDL device index.
    pub port: u8,
    pub controls: &'a BTreeMap<String, Input>,
}

/// Each file belongs to a single resolved physical device. Device filename and
/// assignment selection are separate native contracts, not inferred here.
pub(crate) fn mapping(ports: &[Port<'_>], dead_zone: u8, saturation: u16) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 4,
        "Invalid Flycast mapping port count"
    );
    ensure!(
        dead_zone <= 100 && (50..=200).contains(&saturation),
        "Invalid Flycast dead zone or saturation"
    );
    let mut seen = BTreeSet::new();
    let mut digital = Vec::new();
    let mut analog = Vec::new();
    for port in ports {
        ensure!(
            port.port < 4 && seen.insert(port.port),
            "Invalid or duplicate Flycast port"
        );
        ensure!(!port.controls.is_empty(), "Flycast port has no controls");
        let mut owners = BTreeSet::new();
        for (output, input) in port.controls {
            ensure!(
                OUTPUTS.contains(&output.as_str()),
                "Unknown Flycast native control"
            );
            ensure!(
                owners.insert(*input),
                "Flycast controls share a physical input"
            );
            let key = if port.port == 0 {
                output.clone()
            } else {
                format!("{output}{}", port.port)
            };
            let code = match *input {
                Input::Button(code) | Input::AxisHalf { code, .. } => code,
            };
            // Native loader uses atoi; do not emit out-of-range signed values
            // or the invalid input sentinel (UINT32_MAX).
            ensure!(
                code <= i32::MAX as u32,
                "Flycast native input code exceeds parser range"
            );
            match *input {
                Input::Button(_) => digital.push(format!("{code}:{key}")),
                Input::AxisHalf { positive, .. } => {
                    analog.push(format!("{code}{}:{key}", if positive { "+" } else { "-" }))
                }
            }
        }
    }
    // Version 3 uses independent sequential digital/analog bind lists and is
    // still loaded by the pinned implementation. No combo/hotkey defaults leak in.
    let mut result = format!(
        "[emulator]\nversion = 3\nmapping_name = Lunchbox controller\ndead_zone = {dead_zone}\nsaturation = {saturation}\n\n[digital]\n"
    );
    for (index, entry) in digital.iter().enumerate() {
        result.push_str(&format!("bind{index} = {entry}\n"));
    }
    result.push_str("\n[analog]\n");
    for (index, entry) in analog.iter().enumerate() {
        result.push_str(&format!("bind{index} = {entry}\n"));
    }
    Ok(result)
}

/// Preserve explicitly resolved native trigger classification. Flycast can
/// auto-populate an empty list from SDL, so an empty map is not proof of absence.
pub(crate) fn mapping_with_triggers(
    ports: &[Port<'_>],
    dead_zone: u8,
    saturation: u16,
    triggers: &BTreeMap<u32, bool>,
) -> Result<String> {
    ensure!(
        triggers.keys().all(|code| *code <= 255),
        "Flycast SDL trigger index exceeds event range"
    );
    for port in ports {
        for input in port.controls.values() {
            if let Input::AxisHalf { code, positive } = input {
                ensure!(
                    !triggers.contains_key(code) || *positive,
                    "Flycast classified triggers require positive mapping slots"
                );
            }
        }
    }
    let mut text = mapping(ports, dead_zone, saturation)?;
    if !triggers.is_empty() {
        let value = triggers
            .iter()
            .map(|(code, reversed)| format!("{code}{}", if *reversed { "~" } else { "" }))
            .collect::<Vec<_>>()
            .join(",");
        text = text.replacen(
            "[emulator]\n",
            &format!("[emulator]\ntriggers = {value}\n"),
            1,
        );
    }
    Ok(text)
}
