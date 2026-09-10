//! Standalone SDL PPSSPP 1.19.3 control protocol, not the libretro input API.
//! SDL device order and physical-to-logical translation must be established by
//! the launch adapter. This writer itself neither discovers nor opens hardware.
//! Source pin: e49c0bd8836a8a8f678565357773386f1174d3f5 (v1.19.3).
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod configuration;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod physical;
pub(crate) mod session;
pub(crate) mod settings;

pub(crate) const GAMEPLAY_KEYS: [&str; 16] = [
    "Up", "Down", "Left", "Right", "Cross", "Circle", "Square", "Triangle", "L", "R", "Start",
    "Select", "An.Up", "An.Down", "An.Left", "An.Right",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SdlInput {
    /// SDL_GameControllerButton, NOT a raw joystick button or evdev code.
    Button(u8),
    /// SDL_GameControllerAxis direction, after measured physical translation.
    Axis { index: u8, direction: i8 },
}

/// Connect the shared physical-layout planner to already resolved target SDL
/// controls. Keys in `physical_inputs` are saved physical control IDs, never
/// target PSP labels. The runtime adapter remains responsible for measurements
/// and actual SDL mapping provenance before calling this composition.
pub(crate) fn calibrated_controls(
    calibration: &crate::controller_catalog::Calibration,
    physical_inputs: &BTreeMap<String, SdlInput>,
) -> Result<BTreeMap<String, SdlInput>> {
    use anyhow::Context;
    let catalog = crate::controller_catalog::catalog();
    let profile = catalog
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == "ppsspp:standalone-psp")
        .context("Missing standalone PPSSPP controller profile")?;
    let plan = calibration.plan_profile(profile)?;
    let mut controls = BTreeMap::new();
    for row in plan.rows {
        let physical = row
            .physical_id
            .context("PPSSPP target has no suitable physical control")?;
        ensure!(
            row.input
                .as_ref()
                .is_some_and(|input| input.native.is_some()),
            "PPSSPP physical control requires saved native calibration"
        );
        let input = *physical_inputs
            .get(&physical)
            .context("PPSSPP physical control has no resolved SDL input")?;
        ensure!(
            controls.insert(row.output, input).is_none(),
            "Duplicate native PPSSPP target setting"
        );
    }
    // Device index zero is used here only to validate the shared control graph;
    // these strings are discarded and cannot become a saved player assignment.
    native_bindings(0, &controls)?;
    Ok(controls)
}

impl SdlInput {
    pub(crate) fn key_code(self) -> Result<u32> {
        Ok(match self {
            Self::Button(index) => {
                // SDLJoystick::getKeycodeForButton. PPSSPP maps SDL A/B/X/Y
                // to its numbered keys 2/3/4/1, not Android BUTTON_A/B/X/Y.
                const CODES: [u32; 15] = [
                    189, 190, 191, 188, 196, 4, 197, 106, 107, 193, 192, 19, 20, 21, 22,
                ];
                *CODES
                    .get(usize::from(index))
                    .ok_or_else(|| anyhow::anyhow!("Unsupported PPSSPP SDL button"))?
            }
            Self::Axis { index, direction } => {
                ensure!(
                    index < 6 && matches!(direction, -1 | 1),
                    "Invalid PPSSPP SDL axis half"
                );
                // InputMapping::TranslateKeyCodeFromAxis. SDLJoystick forwards
                // SDL's axis ordinal unchanged, including trigger indices 4/5.
                4000 + u32::from(index) * 2 + u32::from(direction < 0)
            }
        })
    }
}

/// Render all PSP gameplay inputs for one verified SDL device index. The
/// generic device range has ten entries; never wrap into PPSSPP's XInput IDs.
pub(crate) fn native_bindings(
    sdl_device_index: u8,
    controls: &BTreeMap<String, SdlInput>,
) -> Result<BTreeMap<String, String>> {
    ensure!(
        sdl_device_index < 10,
        "PPSSPP SDL device index exceeds the generic pad range"
    );
    ensure!(
        controls.len() == GAMEPLAY_KEYS.len()
            && GAMEPLAY_KEYS.iter().all(|key| controls.contains_key(*key)),
        "PPSSPP mapping requires all sixteen PSP gameplay inputs and no invented controls"
    );
    let mut used = BTreeSet::new();
    let mut result = BTreeMap::new();
    for (key, input) in controls {
        if key.starts_with("An.") {
            ensure!(
                matches!(input, SdlInput::Axis { index: 0..=3, .. }),
                "PPSSPP analog stick requires continuous stick axes, not buttons or triggers"
            );
        }
        let code = input.key_code()?;
        ensure!(
            used.insert(code),
            "PPSSPP gameplay controls share one SDL input"
        );
        result.insert(key.clone(), format!("{}-{code}", 10 + sdl_device_index));
    }
    let mut stick_axes = BTreeSet::new();
    for (negative, positive) in [("An.Left", "An.Right"), ("An.Up", "An.Down")] {
        let (
            SdlInput::Axis {
                index: a,
                direction: da,
            },
            SdlInput::Axis {
                index: b,
                direction: db,
            },
        ) = (controls[negative], controls[positive])
        else {
            anyhow::bail!("PPSSPP analog axis is incomplete");
        };
        ensure!(
            a == b && da == -db && stick_axes.insert(a),
            "PPSSPP stick axes must be independent bipolar pairs"
        );
    }
    Ok(result)
}

/// Replace only gameplay mappings in a private controls.ini text. Existing
/// hotkeys, comments and other sections survive. Repeated gameplay assignments
/// are all removed before inserting exactly one assignment per PSP control.
/// The caller owns writing this to a session directory, never the source file.
pub(crate) fn render_controls(
    original: &str,
    sdl_device_index: u8,
    controls: &BTreeMap<String, SdlInput>,
) -> Result<String> {
    ensure!(
        !original.contains('\0'),
        "NUL in PPSSPP controls configuration"
    );
    let bindings = native_bindings(sdl_device_index, controls)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = String::new();
    let mut in_controls = false;
    let mut inserted = false;
    for line in original.split_inclusive('\n') {
        let content = line.trim_start_matches('\u{feff}').trim();
        if let Some((section, _)) = content
            .strip_prefix('[')
            .and_then(|line| line.split_once(']'))
        {
            in_controls = section.trim().eq_ignore_ascii_case("ControlMapping");
            result.push_str(line);
            if in_controls && !inserted {
                if !result.ends_with('\n') {
                    result.push_str(newline);
                }
                for (key, value) in &bindings {
                    result.push_str(&format!("{key} = {value}{newline}"));
                }
                inserted = true;
            }
        } else if !(in_controls
            && !content.starts_with([';', '#'])
            && content.split_once('=').is_some_and(|(key, _)| {
                GAMEPLAY_KEYS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(key.trim()))
            }))
        {
            result.push_str(line);
        }
    }
    if !inserted {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push_str(newline);
        }
        result.push_str(&format!("[ControlMapping]{newline}"));
        for (key, value) in bindings {
            result.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    Ok(result)
}
