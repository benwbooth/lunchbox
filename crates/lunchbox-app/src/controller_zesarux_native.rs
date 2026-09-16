//! ZEsarUX native Linux real-joystick configuration.
//!
//! The pinned source parses this file as command-line options.  Unlike the
//! SDL-index-only paths, Linux native input accepts an exact device pathname;
//! the writer consequently requires a stable `/dev/input/by-id/` symlink.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

pub(crate) const PROFILE_ID: &str = "zesarux:standalone-spectrum-joystick";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
}

impl Binding {
    fn spec(self) -> String {
        match self {
            Self::Button(index) => index.to_string(),
            Self::Axis { index, positive } => {
                format!("{}{}", if positive { '+' } else { '-' }, index)
            }
        }
    }
}

/// ZX joystick controls consumed by ZEsarUX's `realjoystick` event table.
pub(crate) const CONTROLS: [&str; 5] = ["up", "down", "left", "right", "fire"];

fn valid_device_path(path: &str) -> bool {
    std::path::Path::new(path).is_absolute()
        && !path.ends_with('/')
        && !std::path::Path::new(path)
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        && !path
            .chars()
            .any(|c| c == '"' || c == '\n' || c == '\r' || c.is_control())
}

/// Render an isolated ZEsarUX config file.  It uses `--configfile` at launch;
/// save-state/autosnapshot roots are deliberately caller-owned and are not
/// changed by this mapping-only writer.
pub(crate) fn config_text(
    device_path: &str,
    mappings: &std::collections::BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(
        valid_device_path(device_path),
        "ZEsarUX requires an exact safe absolute joystick path"
    );
    ensure!(
        mappings.len() == CONTROLS.len()
            && mappings.keys().all(|key| CONTROLS.contains(&key.as_str())),
        "ZEsarUX requires exactly the five Kempston controls"
    );
    let mut out = format!("--joystickemulated Kempston\n--realjoystickpath \"{device_path}\"\n");
    let mut used = std::collections::BTreeSet::new();
    for control in CONTROLS {
        let binding = *mappings
            .get(control)
            .ok_or_else(|| anyhow::anyhow!("ZEsarUX control {control} is absent"))?;
        ensure!(used.insert(binding), "ZEsarUX joystick input is reused");
        out.push_str("--joystickevent ");
        out.push_str(&binding.spec());
        out.push(' ');
        out.push_str(control);
        out.push('\n');
    }
    Ok(out)
}

/// Append launch-owned joystick settings to a copied user configuration.
/// Later options win in ZEsarUX's command-style parser, while all unrelated
/// machine, firmware, media and save-state paths remain intact.
pub(crate) fn patch_config(
    baseline: &[u8],
    device_path: &str,
    mappings: &std::collections::BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "ZEsarUX config is too large");
    let original = std::str::from_utf8(baseline).context("ZEsarUX config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "ZEsarUX config contains a NUL byte"
    );
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = original.to_owned();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    output.push_str("# Lunchbox measured native joystick\n");
    output.push_str(&config_text(device_path, mappings)?);
    Ok(output)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        /// Existing user configuration; copied and never edited in place.
        pub config_path: PathBuf,
        pub executable_sha256: String,
        pub controller_id: String,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty() && !self.controller_id.trim().is_empty(),
                "ZEsarUX setup needs emulator and controller identities"
            );
            for path in [&self.content, &self.config_path] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "ZEsarUX setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "ZEsarUX setup needs a trusted executable SHA-256"
            );
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &HashMap<String, Calibration>,
        ) -> Result<serde_json::Value> {
            self.validate()?;
            let calibration = calibrations
                .get(&self.controller_id)
                .context("ZEsarUX controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "ZEsarUX native joystick requires Linux calibration"
            );
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing ZEsarUX Kempston profile")?;
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| {
                    row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())
                }),
                "ZEsarUX needs native calibration for every Kempston control"
            );
            Ok(serde_json::json!({
                "players": [{
                    "player": 1,
                    "controller_id": self.controller_id,
                    "source_layout": calibration.layout,
                    "target_layout": profile.target_layout,
                    "mapping": mapping,
                }],
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch copies the user config, appends a measured joydev Kempston table, and uses a private immutable symlink with --configfile first. Firmware, media, autosnapshot and save-state paths remain caller-owned. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many ZEsarUX saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate ZEsarUX emulator/content setup"
            );
        }
        Ok(())
    }
}

