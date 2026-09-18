//! Native launch-time evdev verification and the private XDG settings file.
//! The user's own Mesen2 configuration is never opened.
use super::{Binding, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{evdev_catalog, file_hash};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

const BTN_GAMEPAD: u16 = 0x130;
const ABS_X: u16 = 0x00;

/// Every `/dev/input/eventN` node in **readdir order**.
///
/// `LinuxKeyManager.cpp CheckForGamepads` iterates
/// `FolderUtilities::GetFilesInFolder("/dev/input/", …)`, a
/// `std::filesystem::directory_iterator`: no sorting anywhere. Lexicographic
/// order is a different sequence on devtmpfs (event10 sorts between event1
/// and event2), so sorting here would assign the wrong Mesen2 pad slot.
fn event_nodes() -> Result<Vec<PathBuf>> {
    Ok(std::fs::read_dir("/dev/input")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("event").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                    })
                })
        })
        .collect())
}

fn catalog(probe: &Path, cancel: &AtomicBool) -> Result<evdev_catalog::EvdevCatalog> {
    let nodes = event_nodes()?;
    let mut command = Command::new(probe);
    for node in &nodes {
        command.arg("--evdev-catalog").arg(node);
    }
    let (output, _) = capture(&mut command, cancel)?;
    serde_json::from_slice(&output).context("Invalid Mesen2 evdev capture")
}

/// Mesen2's acceptance test from LinuxGameController::GetController:
/// `EV_KEY+BTN_GAMEPAD` or `EV_ABS+ABS_X`.
fn qualifies(device: &evdev_catalog::EvdevDevice) -> bool {
    device.buttons.contains(&BTN_GAMEPAD) || device.axes.iter().any(|axis| axis.code == ABS_X)
}

/// Mesen2 opens each candidate `O_RDWR | O_NONBLOCK` and drops it when the
/// open fails, so a node that only its owner can read consumes no pad slot.
/// The catalog reads devices read-only, so this reproduces the other half of
/// the acceptance test before counting slots.
fn mesen_would_open(path: &Path) -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .is_ok()
}

/// Slot index of `selected` among the devices Mesen2 would register, given
/// the catalog's readdir order. Kept separate from the device scan so the
/// counting rule is testable without devices.
fn slot_position<'a>(
    devices: impl IntoIterator<Item = &'a evdev_catalog::EvdevDevice>,
    selected: &Path,
    usable: impl Fn(&Path) -> bool,
) -> Option<u32> {
    let mut slot = 0u32;
    for device in devices {
        if !qualifies(device) || !usable(&device.event) {
            continue;
        }
        if device.event == selected {
            return Some(slot);
        }
        slot += 1;
    }
    None
}

/// The selected node's Mesen2 pad slot: its position among the gamepads
/// Mesen2 registers in `/dev/input` readdir order. Other gamepads simply
/// occupy the slots before it, so a second controller on the host is not an
/// error; only a device Mesen2 would never open is.
fn qualifying_slot<'a>(
    catalog: &'a evdev_catalog::EvdevCatalog,
    event: &Path,
) -> Result<(&'a evdev_catalog::EvdevDevice, u32)> {
    let selected = catalog.device_at_event(event)?;
    ensure!(
        qualifies(selected),
        "Mesen2 only opens gamepads (EV_KEY+BTN_GAMEPAD or EV_ABS+ABS_X); the selected controller is not one"
    );
    ensure!(
        mesen_would_open(event),
        "Mesen2 opens controllers read-write and cannot open {}; fix its device permissions first",
        event.display()
    );
    let slot = slot_position(catalog.devices.iter(), event, mesen_would_open)
        .context("Mesen2 selected gamepad is missing from its own catalog")?;
    ensure!(slot < 20, "Mesen2 addresses twenty pad slots");
    Ok((selected, slot))
}

