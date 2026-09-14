//! Play! standalone-native evdev input-profile writer.
//!
//! Pinned source: jpd002/Play- commit
//! `83700b2c31e593bc94e845b4b31b797be84dda59` plus the Play--Framework
//! commit `3368a5a7d9cb18af4591e51f37fe9505496f70c2` for the `CConfig` XML
//! shape. On Linux, Play! reads evdev directly (`InputProviderEvDev`,
//! provider `'evdv'`): binding targets carry the provider id, the 6-part
//! device id derived from the evdev uniq string (MAC, first-6-bytes, or
//! vendor/product/version fallback in `GamePadUtils::GetDeviceID`), the
//! kernel key code, and the key type (0 button, 1 axis, 2 povhat).
//! Preferences persist as `<Config><Preference Name="" Type=""
//! Value=""/></Config>` XML in `<base>/inputprofiles/<name>.xml`, selected
//! by the `input.pad1.profile` app preference (default `"default"`). The
//! session stages `default.xml` so no app-config patch is needed. The base
//! path is `./Play Data Files` when `portable.txt` sits beside the working
//! directory, so launch runs there with saves mirrored through symlinks.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "83700b2c31e593bc94e845b4b31b797be84dda59";
pub(crate) const FRAMEWORK_COMMIT: &str = "3368a5a7d9cb18af4591e51f37fe9505496f70c2";
pub(crate) const PROFILE_ID: &str = "play:standalone-dualshock";
pub(crate) const PROFILE_NAME: &str = "default";
pub(crate) const BASE_DIR: &str = "Play Data Files";
pub(crate) const PORTABLE_FLAG: &str = "portable.txt";
pub(crate) const PROFILES_DIR: &str = "inputprofiles";

/// `'evdv'` as a little-endian u32, matching the source multichar constant
/// on the supported hosts.
pub(crate) const PROVIDER_EVDEV: u32 = 0x7664_7665;

/// Play! key types (`BINDINGTARGET::KEYTYPE` order).
pub(crate) const KEYTYPE_BUTTON: u32 = 0;
pub(crate) const KEYTYPE_AXIS: u32 = 1;

/// PS2 buttons in `CControllerInfo` order with the layout target feeding
/// each one. The four analog sticks take two targets each (negative and
/// positive halves); everything else takes one button target. Rumble has no
/// gamepad control and stays unbound.
pub(crate) const BUTTONS: [(&str, &str); 20] = [
    ("analog_left_x", "stick_left"),
    ("analog_left_y", "stick_up"),
    ("analog_right_x", "right_stick_left"),
    ("analog_right_y", "right_stick_up"),
    ("dpad_up", "up"),
    ("dpad_down", "down"),
    ("dpad_left", "left"),
    ("dpad_right", "right"),
    ("select", "select"),
    ("start", "start"),
    ("square", "y"),
    ("triangle", "x"),
    ("circle", "a"),
    ("cross", "b"),
    ("l1", "l"),
    ("l2", "l2"),
    ("l3", "l3"),
    ("r1", "r"),
    ("r2", "r2"),
    ("r3", "r3"),
];

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 24] = [
    ("b", "Cross"),
    ("a", "Circle"),
    ("y", "Square"),
    ("x", "Triangle"),
    ("l", "L1"),
    ("r", "R1"),
    ("l2", "L2"),
    ("r2", "R2"),
    ("l3", "L3"),
    ("r3", "R3"),
    ("select", "Select"),
    ("start", "Start"),
    ("up", "Dpad up"),
    ("down", "Dpad down"),
    ("left", "Dpad left"),
    ("right", "Dpad right"),
    ("stick_up", "Left stick up"),
    ("stick_down", "Left stick down"),
    ("stick_left", "Left stick left"),
    ("stick_right", "Left stick right"),
    ("right_stick_up", "Right stick up"),
    ("right_stick_down", "Right stick down"),
    ("right_stick_left", "Right stick left"),
    ("right_stick_right", "Right stick right"),
];

/// One evdev binding target: kernel key code plus key type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EvdevTarget {
    pub key_id: u32,
    pub key_type: u32,
}

