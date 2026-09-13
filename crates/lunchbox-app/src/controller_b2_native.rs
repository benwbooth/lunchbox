//! b2's native JSON persistence contract.
//!
//! b2 stores joystick identity as SDL device names (three slots), not stable
//! OS ids.  The adapter must preserve unrelated JSON fields when updating the
//! input settings and must reject malformed/ambiguous assignments upstream.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

pub(crate) const CONFIG_FILE: &str = "b2/b2.json";
pub(crate) const SOURCE_COMMIT: &str = "e1c9b84a9460909d4dc2030ad1d4e050012b1910";

pub(crate) fn linux_config_path(xdg_config_home: Option<&Path>, home: &Path) -> PathBuf {
    xdg_config_home
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".config"))
        .join(CONFIG_FILE)
}

pub(crate) fn windows_config_path(local_app_data: &Path) -> PathBuf {
    local_app_data.join(CONFIG_FILE)
}

pub(crate) fn macos_config_path(application_support: &Path) -> PathBuf {
    application_support
        .join("com.tom-seddon.b2")
        .join("b2.json")
}

/// Patch only b2's persisted joystick fields. The three entries select BBC
/// analogue 0, analogue 1, and digital input. An empty entry is b2's exact
/// persisted representation for `(none)`; repeating a name intentionally
/// makes one physical controller drive multiple logical inputs.
pub(crate) fn patch_joysticks(
    baseline: &[u8],
    device_names: [&str; 3],
    swap_joysticks_when_shared: bool,
) -> Result<Vec<u8>> {
    ensure!(baseline.len() <= 4 * 1024 * 1024, "b2 config is too large");
    for name in device_names {
        ensure!(
            name.len() <= 512 && !name.contains('\0'),
            "invalid b2 SDL device name"
        );
    }
    let mut root: Value = serde_json::from_slice(baseline)?;
    let joysticks = root.as_object_mut().map(|o| {
        o.entry("joysticks")
            .or_insert_with(|| Value::Object(Default::default()))
    });
    let Some(Value::Object(joysticks)) = joysticks else {
        anyhow::bail!("b2 joysticks is not an object");
    };
    joysticks.insert("device_names".into(), serde_json::json!(device_names));
    joysticks.insert(
        "swap_joysticks_when_shared".into(),
        Value::Bool(swap_joysticks_when_shared),
    );
    Ok(serde_json::to_vec_pretty(&root)?)
}

pub(crate) mod settings {
    use super::*;

