//! Shared DOSBox-X / DOSBox Staging native mapper writer for the standard
//! emulated PC joystick.
//!
//! Pinned sources: `joncampbell123/dosbox-x`
//! `532909c4e84160a5ac2185fbf9c4c97dbe07f85d` `src/gui/mapper.cpp` and
//! `dosbox-staging/dosbox-staging`
//! `d9135010ea56c2faa0bb8062aaf30a8541bf22f3` `src/gui/mapper.cpp`. Both
//! engines serialize the same `EVENT "BIND" "BIND"...` grammar into the
//! `[SDL]` section of the mapper file selected by `mapperfile`, and both bind
//! raw SDL joystick vocabulary: `stick_N axis A 0|1`, `stick_N button B` and
//! `stick_N hat H D` (SDL hat masks up=1, right=2, down=4, left=8).
//!
//! DOSBox's built-in defaults already bind the first physical stick to the
//! emulated joystick, but a loaded mapper file *replaces* those defaults
//! (`MAPPER_LoadBinds` clears every bind before loading), so the writer always
//! starts from a complete baseline and patches only the emulated joystick
//! events. The shipped default baseline is generated from the pinned
//! `CreateDefaultBinds` keyboard/mouse tables with SDL's public scancode
//! values; a mapper file the install already provides is preferred when
//! present, so user keyboard remaps survive.

use crate::controller_catalog::{Calibration, EmulatorProfile, NativeInput};
use crate::controller_launch::JoydevMap;
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "532909c4e84160a5ac2185fbf9c4c97dbe07f85d";

/// Complete DOSBox default mapper (keyboard, modifiers, joystick), generated
/// from the pinned `CreateDefaultBinds` tables. Loading this as the baseline
/// keeps the guest keyboard reachable while the writer replaces joystick
/// events.
pub(crate) const DEFAULT_MAPPER: &str =
    include_str!("../data/controllers/dosbox-default-mapper.map");

/// Target control -> the emulated joystick action it drives, with the preview
/// label used by the guided target list.
pub(crate) const ROUTES: [(&str, &str); 8] = [
    ("up", "Joystick up (Y-)"),
    ("down", "Joystick down (Y+)"),
    ("left", "Joystick left (X-)"),
    ("right", "Joystick right (X+)"),
    ("a", "Button 1"),
    ("b", "Button 2"),
    ("x", "Button 3"),
    ("y", "Button 4"),
];