/// Translate the calibrated controls into KeyMapping codes for the selected
/// device's Mesen2 pad slot, using the setup's system contract.
pub(super) fn calibrated_bindings(
    setup: &settings::SavedSetup,
    calibration: &Calibration,
    device: &evdev_catalog::EvdevDevice,
    pad: u32,
) -> Result<Vec<(String, Binding)>> {
    ensure!(
        calibration.os == "linux",
        "Mesen2 native calibration requires Linux"
    );
    let pce = setup.system == super::SYSTEM_PCE;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| {
            profile.id
                == if pce {
                    settings::PCE_PROFILE_ID
                } else {
                    settings::PROFILE_ID
                }
        })
        .context("Missing native Mesen2 profile")?;
    let controls = if pce {
        super::PCE_CONTROLS
    } else {
        super::CONTROLS
    };
    let mut bindings = Vec::new();
    for row in calibration.plan_profile(profile)?.rows {
        let field = controls
            .iter()
            .find(|(control, _)| *control == row.target_id)
            .map(|(_, field)| (*field).to_owned())
            .with_context(|| {
                format!(
                    "Mesen2 target {} is outside the {} contract",
                    row.target_id,
                    if pce { "PCE" } else { "NES" }
                )
            })?;
        let input = row
            .input
            .context("Mesen2 gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Mesen2 requires measured native controls")?;
        let code = native.code & 0xffff;
        let binding = match native.code >> 16 {
            1 => Binding::from_key(pad, u32::from(code))
                .with_context(|| format!("Mesen2 has no buttonIndex for kernel key {code:#x}"))?,
            3 => {
                let endpoints = input
                    .axis
                    .as_ref()
                    .context("Mesen2 axis measurements are absent")?;
                let axis = device
                    .axes
                    .iter()
                    .chain(&device.hats)
                    .find(|axis| u32::from(axis.code) == code)
                    .with_context(|| format!("Mesen2 device lacks axis {code:#x}"))?;
                let span = f64::from(axis.info.maximum - axis.info.minimum).max(1.0);
                let released = f64::from(endpoints.released - axis.info.minimum) / span;
                let pressed = f64::from(endpoints.pressed - axis.info.minimum) / span;
                ensure!(
                    (0.45..0.55).contains(&released),
                    "Mesen2 axis rest must sit near the middle of the kernel range"
                );
                let positive = pressed > released;
                ensure!(
                    (pressed - released).abs() > super::RANGE_FRACTION,
                    "Mesen2's default 40% axis dead zone cannot represent the measured travel"
                );
                Binding::from_axis(pad, u32::from(code), positive).with_context(|| {
                    format!("Mesen2 has no buttonIndex for kernel axis {code:#x}")
                })?
            }
            other => anyhow::bail!("Mesen2 cannot consume input class {other}"),
        };
        bindings.push((field, binding));
    }
    Ok(bindings)
}

pub(crate) struct PreparedSession {
    directory: tempfile::TempDir,
    /// Private `XDG_CONFIG_HOME`; Mesen2's home is `<config_home>/Mesen2`.
    pub(crate) config_home: PathBuf,
    pub(crate) home: PathBuf,
    /// The file Mesen2 should load: the setup's own content, or the cue/bin
    /// copy staged inside this session for a compressed disc image.
    pub(crate) content: PathBuf,
    topology: InputTopology,
    setup: settings::SavedSetup,
    hashes: std::collections::BTreeMap<PathBuf, String>,
    /// Kernel event node and pinned Mesen2 pad slot for the selected pad.
    event: PathBuf,
    slot: u32,
}

/// Mesen2 keeps everything beside `settings.json` in its home folder. These
/// child directories hold user data, not configuration we author, so they are
/// shared with the private home rather than copied.
const SHARED_HOME_DIRECTORIES: &[&str] = &[
    "Firmware",
    "Saves",
    "SaveStates",
    "Cheats",
    "GameConfig",
    "RecentGames",
    "Satellaview",
    "Debugger",
    "HdPacks",
    "Screenshots",
    "Avi",
    "Movies",
    "Wave",
    "Tests",
    "Backups",
];

/// The user's real Mesen2 home. `ConfigManager.DefaultDocumentsFolder` uses
/// ApplicationData on non-Windows hosts, which .NET maps to `XDG_CONFIG_HOME`
/// (falling back to `~/.config`), not `XDG_DATA_HOME`.
pub(crate) fn real_home() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().context("Missing user directories")?;
    Ok(base.config_dir().join("Mesen2"))
}