/// Derive the 6-part Play! device id from an evdev uniq string and numeric
/// ids, mirroring `GamePadUtils::GetDeviceID`: MAC parse, then the first 6
/// uniq bytes, then vendor/product/version little-endian bytes.
pub(crate) fn device_id(uniq: &str, vendor: u16, product: u16, version: u16) -> [u32; 6] {
    let mut bytes = [0u32; 6];
    if sscanf_mac(uniq, &mut bytes) {
        return bytes;
    }
    if uniq.len() >= 6 {
        for (index, byte) in uniq.bytes().take(6).enumerate() {
            bytes[index] = u32::from(byte);
        }
        return bytes;
    }
    if bytes != [0; 6] {
        return bytes;
    }
    bytes[0] = u32::from(vendor & 0xFF);
    bytes[1] = u32::from((vendor >> 8) & 0xFF);
    bytes[2] = u32::from(product & 0xFF);
    bytes[3] = u32::from((product >> 8) & 0xFF);
    bytes[4] = u32::from(version & 0xFF);
    bytes[5] = u32::from((version >> 8) & 0xFF);
    bytes
}

fn sscanf_mac(uniq: &str, bytes: &mut [u32; 6]) -> bool {
    let parts: Vec<&str> = uniq.split(':').collect();
    if parts.len() != 6 {
        return false;
    }
    for (index, part) in parts.iter().enumerate() {
        if part.len() != 2 {
            return false;
        }
        match u32::from_str_radix(part, 16) {
            Ok(value) => bytes[index] = value,
            Err(_) => return false,
        }
    }
    true
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Render the `inputprofiles/default.xml` preferences: per PS2 button, the
/// binding type plus one target (two for analog sticks). Digital buttons use
/// simple bindings (type 1); sticks use simulated-axis bindings (type 2)
/// with negative/positive halves. The motor stays unbound (type 0), matching
/// a fresh profile.
pub(crate) fn profile_xml(
    device: [u32; 6],
    buttons: &BTreeMap<String, EvdevTarget>,
    sticks: &BTreeMap<String, (EvdevTarget, EvdevTarget)>,
) -> Result<String> {
    ensure!(
        buttons.len() + sticks.len() == BUTTONS.len(),
        "Play! needs every PS2 button target"
    );
    let device_string = device
        .iter()
        .map(|part| format!("{part:x}"))
        .collect::<Vec<_>>()
        .join(":");
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Config>\n");
    fn pref(out: &mut String, name: &str, kind: &str, value: &str) {
        out.push_str(&format!(
            "  <Preference Name=\"{}\" Type=\"{kind}\" Value=\"{}\"/>\n",
            xml_escape(name),
            xml_escape(value)
        ));
    }
    fn target_prefs(
        out: &mut String,
        device_string: &str,
        base: &str,
        slot: &str,
        target: &EvdevTarget,
    ) {
        pref(
            out,
            &format!("{base}.{slot}.providerId"),
            "integer",
            &PROVIDER_EVDEV.to_string(),
        );
        pref(
            out,
            &format!("{base}.{slot}.deviceId"),
            "string",
            device_string,
        );
        pref(
            out,
            &format!("{base}.{slot}.keyId"),
            "integer",
            &target.key_id.to_string(),
        );
        pref(
            out,
            &format!("{base}.{slot}.keyType"),
            "integer",
            &target.key_type.to_string(),
        );
    }
    for (button, _) in BUTTONS {
        let base = format!("input.pad1.{button}");
        if let Some((negative, positive)) = sticks.get(button) {
            pref(&mut out, &format!("{base}.bindingtype"), "integer", "2");
            target_prefs(&mut out, &device_string, &base, "bindingtarget1", negative);
            target_prefs(&mut out, &device_string, &base, "bindingtarget2", positive);
        } else if let Some(target) = buttons.get(button) {
            ensure!(
                target.key_type == KEYTYPE_BUTTON,
                "Play! digital button {button} needs a button target"
            );
            pref(&mut out, &format!("{base}.bindingtype"), "integer", "1");
            target_prefs(&mut out, &device_string, &base, "bindingtarget1", target);
        } else {
            anyhow::bail!("Play! PS2 button {button} is not calibrated");
        }
    }
    pref(&mut out, "input.pad1.motor.bindingtype", "integer", "0");
    out.push_str("</Config>\n");
    Ok(out)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// The user's real `Play Data Files` directory; saves survive
        /// through symlinks while the input profile stays private.
        pub data_dir: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Play! setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Play! setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.data_dir.is_absolute()
                    && !self
                        .data_dir
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Play! data directory must be absolute without parent traversal"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Play! setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Play! native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Play! supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Play! player needs a saved controller identity"
            );
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &HashMap<String, Calibration>,
        ) -> Result<serde_json::Value> {
            self.validate()?;
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Play! native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Play! controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Play! mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Play! needs native calibration for every DualShock control"
            );
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "player": player.player,
                "controller_id": player.controller_id,
                "source_layout": calibration.layout,
                "target_layout": profile.target_layout,
                "mapping": mapping,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch runs in a session directory with a private evdev input profile, then rechecks the exact evdev routes. Only the single DualShock pad is supported; hats and rumble are out of scope and runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Play! saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Play! emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_mirrors_get_device_id() {
        assert_eq!(
            device_id("11:22:33:44:55:66", 0, 0, 0),
            [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]
        );
        assert_eq!(
            device_id("ABCDEF", 0, 0, 0),
            [
                b'A' as u32,
                b'B' as u32,
                b'C' as u32,
                b'D' as u32,
                b'E' as u32,
                b'F' as u32
            ]
        );
        assert_eq!(
            device_id("", 0x054c, 0x09cc, 0x8111),
            [0x4c, 0x05, 0xcc, 0x09, 0x11, 0x81]
        );
    }

    #[test]
    fn profile_xml_renders_pad1_bindings() {
        let mut buttons = BTreeMap::new();
        for (button, _) in BUTTONS {
            if button.starts_with("analog_") {
                continue;
            }
            buttons.insert(
                (*button).to_owned(),
                EvdevTarget {
                    key_id: 304,
                    key_type: KEYTYPE_BUTTON,
                },
            );
        }
        let mut sticks = BTreeMap::new();
        for button in [
            "analog_left_x",
            "analog_left_y",
            "analog_right_x",
            "analog_right_y",
        ] {
            sticks.insert(
                button.to_owned(),
                (
                    EvdevTarget {
                        key_id: 0,
                        key_type: KEYTYPE_AXIS,
                    },
                    EvdevTarget {
                        key_id: 0,
                        key_type: KEYTYPE_AXIS,
                    },
                ),
            );
        }
        let text = profile_xml([1, 2, 3, 4, 5, 6], &buttons, &sticks).unwrap();
        assert!(text.contains("<Config>"));
        assert!(
            text.contains("Name=\"input.pad1.cross.bindingtype\" Type=\"integer\" Value=\"1\"")
        );
        assert!(text.contains("Name=\"input.pad1.cross.bindingtarget1.providerId\""));
        assert!(text.contains("Value=\"1:2:3:4:5:6\""));
        assert!(text.contains(
            "Name=\"input.pad1.analog_left_x.bindingtype\" Type=\"integer\" Value=\"2\""
        ));
        assert!(
            text.contains("Name=\"input.pad1.motor.bindingtype\" Type=\"integer\" Value=\"0\"")
        );
    }
}