/// Target controls that must be measured before a calibrated launch.
const REQUIRED: [&str; 6] = ["up", "down", "left", "right", "a", "b"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Event {
    pub name: String,
    pub binds: Vec<String>,
}

/// DOSBox emulated-joystick event name for one target control. `stick` is the
/// zero-based emulated joystick index (player minus one).
fn event_name(target: &str, stick: usize) -> Option<String> {
    Some(match target {
        "up" => format!("jaxis_{stick}_1-"),
        "down" => format!("jaxis_{stick}_1+"),
        "left" => format!("jaxis_{stick}_0-"),
        "right" => format!("jaxis_{stick}_0+"),
        "a" => format!("jbutton_{stick}_0"),
        "b" => format!("jbutton_{stick}_1"),
        "x" => format!("jbutton_{stick}_2"),
        "y" => format!("jbutton_{stick}_3"),
        _ => return None,
    })
}

/// One DOSBox BIND token for a measured physical input on `device`.
///
/// Raw Linux hat axes (`ABS_HAT0X`/`ABS_HAT0Y`) are exposed by SDL as a hat,
/// so they use the `hat` bind form; buttons and proportional axes use the
/// matching `button`/`axis` form. The dosbox-x Flatpak already forces SDL's
/// classic joystick backend (`SDL_LINUX_JOYSTICK_CLASSIC=1`), whose indices
/// match the Linux `js` numbering `JoydevMap` reads, and the launch plan pins
/// that backend for the native builds too.
fn bind(device: &JoydevMap, input: &NativeInput) -> Result<String> {
    let code = (input.code & 0xffff) as u16;
    if input.code >> 16 == 3 && (0x10..=0x17).contains(&code) {
        let hat = u32::from(code - 0x10) / 2;
        let axis = (code - 0x10) % 2;
        let direction = match (axis, input.direction) {
            (0, d) if d < 0 => 8,
            (0, _) => 2,
            (1, d) if d < 0 => 1,
            (1, _) => 4,
            _ => anyhow::bail!("Recorded hat direction is invalid"),
        };
        return Ok(format!("stick_{} hat {hat} {direction}", device.index));
    }
    let (kind, value) = device.binding(input)?;
    Ok(match kind {
        "btn" => format!("stick_{} button {value}", device.index),
        "axis" => {
            let (sign, index) = value.split_at(1);
            ensure!(
                sign == "+" || sign == "-",
                "Recorded axis direction is invalid"
            );
            format!(
                "stick_{} axis {index} {}",
                device.index,
                if sign == "+" { 1 } else { 0 }
            )
        }
        _ => anyhow::bail!("Recorded input has no DOSBox bind"),
    })
}

/// Emulated-joystick events for one calibrated player.
pub(crate) fn player_events(
    stick: usize,
    profile: &EmulatorProfile,
    calibration: &Calibration,
    device: &JoydevMap,
) -> Result<Vec<Event>> {
    let plan = calibration.plan_profile(profile)?;
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    let mut measured = BTreeSet::new();
    for row in &plan.rows {
        let Some(name) = event_name(&row.target_id, stick) else {
            continue;
        };
        let Some(native) = row.input.as_ref().and_then(|input| input.native.as_ref()) else {
            continue;
        };
        let token = bind(device, native)
            .with_context(|| format!("Mapping target {} to a DOSBox bind", row.target))?;
        ensure!(
            seen.insert((name.clone(), token.clone())),
            "Two DOSBox actions resolve to the same physical input"
        );
        measured.insert(row.target_id.clone());
        events.push(Event {
            name,
            binds: vec![token],
        });
    }
    for control in REQUIRED {
        ensure!(
            measured.contains(control),
            "Finish calibrating this controller for the DOSBox joystick (missing {control})"
        );
    }
    Ok(events)
}

/// Patch the emulated joystick events of a complete baseline mapper, preserving
/// every other bind and comment.
pub(crate) fn mapper(core: &str, baseline: &[u8], events: &[Event]) -> Result<String> {
    ensure!(!events.is_empty(), "DOSBox launch has no mapped controls");
    if core == "dosbox-staging" {
        let staging = events
            .iter()
            .map(|event| crate::controller_dosbox_staging_native::Event {
                name: event.name.clone(),
                binds: event.binds.clone(),
            })
            .collect::<Vec<_>>();
        crate::controller_dosbox_staging_native::patch_mapper(baseline, &staging)
    } else {
        let dosbox_x = events
            .iter()
            .map(|event| crate::controller_dosbox_x_native::Event {
                name: event.name.clone(),
                binds: event.binds.clone(),
            })
            .collect::<Vec<_>>();
        crate::controller_dosbox_x_native::patch_mapper(baseline, &dosbox_x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_catalog::{Calibration, InputBinding, NativeInput, catalog};
    use std::collections::BTreeMap;

    fn device() -> JoydevMap {
        // js3 with button codes BTN_SOUTH=0x130.. and axes
        // ABS_X=0x00, ABS_Y=0x01, ABS_Z=0x02, ABS_RX=0x03, ABS_HAT0X=0x10.
        JoydevMap {
            index: 3,
            buttons: vec![0x130, 0x131, 0x132, 0x133],
            axes: vec![0x00, 0x01, 0x02, 0x03, 0x10],
        }
    }

    /// A complete measured calibration for the standard PC joystick target.
    fn calibration() -> Calibration {
        let codes: [(&str, u32, i8, &str); 8] = [
            ("up", 0x00, -1, "axis"),
            ("down", 0x01, 1, "axis"),
            ("left", 0x02, -1, "axis"),
            ("right", 0x03, 1, "axis"),
            ("a", 0x130, 0, "button"),
            ("b", 0x131, 0, "button"),
            ("x", 0x132, 0, "button"),
            ("y", 0x133, 0, "button"),
        ];
        let bindings = codes
            .into_iter()
            .map(|(id, code, direction, kind)| {
                let native = if kind == "button" {
                    (1 << 16) | code
                } else {
                    (3 << 16) | code
                };
                (
                    id.to_string(),
                    InputBinding {
                        code,
                        kind: kind.into(),
                        direction,
                        logical: id.to_string(),
                        native: Some(NativeInput {
                            code: native,
                            direction,
                        }),
                        axis: None,
                    },
                )
            })
            .collect();
        Calibration {
            target_mappings: BTreeMap::new(),
            layout: "dosbox-joystick".into(),
            os: "linux".into(),
            backend: "gilrs-0.11".into(),
            bindings,
        }
    }

    #[test]
    fn binds_buttons_axes_and_hats_to_the_measured_stick() {
        let device = device();
        assert_eq!(
            bind(
                &device,
                &NativeInput {
                    code: (1 << 16) | 0x130,
                    direction: 0
                }
            )
            .unwrap(),
            "stick_3 button 0"
        );
        assert_eq!(
            bind(
                &device,
                &NativeInput {
                    code: (3 << 16) | 0x00,
                    direction: 1
                }
            )
            .unwrap(),
            "stick_3 axis 0 1"
        );
        assert_eq!(
            bind(
                &device,
                &NativeInput {
                    code: (3 << 16) | 0x01,
                    direction: -1
                }
            )
            .unwrap(),
            "stick_3 axis 1 0"
        );
        assert_eq!(
            bind(
                &device,
                &NativeInput {
                    code: (3 << 16) | 0x10,
                    direction: -1
                }
            )
            .unwrap(),
            "stick_3 hat 0 8"
        );
    }

    #[test]
    fn events_use_the_standard_dos_joystick_vocabulary() {
        assert_eq!(event_name("up", 0).unwrap(), "jaxis_0_1-");
        assert_eq!(event_name("right", 0).unwrap(), "jaxis_0_0+");
        assert_eq!(event_name("a", 0).unwrap(), "jbutton_0_0");
        assert_eq!(event_name("a", 1).unwrap(), "jbutton_1_0");
        assert!(event_name("start", 0).is_none());
    }

    #[test]
    fn baselines_keep_the_keyboard_and_replace_only_joystick_events() {
        assert!(DEFAULT_MAPPER.contains("key_esc \"key 41\""));
        assert!(DEFAULT_MAPPER.contains("jaxis_0_0+ \"stick_0 axis 0 1\""));
        let patched = mapper(
            "dosbox-x",
            DEFAULT_MAPPER.as_bytes(),
            &[Event {
                name: "jbutton_0_0".into(),
                binds: vec!["stick_3 button 5".into()],
            }],
        )
        .unwrap();
        assert!(patched.contains("key_esc \"key 41\""));
        assert!(patched.contains("jbutton_0_0 \"stick_3 button 5\""));
        assert!(!patched.contains("jbutton_0_0 \"stick_0 button 0\""));
        // The staging writer shares the grammar but rejects an empty patch.
        assert!(mapper("dosbox-staging", DEFAULT_MAPPER.as_bytes(), &[]).is_err());
    }

    #[test]
    fn player_events_bind_the_measured_controller_to_the_emulated_joystick() {
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == "dosbox-x:standalone-dosbox-joystick")
            .expect("DOSBox-X native profile");
        let events = player_events(0, profile, &calibration(), &device()).unwrap();
        let map: BTreeMap<&str, &str> = events
            .iter()
            .map(|event| (event.name.as_str(), event.binds[0].as_str()))
            .collect();
        assert_eq!(map["jaxis_0_0-"], "stick_3 axis 2 0");
        assert_eq!(map["jaxis_0_0+"], "stick_3 axis 3 1");
        assert_eq!(map["jaxis_0_1-"], "stick_3 axis 0 0");
        assert_eq!(map["jaxis_0_1+"], "stick_3 axis 1 1");
        assert_eq!(map["jbutton_0_0"], "stick_3 button 0");
        assert_eq!(map["jbutton_0_1"], "stick_3 button 1");
        assert_eq!(map["jbutton_0_2"], "stick_3 button 2");
        assert_eq!(map["jbutton_0_3"], "stick_3 button 3");
    }

    #[test]
    fn player_events_refuse_an_incomplete_calibration() {
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == "dosbox-x:standalone-dosbox-joystick")
            .unwrap();
        let mut calibration = calibration();
        // With no measured controls the resolver cannot fill the required
        // joystick directions/buttons, so the launch must refuse.
        calibration.bindings.clear();
        assert!(player_events(0, profile, &calibration, &device()).is_err());
    }

    /// Scratch diagnostic: bind the operator's real calibrated pad to the DOS
    /// joystick and print the emitted mapper events.
    #[test]
    #[ignore = "scratch diagnostic; needs local settings and controller hardware"]
    fn scratch_real_calibration_maps_to_the_joystick() {
        let store = crate::settings::SettingsStore::open_default().unwrap();
        let settings = store.load().unwrap();
        let mut warnings = Vec::new();
        let inventory = crate::controllers::list_local_controllers(&mut warnings);
        let device = inventory
            .iter()
            .find(|device| {
                settings
                    .controller_mapping
                    .calibrations
                    .contains_key(&device.stable_id)
            })
            .expect("no calibrated controller is connected");
        let calibration = &settings.controller_mapping.calibrations[&device.stable_id];
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == "dosbox-x:standalone-dosbox-joystick")
            .unwrap();
        let joydev = JoydevMap::read(&device.device_path).unwrap();
        println!(
            "DEVICE {} js{} buttons={:?} axes={:?}",
            device.stable_id, joydev.index, joydev.buttons, joydev.axes
        );
        let events = player_events(0, profile, calibration, &joydev).unwrap();
        for event in &events {
            println!("EVENT {} \"{}\"", event.name, event.binds[0]);
        }
        let mapper = mapper("dosbox-x", DEFAULT_MAPPER.as_bytes(), &events).unwrap();
        assert!(mapper.contains("key_esc \"key 41\""));
        assert!(!events.is_empty());
    }
}