/// Seed the private home from the user's own Mesen2 home and patch only the
/// target system's controller contract.
///
/// Two properties matter and are both tested:
/// - nothing outside the target system changes, so the user's video, audio,
///   input and per-system preferences (and their completed first-run state)
///   are exactly what Mesen2 already had;
/// - every data folder Mesen2 keeps beside its settings is shared by symlink,
///   so the PCE CD BIOS, saves, states, per-game configs and captures remain
///   the user's own files. Only `settings.json` is private.
fn seed_private_home(
    source: &Path,
    config_home: &Path,
    bindings: &[(String, Binding)],
    system: &str,
) -> Result<serde_json::Value> {
    let home = config_home.join("Mesen2");
    std::fs::create_dir_all(&home)?;
    for name in SHARED_HOME_DIRECTORIES {
        let source_dir = source.join(name);
        if !source_dir.is_dir() {
            continue;
        }
        std::os::unix::fs::symlink(&source_dir, home.join(name))?;
    }
    let source_settings = source.join("settings.json");
    // Mesen2 writes this file as UTF-8 with a BOM; serde rejects the BOM, and
    // .NET reads either form, so strip it before parsing.
    let raw = std::fs::read(&source_settings)?;
    let text = std::str::from_utf8(raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw))
        .context("Mesen2 settings.json is not UTF-8")?;
    let mut document: serde_json::Value =
        serde_json::from_str(text).context("Invalid Mesen2 settings.json")?;
    let (section, controller, controls) = super::patch_settings(&document, system)?;
    let codes = super::mapping_codes(
        bindings,
        controls,
        if system == super::SYSTEM_PCE {
            "Mesen2 needs every standard PCE control"
        } else {
            "Mesen2 needs every standard NES control"
        },
    )?;
    let root = document
        .as_object_mut()
        .context("Mesen2 settings.json is not an object")?;
    let system_section = root
        .entry(section)
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .with_context(|| format!("Mesen2 {section} section is not an object"))?;
    let port1 = system_section
        .entry("Port1")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Mesen2 Port1 is not an object")?;
    port1.insert("Type".into(), serde_json::json!(controller));
    let mapping = port1
        .entry("Mapping1")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Mesen2 Mapping1 is not an object")?;
    for (field, code) in &codes {
        mapping.insert((*field).into(), serde_json::json!(code));
    }
    // A stale expansion device on port two would compete for the same keys;
    // this contract covers one standard pad per system.
    system_section.insert("Port2".into(), serde_json::json!({"Type": "None"}));
    Ok(document)
}

/// Super CD-ROM² games (TurboGrafx-CD) cannot boot without the CD BIOS.
/// Mesen2 raises its own dialog when it is missing, so fail here instead with
/// the exact folder and file names.
pub(crate) fn require_cd_bios(setup: &settings::SavedSetup) -> Result<()> {
    if setup.system != super::SYSTEM_PCE {
        return Ok(());
    }
    match super::pce_content_kind(&setup.content)? {
        // A staged cue/bin copy needs the same BIOS as the image it came from.
        super::PceContent::Disc | super::PceContent::ConvertibleDisc => require_disc_bios(),
        super::PceContent::Card => Ok(()),
    }
}

/// The BIOS check itself: Super CD-ROM² discs cannot boot without it and
/// Mesen2 answers a missing one with its own dialog.
pub(crate) fn require_disc_bios() -> Result<()> {
    let firmware = real_home()?.join("Firmware");
    let found = ["syscard3.pce", "gecard.pce"]
        .into_iter()
        .any(|name| firmware.join(name).is_file());
    ensure!(
        found,
        "Mesen needs a Super CD-ROM² BIOS to boot this disc. Put syscard3.pce (or gecard.pce) in {}, then launch again. HuCard games do not need it.",
        firmware.display()
    );
    Ok(())
}

/// A `chdman` that can turn a compressed disc image into the cue/bin set
/// Mesen2 boots. Prefers a native executable, then MAME's Flatpak build,
/// which ships the same tool.
pub(crate) enum Chdman {
    Native(PathBuf),
    Flatpak { command: PathBuf, app_id: String },
}

