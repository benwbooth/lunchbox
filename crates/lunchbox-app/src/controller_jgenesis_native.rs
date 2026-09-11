//! jgenesis v0.14.1 native `jgenesis-config.toml` input mappings, not
//! libretro bindings.
//!
//! Functional contract pinned to jsgroth/jgenesis
//! cbe7f129e3f5c805a2a2e4318981834192116e90 (v0.14.1):
//! - `frontend/jgenesis-native-config/src/paths.rs`: the config is
//!   `~/.config/jgenesis/jgenesis-config.toml`.
//! - `frontend/jgenesis-native-config/src/input/mappings.rs`:
//!   `GenesisInputMapping` holds `p1`/`p2` (plus turbo twins) of
//!   `GenesisControllerMapping` whose fields are up/left/right/down, a, b,
//!   c, x, y, z, start, mode. Every struct is `#[serde(default)]`, so a
//!   partial TOML containing only `[input.genesis.p1]` merges safely.
//! - `frontend/jgenesis-native-config/src/input/serialize.rs`: each
//!   `GenericInput` serializes as an inline table
//!   `{ type = "Gamepad", gamepad_idx = N, action = "..." }` where action
//!   strings come from `GamepadAction::from_str`: `Button N`,
//!   `Axis N positive|negative`, `Hat N up|down|left|right`.
//! - `frontend/jgenesis-native-driver/src/input.rs`: `gamepad_idx` is the
//!   position of the device among open joysticks in SDL enumeration order
//!   (`regenerate_id_maps`), and button/axis/hat indices are raw SDL
//!   joystick indices — the same kernel-order semantics our SDL2 probe
//!   captures when a single qualifying device pins the index to zero.
use anyhow::{Result, ensure};

/// Genesis target controls: layout id -> jgenesis mapping field.
pub(crate) const CONTROLS: [(&str, &str); 12] = [
    ("a", "a"),
    ("b", "b"),
    ("c", "c"),
    ("x", "x"),
    ("y", "y"),
    ("z", "z"),
    ("start", "start"),
    ("mode", "mode"),
    ("up", "up"),
    ("down", "down"),
    ("left", "left"),
    ("right", "right"),
];

/// One raw host input backing, in SDL raw joystick terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { hat: u32, direction: u8 },
}

impl Binding {
    /// The GenericInput inline-table TOML for gamepad zero.
    pub(crate) fn toml(&self) -> Result<String> {
        let action = match *self {
            Binding::Button(index) => {
                ensure!(index < 32, "jgenesis button index overflows its byte");
                format!("Button {index}")
            }
            Binding::Axis { index, positive } => {
                ensure!(index < 32, "jgenesis axis index overflows its byte");
                format!(
                    "Axis {index} {}",
                    if positive { "positive" } else { "negative" }
                )
            }
            Binding::Hat { hat, direction } => {
                let direction = match direction {
                    1 => "up",
                    2 => "right",
                    4 => "down",
                    8 => "left",
                    _ => anyhow::bail!("jgenesis hat direction must be a cardinal SDL mask"),
                };
                format!("Hat {hat} {direction}")
            }
        };
        Ok(format!(
            "{{ type = \"Gamepad\", gamepad_idx = 0, action = \"{action}\" }}"
        ))
    }
}

/// Render the partial TOML: only the genesis p1/p2 input sections. Every
/// other key in jgenesis's serde-default config stays at its default.
fn write_player(out: &mut String, controls: &[(u8, &str, Binding)]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for (field, binding) in controls
        .iter()
        .map(|(port, field, binding)| (*field, *binding))
    {
        ensure!(
            seen.insert(field),
            "jgenesis mapping field {} appears twice",
            field
        );
        ensure!(
            CONTROLS.iter().any(|(id, _)| *id == field),
            "jgenesis target {} is outside the six-button contract",
            field
        );
        out.push_str(&format!("{field} = {}\n", binding.toml()?));
    }
    Ok(())
}

/// Render the partial TOML: only the genesis p1 input section. Every other
/// key in jgenesis's serde-default config stays at its default.
pub(crate) fn config_toml(p1: &[(u8, &str, Binding)]) -> Result<String> {
    let mut result = String::from("[input.genesis.p1]\n");
    write_player(&mut result, p1)?;
    result.push('\n');
    Ok(result)
}

