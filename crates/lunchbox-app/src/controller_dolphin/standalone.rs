//! Dolphin 2606 native GameCube settings, separate from the libretro adapter.
//! Source pin: 6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4.
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod configuration;
pub(crate) mod evdev;
pub(crate) mod inventory;
pub(crate) mod isolation;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

pub(crate) const CONTROLS: [&str; 22] = [
    "Buttons/A",
    "Buttons/B",
    "Buttons/X",
    "Buttons/Y",
    "Buttons/Z",
    "Buttons/Start",
    "D-Pad/Up",
    "D-Pad/Down",
    "D-Pad/Left",
    "D-Pad/Right",
    "Main Stick/Up",
    "Main Stick/Down",
    "Main Stick/Left",
    "Main Stick/Right",
    "C-Stick/Up",
    "C-Stick/Down",
    "C-Stick/Left",
    "C-Stick/Right",
    "Triggers/L",
    "Triggers/R",
    "Triggers/L-Analog",
    "Triggers/R-Analog",
];

pub(crate) fn analog_target(setting: &str) -> bool {
    setting.starts_with("Main Stick/")
        || setting.starts_with("C-Stick/")
        || setting.ends_with("-Analog")
}

#[derive(Clone, PartialEq)]
pub(crate) struct ResolvedControl {
    /// Exact name from the selected native Dolphin backend's input inventory.
    pub name: String,
    pub analog: bool,
    /// Native normalization/scaling established by physical calibration.
    pub range_percent: f64,
}

pub(crate) struct Pad {
    pub port: u8,
    /// Native backend/id/name qualifier, not a model name or raw evdev path.
    pub device: String,
    pub controls: BTreeMap<String, ResolvedControl>,
}

pub(crate) fn calibrated_pad(
    calibration: &crate::controller_catalog::Calibration,
    port: u8,
    device: String,
    physical: &BTreeMap<String, ResolvedControl>,
) -> Result<Pad> {
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "dolphin:standalone-gamecube")
        .context("Missing native Dolphin GameCube profile")?;
    let mut controls = BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let id = row
            .physical_id
            .context("Dolphin gameplay target has no physical control")?;
        let input = row
            .input
            .context("Dolphin physical input is not calibrated")?;
        ensure!(
            input.native.is_some(),
            "Dolphin requires saved native physical calibration"
        );
        if analog_target(&row.output) {
            ensure!(
                input.axis.is_some(),
                "Dolphin analog target requires measured axis endpoints"
            );
        }
        let resolved = physical
            .get(&id)
            .context("Dolphin physical control has no native backend resolution")?;
        ensure!(
            controls.insert(row.output, resolved.clone()).is_none(),
            "Duplicate Dolphin native target"
        );
    }
    let pad = Pad {
        port,
        device,
        controls,
    };
    // Validate without producing or saving a synthetic device assignment.
    render("", "", std::slice::from_ref(&pad))?;
    Ok(pad)
}

fn quoted_control(input: &ResolvedControl) -> Result<String> {
    ensure!(
        !input.name.trim().is_empty()
            && input.name.len() <= 256
            && !input
                .name
                .chars()
                .any(|ch| ch.is_control() || matches!(ch, '`' | ':' | '"')),
        "Dolphin input must be one resolved local control, not an expression or device override"
    );
    ensure!(
        input.range_percent.is_finite()
            && input.range_percent > 0.0
            && input.range_percent <= 500.0,
        "Dolphin input scaling is outside the supported range"
    );
    Ok(format!("`{}`", input.name))
}

/// Return native GCPadNew.ini and Dolphin.ini text for private session copies.
/// Per-game InputProfile overlays and actual backend identity must be resolved
/// by the launch owner; this writer does not claim that a pad is connected.
pub(crate) fn render(pad_ini: &str, dolphin_ini: &str, pads: &[Pad]) -> Result<(String, String)> {
    ensure!(
        !pads.is_empty() && pads.len() <= 4,
        "Dolphin needs one to four GameCube pads"
    );
    let mut ports = BTreeSet::new();
    let mut devices = BTreeSet::new();
    let mut pad_result = pad_ini.to_owned();
    let mut si = BTreeMap::new();
    for pad in pads {
        ensure!(
            (1..=4).contains(&pad.port) && ports.insert(pad.port),
            "Duplicate or invalid Dolphin pad port"
        );
        let parts: Vec<_> = pad.device.splitn(3, '/').collect();
        ensure!(
            parts.len() == 3
                && !parts[0].is_empty()
                && parts[1].parse::<u32>().is_ok()
                && !parts[2].is_empty()
                && pad.device.len() <= 512
                && !pad
                    .device
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '"' | '`')),
            "Dolphin requires an exact backend/id/name device qualifier"
        );
        ensure!(
            devices.insert(&pad.device),
            "Dolphin physical device is assigned to multiple pads"
        );
        ensure!(
            pad.controls.len() == CONTROLS.len()
                && CONTROLS.iter().all(|key| pad.controls.contains_key(*key)),
            "Dolphin requires all standard GameCube controls, including analog trigger travel and clicks"
        );
        let mut fields = BTreeMap::from([("Device".to_owned(), pad.device.clone())]);
        let mut owners = BTreeMap::<&str, &str>::new();
        for (setting, input) in &pad.controls {
            ensure!(
                !analog_target(setting) || input.analog,
                "Dolphin analog target cannot be driven by a binary input"
            );
            if let Some(previous) = owners.insert(&input.name, setting) {
                // Native MixedTriggers supports sharing a continuous trigger
                // with its thresholded click. No other duplicate ownership.
                let paired = matches!(
                    (previous, setting.as_str()),
                    ("Triggers/L", "Triggers/L-Analog") | ("Triggers/R", "Triggers/R-Analog")
                );
                ensure!(
                    paired
                        && input.analog
                        && pad.controls[previous].analog
                        && pad.controls[previous].range_percent == input.range_percent,
                    "Dolphin native input has competing gameplay owners"
                );
            }
            fields.insert(setting.clone(), quoted_control(input)?);
            fields.insert(format!("{setting}/Range"), input.range_percent.to_string());
        }
        // Native modifier expressions from a previous keyboard map must not
        // silently halve the newly calibrated stick's travel.
        fields.insert("Main Stick/Modifier".into(), String::new());
        fields.insert("C-Stick/Modifier".into(), String::new());
        pad_result = patch_section(&pad_result, &format!("GCPad{}", pad.port), &fields)?;
        // SerialInterface::SIDevices: 6 is the standard emulated GC controller.
        si.insert(format!("SIDevice{}", pad.port - 1), "6".into());
    }
    Ok((pad_result, patch_section(dolphin_ini, "Core", &si)?))
}