// Linux-only by source contract: the session reads the kernel joystick
// device directly (`linux_classic::read_raw`) for the Kempston mapper.
// Other hosts have no joydev nodes, so no port is staged.
#[cfg(target_os = "linux")]
mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology, controller_catalog::Calibration,
        controller_native_process::cancelled, controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{file_hash, linux_classic::RawMap};
    use std::{
        collections::{BTreeMap, HashMap},
        path::PathBuf,
        sync::atomic::AtomicBool,
    };

    fn binding(raw: &RawMap, input: &crate::controller_catalog::InputBinding) -> Result<Binding> {
        let native = input
            .native
            .as_ref()
            .context("ZEsarUX requires a measured native control")?;
        let code = (native.code & 0xffff) as u16;
        match native.code >> 16 {
            1 => raw
                .buttons
                .iter()
                .position(|candidate| *candidate == code)
                .map(|index| Binding::Button(index as u32))
                .context("ZEsarUX physical button is absent from joydev"),
            3 => {
                let code = u8::try_from(code).context("Invalid ZEsarUX physical axis")?;
                let index = raw
                    .axes
                    .iter()
                    .position(|candidate| *candidate == code)
                    .context("ZEsarUX physical axis is absent from joydev")?;
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("ZEsarUX axis measurements are absent")?;
                let correction = raw
                    .corrections
                    .get(&code)
                    .context("ZEsarUX joydev axis correction is absent")?;
                let released = correction.apply(endpoints.released)?;
                let pressed = correction.apply(endpoints.pressed)?;
                ensure!(
                    released == 0 && pressed != 0,
                    "ZEsarUX axis must return to joydev center and leave it when pressed"
                );
                Ok(Binding::Axis {
                    index: index as u32,
                    positive: pressed > 0,
                })
            }
            _ => anyhow::bail!("ZEsarUX calibration uses an unsupported event type"),
        }
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        private_joystick: PathBuf,
        topology: InputTopology,
        selected_path: PathBuf,
        raw: RawMap,
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
            let matches = inventory
                .iter()
                .filter(|device| device.stable_id == setup.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                matches.len() == 1 && !matches[0].is_virtual,
                "ZEsarUX physical controller is missing or ambiguous"
            );
            let selected_path = matches[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected_path))?;
            let raw = lunchbox_controller_probe::linux_classic::read_raw(&selected_path)?;
            let calibration = calibrations
                .get(&setup.controller_id)
                .context("ZEsarUX calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing ZEsarUX Kempston profile")?;
            let mut mappings = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                let input = row
                    .input
                    .as_ref()
                    .context("ZEsarUX Kempston control is not calibrated")?;
                ensure!(
                    CONTROLS.contains(&row.target_id.as_str()),
                    "ZEsarUX target is outside the Kempston contract"
                );
                ensure!(
                    mappings
                        .insert(row.target_id, binding(&raw, input)?)
                        .is_none(),
                    "ZEsarUX target control appears twice"
                );
            }
            topology.verify()?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-zesarux-native-")
                .tempdir()?;
            let private_joystick = directory.path().join("joystick");
            std::os::unix::fs::symlink(&selected_path, &private_joystick)?;
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared ZEsarUX configuration")?;
            let private_config = directory.path().join("zesarux.conf");
            std::fs::write(
                &private_config,
                patch_config(
                    &baseline,
                    private_joystick
                        .to_str()
                        .context("ZEsarUX private path is not UTF-8")?,
                    &mappings,
                )?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [&setup.content, &setup.config_path, &private_config] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                private_config,
                private_joystick,
                topology,
                selected_path,
                raw,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir()
                    && self.private_config.is_file()
                    && std::fs::read_link(&self.private_joystick)? == self.selected_path,
                "ZEsarUX private joystick session changed"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "ZEsarUX launch input changed"
                );
            }
            // Re-read the exact joydev numbering; topology equality alone does
            // not describe driver-provided button/axis maps.
            ensure!(
                lunchbox_controller_probe::linux_classic::read_raw(&self.selected_path)?
                    == self.raw,
                "ZEsarUX joydev numbering changed"
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
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

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
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "ZEsarUX executable differs from the saved trusted runtime"
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
                "ZEsarUX launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
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
            anyhow::bail!("ZEsarUX calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("zesarux")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "ZEsarUX identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "ZEsarUX launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "ZEsarUX calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "ZEsarUX executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.arguments = vec![
            "--configfile".into(),
            inputs.private_config.as_os_str().to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Linux-only: asserts the stable /dev/input/by-id grammar; other
    /// hosts pin SDL paths instead and never see joydev nodes.
    #[cfg(target_os = "linux")]
    #[test]
    fn emits_source_option_grammar_for_stable_linux_path() {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "up".into(),
            Binding::Axis {
                index: 1,
                positive: false,
            },
        );
        map.insert(
            "down".into(),
            Binding::Axis {
                index: 1,
                positive: true,
            },
        );
        map.insert(
            "left".into(),
            Binding::Axis {
                index: 0,
                positive: false,
            },
        );
        map.insert(
            "right".into(),
            Binding::Axis {
                index: 0,
                positive: true,
            },
        );
        map.insert("fire".into(), Binding::Button(0));
        let text = config_text("/dev/input/by-id/usb-Test-event-joystick", &map).unwrap();
        assert!(text.contains("--realjoystickpath \"/dev/input/by-id/usb-Test-event-joystick\""));
        assert!(text.contains("--joystickevent +0 right"));
    }

    #[test]
    fn rejects_unstable_or_ambiguous_paths() {
        let map = std::collections::BTreeMap::new();
        assert!(config_text("relative/js0", &map).is_err());
        assert!(config_text("/dev/input/by-id/one\"two", &map).is_err());
    }
}