impl Chdman {
    /// Discover the tool. `LUNCHBOX_CHDMAN` overrides discovery for tests and
    /// unusual installs.
    pub(crate) fn discover(cancel: &AtomicBool) -> Result<Self> {
        if let Some(path) = std::env::var_os("LUNCHBOX_CHDMAN") {
            let path = PathBuf::from(path);
            ensure!(
                path.is_file(),
                "LUNCHBOX_CHDMAN does not name a chdman executable"
            );
            return Ok(Self::Native(path));
        }
        if let Some(path) = ["chdman", "/usr/bin/chdman", "/usr/local/bin/chdman"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.is_absolute() && path.is_file())
            .or_else(|| {
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                    .map(|dir| dir.join("chdman"))
                    .find(|path| path.is_file())
            })
        {
            return Ok(Self::Native(path));
        }
        let command = ["/run/current-system/sw/bin/flatpak", "/usr/bin/flatpak"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.is_file())
            .context(
                "Staging a compressed disc image needs chdman: install chdman or MAME (Flatpak) first",
            )?;
        let app_id = "org.mamedev.MAME".to_owned();
        let mut probe = crate::platform_process::host_command(&command);
        probe.args(["info", &app_id]);
        let (_, _) = capture(&mut probe, cancel).context(
            "Staging a compressed disc image needs chdman: install MAME (Flatpak) or chdman first",
        )?;
        Ok(Self::Flatpak { command, app_id })
    }