    /// b2's three persisted destinations: analogue port 0, analogue port 1,
    /// and the separate digital joystick input.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Slot {
        pub slot: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        #[serde(default)]
        pub swap_joysticks_when_shared: bool,
        pub slots: Vec<Slot>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "b2 needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, Component::ParentDir)),
                    "b2 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("b2.json"),
                "b2 config_path must name b2.json"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "b2 needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=3).contains(&self.slots.len()),
                "b2 needs one to three slots"
            );
            let mut slots = BTreeSet::new();
            for slot in &self.slots {
                ensure!(
                    slot.slot <= 2
                        && slots.insert(slot.slot)
                        && !slot.controller_id.trim().is_empty(),
                    "b2 slots must be unique values 0, 1, or 2 with a controller"
                );
            }
            Ok(())
        }

        pub(crate) fn review(&self) -> Result<Value> {
            self.validate()?;
            let slots = self
                .slots
                .iter()
                .map(|slot| {
                    serde_json::json!({
                        "slot": slot.slot,
                        "name": match slot.slot {
                            0 => "Analogue joystick 0",
                            1 => "Analogue joystick 1",
                            _ => "Digital joystick",
                        },
                        "controller_id": slot.controller_id,
                    })
                })
                .collect::<Vec<_>>();
            Ok(serde_json::json!({
                "slots": slots,
                "players": [],
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "b2 native Linux launch dispatch is connected but runtime-untested. Launch resolves exact physical devices through the trusted SDL2 runtime, rejects duplicate runtime names, and patches a copied b2.json under a private XDG_CONFIG_HOME. Disk images remain at their original paths and b2 has no persistent save-state file."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many b2 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate b2 emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{file_hash, sdl2::Snapshot};
    use std::{process::Command, sync::atomic::AtomicBool};

    fn observe(
        setup: &settings::SavedSetup,
        control_paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        for path in control_paths {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        serde_json::from_slice(&output).context("Invalid b2 SDL2 capture")
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) config_home: PathBuf,
        runtime_paths: Vec<String>,
        topology: InputTopology,
        snapshot: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.validate()?;
            let mut selected_by_id = BTreeMap::<String, PathBuf>::new();
            for slot in &setup.slots {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == slot.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "b2 physical controller is missing or ambiguous"
                );
                selected_by_id
                    .entry(slot.controller_id.clone())
                    .or_insert_with(|| found[0].device_path.clone());
            }
            let selected = selected_by_id.values().cloned().collect::<Vec<_>>();
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            let mut runtime_by_id = BTreeMap::new();
            let visible_paths = initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref());
            let visible_paths = visible_paths.collect::<Vec<_>>();
            for (id, path) in &selected_by_id {
                runtime_by_id.insert(
                    id.clone(),
                    topology.resolve_runtime_path(path, visible_paths.iter().copied())?,
                );
            }
            let runtime_paths = runtime_by_id.values().cloned().collect::<Vec<_>>();
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            initial.ensure_same_routing(&snapshot)?;

            let mut names_by_id = BTreeMap::new();
            for (id, path) in &runtime_by_id {
                let device = snapshot.device_at_path(path)?;
                ensure!(device.is_game_controller, "b2 needs an SDL2 GameController");
                ensure!(
                    device.mapping.is_some(),
                    "b2 SDL2 GameController mapping is absent"
                );
                let name = device
                    .name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .context("b2 SDL2 GameController name is absent")?;
                ensure!(
                    snapshot
                        .devices
                        .iter()
                        .filter(|candidate| candidate.name.as_deref() == Some(name))
                        .count()
                        == 1,
                    "b2 cannot distinguish attached SDL2 controllers with the same name"
                );
                names_by_id.insert(id.clone(), name.to_owned());
            }
            let mut device_names = ["", "", ""].map(str::to_owned);
            for slot in &setup.slots {
                device_names[usize::from(slot.slot)] = names_by_id[&slot.controller_id].clone();
            }

            let baseline = std::fs::read(&setup.config_path).context("Reading b2.json")?;
            let directory = tempfile::Builder::new().prefix("lunchbox-b2-").tempdir()?;
            let config_home = directory.path().join("config-home");
            let target_dir = config_home.join("b2");
            std::fs::create_dir_all(&target_dir)?;
            let private_config = target_dir.join("b2.json");
            std::fs::write(
                &private_config,
                patch_joysticks(
                    &baseline,
                    [&device_names[0], &device_names[1], &device_names[2]],
                    setup.swap_joysticks_when_shared,
                )?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.content,
                &setup.config_path,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let session = Self {
                directory,
                config_home,
                runtime_paths,
                topology,
                snapshot,
                setup: setup.clone(),
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir(),
                "b2 private config disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(file_hash(path)? == *expected, "b2 launch input changed");
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            self.snapshot.ensure_same_routing(&current)?;
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
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::sync::atomic::AtomicBool;

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
                "b2 executable differs from the saved trusted runtime"
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
                "b2 launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("b2 calibrated launch requires native Linux, not Wine/Flatpak");
        };
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "b2 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "b2 launch executable differs from selection"
        );
        ensure!(
            original
                .arguments
                .iter()
                .any(|argument| argument == setup.content.as_os_str()),
            "b2 launch does not contain the saved content path"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "b2 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, inventory, cancel)?;
        let mut plan = original.clone();
        plan.environment.push((
            "XDG_CONFIG_HOME".into(),
            inputs.config_home.as_os_str().to_owned(),
        ));
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
    #[test]
    fn patch_preserves_other_settings() {
        let out = patch_joysticks(br#"{"video":{"scale":3}}"#, ["a", "b", "c"], true).unwrap();
        let value: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(value["video"]["scale"], 3);
        assert_eq!(value["joysticks"]["device_names"][1], "b");
    }

    #[test]
    fn empty_and_shared_device_names_follow_b2_semantics() {
        let out = patch_joysticks(br#"{}"#, ["Shared Pad", "Shared Pad", ""], false).unwrap();
        let value: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            value["joysticks"]["device_names"],
            serde_json::json!(["Shared Pad", "Shared Pad", ""])
        );
    }

    #[test]
    fn setup_validation_allows_shared_controller_but_not_duplicate_slot() {
        let base = settings::SavedSetup {
            emulator_id: "b2".into(),
            content: "/tmp/game.ssd".into(),
            config_path: "/tmp/b2.json".into(),
            probe_program: "/tmp/probe".into(),
            sdl_library: "/tmp/libSDL2.so".into(),
            executable_sha256: "a".repeat(64),
            swap_joysticks_when_shared: false,
            slots: vec![
                settings::Slot {
                    slot: 0,
                    controller_id: "pad".into(),
                },
                settings::Slot {
                    slot: 1,
                    controller_id: "pad".into(),
                },
            ],
        };
        base.validate().unwrap();
        let mut duplicate = base;
        duplicate.slots[1].slot = 0;
        assert!(duplicate.validate().is_err());
    }
}