/// Native SDL3 launch-time verification and private config ownership.
pub(crate) mod session {
    use super::{Binding, CONTROLS, config_toml};
    use crate::controller_bizhawk_guard::InputTopology;
    use crate::controller_catalog::Calibration;
    use crate::controller_native_process::{cancelled, capture};
    use crate::controllers::ControllerDevice;
    use anyhow::{Context, Result, ensure};
    use lunchbox_controller_probe::{Snapshot, file_hash};
    use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

    pub(crate) struct PreparedSession {
        pub(crate) directory: tempfile::TempDir,
        pub(crate) config_path: std::path::PathBuf,
        pub(crate) runtime_path: String,
        pub(crate) topology: InputTopology,
        setup: crate::controller_jgenesis_native::settings::SavedSetup,
        hashes: std::collections::BTreeMap<std::path::PathBuf, String>,
        initial: Snapshot,
    }

    fn observe(
        setup: &crate::controller_jgenesis_native::settings::SavedSetup,
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command.arg("--sdl-library").arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid jgenesis SDL capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "jgenesis helper inspected a different SDL runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.resolved = None;
            device.linux_classic = None;
        }
        snapshot
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &crate::controller_jgenesis_native::settings::SavedSetup,
            calibrations: &HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.validate().map_err(|e| anyhow::anyhow!("{e:#}"))?;
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == setup.controller_id);
            let device = matches
                .next()
                .context("jgenesis selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "jgenesis requires an unambiguous physical controller"
            );
            let selected = device.device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = routing(observe(setup, None, cancel)?);
            let runtime_path = topology.resolve_runtime_path(
                &selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(setup, Some(&runtime_path), cancel)?;
            let device_index = captured
                .devices
                .iter()
                .find(|device| device.path.as_deref() == Some(runtime_path.as_str()))
                .map(|device| device.instance_id)
                .context("jgenesis device disappeared")?;
            let calibration = calibrations
                .get(&setup.controller_id)
                .context("jgenesis calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|p| p.id == "jgenesis:standalone-genesis")
                .context("Missing native jgenesis profile")?;
            let plan = calibration.plan_profile(profile)?;
            let mut entries = Vec::new();
            for row in plan.rows {
                let field = super::CONTROLS
                    .iter()
                    .find(|(control, _)| *control == row.target_id)
                    .map(|(_, field)| *field)
                    .with_context(|| {
                        format!("jgenesis target {} is outside the contract", row.target_id)
                    })?;
                let input = row
                    .input
                    .as_ref()
                    .context("jgenesis control not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("jgenesis requires measured native controls")?;
                let code = native.code & 0xffff;
                let binding = match native.code >> 16 {
                    1 => Binding::Button(u32::from(code as u16)),
                    3 => Binding::Axis {
                        index: u32::from(code as u16),
                        positive: native.direction > 0,
                    },
                    other => anyhow::bail!("jgenesis cannot consume input class {other}"),
                };
                entries.push((0u8, field, binding));
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-jgenesis-")
                .tempdir()?;
            let config_path = directory.path().join("jgenesis-config.toml");
            std::fs::write(&config_path, config_toml(&entries)?)?;
            let mut hashes = std::collections::BTreeMap::new();
            for path in [&setup.probe_program, &setup.sdl_library, &setup.content] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            hashes.insert(config_path.clone(), file_hash(&config_path)?);
            let session = Self {
                directory,
                config_path,
                runtime_path,
                topology,
                setup: setup.clone(),
                hashes,
                initial,
            };
            let _ = device_index;
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "jgenesis launch input changed"
                );
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            ensure!(
                fresh
                    .devices
                    .iter()
                    .any(|device| device.path.as_deref() == Some(self.runtime_path.as_str())),
                "jgenesis controller disappeared"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }

        pub(crate) fn overlay_arguments(
            &self,
            arguments: &[std::ffi::OsString],
        ) -> Result<Vec<std::ffi::OsString>> {
            ensure!(
                self.config_path.is_absolute(),
                "jgenesis private config path must be absolute"
            );
            let mut result = Vec::with_capacity(arguments.len() + 2);
            result.push("--config".into());
            result.push(self.config_path.as_os_str().to_owned());
            result.extend_from_slice(arguments);
            Ok(result)
        }
    }
}

/// Saved native launch setup; no device I/O.
pub(crate) mod settings {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: std::path::PathBuf,
        pub controller_id: String,
        pub probe_program: std::path::PathBuf,
        pub sdl_library: std::path::PathBuf,
        pub executable_sha256: String,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> anyhow::Result<()> {
            anyhow::ensure!(
                !self.emulator_id.trim().is_empty(),
                "jgenesis needs an emulator identity"
            );
            anyhow::ensure!(
                !self.controller_id.trim().is_empty(),
                "jgenesis needs a controller identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                anyhow::ensure!(path.is_absolute(), "jgenesis paths must be absolute");
            }
            anyhow::ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit()),
                "jgenesis needs a trusted executable SHA-256"
            );
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &std::collections::HashMap<
                String,
                crate::controller_catalog::Calibration,
            >,
        ) -> anyhow::Result<serde_json::Value> {
            self.validate()?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|p| p.id == "jgenesis:standalone-genesis")
                .ok_or_else(|| anyhow::anyhow!("Missing native jgenesis profile"))?;
            let calibration = calibrations
                .get(&self.controller_id)
                .ok_or_else(|| anyhow::anyhow!("jgenesis calibration disappeared"))?;
            let mapping = calibration.plan_profile(profile)?;
            Ok(serde_json::json!({"launch_ready":false,
                "launch_integration":"partial",
                "target_layout":profile.target_layout,
                "mapping":mapping}))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> anyhow::Result<()> {
        anyhow::ensure!(setups.len() <= 1024, "Too many jgenesis saved setups");
        Ok(())
    }
}