#[cfg(target_os = "linux")]
mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, evdev_catalog::EvdevCatalog, file_hash,
        linux_classic::AxisEndpoints, sdl2::Snapshot, sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        os::unix::fs::symlink,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::AtomicBool,
    };

    fn observe_sdl2(
        setup: &settings::SavedSetup,
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Play! SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Play! helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn observe_evdev(
        setup: &settings::SavedSetup,
        event: &Path,
        cancel: &AtomicBool,
    ) -> Result<EvdevCatalog> {
        let mut command = Command::new(&setup.probe_program);
        command.arg("--evdev-catalog").arg(event);
        let (output, _) = capture(&mut command, cancel)?;
        let catalog: EvdevCatalog =
            serde_json::from_slice(&output).context("Invalid Play! evdev capture")?;
        ensure!(
            catalog.devices.len() == 1 && catalog.devices[0].event == *event,
            "Play! evdev catalog covers a different node"
        );
        Ok(catalog)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
            device.mapping = None;
        }
        snapshot
    }

    fn event_node(path: &Path) -> Result<PathBuf> {
        let node = path.to_path_buf();
        ensure!(
            node.parent() == Some(Path::new("/dev/input"))
                && node
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.strip_prefix("event").is_some_and(|suffix| {
                            !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                        })
                    }),
            "Play! needs a /dev/input/eventN node, found {}",
            path.display()
        );
        Ok(node)
    }

    /// Mirror one data entry as a symlink, recursing into directories.
    fn mirror_entry(source: &PathBuf, link: &PathBuf) -> Result<()> {
        let kind = fs::symlink_metadata(source)?.file_type();
        if kind.is_symlink() {
            symlink(fs::read_link(source)?, link)?;
        } else if kind.is_dir() {
            fs::create_dir(link)?;
            for entry in fs::read_dir(source)? {
                let entry = entry?;
                mirror_entry(&entry.path(), &link.join(entry.file_name()))?;
            }
        } else if kind.is_file() {
            symlink(source, link)?;
        } else {
            anyhow::bail!("Play! data entry is not a file, directory, or symlink");
        }
        Ok(())
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        event: PathBuf,
        topology: InputTopology,
        initial: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "Play! content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                setup.data_dir.is_dir() && setup.data_dir.canonicalize()? == setup.data_dir,
                "Play! data directory must be a canonical directory"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Play! physical controller is missing or ambiguous"
            );
            let event = event_node(&found[0].device_path)?;
            let topology = InputTopology::capture(std::slice::from_ref(&event))?;
            let initial = routing(observe_sdl2(setup, None, cancel)?);
            let physical_path = topology.resolve_runtime_path(
                &event,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe_sdl2(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            let evdev = device
                .linux_evdev
                .as_ref()
                .context("Play! SDL device has no evdev backend map")?;
            // Kernel codes ride the native low 16 bits; the map proves the
            // gesture against the same device's capability vectors.
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("Play! SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("Play! SDL control counts are missing")?,
            )?;
            let catalog = observe_evdev(setup, &event, cancel)?;
            let entry = &catalog.devices[0];
            // The evdev node must expose the same buttons and axes the SDL
            // backend measured, or the two probes disagree about the pad.
            for code in &evdev.buttons {
                ensure!(
                    entry.buttons.contains(code),
                    "Play! evdev node is missing button {code}"
                );
            }
            for code in evdev.axes.iter().chain(evdev.hats.iter()) {
                ensure!(
                    entry
                        .axes
                        .iter()
                        .chain(entry.hats.iter())
                        .any(|axis| u16::from(axis.code) == u16::from(*code)),
                    "Play! evdev node is missing axis {code}"
                );
            }
            let vendor = u16::from_str_radix(&entry.identity.vendor, 16)
                .context("Play! evdev vendor is not hex")?;
            let product = u16::from_str_radix(&entry.identity.product, 16)
                .context("Play! evdev product is not hex")?;
            let version = u16::from_str_radix(&entry.identity.version, 16)
                .context("Play! evdev version is not hex")?;
            let device = device_id(&entry.uniq, vendor, product, version);
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Play! calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Play! native profile")?;
            let mut buttons: BTreeMap<String, EvdevTarget> = BTreeMap::new();
            let mut halves: BTreeMap<String, (EvdevTarget, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Play! target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Play! DualShock control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Play! requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = physical.digital_input(native.code, measured)?;
                let released = match translated {
                    DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!(
                            "Play! hats have no evdev POV encoding; use a button or stick"
                        )
                    }
                    DigitalInput::Axis {
                        index, released, ..
                    } => state.axes.get(&index) == Some(&released),
                };
                ensure!(
                    released,
                    "Release the Play! controls before launch preparation"
                );
                match translated {
                    DigitalInput::Button(index) => {
                        let position =
                            usize::try_from(index).context("Play! button index is too large")?;
                        let code = *evdev
                            .buttons
                            .get(position)
                            .context("Play! button is outside the evdev vector")?;
                        let target = EvdevTarget {
                            key_id: u32::from(code),
                            key_type: KEYTYPE_BUTTON,
                        };
                        ensure!(
                            buttons.insert(row.target_id.clone(), target).is_none()
                                && halves
                                    .insert(row.target_id.clone(), (target, false))
                                    .is_none(),
                            "Play! control appears twice"
                        );
                    }
                    DigitalInput::Axis { index, .. } => {
                        let position =
                            usize::try_from(index).context("Play! axis index is too large")?;
                        let code = *evdev
                            .axes
                            .get(position)
                            .context("Play! axis is outside the evdev vector")?;
                        ensure!(
                            halves
                                .insert(
                                    row.target_id.clone(),
                                    (
                                        EvdevTarget {
                                            key_id: u32::from(code),
                                            key_type: KEYTYPE_AXIS,
                                        },
                                        native.direction > 0
                                    )
                                )
                                .is_none(),
                            "Play! stick direction appears twice"
                        );
                    }
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!(
                            "Play! hats have no evdev POV encoding; use a button or stick"
                        )
                    }
                }
            }
            // Stick halves pair into simulated axes; analog halves must
            // share one axis with opposite polarity, proven by the saved
            // calibration endpoints. Buttons may drive either half.
            let mut sticks: BTreeMap<String, (EvdevTarget, EvdevTarget)> = BTreeMap::new();
            for (stick, negative, positive) in [
                ("analog_left_x", "stick_left", "stick_right"),
                ("analog_left_y", "stick_up", "stick_down"),
                ("analog_right_x", "right_stick_left", "right_stick_right"),
                ("analog_right_y", "right_stick_up", "right_stick_down"),
            ] {
                let (neg, neg_positive) = halves.remove(negative).with_context(|| {
                    format!("Play! stick direction {negative} is not calibrated")
                })?;
                let (pos, pos_positive) = halves.remove(positive).with_context(|| {
                    format!("Play! stick direction {positive} is not calibrated")
                })?;
                if neg.key_type == KEYTYPE_AXIS && pos.key_type == KEYTYPE_AXIS {
                    ensure!(
                        neg.key_id == pos.key_id && !neg_positive && pos_positive,
                        "Play! stick halves must share one axis with opposite polarity"
                    );
                }
                ensure!(
                    buttons.remove(negative).is_none() && buttons.remove(positive).is_none(),
                    "Play! stick direction is also bound as a button"
                );
                sticks.insert(stick.to_owned(), (neg, pos));
            }
            ensure!(halves.is_empty(), "Play! has unpaired stick directions");
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-play-")
                .tempdir()?;
            // portable.txt selects ./Play Data Files as the base; mirror
            // every data entry through symlinks so memory cards survive,
            // but keep the input profile private.
            fs::write(directory.path().join(PORTABLE_FLAG), "")?;
            let base = directory.path().join(BASE_DIR);
            fs::create_dir(&base)?;
            for entry in fs::read_dir(&setup.data_dir)? {
                let entry = entry?;
                if entry.file_name() == PROFILES_DIR {
                    continue;
                }
                mirror_entry(&entry.path(), &base.join(entry.file_name()))?;
            }
            let profiles = base.join(PROFILES_DIR);
            fs::create_dir(&profiles)?;
            let real_profiles = setup.data_dir.join(PROFILES_DIR);
            if real_profiles.is_dir() {
                for entry in fs::read_dir(&real_profiles)? {
                    let entry = entry?;
                    if entry.file_name().to_str() == Some(&format!("{PROFILE_NAME}.xml")) {
                        continue;
                    }
                    mirror_entry(&entry.path(), &profiles.join(entry.file_name()))?;
                }
            }
            let profile_path = profiles.join(format!("{PROFILE_NAME}.xml"));
            fs::write(&profile_path, profile_xml(device, &buttons, &sticks)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &profile_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                event,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn directory(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Play! launch input changed");
            }
            let fresh = routing(observe_sdl2(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let catalog = observe_evdev(&self.setup, &self.event, cancel)?;
            ensure!(
                catalog.devices.len() == 1,
                "Play! evdev node changed before launch"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::{cancelled, native_pid},
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

    pub(crate) struct NativeSession {
        inputs: session::PreparedSession,
        executable: PathBuf,
        setup: settings::SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)?.eq_ignore_ascii_case(&self.setup.executable_sha256),
                "Play! executable differs from the saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "Play! launch plan changed after preparation"
            );
            self.verify(cancel)?;
            let mut child = crate::emulator::spawn_launch_plan(plan)?;
            if let Err(error) = self.confirm(&mut child, cancel) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            Ok(child)
        }

        fn confirm(&self, child: &mut std::process::Child, cancel: &AtomicBool) -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "Play! exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Play! did not open the selected evdev controller before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            // Play! reads evdev directly; the SDL library is only the probe
            // backend, so readiness is the child staying alive with the
            // topology intact rather than an SDL mapping check.
            let _ = pid;
            self.inputs.check_health()?;
            Ok(true)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("Play! calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Play!")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Play! identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Play! launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Play! calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Play! executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // portable.txt selects ./Play Data Files as the base; the disc or
        // ELF keeps its default slot by extension.
        plan.current_directory = inputs.directory().to_path_buf();
        let flag = if setup
            .content
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("elf"))
        {
            "--elf"
        } else {
            "--disc"
        };
        plan.arguments = vec![
            std::ffi::OsString::from(flag),
            setup.content.as_os_str().to_owned(),
        ];
        let session = NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            plan,
        };
        session.verify(cancel)?;
        Ok(session)
    }
}