/// Update a private copy of an effective per-game controller profile. Dolphin
/// loads its [Profile] section instead of GCPadNew.ini for a selected player.
pub(crate) fn render_profile(original: &str, pad: &Pad) -> Result<String> {
    // Use the same complete pad/range/ownership validation as the main writer.
    render("", "", std::slice::from_ref(pad))?;
    let mut fields = BTreeMap::from([
        ("Device".to_owned(), pad.device.clone()),
        ("Main Stick/Modifier".to_owned(), String::new()),
        ("C-Stick/Modifier".to_owned(), String::new()),
    ]);
    for (setting, input) in &pad.controls {
        fields.insert(setting.clone(), quoted_control(input)?);
        fields.insert(format!("{setting}/Range"), input.range_percent.to_string());
    }
    patch_section(original, "Profile", &fields)
}

/// Route selected gamepad ports to session-owned profile basenames. The caller
/// puts the corresponding .ini files under the private Profiles/GCPad directory
/// and applies this to the final effective per-game INI layer. No source file
/// or other player's profile choice is modified here.
pub(crate) fn select_game_profiles(
    original: &str,
    selections: &BTreeMap<u8, String>,
) -> Result<String> {
    ensure!(
        !selections.is_empty() && selections.len() <= 4,
        "Dolphin needs one to four game profile selections"
    );
    let mut fields = BTreeMap::new();
    let mut modern_profiles = BTreeMap::new();
    let mut serial_devices = BTreeMap::new();
    let mut names = BTreeSet::new();
    for (port, name) in selections {
        ensure!(
            (1..=4).contains(port)
                && !name.is_empty()
                && name.len() <= 128
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                && names.insert(name),
            "Dolphin session profile names must be distinct filename-safe basenames"
        );
        // InputProfile::GetProfilesFromSetting appends .ini; never put the
        // extension, commas, directory choices or traversal in this value.
        fields.insert(format!("PadProfile{port}"), name.clone());
        // The legacy InputConfig loader and modern game configuration layer
        // independently select profiles. Pin both to the same private file.
        modern_profiles.insert(format!("PadProfile{port}"), name.clone());
        fields.insert(format!("PadType{}", port - 1), "6".into());
        serial_devices.insert(format!("SIDevice{}", port - 1), "6".into());
    }
    let result = patch_section(original, "Controls", &fields)?;
    let result = patch_section(&result, "GCPad.Controls", &modern_profiles)?;
    // All three spellings resolve to Main/Core/SIDeviceN in Dolphin 2606.
    // Existing aliases must agree regardless of the source section order.
    let result = patch_section(&result, "Core", &serial_devices)?;
    patch_section(&result, "Main.Core", &serial_devices)
}

fn patch_section(
    original: &str,
    section: &str,
    fields: &BTreeMap<String, String>,
) -> Result<String> {
    ensure!(!original.contains('\0'), "Dolphin INI contains a NUL byte");
    let (bom, text) = original
        .strip_prefix('\u{feff}')
        .map_or(("", original), |text| ("\u{feff}", text));
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut result = bom.to_owned();
    let mut active = false;
    let mut inserted = false;
    for line in text.split_inclusive('\n') {
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name))
        {
            active = header.eq_ignore_ascii_case(section);
            result.push_str(line);
            if active && !inserted {
                if !result.ends_with('\n') {
                    result.push_str(newline);
                }
                for (key, value) in fields {
                    result.push_str(&format!("{key} = {value}{newline}"));
                }
                inserted = true;
            }
        } else {
            let owned = !line.starts_with(['#', '$', '+', '*'])
                && line.split_once('=').is_some_and(|(key, _)| {
                    fields
                        .keys()
                        .any(|known| key.trim().eq_ignore_ascii_case(known))
                });
            if !active || !owned {
                result.push_str(line);
            }
        }
    }
    if !inserted {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push_str(newline);
        }
        result.push_str(&format!("[{section}]{newline}"));
        for (key, value) in fields {
            result.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    Ok(result)
}