/// Native SDL3 launch-time verification and private config ownership.
#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::settings::SavedSetup;
    use crate::controller_bizhawk_guard::InputTopology;
    use crate::controller_catalog::Calibration;
    use crate::controller_native_process::{cancelled, capture};
    use crate::controllers::ControllerDevice;
    use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
    use anyhow::{Context, Result, ensure};
    use lunchbox_controller_probe::{Snapshot, file_hash};
    use std::{collections::HashMap, process::Command, sync::atomic::AtomicBool};

    pub(crate) struct NativeSession {
        pub(crate) inputs: crate::controller_jgenesis_native::session::PreparedSession,
        pub(crate) executable: std::path::PathBuf,
        pub(crate) setup: SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> anyhow::Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> anyhow::Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "jgenesis launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> anyhow::Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "jgenesis executable differs from the saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }
    }

    pub(crate) fn prepare(
        setup: &SavedSetup,
        calibrations: &HashMap<String, crate::controller_catalog::Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> anyhow::Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("jgenesis calibrated launch requires native Linux, not Wine/Flatpak");
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "jgenesis identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "jgenesis launch executable differs from selection"
        );
        ensure!(
            original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
            "jgenesis native calibrated launch requires exactly the saved game argument"
        );
        let inputs = crate::controller_jgenesis_native::session::PreparedSession::prepare(
            setup,
            calibrations,
            inventory,
            cancel,
        )?;
        let mut plan = original.clone();
        plan.arguments = inputs.overlay_arguments(&original.arguments)?;
        Ok(NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            plan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_render_the_pinned_inline_tables() {
        assert_eq!(
            Binding::Button(5).toml().unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Button 5\" }"
        );
        assert_eq!(
            Binding::Axis {
                index: 0,
                positive: false
            }
            .toml()
            .unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Axis 0 negative\" }"
        );
        assert_eq!(
            Binding::Hat {
                hat: 0,
                direction: 8
            }
            .toml()
            .unwrap(),
            "{ type = \"Gamepad\", gamepad_idx = 0, action = \"Hat 0 left\" }"
        );
        assert!(
            Binding::Hat {
                hat: 0,
                direction: 3
            }
            .toml()
            .is_err()
        );
        assert!(Binding::Button(64).toml().is_err());
    }

    #[test]
    fn partial_toml_targets_only_genesis_p1() {
        let p1: Vec<(u8, &str, Binding)> = vec![
            (1, "a", Binding::Button(5)),
            (
                1,
                "up",
                Binding::Hat {
                    hat: 0,
                    direction: 1,
                },
            ),
        ];
        let text = config_toml(&p1).unwrap();
        assert!(text.starts_with("[input.genesis.p1]\n"));
        assert!(
            text.contains("a = { type = \"Gamepad\", gamepad_idx = 0, action = \"Button 5\" }\n")
        );
        assert!(
            text.contains("up = { type = \"Gamepad\", gamepad_idx = 0, action = \"Hat 0 up\" }\n")
        );
        assert!(!text.contains("[input.genesis.p2]"));
    }
}