    /// Extract `source` into `destination` as a cue/bin pair and return the
    /// cue. `destination` must already exist; chdman writes both files
    /// beside the cue, which is where Mesen resolves them from.
    fn extract(&self, source: &Path, destination: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
        let stem = source
            .file_stem()
            .filter(|stem| !stem.is_empty())
            .context("Disc image has no usable file name")?
            .to_string_lossy()
            .into_owned();
        let cue = destination.join(format!("{stem}.cue"));
        let bin = destination.join(format!("{stem}.bin"));
        let mut command = match self {
            Self::Native(program) => {
                let mut command = crate::platform_process::host_command(program);
                command.args(["extractcd", "-i"]).arg(source);
                command
            }
            Self::Flatpak { command, app_id } => {
                let mut process = crate::platform_process::host_command(command);
                process
                    .arg("run")
                    .arg("--command=chdman")
                    .arg(format!("--filesystem={}", destination.display()))
                    .arg(format!(
                        "--filesystem={}:ro",
                        source
                            .parent()
                            .context("Disc image has no containing directory")?
                            .display()
                    ))
                    .arg(app_id)
                    .args(["extractcd", "-i"])
                    .arg(source);
                process
            }
        };
        command.arg("-o").arg(&cue).arg("-ob").arg(&bin);
        cancelled(cancel)?;
        let (_, stderr) = capture(&mut command, cancel).with_context(|| {
            format!(
                "Mesen needs a cue/bin copy of {} and chdman failed",
                source.display()
            )
        })?;
        cancelled(cancel)?;
        ensure!(
            cue.is_file() && bin.is_file(),
            "chdman did not produce a cue/bin pair for {}: {}",
            source.display(),
            String::from_utf8_lossy(&stderr).trim()
        );
        Ok(cue)
    }
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
        let mut matches = inventory
            .iter()
            .filter(|device| device.stable_id == setup.controller_id);
        let device = matches
            .next()
            .context("Mesen2 selected controller is disconnected")?;
        ensure!(
            matches.next().is_none() && !device.is_virtual,
            "Mesen2 requires an unambiguous physical controller"
        );
        let event = device
            .event_paths
            .first()
            .cloned()
            .context("Mesen2 needs the controller's event node")?;
        let topology = InputTopology::capture(std::slice::from_ref(&device.device_path))?;
        let catalog = catalog(&setup.probe_program, cancel)?;
        let (selected, slot) = qualifying_slot(&catalog, &event)?;
        let bindings = calibrated_bindings(
            setup,
            calibrations
                .get(&setup.controller_id)
                .context("Mesen2 calibration disappeared")?,
            selected,
            slot,
        )?;
        let source = real_home()?;
        let source_settings = source.join("settings.json");
        ensure!(
            source_settings.is_file(),
            "Open Mesen once to create {}, then launch through Lunchbox again",
            source_settings.display()
        );
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-mesen2-")
            .tempdir()?;
        // A compressed disc image is staged as cue/bin inside this session's
        // own temp dir, so it disappears with the session and the user's
        // library file is never rewritten.
        let content = match super::pce_content_kind(&setup.content)? {
            super::PceContent::ConvertibleDisc => {
                let media = directory.path().join("media");
                std::fs::create_dir_all(&media)?;
                let cue = Chdman::discover(cancel)?.extract(&setup.content, &media, cancel)?;
                require_disc_bios()?;
                cue
            }
            super::PceContent::Disc => {
                require_cd_bios(setup)?;
                setup.content.clone()
            }
            super::PceContent::Card => setup.content.clone(),
        };
        let config_home = directory.path().join("config");
        let document = seed_private_home(&source, &config_home, &bindings, &setup.system)?;
        let home = config_home.join("Mesen2");
        let keyfile = home.join("settings.json");
        std::fs::write(
            &keyfile,
            serde_json::to_vec_pretty(&document).context("Serializing Mesen2 settings")?,
        )?;
        let mut hashes = std::collections::BTreeMap::new();
        // The setup's own content is a launch input even when a staged cue is
        // what Mesen2 loads, so a changed library file is caught either way.
        for path in [&setup.content, &setup.probe_program, &content] {
            hashes.insert(path.clone(), file_hash(path)?);
        }
        hashes.insert(keyfile.clone(), file_hash(&keyfile)?);
        let session = Self {
            directory,
            config_home,
            home,
            content,
            topology,
            setup: setup.clone(),
            hashes,
            event,
            slot,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        for (path, expected) in &self.hashes {
            ensure!(file_hash(path)? == *expected, "Mesen2 launch input changed");
        }
        // The pad slot was pinned at preparation: another gamepad plugging in
        // ahead of ours would renumber it, so the slot must still match.
        let catalog = catalog(&self.setup.probe_program, cancel)?;
        let (_, slot) = qualifying_slot(&catalog, &self.event)?;
        ensure!(
            slot == self.slot,
            "Mesen2 pad slot moved before launch; review the controller setup again"
        );
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    /// Uncertainty note for the launch banner. Mesen2's own enumeration order
    /// is not observable, so a non-zero slot is our best reading of
    /// `/dev/input` order; the user should know when it applies.
    pub(crate) fn slot_note(&self) -> Option<String> {
        (self.slot != 0).then(|| {
            format!(
                "Mesen2 pad slot {} (gamepads register in /dev/input order; unplug other gamepads if input does not respond)",
                self.slot
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nes_bindings() -> Vec<(String, Binding)> {
        crate::controller_mesen2_native::CONTROLS
            .iter()
            .enumerate()
            .map(|(index, (_, field))| {
                (
                    (*field).to_owned(),
                    Binding::button_index(0, index as u32 + 2).unwrap(),
                )
            })
            .collect()
    }

    /// The user's real home is the source of truth: their other systems, video
    /// and audio settings survive, the target port is authored, and every data
    /// folder is shared rather than copied. This is what keeps the PCE CD BIOS
    /// and the user's saves reachable from the private home.
    #[test]
    fn private_home_seeds_the_users_config_and_shares_their_data() {
        let source = tempfile::tempdir().unwrap();
        let home = source.path();
        std::fs::create_dir_all(home.join("Firmware")).unwrap();
        std::fs::write(home.join("Firmware/syscard3.pce"), b"bios").unwrap();
        std::fs::create_dir_all(home.join("Saves")).unwrap();
        std::fs::write(home.join("Saves/game.sav"), b"save").unwrap();
        std::fs::write(
            home.join("settings.json"),
            serde_json::json!({
                "Version": "2.1.1",
                "ConfigUpgrade": 5,
                "Video": {"AspectRatio": "NoStretching"},
                "Nes": {"Port1": {"Type": "NesController", "Mapping1": {"A": 1, "TurboA": 7}}, "Port2": {"Type": "FourScore"}},
                "PcEngine": {"Port1": {"Type": "PceController", "Mapping1": {"A": 1, "TurboA": 9}}},
            })
            .to_string(),
        )
        .unwrap();

        let config_home = source.path().join("private/config");
        let document = seed_private_home(
            home,
            &config_home,
            &nes_bindings(),
            crate::controller_mesen2_native::SYSTEM_NES,
        )
        .unwrap();
        let private = config_home.join("Mesen2");

        // Unrelated sections and Mesen's own first-run state are preserved.
        assert_eq!(document["Version"], "2.1.1");
        assert_eq!(document["ConfigUpgrade"], 5);
        assert_eq!(document["Video"]["AspectRatio"], "NoStretching");
        // The other system is untouched.
        assert_eq!(document["PcEngine"]["Port1"]["Mapping1"]["TurboA"], 9);
        // The target system's port one is authored, including its unmapped
        // turbo field, and port two is disconnected.
        assert_eq!(document["Nes"]["Port1"]["Mapping1"]["A"], 4098);
        assert_eq!(document["Nes"]["Port1"]["Mapping1"]["TurboA"], 7);
        assert_eq!(document["Nes"]["Port2"]["Type"], "None");
        // Data folders are symlinks to the user's own directories, so the CD
        // BIOS and saves stay live.
        for name in ["Firmware", "Saves"] {
            let shared = private.join(name);
            let metadata = std::fs::symlink_metadata(&shared).unwrap();
            assert!(metadata.file_type().is_symlink(), "{name} must be shared");
            assert_eq!(std::fs::read_link(&shared).unwrap(), home.join(name));
        }
        assert_eq!(
            std::fs::read(private.join("Firmware/syscard3.pce")).unwrap(),
            b"bios"
        );
    }

    /// Mesen2 registers pads in readdir order and skips anything it cannot
    /// open read-write, so slot counting must skip exactly the same devices.
    #[test]
    fn slot_position_matches_mesens_registration_rule() {
        use lunchbox_controller_probe::evdev_catalog::EvdevDevice;
        let device = |name: &str, gamepad: bool| EvdevDevice {
            event: std::path::PathBuf::from(format!("/dev/input/{name}")),
            buttons: if gamepad { vec![0x130] } else { vec![] },
            axes: if gamepad {
                vec![]
            } else {
                vec![lunchbox_controller_probe::evdev_catalog::AbsAxis {
                    // ABS_Y only: no ABS_X, so Mesen2's acceptance test
                    // rejects this node and it holds no slot.
                    code: 0x01,
                    info: lunchbox_controller_probe::evdev_catalog::AbsInfo {
                        value: 0,
                        minimum: -32768,
                        maximum: 32767,
                        fuzz: 16,
                        flat: 128,
                        resolution: 0,
                    },
                }]
            },
            hats: vec![],
            uniq: String::new(),
            identity: lunchbox_controller_probe::evdev_catalog::SysfsIdentity {
                input: std::path::PathBuf::from("/sys/class/input/event0"),
                bustype: "0003".into(),
                vendor: "045e".into(),
                product: "028e".into(),
                version: "0114".into(),
                usb_root: None,
            },
        };
        let devices = vec![
            device("event0", false), // no gamepad buttons and no axes
            device("event1", true),
            device("event2", true),
            device("event3", true),
        ];
        let selected = std::path::Path::new("/dev/input/event3");
        // Every candidate usable: event3 is the third registered pad.
        assert_eq!(slot_position(devices.iter(), selected, |_| true), Some(2));
        // event1 is unusable, exactly as an unopenable node is for Mesen2:
        // event3 shifts down one slot.
        assert_eq!(
            slot_position(devices.iter(), selected, |path| path
                != std::path::Path::new("/dev/input/event1")),
            Some(1)
        );
        // A device Mesen2 would not register is not in the catalog space.
        assert_eq!(
            slot_position(
                devices.iter(),
                std::path::Path::new("/dev/input/event9"),
                |_| true
            ),
            None
        );
        // A non-gamepad node never holds a slot.
        assert_eq!(
            slot_position(
                devices.iter(),
                std::path::Path::new("/dev/input/event0"),
                |_| true
            ),
            None
        );
    }

    /// A missing source config must fail with the actionable instruction, not
    /// write a half-empty file that would make Mesen2 run its setup wizard.
    #[test]
    fn private_home_refuses_a_missing_source_config() {
        let source = tempfile::tempdir().unwrap();
        assert!(
            seed_private_home(
                source.path(),
                &source.path().join("private/config"),
                &nes_bindings(),
                crate::controller_mesen2_native::SYSTEM_NES,
            )
            .is_err()
        );
    }

    /// Mesen2 writes settings.json as UTF-8 with a BOM, which serde rejects.
    #[test]
    fn private_home_reads_a_bom_prefixed_config() {
        let source = tempfile::tempdir().unwrap();
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(br#"{"Version":"2.1.1","Nes":{"Port1":{"Type":"NesController"}}}"#);
        std::fs::write(source.path().join("settings.json"), bytes).unwrap();
        let document = seed_private_home(
            source.path(),
            &source.path().join("private/config"),
            &nes_bindings(),
            crate::controller_mesen2_native::SYSTEM_NES,
        )
        .unwrap();
        assert_eq!(document["Version"], "2.1.1");
        assert_eq!(document["Nes"]["Port1"]["Mapping1"]["A"], 4098);
    }
}
