//! Native BizHawk config encoding, independent of RetroArch remap syntax.
//! Contract: BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628,
//! Config.cs, ConfigExtensions.cs, AnalogBind.cs and NymaCore.Settings.cs.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

const NYMASHOCK: &str = "BizHawk.Emulation.Cores.Sony.PSX.Nymashock";
const PSX_DECK: &str = "PSX Front Panel";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLogicalCalibration {
    pub emulator_id: String,
    pub scope: String,
    pub controller_id: String,
    pub calibration: LogicalCalibration,
}
pub(crate) mod cartridge_identity;
pub(crate) mod database_snapshot;
pub(crate) mod definition_capture;
mod digital_pad;
pub(crate) mod digital_session;
pub(crate) mod gpgx;
pub(crate) mod guided;
pub(crate) mod neshawk;
pub(crate) mod pcehawk;
pub(crate) mod smshawk;
pub(crate) mod snes9x;
pub(crate) mod turbonyma;

pub(crate) use lunchbox_controller_probe::sdl2_physical::PhysicalMap;

/// The pinned Linux package puts its no-SONAME SDL build here, and its launcher
/// prepends dll/ to LD_LIBRARY_PATH. Do not let an unrelated system SDL stand in
/// for an existing bundled library. Absence is not proof of a custom launcher's
/// search path; that runtime contract still belongs to its launch adapter.
fn verify_bundled_sdl(exe_directory: &Path, configured_library: &Path) -> Result<()> {
    let bundled = exe_directory.join("dll/libSDL2.so");
    match std::fs::symlink_metadata(&bundled) {
        Ok(_) => ensure!(
            std::fs::canonicalize(&bundled).context("Resolving bundled BizHawk SDL2")?
                == std::fs::canonicalize(configured_library)
                    .context("Resolving selected BizHawk SDL2")?,
            "Selected probe library differs from this BizHawk installation's bundled dll/libSDL2.so"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("Inspecting bundled BizHawk SDL2"),
    }
    Ok(())
}

pub(crate) struct CapturedGesture {
    pub context: lunchbox_controller_probe::sdl2_mapping::MappingContext,
    pub changes: Vec<lunchbox_controller_probe::sdl2_mapping::OutputChange>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalCalibration {
    pub layout: String,
    pub context: lunchbox_controller_probe::sdl2_mapping::MappingContext,
    pub bindings: BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
}

impl LogicalCalibration {
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        let layout = crate::controller_catalog::catalog()
            .layout(&self.layout)
            .context("Unknown logical calibration layout")?;
        ensure!(
            !self.bindings.is_empty() && self.bindings.len() <= layout.controls.len(),
            "Invalid logical calibration binding count"
        );
        let mapping = lunchbox_controller_probe::sdl2_mapping::parse(&self.context.mapping)?;
        for (id, gesture) in &self.bindings {
            let control = layout
                .controls
                .iter()
                .find(|control| control.id == *id)
                .context("Logical calibration control is absent from its layout")?;
            let output = mapping
                .iter()
                .find(|binding| binding.output == gesture.output)
                .context("Logical calibration output is absent from SDL2 mapping")?;
            ensure!(
                gesture.analog == output.output_range.is_some()
                    && gesture.released != gesture.pressed,
                "Logical calibration has inconsistent output type or unchanged samples"
            );
            let valid = |value: i32| {
                if gesture.analog {
                    (-32768..=32767).contains(&value)
                } else {
                    (0..=1).contains(&value)
                }
            };
            ensure!(
                valid(gesture.released) && valid(gesture.pressed),
                "Invalid logical calibration samples"
            );
            ensure!(
                !control.analog || gesture.analog,
                "An analog layout control requires a measured SDL2 axis"
            );
        }
        Ok(())
    }
}

/// Two-phase calibration owned by the app, not a guessed neutral launch sample.
/// The UI starts this after the user releases controls, then finishes it while
/// the intended control is held. Dropping it cancels without saving anything.
#[cfg(target_os = "linux")]
pub(crate) struct NativeGestureCapture {
    topology: crate::controller_bizhawk_guard::InputTopology,
    released: lunchbox_controller_probe::sdl2::Snapshot,
    path: String,
    probe_program: PathBuf,
    sdl_library: PathBuf,
    working_directory: PathBuf,
    exe_directory: PathBuf,
    environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

#[cfg(target_os = "linux")]
impl NativeGestureCapture {
    pub(crate) fn begin(
        selected_path: &Path,
        probe_program: &Path,
        sdl_library: &Path,
        working_directory: &Path,
        exe_directory: &Path,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Self> {
        verify_bundled_sdl(exe_directory, sdl_library)?;
        let topology =
            crate::controller_bizhawk_guard::InputTopology::capture(&[selected_path.to_owned()])?;
        let discovery = probe_runtime(
            probe_program,
            sdl_library,
            working_directory,
            environment,
            &[],
            cancel,
        )?;
        let path = topology.resolve_runtime_path(
            selected_path,
            discovery
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref()),
        )?;
        let released = probe_runtime(
            probe_program,
            sdl_library,
            working_directory,
            environment,
            std::slice::from_ref(&path),
            cancel,
        )?;
        topology.verify()?;
        let device = released.device_at_path(&path)?;
        ensure!(
            device.is_game_controller,
            "Use physical calibration for SDL2 raw joysticks"
        );
        let mapping = lunchbox_controller_probe::sdl2_mapping::parse(
            device
                .mapping
                .as_deref()
                .context("Selected SDL2 controller has no effective mapping")?,
        )?;
        device
            .sampled_state
            .as_ref()
            .context("SDL2 released sample is absent")?
            .outputs(
                &mapping,
                device.controls.as_ref().context("SDL2 counts are absent")?,
            )?;
        Ok(Self {
            topology,
            released,
            path,
            probe_program: probe_program.to_owned(),
            sdl_library: sdl_library.to_owned(),
            working_directory: working_directory.to_owned(),
            exe_directory: exe_directory.to_owned(),
            environment: environment.to_vec(),
        })
    }

    pub(crate) fn finish(self, cancel: &std::sync::atomic::AtomicBool) -> Result<CapturedGesture> {
        verify_bundled_sdl(&self.exe_directory, &self.sdl_library)?;
        self.topology.verify()?;
        let pressed = probe_runtime(
            &self.probe_program,
            &self.sdl_library,
            &self.working_directory,
            &self.environment,
            std::slice::from_ref(&self.path),
            cancel,
        )?;
        self.topology.verify()?;
        let changes = self
            .released
            .game_controller_changes(&pressed, &self.path)?;
        ensure!(
            !changes.is_empty(),
            "No SDL2 control change detected; repeat the gesture"
        );
        Ok(CapturedGesture {
            context: lunchbox_controller_probe::sdl2_mapping::MappingContext::capture(
                &self.released,
                &self.path,
            )?,
            changes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLaunchSettings {
    pub emulator_id: String,
    pub exe_directory: PathBuf,
    pub probe_program: PathBuf,
    pub sdl_library: PathBuf,
    /// Opt into direct Mono invocation rather than executing an external
    /// EmuHawk wrapper whose environment changes the probe cannot observe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mono_program: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub library_paths: Vec<PathBuf>,
    /// Additional managed assembly directories for explicit split packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub managed_paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unmanaged_directory: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_directory: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_config: Option<PathBuf>,
    pub multitaps: [bool; 2],
    /// Absent digital-core selections preserve the legacy Nymashock contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snes9x_ports: Option<[snes9x::PadPort; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neshawk_ports: Option<[neshawk::PadPort; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smshawk_system: Option<smshawk::System>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pcehawk_ports: Option<[bool; 5]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turbonyma_topology: Option<turbonyma::Topology>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpgx_topology: Option<gpgx::Topology>,
    pub players: Vec<NativePlayerSettings>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePlayerSettings {
    pub controller_id: String,
    #[serde(default)]
    pub normalized_input: bool,
    pub virtual_port: u8,
    pub dualshock: bool,
    #[serde(default)]
    pub dualanalog: bool,
    #[serde(default)]
    pub analog_joystick: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rhythm: Option<DigitalPeripheral>,
    #[serde(default)]
    pub negcon: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<PointerPeripheral>,
    #[serde(default)]
    pub desktop_cursor: bool,
    #[serde(default = "default_mouse_speed")]
    pub mouse_speed_basis_points: u16,
    pub analog_toggle_id: Option<String>,
    /// Fractional deadzone in ten-thousandths, avoiding floating-point settings.
    pub deadzone_basis_points: u16,
    pub rumble: bool,
}

fn default_mouse_speed() -> u16 {
    10000
}

pub(crate) fn mode_layout_id(
    dualshock: bool,
    dualanalog: bool,
    analog_joystick: bool,
    rhythm: Option<DigitalPeripheral>,
    negcon: bool,
    pointer: Option<PointerPeripheral>,
) -> &'static str {
    if let Some(kind) = pointer {
        kind.layout_id()
    } else if negcon {
        "playstation-negcon"
    } else if let Some(kind) = rhythm {
        kind.layout_id()
    } else if analog_joystick {
        "playstation-analog-joystick"
    } else if dualshock || dualanalog {
        "dualshock"
    } else {
        "playstation-digital"
    }
}

/// Semantic candidates only: callers still validate calibration, live SDL
/// context and native gesture translation. Keep preview and launch identical.
pub(crate) fn calibrated_control_ids<'a>(
    physical: &'a crate::controller_catalog::Calibration,
    logical: Option<&LogicalCalibration>,
    reserved_toggle: Option<&str>,
) -> std::collections::BTreeSet<&'a str> {
    physical
        .bindings
        .keys()
        .map(String::as_str)
        .filter(|id| Some(*id) != reserved_toggle)
        .filter(|id| logical.is_none_or(|saved| saved.bindings.contains_key(*id)))
        .collect()
}

pub(crate) fn requested_control_ids(
    target: &crate::controller_catalog::Layout,
    desktop_cursor: bool,
) -> std::collections::BTreeSet<&str> {
    target
        .controls
        .iter()
        .filter(|control| !desktop_cursor || !control.analog)
        .map(|control| control.id.as_str())
        .collect()
}

impl NativeLaunchSettings {
    pub(crate) fn scope_id(&self) -> &'static str {
        if self.snes9x_ports.is_some() {
            "snes9x"
        } else if self.neshawk_ports.is_some() {
            "neshawk"
        } else if let Some(system) = self.smshawk_system {
            system.layout_id()
        } else if self.pcehawk_ports.is_some() {
            "pcehawk"
        } else if self.turbonyma_topology.is_some() {
            "turbonyma"
        } else if self.gpgx_topology.is_some() {
            "gpgx"
        } else {
            "nymashock"
        }
    }

    pub(crate) fn matches_platform(&self, platform: &str) -> bool {
        let native_platforms: &[&str] = if self.snes9x_ports.is_some() {
            &[
                "Super Nintendo Entertainment System",
                "Super Nintendo",
                "SNES",
                "Nintendo Super Famicom",
                "Super Famicom",
            ]
        } else if self.neshawk_ports.is_some() {
            &["Nintendo Entertainment System", "NES"]
        } else if let Some(system) = self.smshawk_system {
            match system {
                crate::controller_bizhawk::smshawk::System::MasterSystem => &[
                    "Sega Master System",
                    "Sega - Master System",
                    "Master System",
                    "SMS",
                ],
                crate::controller_bizhawk::smshawk::System::GameGear => {
                    &["Sega Game Gear", "Sega - Game Gear", "Game Gear", "GG"]
                }
                crate::controller_bizhawk::smshawk::System::Sg1000 => {
                    &["Sega SG-1000", "Sega - SG-1000", "SG-1000", "SG1000"]
                }
            }
        } else if self.pcehawk_ports.is_some() || self.turbonyma_topology.is_some() {
            &[
                "NEC TurboGrafx-16",
                "TurboGrafx-16",
                "NEC PC Engine",
                "PC Engine",
                "NEC - PC Engine - TurboGrafx 16",
                "NEC TurboGrafx-CD",
                "TurboGrafx-CD",
                "NEC PC Engine CD",
                "PC Engine CD",
                "NEC - PC Engine CD - TurboGrafx-CD",
                "NEC SuperGrafx",
                "NEC PC Engine SuperGrafx",
                "SuperGrafx",
                "NEC - SuperGrafx",
            ]
        } else if self.gpgx_topology.is_some() {
            &[
                "Sega Genesis",
                "Genesis",
                "Sega Mega Drive",
                "Mega Drive",
                "Sega - Mega Drive - Genesis",
                "Sega CD",
                "Sega Mega CD",
                "Mega CD",
                "Sega - Mega-CD - Sega CD",
            ]
        } else {
            &[
                "Sony Playstation",
                "PlayStation",
                "Sony PlayStation 1",
                "PSX",
            ]
        };
        native_platforms
            .iter()
            .any(|name| platform.trim().eq_ignore_ascii_case(name))
    }
    pub(crate) fn digital_deck(&self) -> Option<digital_session::DigitalDeck> {
        self.snes9x_ports
            .map(digital_session::DigitalDeck::Snes9x)
            .or_else(|| {
                self.neshawk_ports
                    .map(digital_session::DigitalDeck::NesHawk)
            })
            .or_else(|| {
                self.smshawk_system
                    .map(digital_session::DigitalDeck::SmsHawk)
            })
            .or_else(|| {
                self.pcehawk_ports
                    .map(digital_session::DigitalDeck::PceHawk)
            })
            .or_else(|| {
                self.turbonyma_topology
                    .map(|topology| digital_session::DigitalDeck::TurboNyma {
                        ports: topology.ports,
                        multitap: topology.multitap,
                    })
            })
            .or_else(|| self.gpgx_topology.map(digital_session::DigitalDeck::Gpgx))
    }

    pub(crate) fn working_directory(&self) -> &Path {
        self.data_directory
            .as_deref()
            .unwrap_or(&self.exe_directory)
    }
    pub(crate) fn controlled_environment(
        &self,
    ) -> Result<Vec<(std::ffi::OsString, std::ffi::OsString)>> {
        self.validate()?;
        if self.mono_program.is_none() {
            return Ok(Vec::new());
        }
        let sdl_directory = self
            .sdl_library
            .parent()
            .context("SDL2 library has no directory")?;
        let unmanaged = self
            .unmanaged_directory
            .clone()
            .unwrap_or_else(|| self.exe_directory.join("dll"));
        // Staging validates only saved structure. At launch, reject missing
        // search roots before a probe or Mono can silently search elsewhere.
        for (role, directory) in [
            ("installation", self.exe_directory.as_path()),
            ("SDL library", sdl_directory),
            ("native BizHawk library", unmanaged.as_path()),
            ("working/data", self.working_directory()),
        ]
        .into_iter()
        .chain(
            self.library_paths
                .iter()
                .map(|path| ("native dependency", path.as_path())),
        )
        .chain(
            self.managed_paths
                .iter()
                .map(|path| ("managed assembly", path.as_path())),
        ) {
            let metadata = std::fs::metadata(directory).with_context(|| {
                format!(
                    "Cannot access BizHawk {role} directory {}",
                    directory.display()
                )
            })?;
            ensure!(
                metadata.is_dir(),
                "BizHawk {role} path is not a directory: {}",
                directory.display()
            );
        }
        let mut paths = vec![
            sdl_directory.to_owned(),
            unmanaged.clone(),
            self.exe_directory.join("dll"),
            self.exe_directory.clone(),
        ];
        paths.extend(self.library_paths.iter().cloned());
        let search =
            std::env::join_paths(paths).context("Invalid native library search directories")?;
        let managed_search = std::env::join_paths(
            std::iter::once(self.exe_directory.join("dll"))
                .chain(self.managed_paths.iter().cloned()),
        )
        .context("Invalid managed assembly search directories")?;
        Ok(vec![
            ("LD_LIBRARY_PATH".into(), search),
            (
                "BIZHAWK_HOME".into(),
                self.exe_directory.as_os_str().to_owned(),
            ),
            ("BIZHAWK_INT_SYSLIB_PATH".into(), unmanaged.into_os_string()),
            (
                "BIZHAWK_DATA_HOME".into(),
                self.working_directory().as_os_str().to_owned(),
            ),
            ("MONO_PATH".into(), managed_search),
            ("MONO_CRASH_NOFILE".into(), "1".into()),
            ("MONO_WINFORMS_XIM_STYLE".into(), "disabled".into()),
        ])
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.library_paths.len() + self.managed_paths.len() <= 64,
            "Too many native or managed library search directories"
        );
        for path in self
            .mono_program
            .iter()
            .chain(self.library_paths.iter())
            .chain(self.managed_paths.iter())
            .chain(self.unmanaged_directory.iter())
            .chain(self.data_directory.iter())
            .chain(self.base_config.iter())
        {
            ensure!(
                path.is_absolute()
                    && path.to_str().is_some_and(|text| text.len() <= 4096
                        && !text.chars().any(char::is_control)
                        && !text.contains(':')),
                "Native Mono and library paths must be absolute, bounded paths without search separators"
            );
        }
        ensure!(
            self.mono_program.is_some()
                || (self.library_paths.is_empty()
                    && self.managed_paths.is_empty()
                    && self.unmanaged_directory.is_none()
                    && self.data_directory.is_none()
                    && self.base_config.is_none()),
            "Explicit split-runtime paths require direct Mono mode"
        );
        ensure!(
            !self.emulator_id.trim().is_empty() && self.emulator_id.len() <= 256,
            "Native BizHawk emulator identity is missing or oversized"
        );
        for path in [&self.exe_directory, &self.probe_program, &self.sdl_library] {
            ensure!(
                path.is_absolute()
                    && path.to_str().is_some_and(
                        |text| text.len() <= 4096 && !text.chars().any(char::is_control)
                    ),
                "Native BizHawk runtime paths must be absolute, bounded UTF-8 paths"
            );
        }
        ensure!(
            !self.players.is_empty() && self.players.len() <= 8,
            "Select one to eight BizHawk players"
        );
        ensure!(
            [
                self.snes9x_ports.is_some(),
                self.neshawk_ports.is_some(),
                self.smshawk_system.is_some(),
                self.pcehawk_ports.is_some(),
                self.turbonyma_topology.is_some(),
                self.gpgx_topology.is_some()
            ]
            .into_iter()
            .filter(|selected| *selected)
            .count()
                <= 1,
            "Choose only one native digital core"
        );
        let available = if let Some(deck) = self.digital_deck() {
            let count = usize::from(deck.players());
            deck.validate_players(
                self.players
                    .iter()
                    .map(|player| player.virtual_port.saturating_add(1)),
            )?;
            ensure!(
                self.multitaps == [false, false],
                "Native digital cores use their own topology, not PlayStation multitaps"
            );
            ensure!(
                self.players.iter().all(|player| !player.dualshock
                    && !player.dualanalog
                    && !player.analog_joystick
                    && player.rhythm.is_none()
                    && !player.negcon
                    && player.pointer.is_none()
                    && !player.desktop_cursor
                    && player.analog_toggle_id.is_none()
                    && !player.rumble),
                "Native digital cores require controls without rumble or PlayStation peripheral modes"
            );
            count
        } else {
            2 + 3 * self.multitaps.iter().filter(|enabled| **enabled).count()
        };
        let mut ports = std::collections::BTreeSet::new();
        let mut devices = std::collections::BTreeSet::new();
        ensure!(
            self.players
                .iter()
                .filter(|player| player.desktop_cursor)
                .count()
                <= 1,
            "Only one player can own BizHawk's desktop cursor"
        );
        for player in &self.players {
            ensure!(
                (1..=40000).contains(&player.mouse_speed_basis_points),
                "Mouse speed must be greater than zero and at most 400 percent"
            );
            ensure!(
                !player.desktop_cursor
                    || matches!(
                        player.pointer,
                        Some(PointerPeripheral::GunCon | PointerPeripheral::Justifier)
                    ),
                "Desktop cursor aim requires a lightgun mode"
            );
            ensure!(
                !player.normalized_input || !player.rumble,
                "Normalized gamepad transport does not forward rumble"
            );
            ensure!(
                (player.virtual_port as usize) < available && ports.insert(player.virtual_port),
                "Invalid or repeated native BizHawk port"
            );
            ensure!(
                !player.controller_id.trim().is_empty()
                    && player.controller_id.len() <= 1024
                    && devices.insert(&player.controller_id),
                "Invalid or repeated native BizHawk controller"
            );
            ensure!(
                player.deadzone_basis_points < 10000,
                "Native BizHawk deadzone must be below 100 percent"
            );
            ensure!(
                [
                    player.dualshock,
                    player.dualanalog,
                    player.analog_joystick,
                    player.rhythm.is_some(),
                    player.negcon,
                    player.pointer.is_some()
                ]
                .into_iter()
                .filter(|mode| *mode)
                .count()
                    <= 1,
                "Select only one PlayStation controller mode"
            );
            ensure!(
                player.dualshock == player.analog_toggle_id.is_some(),
                "DualShock requires an explicit analog toggle"
            );
            if let Some(id) = &player.analog_toggle_id {
                ensure!(
                    !id.trim().is_empty() && id.len() <= 256 && !id.chars().any(char::is_control),
                    "Invalid DualShock analog-toggle control ID"
                );
            }
            ensure!(
                player.dualshock || !player.rumble,
                "Digital PlayStation controllers have no rumble"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
pub(crate) struct NativePadRequest<'a> {
    pub calibration: &'a crate::controller_catalog::Calibration,
    pub logical: Option<&'a LogicalCalibration>,
    pub normalized_input: bool,
    /// Caller resolves this from the selected physical controller identity.
    pub runtime_path: &'a str,
    pub virtual_port: u8,
    pub dualshock: bool,
    pub dualanalog: bool,
    pub analog_joystick: bool,
    pub rhythm: Option<DigitalPeripheral>,
    pub negcon: bool,
    pub pointer: Option<PointerPeripheral>,
    pub desktop_cursor: bool,
    pub mouse_speed: f32,
    pub analog_toggle_id: Option<&'a str>,
    pub deadzone: f32,
    pub rumble: bool,
}

/// Native BizHawk preparation transaction. Caller establishes that `plan` is
/// EmuHawk in this runtime and supplies its actual executable directory, not a
/// wrapper's directory. This does not yet support Flatpak or Wine namespaces.
#[cfg(target_os = "linux")]
pub(crate) fn prepare_native_session(
    plan: &mut crate::emulator::LaunchPlan,
    exe_directory: &Path,
    probe_program: &Path,
    sdl_library: &Path,
    requests: &[NativePadRequest<'_>],
    multitaps: [bool; 2],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<crate::controller_launch::CalibratedLaunch> {
    use std::sync::atomic::Ordering;
    verify_bundled_sdl(exe_directory, sdl_library)?;
    let assembly = exe_directory.join("EmuHawk.exe");
    let runtime_artifacts = [
        plan.program.as_path(),
        assembly.as_path(),
        sdl_library,
        probe_program,
    ]
    .into_iter()
    .map(RuntimeArtifact::capture)
    .collect::<Result<Vec<_>>>()?;
    ensure!(
        !requests.is_empty() && requests.len() <= 8,
        "Select between one and eight native controllers"
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Native controller preparation cancelled"
    );
    let paths: Vec<String> = requests
        .iter()
        .map(|request| request.runtime_path.to_owned())
        .collect();
    ensure!(
        paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == paths.len(),
        "Native controllers must have distinct runtime paths"
    );
    let topology = crate::controller_bizhawk_guard::InputTopology::capture(
        &paths.iter().map(PathBuf::from).collect::<Vec<_>>(),
    )?;
    let discovery = probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &[],
        cancel,
    )?;
    let runtime_paths = paths
        .iter()
        .map(|path| {
            topology.resolve_runtime_path(
                Path::new(path),
                discovery
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        runtime_paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == runtime_paths.len(),
        "Selected controllers resolve to the same SDL2 node"
    );
    let inventory = probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &runtime_paths,
        cancel,
    )?;
    topology.verify()?;
    ensure_discovery_routing(discovery, &inventory)?;
    let mut selected = Vec::with_capacity(requests.len());
    let mut warnings = Vec::new();
    for (request, runtime_path) in requests.iter().zip(&runtime_paths) {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Native controller preparation cancelled"
        );
        let (mode, assignment_warnings) = if inventory
            .device_at_path(runtime_path)?
            .is_game_controller
        {
            if request.normalized_input {
                let generated =
                    normalized_logical_calibration(request.calibration, &inventory, runtime_path)?;
                let normalized = NativePadRequest {
                    logical: Some(&generated),
                    ..*request
                };
                logical_pad_from_calibration(&normalized, &inventory, runtime_path)?
            } else {
                logical_pad_from_calibration(request, &inventory, runtime_path)?
            }
        } else if let Some(kind) = request.rhythm {
            raw_peripheral_from_calibration(request.calibration, &inventory, runtime_path, kind)?
        } else if let Some(kind) = request.pointer {
            raw_pointer_from_calibration(
                request.calibration,
                &inventory,
                runtime_path,
                kind,
                request.desktop_cursor,
                request.deadzone,
                request.mouse_speed,
            )?
        } else if request.negcon {
            raw_negcon_from_calibration(
                request.calibration,
                &inventory,
                runtime_path,
                request.deadzone,
            )?
        } else {
            raw_pad_from_probed_calibration(
                request.calibration,
                &inventory,
                runtime_path,
                request.dualshock,
                request.dualanalog,
                request.analog_joystick,
                request.analog_toggle_id,
                request.deadzone,
                request.rumble,
            )?
        };
        warnings.extend(
            assignment_warnings
                .into_iter()
                .map(|warning| format!("Port {}: {warning}", u16::from(request.virtual_port) + 1)),
        );
        selected.push(SelectedPad {
            virtual_port: request.virtual_port,
            runtime_path: runtime_path.clone(),
            mode,
        });
    }
    let mut arguments = plan.arguments.clone();
    let mut config = prepare_selected_arguments(
        &mut arguments,
        &plan.current_directory,
        exe_directory,
        &inventory,
        &selected,
        multitaps,
    )?;
    config.runtime_artifacts = runtime_artifacts;
    let fresh = probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &runtime_paths,
        cancel,
    )?;
    inventory.ensure_same_routing(&fresh)?;
    verify_bundled_sdl(exe_directory, sdl_library)?;
    topology.verify()?;
    config.verify_source()?;
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Native controller preparation cancelled"
    );
    let mut description = format!(
        "Prepared {} calibrated controller(s) for native BizHawk/Nymashock",
        requests.len()
    );
    if !warnings.is_empty() {
        description.push_str(" · ");
        description.push_str(&warnings.join(" · "));
    }
    let session =
        crate::controller_launch::CalibratedLaunch::from_bizhawk(config, topology, description);
    session.check_launch_inputs()?;
    plan.arguments = arguments;
    Ok(session)
}

fn ensure_discovery_routing(
    mut discovery: lunchbox_controller_probe::sdl2::Snapshot,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
) -> Result<()> {
    // Opening devices adds metadata, not permission to change ordered identities.
    let mut opened = lunchbox_controller_probe::sdl2::Snapshot {
        schema_version: inventory.schema_version,
        library: inventory.library.clone(),
        library_sha256: inventory.library_sha256.clone(),
        input_environment_sha256: inventory.input_environment_sha256.clone(),
        version: inventory.version,
        devices: inventory.devices.clone(),
    };
    for device in discovery
        .devices
        .iter_mut()
        .chain(opened.devices.iter_mut())
    {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
    }
    discovery.ensure_same_routing(&opened)
}

/// Run the trusted probe executable in the native emulator's environment.
/// Library loading stays outside the GUI process. This function never launches
/// EmuHawk or falls back to a different SDL library when probing fails.
pub(crate) fn probe_runtime(
    probe_program: &Path,
    sdl_library: &Path,
    working_directory: &Path,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
    control_paths: &[String],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<lunchbox_controller_probe::sdl2::Snapshot> {
    use std::process::{Command, Stdio};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    ensure!(
        probe_program.is_absolute() && sdl_library.is_absolute() && working_directory.is_absolute(),
        "Resolve the native probe, SDL library, and working directory first"
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Native controller probe cancelled"
    );
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-sdl2-probe-")
        .tempdir()?;
    let stdout_path = directory.path().join("stdout.json");
    let stderr_path = directory.path().join("stderr.txt");
    let stdout = std::fs::File::create(&stdout_path)?;
    let stderr = std::fs::File::create(&stderr_path)?;
    let mut command = Command::new(probe_program);
    command
        .current_dir(working_directory)
        .envs(environment.iter().cloned())
        .args(["--sdl2-inventory", "--sdl-library"])
        .arg(sdl_library)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    for path in control_paths {
        command.arg("--sdl2-controls-for-path").arg(path);
    }
    struct ProbeChild(Option<std::process::Child>);
    impl Drop for ProbeChild {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let mut child = ProbeChild(Some(
        command
            .spawn()
            .context("Starting isolated SDL2 controller probe")?,
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    const MAX_OUTPUT: u64 = 4 * 1024 * 1024;
    let status = loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Native controller probe cancelled"
        );
        ensure!(
            Instant::now() < deadline,
            "Native controller probe timed out"
        );
        ensure!(
            std::fs::metadata(&stdout_path)?.len() <= MAX_OUTPUT
                && std::fs::metadata(&stderr_path)?.len() <= MAX_OUTPUT,
            "Native controller probe output exceeded its limit"
        );
        if let Some(status) = child
            .0
            .as_mut()
            .expect("probe child is retained")
            .try_wait()?
        {
            child.0.take();
            break status;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let read_output = |path: &Path| -> Result<String> {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(MAX_OUTPUT + 1)
            .read_to_string(&mut text)?;
        ensure!(
            text.len() as u64 <= MAX_OUTPUT,
            "Native controller probe output exceeded its limit"
        );
        Ok(text)
    };
    ensure!(
        status.success(),
        "Native SDL2 probe failed: {}",
        read_output(&stderr_path)?
    );
    let inventory: lunchbox_controller_probe::sdl2::Snapshot =
        serde_json::from_str(&read_output(&stdout_path)?)
            .context("Decoding native SDL2 inventory")?;
    ensure!(
        inventory.library == std::fs::canonicalize(sdl_library)?,
        "Native probe loaded a different SDL library"
    );
    for path in control_paths {
        let device = inventory.device_at_path(path)?;
        ensure!(
            device.controls.is_some(),
            "Native probe omitted requested controller counts"
        );
    }
    Ok(inventory)
}

/// Retain this object until the emulator exits. BizHawk may save its config on
/// shutdown; that write belongs in this private directory, not the source file.
pub(crate) struct PreparedConfig {
    directory: tempfile::TempDir,
    requested_source: PathBuf,
    source_path: PathBuf,
    source_text: String,
    runtime_artifacts: Vec<RuntimeArtifact>,
}

/// Selected top-level runtime files only, not a transitive loader manifest.
/// Preserve both the requested path and resolved target so retargeting a
/// selected symlink cannot silently change the prepared invocation.
#[derive(Clone)]
struct RuntimeArtifact {
    requested: PathBuf,
    resolved: PathBuf,
    sha256: [u8; 32],
}

impl RuntimeArtifact {
    fn capture(path: &Path) -> Result<Self> {
        use sha2::{Digest, Sha256};
        const MAX_BYTES: u64 = 512 * 1024 * 1024;
        ensure!(
            path.is_absolute(),
            "Native runtime artifact must use an absolute path"
        );
        let resolved = path.canonicalize()?;
        let mut file = std::fs::File::open(&resolved)?;
        let before = file.metadata()?;
        ensure!(
            before.is_file() && before.len() <= MAX_BYTES,
            "Native runtime artifact is not a bounded regular file"
        );
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut count = 0u64;
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            count += read as u64;
            ensure!(
                count <= MAX_BYTES,
                "Native runtime artifact grew beyond its bound"
            );
            digest.update(&buffer[..read]);
        }
        let after = file.metadata()?;
        ensure!(
            count == before.len()
                && after.len() == before.len()
                && after.modified()? == before.modified()?
                && path.canonicalize()? == resolved,
            "Native runtime artifact changed while reading"
        );
        Ok(Self {
            requested: path.to_owned(),
            resolved,
            sha256: digest.finalize().into(),
        })
    }

    fn verify(&self) -> Result<()> {
        let current = Self::capture(&self.requested)?;
        ensure!(
            current.resolved == self.resolved && current.sha256 == self.sha256,
            "Native runtime artifact changed after controller preparation: {}",
            self.requested.display()
        );
        Ok(())
    }
}

impl PreparedConfig {
    pub(crate) fn path(&self) -> PathBuf {
        self.directory.path().join("config.ini")
    }

    pub(crate) fn verify_source(&self) -> Result<()> {
        for artifact in &self.runtime_artifacts {
            artifact.verify()?;
        }
        ensure!(
            std::fs::canonicalize(&self.requested_source)? == self.source_path,
            "BizHawk source config symlink changed during controller preparation"
        );
        ensure!(
            read_config(&self.source_path)? == self.source_text,
            "BizHawk source configuration changed during controller preparation"
        );
        Ok(())
    }

    /// Native EmuHawk arguments, placed before any end-of-options marker.
    /// Caller must remove an existing --config selection only after resolving it
    /// as source_path, and must retain all unrelated arguments.
    pub(crate) fn config_arguments(&self) -> [std::ffi::OsString; 2] {
        ["--config".into(), self.path().into_os_string()]
    }
}

fn read_config(path: &Path) -> Result<String> {
    const MAX_CONFIG_BYTES: u64 = 16 * 1024 * 1024;
    let file = std::fs::File::open(path)
        .with_context(|| format!("Opening BizHawk config {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "BizHawk config must be a regular file"
    );
    let mut text = String::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_string(&mut text)
        .context("Reading UTF-8 BizHawk configuration")?;
    ensure!(
        text.len() as u64 <= MAX_CONFIG_BYTES,
        "BizHawk config exceeds 16 MiB"
    );
    Ok(text)
}

pub(crate) fn prepare_config(
    source: &Path,
    bindings: &NymashockBindings,
) -> Result<PreparedConfig> {
    prepare_config_using(source, |text| encode_nymashock_config(text, bindings))
}

fn prepare_config_using(
    source: &Path,
    encode: impl FnOnce(&str) -> Result<String>,
) -> Result<PreparedConfig> {
    // Resolve once: changing cwd later must not change which source is checked.
    let requested_source = if source.is_absolute() {
        source.to_owned()
    } else {
        std::env::current_dir()?.join(source)
    };
    let source_path =
        std::fs::canonicalize(&requested_source).context("Resolving BizHawk source config")?;
    let source_text = read_config(&source_path)?;
    let encoded = encode(&source_text)?;
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-bizhawk-")
        .tempdir()
        .context("Creating private BizHawk configuration directory")?;
    let prepared = PreparedConfig {
        directory,
        requested_source,
        source_path,
        source_text,
        runtime_artifacts: Vec::new(),
    };
    std::fs::write(prepared.path(), encoded).context("Writing private BizHawk configuration")?;
    prepared.verify_source()?;
    Ok(prepared)
}

/// Prepare native EmuHawk arguments transactionally. `exe_directory` is the
/// actual EmuHawk directory (not a wrapper or Mono executable's directory).
/// Program.cs selects ExeDirectoryPath/config.ini when --config is absent.
pub(crate) fn prepare_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &Path,
    exe_directory: &Path,
    bindings: &NymashockBindings,
) -> Result<PreparedConfig> {
    prepare_arguments_using(arguments, working_directory, exe_directory, |source| {
        prepare_config(source, bindings)
    })
}

fn prepare_arguments_using(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &Path,
    exe_directory: &Path,
    prepare: impl FnOnce(&Path) -> Result<PreparedConfig>,
) -> Result<PreparedConfig> {
    ensure!(
        working_directory.is_absolute() && exe_directory.is_absolute(),
        "BizHawk launch directories must be resolved before input preparation"
    );
    let mut rewritten = Vec::with_capacity(arguments.len() + 2);
    let mut selected: Option<PathBuf> = None;
    let mut index = 0;
    let mut insertion = None;
    while index < arguments.len() {
        let argument = &arguments[index];
        if argument == "--" {
            insertion = Some(rewritten.len());
            rewritten.extend_from_slice(&arguments[index..]);
            break;
        }
        let text = argument
            .to_str()
            .context("BizHawk arguments must be valid UTF-8")?;
        ensure!(
            !text.starts_with('@'),
            "Resolve BizHawk response-file arguments before controller preparation"
        );
        let config = if text == "--config" {
            index += 1;
            let value = arguments
                .get(index)
                .context("BizHawk --config is missing its path")?;
            let value = value
                .to_str()
                .context("BizHawk config path must be UTF-8")?;
            ensure!(
                !value.is_empty() && !value.starts_with('-'),
                "Ambiguous BizHawk --config path"
            );
            Some(PathBuf::from(value))
        } else if let Some(value) = text
            .strip_prefix("--config=")
            .or_else(|| text.strip_prefix("--config:"))
        {
            ensure!(!value.is_empty(), "BizHawk --config is missing its path");
            Some(PathBuf::from(value))
        } else {
            None
        };
        if let Some(path) = config {
            ensure!(selected.is_none(), "Duplicate BizHawk --config arguments");
            selected = Some(if path.is_absolute() {
                path
            } else {
                working_directory.join(path)
            });
        } else {
            rewritten.push(argument.clone());
        }
        index += 1;
    }
    let source = selected.unwrap_or_else(|| exe_directory.join("config.ini"));
    let prepared = prepare(&source)?;
    let insertion = insertion.unwrap_or(rewritten.len());
    rewritten.splice(insertion..insertion, prepared.config_arguments());
    // Preserve the original argument vector on every preparation failure.
    *arguments = rewritten;
    Ok(prepared)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AnalogBinding {
    pub value: String,
    pub mult: f32,
    pub deadzone: f32,
    pub button_bind_positive: String,
    pub button_bind_negative: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct FeedbackBinding {
    pub channels: String,
    pub gamepad_prefix: String,
    pub prescale: f32,
}

/// Translate shared PlayStation wire identities into SCPH-1110's native names.
/// Select/Start retain their spelling; stick-click and toggle inputs are absent.
fn analog_joystick_name(name: &str) -> &str {
    match name {
        "D-Pad Up" => "Thumbstick Up",
        "D-Pad Down" => "Thumbstick Down",
        "D-Pad Left" => "Thumbstick Left",
        "D-Pad Right" => "Thumbstick Right",
        "L2" => "Left Stick, Trigger",
        "R2" => "Left Stick, Pinky",
        "L1" => "Left Stick, L-Thumb",
        "R1" => "Left Stick, R-Thumb",
        "△" => "Right Stick, Pinky",
        "○" => "Right Stick, R-Thumb",
        "X" => "Right Stick, L-Thumb",
        "□" => "Right Stick, Trigger",
        "Left Stick Left / Right" => "Left Stick, Left / Right",
        "Left Stick Up / Down" => "Left Stick, Up / Down",
        "Right Stick Left / Right" => "Right Stick, Left / Right",
        "Right Stick Up / Down" => "Right Stick, Up / Down",
        other => other,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DigitalPeripheral {
    DancePad,
    PopnMusic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerPeripheral {
    Mouse,
    GunCon,
    Justifier,
}

impl PointerPeripheral {
    pub(crate) fn layout_id(self) -> &'static str {
        match self {
            Self::Mouse => "playstation-mouse-controller",
            Self::GunCon => "playstation-guncon-controller",
            Self::Justifier => "playstation-justifier-controller",
        }
    }

    fn control_names(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Mouse => &[("b", "Left Button"), ("a", "Right Button")],
            Self::GunCon => &[
                ("b", "Trigger"),
                ("a", "A"),
                ("y", "B"),
                ("select", "Offscreen Shot"),
            ],
            Self::Justifier => &[
                ("b", "Trigger"),
                ("a", "O"),
                ("start", "Start"),
                ("select", "Offscreen Shot"),
            ],
        }
    }

    fn axis_names(self) -> [&'static str; 2] {
        match self {
            Self::Mouse => ["Motion Left / Right", "Motion Up / Down"],
            Self::GunCon | Self::Justifier => ["X Axis", "Y Axis"],
        }
    }
}

impl DigitalPeripheral {
    pub(crate) fn layout_id(self) -> &'static str {
        match self {
            Self::DancePad => "playstation-dance-pad",
            Self::PopnMusic => "playstation-popn-music",
        }
    }

    fn control_names(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::DancePad => &[
                ("select", "Select"),
                ("start", "Start"),
                ("up", "Up"),
                ("down", "Down"),
                ("left", "Left"),
                ("right", "Right"),
                ("x", "△"),
                ("a", "○"),
                ("b", "X"),
                ("y", "□"),
            ],
            Self::PopnMusic => &[
                ("select", "Select"),
                ("start", "Start"),
                ("x", "Left White"),
                ("a", "Left Yellow"),
                ("r", "Left Green"),
                ("b", "Left Blue"),
                ("l", "Red"),
                ("y", "Right Blue"),
                ("r2", "Right Green"),
                ("up", "Right Yellow"),
                ("l2", "Right White"),
            ],
        }
    }

    fn device_id(self) -> &'static str {
        match self {
            Self::DancePad => "dancepad",
            Self::PopnMusic => "popnmusic",
        }
    }

    /// Exact Nyma names from Mednafen gamepad.cpp, after button-name overrides.
    /// Dance-pad directions are independent buttons, not an exclusive hat.
    fn buttons(self) -> &'static [&'static str] {
        match self {
            Self::DancePad => &[
                "Select", "Start", "Up", "Down", "Left", "Right", "△", "○", "X", "□",
            ],
            Self::PopnMusic => &[
                "Select",
                "Start",
                "Left White",
                "Left Yellow",
                "Left Green",
                "Left Blue",
                "Red",
                "Right Blue",
                "Right Green",
                "Right Yellow",
                "Right White",
            ],
        }
    }
}

pub(crate) enum PadMode {
    CalibratedPointer {
        kind: PointerPeripheral,
        digital: BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
        analog: BTreeMap<String, AnalogBinding>,
    },
    RawPointer {
        kind: PointerPeripheral,
        desktop_cursor: bool,
        digital: BTreeMap<String, RawDigitalInput>,
        analog: BTreeMap<String, RawAnalogInput>,
    },
    RawNegcon {
        digital: BTreeMap<String, RawDigitalInput>,
        analog: BTreeMap<String, RawAnalogInput>,
    },
    CalibratedNegcon {
        digital: BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
        analog: BTreeMap<String, AnalogBinding>,
    },
    Digital,
    CalibratedDigital(BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>),
    CalibratedPeripheral {
        kind: DigitalPeripheral,
        digital: BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
    },
    RawPeripheral {
        kind: DigitalPeripheral,
        digital: BTreeMap<String, RawDigitalInput>,
    },
    CalibratedDualShock {
        digital: BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
        analog: BTreeMap<String, AnalogBinding>,
        rumble: bool,
        dualanalog: bool,
        analog_joystick: bool,
    },
    DualShock {
        analog_toggle: String,
        deadzone: f32,
        rumble: bool,
    },
    DualAnalog {
        deadzone: f32,
    },
    AnalogJoystick {
        deadzone: f32,
    },
    Raw {
        digital: BTreeMap<String, RawDigitalInput>,
        analog: BTreeMap<String, RawAnalogInput>,
        dualshock: bool,
        dualanalog: bool,
        analog_joystick: bool,
        rumble: bool,
    },
}

#[cfg(target_os = "linux")]
fn logical_pad_from_calibration(
    request: &NativePadRequest<'_>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(PadMode, Vec<String>)> {
    ensure!(
        [
            request.dualshock,
            request.dualanalog,
            request.analog_joystick,
            request.rhythm.is_some(),
            request.negcon,
            request.pointer.is_some()
        ]
        .into_iter()
        .filter(|mode| *mode)
        .count()
            <= 1
            && request.dualshock == request.analog_toggle_id.is_some()
            && (request.dualshock || !request.rumble),
        "Only DualShock uses an explicit analog toggle and rumble"
    );
    request.calibration.validate()?;
    let logical = request
        .logical
        .context("Capture logical SDL2 bindings for this recognized controller first")?;
    logical.validate()?;
    logical.context.ensure_current(inventory, path)?;
    ensure!(
        logical.layout == request.calibration.layout,
        "Physical and SDL2 calibration layouts disagree"
    );
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&logical.layout)
        .context("Unknown physical layout")?;
    let target = catalog
        .layout(mode_layout_id(
            request.dualshock,
            request.dualanalog,
            request.analog_joystick,
            request.rhythm,
            request.negcon,
            request.pointer,
        ))
        .context("Missing PlayStation layout")?;
    let available =
        calibrated_control_ids(request.calibration, Some(logical), request.analog_toggle_id);
    let requested = requested_control_ids(target, request.desktop_cursor);
    let resolution = guided::resolve(request.calibration, source, target, &available, &requested)?;
    if let Some(kind) = request.pointer {
        ensure!(
            !request.desktop_cursor || kind != PointerPeripheral::Mouse,
            "Absolute cursor cannot supply relative mouse motion"
        );
        let device = inventory.device_at_path(path)?;
        let assigned = |id: &str| -> Result<&String> {
            resolution
                .assignments
                .get(id)
                .with_context(|| format!("No pointer assignment for {id}"))
        };
        let gesture = |id: &str| -> Result<&lunchbox_controller_probe::sdl2_mapping::OutputChange> {
            logical
                .bindings
                .get(assigned(id)?)
                .context("Resolved pointer gesture is absent")
        };
        let mut digital = BTreeMap::new();
        for (id, name) in kind.control_names() {
            let input = gesture(id)?;
            logical_digital_source(device, input)?;
            digital.insert((*name).into(), input.clone());
        }
        let mut analog = BTreeMap::new();
        let mut physical_axes = std::collections::BTreeSet::new();
        let mut logical_axes = std::collections::BTreeSet::new();
        for ((negative, positive), name) in
            [("stick_left", "stick_right"), ("stick_up", "stick_down")]
                .into_iter()
                .zip(kind.axis_names())
        {
            if request.desktop_cursor {
                continue;
            }
            let physical_code = |id: &str| -> Result<u32> {
                Ok(request
                    .calibration
                    .bindings
                    .get(assigned(id)?)
                    .and_then(|binding| binding.native.as_ref())
                    .context("Pointer axes require physical identity")?
                    .code)
            };
            let code = physical_code(negative)?;
            ensure!(
                code == physical_code(positive)? && physical_axes.insert(code),
                "Pointer coordinates require two distinct physical axes with paired directions"
            );
            ensure!(
                logical_axes.insert(gesture(negative)?.output.clone()),
                "Pointer coordinates share a logical axis"
            );
            analog.insert(
                name.into(),
                logical_analog_source(
                    device,
                    &logical.context,
                    gesture(negative)?,
                    gesture(positive)?,
                    request.deadzone,
                )?,
            );
        }
        if request.desktop_cursor {
            analog = desktop_cursor_axes();
        }
        if kind == PointerPeripheral::Mouse {
            ensure!(
                request.mouse_speed.is_finite()
                    && request.mouse_speed > 0.0
                    && request.mouse_speed <= 4.0,
                "Invalid mouse speed"
            );
            for axis in analog.values_mut() {
                axis.mult *= request.mouse_speed;
            }
        }
        let warnings = resolution
            .rules
            .iter()
            .filter(|(_, rule)| !matches!(rule, crate::controller_layout::Rule::Identity))
            .map(|(id, rule)| format!("{id}: {}", rule.description()))
            .collect();
        return Ok((
            PadMode::CalibratedPointer {
                kind,
                digital,
                analog,
            },
            warnings,
        ));
    }
    if request.negcon {
        let device = inventory.device_at_path(path)?;
        let assigned = |id: &str| -> Result<&String> {
            resolution
                .assignments
                .get(id)
                .with_context(|| format!("No neGcon assignment for {id}"))
        };
        let gesture = |id: &str| -> Result<&lunchbox_controller_probe::sdl2_mapping::OutputChange> {
            logical
                .bindings
                .get(assigned(id)?)
                .context("Resolved neGcon logical gesture is absent")
        };
        let mut digital = BTreeMap::new();
        for (id, name) in [
            ("start", "Start"),
            ("up", "D-Pad Up"),
            ("down", "D-Pad Down"),
            ("left", "D-Pad Left"),
            ("right", "D-Pad Right"),
            ("r", "R"),
            ("a", "A"),
            ("b", "B"),
        ] {
            let input = gesture(id)?;
            logical_digital_source(device, input)?;
            digital.insert(name.into(), input.clone());
        }
        let mut analog = BTreeMap::new();
        analog.insert(
            "Twist Ccwise / Cwise".into(),
            logical_analog_source(
                device,
                &logical.context,
                gesture("stick_left")?,
                gesture("stick_right")?,
                request.deadzone,
            )?,
        );
        let mut axis_outputs = std::collections::BTreeSet::new();
        axis_outputs.insert(gesture("stick_left")?.output.clone());
        let physical_code = |id: &str| -> Result<u32> {
            Ok(request
                .calibration
                .bindings
                .get(assigned(id)?)
                .and_then(|binding| binding.native.as_ref())
                .context("neGcon axes require physical identity")?
                .code)
        };
        let twist_code = physical_code("stick_left")?;
        ensure!(
            twist_code == physical_code("stick_right")?,
            "Twist directions must share one physical axis"
        );
        let mut physical_axes = std::collections::BTreeSet::from([twist_code]);
        for (id, name) in [
            ("pressure_i", "I"),
            ("pressure_ii", "II"),
            ("pressure_l", "L"),
        ] {
            ensure!(
                physical_axes.insert(physical_code(id)?),
                "neGcon pressure and twist share a physical axis"
            );
            let input = gesture(id)?;
            ensure!(
                axis_outputs.insert(input.output.clone()),
                "neGcon pressure and twist controls require independent axes"
            );
            let measurement = request
                .calibration
                .bindings
                .get(assigned(id)?)
                .and_then(|binding| binding.axis.as_ref())
                .context("neGcon pressure requires physical measurement")?;
            analog.insert(
                name.into(),
                logical_pressure_source(
                    device,
                    &logical.context,
                    input,
                    measurement,
                    request.deadzone,
                )?,
            );
        }
        let warnings = resolution
            .rules
            .iter()
            .filter(|(_, rule)| !matches!(rule, crate::controller_layout::Rule::Identity))
            .map(|(id, rule)| format!("{id}: {}", rule.description()))
            .collect();
        return Ok((PadMode::CalibratedNegcon { digital, analog }, warnings));
    }
    if let Some(kind) = request.rhythm {
        let device = inventory.device_at_path(path)?;
        let mut digital = BTreeMap::new();
        let mut warnings = Vec::new();
        for (id, name) in kind.control_names() {
            let assigned = resolution
                .assignments
                .get(*id)
                .with_context(|| format!("No calibrated rhythm assignment for {name}"))?;
            let gesture = logical
                .bindings
                .get(assigned)
                .context("Resolved rhythm gesture is absent")?;
            logical_digital_source(device, gesture)?;
            digital.insert((*name).into(), gesture.clone());
            if let Some(rule) = resolution.rules.get(*id) {
                if !matches!(rule, crate::controller_layout::Rule::Identity) {
                    warnings.push(format!("{name} uses {assigned}: {}", rule.description()));
                }
            }
        }
        ensure_independent_rhythm_inputs(device, &digital)?;
        return Ok((PadMode::CalibratedPeripheral { kind, digital }, warnings));
    }
    let mut digital = BTreeMap::new();
    let mut warnings = Vec::new();
    for (id, name) in [
        ("b", "X"),
        ("a", "○"),
        ("y", "□"),
        ("x", "△"),
        ("select", "Select"),
        ("start", "Start"),
        ("l", "L1"),
        ("r", "R1"),
        ("l2", "L2"),
        ("r2", "R2"),
        ("up", "Up"),
        ("down", "Down"),
        ("left", "Left"),
        ("right", "Right"),
    ] {
        let source_id = resolution
            .assignments
            .get(id)
            .with_context(|| format!("No calibrated SDL2 assignment for PlayStation {id}"))?;
        let gesture = logical
            .bindings
            .get(source_id)
            .context("Resolved SDL2 gesture is missing")?;
        logical_digital_source(inventory.device_at_path(path)?, gesture)?;
        let name = if (request.dualshock || request.dualanalog || request.analog_joystick)
            && matches!(id, "up" | "down" | "left" | "right")
        {
            format!("D-Pad {name}")
        } else {
            name.to_owned()
        };
        let name = if request.analog_joystick {
            analog_joystick_name(&name).to_owned()
        } else {
            name
        };
        digital.insert(name.clone(), gesture.clone());
        if let Some(rule) = resolution.rules.get(id)
            && !matches!(rule, crate::controller_layout::Rule::Identity)
        {
            warnings.push(format!("{name} uses {source_id}: {}", rule.description()));
        }
    }
    if !request.dualshock && !request.dualanalog && !request.analog_joystick {
        return Ok((PadMode::CalibratedDigital(digital), warnings));
    }
    let device = inventory.device_at_path(path)?;
    let resolve = |id: &str| -> Result<&lunchbox_controller_probe::sdl2_mapping::OutputChange> {
        let source = resolution
            .assignments
            .get(id)
            .with_context(|| format!("No logical DualShock assignment for {id}"))?;
        logical
            .bindings
            .get(source)
            .context("Resolved logical DualShock control is absent")
    };
    for (id, name) in [("l3", "Left Stick, Button"), ("r3", "Right Stick, Button")] {
        if request.analog_joystick {
            continue;
        }
        let gesture = resolve(id)?;
        logical_digital_source(device, gesture)?;
        digital.insert(name.into(), gesture.clone());
    }
    if request.dualshock {
        let toggle_id = request
            .analog_toggle_id
            .context("Assign an analog toggle")?;
        ensure!(
            request.calibration.bindings.contains_key(toggle_id),
            "Analog toggle has no physical calibration"
        );
        let toggle = logical
            .bindings
            .get(toggle_id)
            .context("Capture the logical analog toggle first")?;
        logical_digital_source(device, toggle)?;
        digital.insert("Analog".into(), toggle.clone());
    }
    let mut analog = BTreeMap::new();
    for (negative, positive, name) in [
        ("stick_left", "stick_right", "Left Stick Left / Right"),
        ("stick_up", "stick_down", "Left Stick Up / Down"),
        (
            "right_stick_left",
            "right_stick_right",
            "Right Stick Left / Right",
        ),
        (
            "right_stick_up",
            "right_stick_down",
            "Right Stick Up / Down",
        ),
    ] {
        analog.insert(
            if request.analog_joystick {
                analog_joystick_name(name)
            } else {
                name
            }
            .into(),
            logical_analog_source(
                device,
                &logical.context,
                resolve(negative)?,
                resolve(positive)?,
                request.deadzone,
            )?,
        );
    }
    Ok((
        PadMode::CalibratedDualShock {
            digital,
            analog,
            rumble: request.rumble,
            dualanalog: request.dualanalog,
            analog_joystick: request.analog_joystick,
        },
        warnings,
    ))
}

/// Derive bindings only for an owned normalized virtual device. Its neutral
/// and gesture endpoints are defined by the frame publisher, not inferred from
/// the current position of an arbitrary physical controller.
#[cfg(target_os = "linux")]
fn normalized_logical_calibration(
    calibration: &crate::controller_catalog::Calibration,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<LogicalCalibration> {
    use lunchbox_controller_probe::linux_classic::{AxisEndpoints, Control};
    use lunchbox_controller_probe::sdl2_mapping::{
        InputState, MappingContext, changed_outputs, parse,
    };
    let device = inventory.device_at_path(path)?;
    let physical = PhysicalMap::from_device(device)?;
    let context = MappingContext::capture(inventory, path)?;
    let mapping = parse(&context.mapping)?;
    let layout = crate::controller_catalog::catalog()
        .layout(&calibration.layout)
        .context("Unknown virtual controller layout")?;
    let mut neutral = InputState::default();
    neutral
        .buttons
        .extend((0..context.counts.buttons).map(|index| (index, false)));
    neutral
        .hats
        .extend((0..context.counts.hats).map(|index| (index, 0)));
    for input in calibration.bindings.values() {
        let native = input
            .native
            .as_ref()
            .context("Virtual binding has no input identity")?;
        if let Control::Axis(index) = physical.control(native.code)? {
            let measured = input
                .axis
                .as_ref()
                .context("Virtual axis has no expected measurement")?;
            let value = physical.axis_value((native.code & 0xffff) as u8, measured.released)?;
            if let Some(previous) = neutral.axes.insert(index, value) {
                ensure!(
                    previous == value,
                    "Virtual bindings disagree about axis neutral"
                );
            }
        }
    }
    ensure!(
        neutral.axes.len() == context.counts.axes as usize,
        "Virtual neutral contract is missing an SDL2 axis"
    );
    let mut bindings = BTreeMap::new();
    for (id, input) in &calibration.bindings {
        let native = input
            .native
            .as_ref()
            .context("Virtual binding has no input identity")?;
        let mut pressed = neutral.clone();
        match physical.control(native.code)? {
            Control::Button(index) => {
                pressed.buttons.insert(index, true);
            }
            Control::Axis(index) => {
                let measured = input
                    .axis
                    .as_ref()
                    .context("Virtual axis measurement is absent")?;
                pressed.axes.insert(
                    index,
                    physical.axis_value((native.code & 0xffff) as u8, measured.pressed)?,
                );
            }
            Control::HatAxis { .. } => {
                let measured = input
                    .axis
                    .as_ref()
                    .context("Virtual hat measurement is absent")?;
                let translated = physical.digital_input(
                    native.code,
                    Some(AxisEndpoints {
                        released: measured.released,
                        pressed: measured.pressed,
                    }),
                )?;
                let lunchbox_controller_probe::duckstation::DigitalInput::Hat { index, direction } =
                    translated
                else {
                    anyhow::bail!("Virtual hat was not translated as a hat");
                };
                pressed.hats.insert(index, direction);
            }
        }
        let control = layout
            .controls
            .iter()
            .find(|control| control.id == *id)
            .context("Unknown virtual semantic control")?;
        let candidates: Vec<_> = changed_outputs(&mapping, &context.counts, &neutral, &pressed)?
            .into_iter()
            .filter(|gesture| {
                if control.analog && control.group == "pressure" {
                    gesture.analog
                        && matches!(
                            gesture.output.as_str(),
                            "leftx"
                                | "lefty"
                                | "rightx"
                                | "righty"
                                | "lefttrigger"
                                | "righttrigger"
                        )
                } else if control.analog {
                    gesture.analog
                        && matches!(
                            gesture.output.as_str(),
                            "leftx" | "lefty" | "rightx" | "righty"
                        )
                } else {
                    logical_digital_source(device, gesture).is_ok()
                }
            })
            .collect();
        ensure!(
            candidates.len() == 1,
            "Virtual SDL2 mapping must expose exactly one compatible output for {id}; found {}",
            candidates.len()
        );
        bindings.insert(
            id.clone(),
            candidates.into_iter().next().context("No virtual output")?,
        );
    }
    let logical = LogicalCalibration {
        layout: calibration.layout.clone(),
        context,
        bindings,
    };
    logical.validate()?;
    Ok(logical)
}

pub(crate) struct RawAnalogInput {
    pub axis_index: u32,
    pub multiplier: f32,
    pub deadzone: f32,
}

fn raw_digital_from_calibrated_input(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    input: &crate::controller_catalog::InputBinding,
) -> Result<RawDigitalInput> {
    let native = input
        .native
        .as_ref()
        .context("Recalibrate to capture physical controller inputs")?;
    let measured = if let Some(axis) = &input.axis {
        axis.validate()?;
        ensure!(
            native.direction == axis.direction(),
            "Saved axis direction disagrees with its measurement"
        );
        Some(lunchbox_controller_probe::linux_classic::AxisEndpoints {
            released: axis.released,
            pressed: axis.pressed,
        })
    } else {
        None
    };
    raw_digital_from_physical(device, physical, native.code, measured)
}

fn ensure_independent_raw_rhythm_inputs(digital: &BTreeMap<String, RawDigitalInput>) -> Result<()> {
    let mut channels = std::collections::BTreeSet::new();
    let mut hats = BTreeMap::<u32, u8>::new();
    for input in digital.values() {
        match *input {
            RawDigitalInput::Button(index) => ensure!(
                channels.insert((0u8, index)),
                "Rhythm buttons share physical button {index}"
            ),
            RawDigitalInput::AxisDirection { index, .. } => ensure!(
                channels.insert((1u8, index)),
                "Independent rhythm buttons cannot share axis {index}"
            ),
            RawDigitalInput::HatDirection { index, mask } => {
                let previous = hats.entry(index).or_default();
                let combined = *previous | mask;
                ensure!(
                    *previous & mask == 0 && combined & 5 != 5 && combined & 10 != 10,
                    "Rhythm buttons require conflicting positions of hat {index}"
                );
                *previous = combined;
            }
        }
    }
    Ok(())
}

fn desktop_cursor_axes() -> BTreeMap<String, AnalogBinding> {
    [("X Axis", "WMouse X"), ("Y Axis", "WMouse Y")]
        .into_iter()
        .map(|(target, source)| {
            (
                target.into(),
                AnalogBinding {
                    value: source.into(),
                    mult: 1.0,
                    deadzone: 0.0,
                    button_bind_positive: String::new(),
                    button_bind_negative: String::new(),
                },
            )
        })
        .collect()
}

fn raw_pointer_from_calibration(
    calibration: &crate::controller_catalog::Calibration,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    kind: PointerPeripheral,
    desktop_cursor: bool,
    deadzone: f32,
    mouse_speed: f32,
) -> Result<(PadMode, Vec<String>)> {
    ensure!(
        !desktop_cursor || kind != PointerPeripheral::Mouse,
        "Absolute cursor cannot supply relative mouse motion"
    );
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && std::env::consts::OS == "linux",
        "Native pointer translation requires Linux calibration"
    );
    let device = inventory.device_at_path(runtime_path)?;
    let physical = PhysicalMap::from_device(device)?;
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&calibration.layout)
        .context("Unknown pointer source layout")?;
    let target = catalog
        .layout(kind.layout_id())
        .context("Missing pointer target layout")?;
    let available = calibrated_control_ids(calibration, None, None);
    let requested = requested_control_ids(target, desktop_cursor);
    let resolution = guided::resolve(calibration, source, target, &available, &requested)?;
    let input = |id: &str| -> Result<&crate::controller_catalog::InputBinding> {
        let assigned = resolution
            .assignments
            .get(id)
            .with_context(|| format!("No pointer assignment for {id}"))?;
        calibration
            .bindings
            .get(assigned)
            .context("Resolved pointer input is absent")
    };
    let measured = |id: &str| -> Result<(u32, &crate::controller_axis::AxisMeasurement)> {
        let binding = input(id)?;
        let native = binding
            .native
            .as_ref()
            .context("Pointer axis identity is absent")?;
        let axis = binding
            .axis
            .as_ref()
            .context("Pointer axis measurement is absent")?;
        axis.validate()?;
        ensure!(
            native.direction == axis.direction(),
            "Pointer direction disagrees with its measurement"
        );
        Ok((native.code, axis))
    };
    let mut digital = BTreeMap::new();
    for (id, name) in kind.control_names() {
        digital.insert(
            (*name).into(),
            raw_digital_from_calibrated_input(device, physical, input(id)?)?,
        );
    }
    let mut analog = BTreeMap::new();
    let mut channels = std::collections::BTreeSet::new();
    for ((negative, positive), name) in [("stick_left", "stick_right"), ("stick_up", "stick_down")]
        .into_iter()
        .zip(kind.axis_names())
    {
        if desktop_cursor {
            continue;
        }
        let (code, low) = measured(negative)?;
        let (other, high) = measured(positive)?;
        ensure!(
            code == other && channels.insert(code),
            "Pointer coordinates require distinct physical axes with paired directions"
        );
        analog.insert(
            name.into(),
            raw_analog_from_physical(device, physical, code, low, high, deadzone)?,
        );
    }
    if kind == PointerPeripheral::Mouse {
        ensure!(
            mouse_speed.is_finite() && mouse_speed > 0.0 && mouse_speed <= 4.0,
            "Invalid mouse speed"
        );
        for axis in analog.values_mut() {
            axis.multiplier *= mouse_speed;
        }
    }
    let warnings = resolution
        .rules
        .iter()
        .filter(|(_, rule)| !matches!(rule, crate::controller_layout::Rule::Identity))
        .map(|(id, rule)| format!("{id}: {}", rule.description()))
        .collect();
    Ok((
        PadMode::RawPointer {
            kind,
            desktop_cursor,
            digital,
            analog,
        },
        warnings,
    ))
}

fn raw_negcon_from_calibration(
    calibration: &crate::controller_catalog::Calibration,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    deadzone: f32,
) -> Result<(PadMode, Vec<String>)> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && std::env::consts::OS == "linux",
        "Native neGcon translation requires Linux calibration"
    );
    let device = inventory.device_at_path(runtime_path)?;
    let physical = PhysicalMap::from_device(device)?;
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&calibration.layout)
        .context("Unknown neGcon source layout")?;
    let target = catalog
        .layout("playstation-negcon")
        .context("Missing neGcon layout")?;
    let available = calibrated_control_ids(calibration, None, None);
    let requested = requested_control_ids(target, false);
    let resolution = guided::resolve(calibration, source, target, &available, &requested)?;
    let input = |id: &str| -> Result<&crate::controller_catalog::InputBinding> {
        let assigned = resolution
            .assignments
            .get(id)
            .with_context(|| format!("No neGcon assignment for {id}"))?;
        calibration
            .bindings
            .get(assigned)
            .context("Resolved neGcon input is absent")
    };
    let measured = |id: &str| -> Result<(u32, &crate::controller_axis::AxisMeasurement)> {
        let binding = input(id)?;
        let native = binding
            .native
            .as_ref()
            .context("neGcon axis identity is absent")?;
        let axis = binding
            .axis
            .as_ref()
            .context("neGcon axis measurement is absent")?;
        axis.validate()?;
        ensure!(
            native.direction == axis.direction(),
            "neGcon axis direction disagrees with measurement"
        );
        Ok((native.code, axis))
    };
    let mut digital = BTreeMap::new();
    for (id, name) in [
        ("start", "Start"),
        ("up", "D-Pad Up"),
        ("down", "D-Pad Down"),
        ("left", "D-Pad Left"),
        ("right", "D-Pad Right"),
        ("r", "R"),
        ("a", "A"),
        ("b", "B"),
    ] {
        digital.insert(
            name.into(),
            raw_digital_from_calibrated_input(device, physical, input(id)?)?,
        );
    }
    let (twist_code, negative) = measured("stick_left")?;
    let (positive_code, positive) = measured("stick_right")?;
    ensure!(
        twist_code == positive_code,
        "Twist directions must use one physical axis"
    );
    let mut analog = BTreeMap::new();
    analog.insert(
        "Twist Ccwise / Cwise".into(),
        raw_analog_from_physical(device, physical, twist_code, negative, positive, deadzone)?,
    );
    let mut channels = std::collections::BTreeSet::from([twist_code]);
    for (id, name) in [
        ("pressure_i", "I"),
        ("pressure_ii", "II"),
        ("pressure_l", "L"),
    ] {
        let (code, measurement) = measured(id)?;
        ensure!(
            channels.insert(code),
            "neGcon pressure and twist require independent physical axes"
        );
        analog.insert(
            name.into(),
            raw_pressure_from_physical(device, physical, code, measurement, deadzone)?,
        );
    }
    let warnings = resolution
        .rules
        .iter()
        .filter(|(_, rule)| !matches!(rule, crate::controller_layout::Rule::Identity))
        .map(|(id, rule)| format!("{id}: {}", rule.description()))
        .collect();
    Ok((PadMode::RawNegcon { digital, analog }, warnings))
}

fn raw_peripheral_from_calibration(
    calibration: &crate::controller_catalog::Calibration,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    kind: DigitalPeripheral,
) -> Result<(PadMode, Vec<String>)> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && std::env::consts::OS == "linux",
        "Native rhythm translation requires Linux calibration"
    );
    let device = inventory.device_at_path(runtime_path)?;
    let physical = PhysicalMap::from_device(device)?;
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&calibration.layout)
        .context("Unknown rhythm source layout")?;
    let target = catalog
        .layout(kind.layout_id())
        .context("Missing rhythm target layout")?;
    let available = calibrated_control_ids(calibration, None, None);
    let requested = requested_control_ids(target, false);
    let resolution = guided::resolve(calibration, source, target, &available, &requested)?;
    let mut digital = BTreeMap::new();
    let mut warnings = Vec::new();
    for (id, name) in kind.control_names() {
        let assigned = resolution
            .assignments
            .get(*id)
            .with_context(|| format!("No raw rhythm assignment for {name}"))?;
        let input = calibration
            .bindings
            .get(assigned)
            .context("Resolved rhythm input is absent")?;
        digital.insert(
            (*name).into(),
            raw_digital_from_calibrated_input(device, physical, input)?,
        );
        if let Some(rule) = resolution.rules.get(*id) {
            if !matches!(rule, crate::controller_layout::Rule::Identity) {
                warnings.push(format!("{name} uses {assigned}: {}", rule.description()));
            }
        }
    }
    ensure_independent_raw_rhythm_inputs(&digital)?;
    Ok((PadMode::RawPeripheral { kind, digital }, warnings))
}

pub(crate) fn raw_pad_from_probed_calibration(
    calibration: &crate::controller_catalog::Calibration,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    runtime_path: &str,
    dualshock: bool,
    dualanalog: bool,
    analog_joystick: bool,
    analog_toggle_id: Option<&str>,
    deadzone: f32,
    rumble: bool,
) -> Result<(PadMode, Vec<String>)> {
    let device = inventory.device_at_path(runtime_path)?;
    let physical = PhysicalMap::from_device(device)?;
    raw_pad_from_calibration(
        calibration,
        device,
        physical,
        dualshock,
        dualanalog,
        analog_joystick,
        analog_toggle_id,
        deadzone,
        rumble,
    )
}

pub(crate) fn raw_pad_from_calibration(
    calibration: &crate::controller_catalog::Calibration,
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    dualshock: bool,
    dualanalog: bool,
    analog_joystick: bool,
    analog_toggle_id: Option<&str>,
    deadzone: f32,
    rumble: bool,
) -> Result<(PadMode, Vec<String>)> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && std::env::consts::OS == "linux",
        "SDL2 physical translation requires a native Linux calibration"
    );
    ensure!(
        [dualshock, dualanalog, analog_joystick]
            .into_iter()
            .filter(|mode| *mode)
            .count()
            <= 1
            && dualshock == analog_toggle_id.is_some(),
        "Assign an analog-mode control only when selecting DualShock"
    );
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&calibration.layout)
        .context("Unknown physical controller layout")?;
    let target = catalog
        .layout(mode_layout_id(
            dualshock,
            dualanalog,
            analog_joystick,
            None,
            false,
            None,
        ))
        .context("PlayStation controller layout is unavailable")?;
    let toggle = analog_toggle_id
        .map(|id| {
            calibration
                .bindings
                .get(id)
                .with_context(|| format!("Analog toggle {id} has not been calibrated"))
        })
        .transpose()?;
    // Reserve the user's mode switch before assigning gameplay controls.
    let available = calibrated_control_ids(calibration, None, analog_toggle_id);
    let requested = requested_control_ids(target, false);
    let resolution = guided::resolve(calibration, source, target, &available, &requested)?;
    let mut inputs = BTreeMap::new();
    let mut warnings = Vec::new();
    for control in &target.controls {
        let source_id = resolution.assignments.get(&control.id).with_context(|| {
            let reason = resolution
                .missing
                .get(&control.id)
                .map(|missing| missing.description())
                .unwrap_or("No compatible assignment");
            format!("Cannot map {}: {reason}", control.label)
        })?;
        inputs.insert(
            control.id.clone(),
            calibration
                .bindings
                .get(source_id)
                .context("Resolved physical control has no calibration")?
                .clone(),
        );
        if let Some(rule) = resolution.rules.get(&control.id) {
            if !matches!(rule, crate::controller_layout::Rule::Identity) {
                warnings.push(format!(
                    "{} uses {}: {}",
                    control.label,
                    source_id,
                    rule.description()
                ));
            }
        }
    }
    let mode = raw_pad_from_resolved_inputs(
        device,
        physical,
        &inputs,
        toggle,
        dualshock,
        dualanalog,
        analog_joystick,
        deadzone,
        rumble,
    )?;
    Ok((mode, warnings))
}

/// Assemble native bindings from layout-resolved saved calibration inputs.
/// Keys are the catalog's PlayStation semantic IDs, not physical layout IDs.
pub(crate) fn raw_pad_from_resolved_inputs(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    inputs: &BTreeMap<String, crate::controller_catalog::InputBinding>,
    analog_toggle: Option<&crate::controller_catalog::InputBinding>,
    dualshock: bool,
    dualanalog: bool,
    analog_joystick: bool,
    deadzone: f32,
    rumble: bool,
) -> Result<PadMode> {
    ensure!(
        [dualshock, dualanalog, analog_joystick]
            .into_iter()
            .filter(|mode| *mode)
            .count()
            <= 1
            && dualshock == analog_toggle.is_some()
            && (dualshock || !rumble),
        "Only DualShock accepts an analog toggle or rumble"
    );
    let translate = |input: &crate::controller_catalog::InputBinding| {
        raw_digital_from_calibrated_input(device, physical, input)
    };
    let mut digital = BTreeMap::new();
    for (id, target) in [
        ("b", "X"),
        ("a", "○"),
        ("y", "□"),
        ("x", "△"),
        ("select", "Select"),
        ("start", "Start"),
        ("l", "L1"),
        ("r", "R1"),
        ("l2", "L2"),
        ("r2", "R2"),
        ("up", "Up"),
        ("down", "Down"),
        ("left", "Left"),
        ("right", "Right"),
    ] {
        let input = inputs
            .get(id)
            .with_context(|| format!("Missing resolved PlayStation control {id}"))?;
        let target = if (dualshock || dualanalog || analog_joystick)
            && matches!(id, "up" | "down" | "left" | "right")
        {
            format!("D-Pad {target}")
        } else {
            target.into()
        };
        let target = if analog_joystick {
            analog_joystick_name(&target).to_owned()
        } else {
            target
        };
        digital.insert(target, translate(input)?);
    }
    let mut analog = BTreeMap::new();
    if dualshock || dualanalog || analog_joystick {
        for (id, target) in [("l3", "Left Stick, Button"), ("r3", "Right Stick, Button")] {
            if analog_joystick {
                continue;
            }
            digital.insert(
                target.into(),
                translate(
                    inputs
                        .get(id)
                        .with_context(|| format!("Missing resolved DualShock control {id}"))?,
                )?,
            );
        }
        if dualshock {
            digital.insert(
                "Analog".into(),
                translate(analog_toggle.context("DualShock analog toggle has not been assigned")?)?,
            );
        }
        for (negative_id, positive_id, target) in [
            ("stick_left", "stick_right", "Left Stick Left / Right"),
            ("stick_up", "stick_down", "Left Stick Up / Down"),
            (
                "right_stick_left",
                "right_stick_right",
                "Right Stick Left / Right",
            ),
            (
                "right_stick_up",
                "right_stick_down",
                "Right Stick Up / Down",
            ),
        ] {
            let negative = inputs
                .get(negative_id)
                .with_context(|| format!("Missing {negative_id}"))?;
            let positive = inputs
                .get(positive_id)
                .with_context(|| format!("Missing {positive_id}"))?;
            let low_native = negative
                .native
                .as_ref()
                .context("Missing physical stick identity")?;
            let high_native = positive
                .native
                .as_ref()
                .context("Missing physical stick identity")?;
            ensure!(
                low_native.code == high_native.code,
                "Opposite stick directions use different physical axes"
            );
            let low = negative
                .axis
                .as_ref()
                .context("Missing negative stick measurement")?;
            let high = positive
                .axis
                .as_ref()
                .context("Missing positive stick measurement")?;
            ensure!(
                low_native.direction == low.direction()
                    && high_native.direction == high.direction(),
                "Saved stick directions disagree with their measurements"
            );
            analog.insert(
                if analog_joystick {
                    analog_joystick_name(target)
                } else {
                    target
                }
                .into(),
                raw_analog_from_physical(device, physical, low_native.code, low, high, deadzone)?,
            );
        }
    } else {
        ensure!(
            analog_toggle.is_none() && !rumble,
            "Digital PlayStation pad has no analog toggle or rumble"
        );
    }
    Ok(PadMode::Raw {
        digital,
        analog,
        dualshock,
        dualanalog,
        analog_joystick,
        rumble,
    })
}

pub(crate) struct SelectedPad {
    pub virtual_port: u8,
    /// Exact runtime path of the calibrated SDL GameController. Model GUIDs,
    /// names, and VID/PID pairs do not distinguish identical connected pads.
    pub runtime_path: String,
    pub mode: PadMode,
}

/// Assemble mappings from an inventory collected in the emulator's runtime.
/// This does not establish that a raw physical pad has already been calibrated:
/// callers must route through their calibration transport before taking inventory.
pub(crate) fn bindings_from_inventory(
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    selected: &[SelectedPad],
    multitaps: [bool; 2],
) -> Result<NymashockBindings> {
    ensure!(
        inventory.schema_version == 1 && inventory.version[0] == 2 && inventory.version[1] >= 24,
        "Unsupported BizHawk SDL2 inventory contract"
    );
    ensure!(!selected.is_empty(), "No BizHawk controllers selected");
    // Reject malformed deserialized snapshots instead of trusting stored indices.
    let mut instances = std::collections::BTreeSet::new();
    for (index, device) in inventory.devices.iter().enumerate() {
        ensure!(
            device.device_index as usize == index
                && device.instance_id >= 0
                && instances.insert(device.instance_id),
            "Invalid SDL2 inventory numbering"
        );
    }
    let mut bindings = NymashockBindings {
        multitaps,
        ..Default::default()
    };
    let mut assigned = std::collections::BTreeSet::new();
    for pad in selected {
        let device = inventory.device_at_path(&pad.runtime_path)?;
        if matches!(
            &pad.mode,
            PadMode::Digital
                | PadMode::DualShock { .. }
                | PadMode::DualAnalog { .. }
                | PadMode::AnalogJoystick { .. }
        ) {
            ensure!(
                device.is_game_controller,
                "Selected SDL2 device is a raw joystick and needs native binding translation: {}",
                pad.runtime_path
            );
            let mapping = device
                .mapping
                .as_deref()
                .context("SDL2 did not provide this controller's effective mapping")?;
            let decoded = lunchbox_controller_probe::sdl2_mapping::parse(mapping)?;
            // Recognition alone is insufficient: SDL mappings can omit controls.
            // Do not write a launch binding that this device can never deliver.
            let mut required = vec![
                "a",
                "b",
                "x",
                "y",
                "back",
                "start",
                "dpup",
                "dpdown",
                "dpleft",
                "dpright",
                "leftshoulder",
                "rightshoulder",
                "lefttrigger",
                "righttrigger",
            ];
            if matches!(&pad.mode, PadMode::AnalogJoystick { .. }) {
                required.extend(["leftx", "lefty", "rightx", "righty"]);
            }
            if matches!(
                &pad.mode,
                PadMode::DualShock { .. } | PadMode::DualAnalog { .. }
            ) {
                required.extend([
                    "leftstick",
                    "rightstick",
                    "leftx",
                    "lefty",
                    "rightx",
                    "righty",
                ]);
            }
            for output in required {
                ensure!(
                    decoded.iter().any(|binding| binding.output == output),
                    "Selected SDL2 controller cannot deliver required {output}; resolve its calibration mapping first"
                );
            }
        }
        ensure!(
            assigned.insert(device.device_index),
            "One SDL2 controller was assigned to multiple BizHawk ports"
        );
        match &pad.mode {
            PadMode::CalibratedPointer {
                kind,
                digital,
                analog,
            } => {
                let prefix = sdl_gamepad_prefix(device.device_index, true)?;
                ensure!(
                    analog
                        .values()
                        .all(|axis| (*kind != PointerPeripheral::Mouse
                            && matches!(axis.value.as_str(), "WMouse X" | "WMouse Y"))
                            || ["LeftThumbX", "LeftThumbY", "RightThumbX", "RightThumbY"]
                                .iter()
                                .any(|name| axis.value == format!("{prefix}{name} Axis"))),
                    "Pointer axes point outside the selected controller"
                );
                let buttons = digital
                    .iter()
                    .map(|(name, gesture)| {
                        Ok((name.clone(), logical_digital_source(device, gesture)?))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                bindings.add_pointer_peripheral(pad.virtual_port, *kind, &buttons, analog)?;
            }
            PadMode::RawPointer {
                kind,
                desktop_cursor,
                digital,
                analog,
            } => {
                let buttons = digital
                    .iter()
                    .map(|(name, input)| Ok((name.clone(), raw_digital_source(device, *input)?)))
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let mut axes = analog
                    .iter()
                    .map(|(name, input)| {
                        Ok((
                            name.clone(),
                            raw_analog_source(
                                device,
                                input.axis_index,
                                input.multiplier,
                                input.deadzone,
                            )?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                if *desktop_cursor {
                    ensure!(
                        *kind != PointerPeripheral::Mouse && analog.is_empty(),
                        "Desktop cursor must not mix with controller axes"
                    );
                    axes = desktop_cursor_axes();
                }
                bindings.add_pointer_peripheral(pad.virtual_port, *kind, &buttons, &axes)?;
            }
            PadMode::RawNegcon { digital, analog } => {
                let buttons = digital
                    .iter()
                    .map(|(name, input)| Ok((name.clone(), raw_digital_source(device, *input)?)))
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let axes = analog
                    .iter()
                    .map(|(name, input)| {
                        Ok((
                            name.clone(),
                            raw_analog_source(
                                device,
                                input.axis_index,
                                input.multiplier,
                                input.deadzone,
                            )?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                bindings.add_negcon(pad.virtual_port, &buttons, &axes)?;
            }
            PadMode::CalibratedNegcon { digital, analog } => {
                let prefix = sdl_gamepad_prefix(device.device_index, true)?;
                ensure!(
                    analog.values().all(|axis| [
                        "LeftThumbX",
                        "LeftThumbY",
                        "RightThumbX",
                        "RightThumbY",
                        "LeftTrigger",
                        "RightTrigger"
                    ]
                    .iter()
                    .any(|name| axis.value == format!("{prefix}{name} Axis"))),
                    "neGcon logical axes point outside the selected controller"
                );
                let translated = digital
                    .iter()
                    .map(|(name, gesture)| {
                        Ok((name.clone(), logical_digital_source(device, gesture)?))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                bindings.add_negcon(pad.virtual_port, &translated, analog)?;
            }
            PadMode::RawPeripheral { kind, digital } => {
                ensure_independent_raw_rhythm_inputs(digital)?;
                let translated = digital
                    .iter()
                    .map(|(name, input)| Ok((name.clone(), raw_digital_source(device, *input)?)))
                    .collect::<Result<BTreeMap<_, _>>>()?;
                bindings.add_digital_peripheral(pad.virtual_port, *kind, &translated)?;
            }
            PadMode::CalibratedPeripheral { kind, digital } => {
                ensure_independent_rhythm_inputs(device, digital)?;
                let translated = digital
                    .iter()
                    .map(|(name, gesture)| {
                        Ok((name.clone(), logical_digital_source(device, gesture)?))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                bindings.add_digital_peripheral(pad.virtual_port, *kind, &translated)?;
            }
            PadMode::CalibratedDualShock {
                digital,
                analog,
                rumble,
                dualanalog,
                analog_joystick,
            } => {
                let translated = digital
                    .iter()
                    .map(|(name, gesture)| {
                        Ok((name.clone(), logical_digital_source(device, gesture)?))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let mut required = vec![
                    "Select",
                    "Start",
                    "L1",
                    "R1",
                    "L2",
                    "R2",
                    "△",
                    "○",
                    "X",
                    "□",
                    "D-Pad Up",
                    "D-Pad Down",
                    "D-Pad Left",
                    "D-Pad Right",
                    "Left Stick, Button",
                    "Right Stick, Button",
                ];
                if *analog_joystick {
                    required.retain(|name| {
                        !matches!(*name, "Left Stick, Button" | "Right Stick, Button")
                    });
                    required = required.into_iter().map(analog_joystick_name).collect();
                }
                if !dualanalog && !analog_joystick {
                    required.push("Analog");
                }
                ensure!(
                    !(*dualanalog || *analog_joystick) || !rumble,
                    "Only DualShock has rumble"
                );
                let required_axes = [
                    "Left Stick Left / Right",
                    "Left Stick Up / Down",
                    "Right Stick Left / Right",
                    "Right Stick Up / Down",
                ]
                .map(|name| {
                    if *analog_joystick {
                        analog_joystick_name(name)
                    } else {
                        name
                    }
                });
                ensure!(
                    translated.len() == required.len()
                        && required.iter().all(|name| translated.contains_key(*name))
                        && analog.len() == required_axes.len()
                        && required_axes.iter().all(|name| analog.contains_key(*name)),
                    "Calibrated DualShock controls are incomplete or unexpected"
                );
                let prefix = sdl_gamepad_prefix(device.device_index, true)?;
                for axis in analog.values() {
                    ensure!(
                        ["LeftThumbX", "LeftThumbY", "RightThumbX", "RightThumbY"]
                            .iter()
                            .any(|name| axis.value == format!("{prefix}{name} Axis"))
                            && axis.mult.is_finite()
                            && axis.deadzone.is_finite()
                            && (0.0..1.0).contains(&axis.deadzone)
                            && axis.button_bind_positive.is_empty()
                            && axis.button_bind_negative.is_empty(),
                        "Calibrated DualShock axis has invalid source or scale"
                    );
                }
                if *analog_joystick {
                    bindings.add_analog_joystick(pad.virtual_port, device.device_index, 0.0)?;
                } else if *dualanalog {
                    bindings.add_dualanalog(pad.virtual_port, device.device_index, 0.0)?;
                } else {
                    bindings.add_dualshock(
                        pad.virtual_port,
                        device.device_index,
                        translated.get("Analog").context("Missing analog toggle")?,
                        0.0,
                        *rumble,
                    )?;
                }
                for (name, source) in translated {
                    bindings
                        .buttons
                        .insert(format!("P{} {name}", pad.virtual_port + 1), source);
                }
                for (name, axis) in analog {
                    bindings
                        .axes
                        .insert(format!("P{} {name}", pad.virtual_port + 1), axis.clone());
                }
            }
            PadMode::CalibratedDigital(digital) => {
                let translated = digital
                    .iter()
                    .map(|(target, gesture)| {
                        Ok((target.clone(), logical_digital_source(device, gesture)?))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let required = [
                    "Select", "Start", "L1", "R1", "L2", "R2", "△", "○", "X", "□", "Up", "Down",
                    "Left", "Right",
                ];
                ensure!(
                    translated.len() == required.len()
                        && required.iter().all(|name| translated.contains_key(*name)),
                    "Calibrated digital pad has incomplete or unexpected controls"
                );
                bindings.add_digital_pad(pad.virtual_port, device.device_index)?;
                for (name, source) in translated {
                    bindings
                        .buttons
                        .insert(format!("P{} {name}", pad.virtual_port + 1), source);
                }
            }
            PadMode::Digital => bindings.add_digital_pad(pad.virtual_port, device.device_index)?,
            PadMode::DualAnalog { deadzone } => {
                bindings.add_dualanalog(pad.virtual_port, device.device_index, *deadzone)?;
            }
            PadMode::AnalogJoystick { deadzone } => {
                bindings.add_analog_joystick(pad.virtual_port, device.device_index, *deadzone)?;
            }
            PadMode::DualShock {
                analog_toggle,
                deadzone,
                rumble,
            } => {
                bindings.add_dualshock(
                    pad.virtual_port,
                    device.device_index,
                    analog_toggle,
                    *deadzone,
                    *rumble,
                )?;
            }
            PadMode::Raw {
                digital,
                analog,
                dualshock,
                dualanalog,
                analog_joystick,
                rumble,
            } => {
                let mut axes = BTreeMap::new();
                for (target, input) in analog {
                    axes.insert(
                        target.clone(),
                        raw_analog_source(
                            device,
                            input.axis_index,
                            input.multiplier,
                            input.deadzone,
                        )?,
                    );
                }
                bindings.add_raw_pad(
                    pad.virtual_port,
                    device,
                    digital,
                    &axes,
                    *dualshock,
                    *dualanalog,
                    *analog_joystick,
                    *rumble,
                )?;
            }
        }
    }
    Ok(bindings)
}

pub(crate) fn prepare_selected_arguments(
    arguments: &mut Vec<std::ffi::OsString>,
    working_directory: &Path,
    exe_directory: &Path,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    selected: &[SelectedPad],
    multitaps: [bool; 2],
) -> Result<PreparedConfig> {
    let bindings = bindings_from_inventory(inventory, selected, multitaps)?;
    prepare_arguments(arguments, working_directory, exe_directory, &bindings)
}

/// `device_index` must come from the emulator's SDL enumeration, never joydev.
/// Both recognized and raw controllers share that enumeration.
pub(crate) fn sdl_gamepad_prefix(device_index: u32, recognized: bool) -> Result<String> {
    let ordinal = device_index
        .checked_add(1)
        .context("SDL device index overflow")?;
    Ok(format!("{}{ordinal} ", if recognized { 'X' } else { 'J' }))
}

/// SDL2Gamepad.CreateGameControllerButtonGetters, including its distinct
/// stick and trigger thresholds. Touchpad contact is not SDL's touchpad button.
fn ensure_independent_rhythm_inputs(
    device: &lunchbox_controller_probe::sdl2::Device,
    digital: &BTreeMap<String, lunchbox_controller_probe::sdl2_mapping::OutputChange>,
) -> Result<()> {
    use lunchbox_controller_probe::sdl2_mapping::{Input, parse};
    let mapping = parse(
        device
            .mapping
            .as_deref()
            .context("Rhythm controls require an SDL mapping")?,
    )?;
    let mut channels = std::collections::BTreeSet::new();
    let mut hats = BTreeMap::<u32, u8>::new();
    for (name, gesture) in digital {
        let candidates: Vec<_> = mapping
            .iter()
            .filter(|binding| binding.output == gesture.output)
            .collect();
        ensure!(
            candidates.len() == 1,
            "Ambiguous SDL source for rhythm button {name}"
        );
        match candidates[0].input {
            Input::Button(index) => ensure!(
                channels.insert((0u8, index)),
                "Rhythm buttons share physical button {index}"
            ),
            Input::Axis { index, .. } => ensure!(
                channels.insert((1u8, index)),
                "Independent rhythm buttons cannot share axis {index}"
            ),
            Input::Hat { index, mask } => {
                let previous = hats.entry(index).or_default();
                let combined = *previous | mask;
                ensure!(
                    *previous & mask == 0 && combined & 5 != 5 && combined & 10 != 10,
                    "Rhythm buttons require conflicting positions of hat {index}"
                );
                *previous = combined;
            }
        }
    }
    Ok(())
}

fn logical_digital_source(
    device: &lunchbox_controller_probe::sdl2::Device,
    gesture: &lunchbox_controller_probe::sdl2_mapping::OutputChange,
) -> Result<String> {
    ensure!(
        device.is_game_controller,
        "Logical bindings require SDL2 GameController mode"
    );
    let suffix = if !gesture.analog {
        ensure!(
            gesture.released == 0 && gesture.pressed == 1,
            "Digital mapping requires a released-to-pressed gesture"
        );
        match gesture.output.as_str() {
            "a" => "A",
            "b" => "B",
            "x" => "X",
            "y" => "Y",
            "back" => "Back",
            "guide" => "Guide",
            "start" => "Start",
            "leftstick" => "LeftThumb",
            "rightstick" => "RightThumb",
            "leftshoulder" => "LeftShoulder",
            "rightshoulder" => "RightShoulder",
            "dpup" => "DpadUp",
            "dpdown" => "DpadDown",
            "dpleft" => "DpadLeft",
            "dpright" => "DpadRight",
            "misc1" => "Misc",
            "paddle1" => "Paddle1",
            "paddle2" => "Paddle2",
            "paddle3" => "Paddle3",
            "paddle4" => "Paddle4",
            _ => anyhow::bail!("This SDL2 output has no matching BizHawk button getter"),
        }
    } else if matches!(gesture.output.as_str(), "lefttrigger" | "righttrigger") {
        ensure!(
            gesture.released <= 5041 && gesture.pressed > 5041,
            "Measured trigger does not cross BizHawk's digital threshold"
        );
        if gesture.output == "lefttrigger" {
            "LeftTrigger"
        } else {
            "RightTrigger"
        }
    } else {
        let positive = gesture.pressed > gesture.released;
        ensure!(
            if positive {
                gesture.released < 13107 && gesture.pressed >= 13107
            } else {
                gesture.released > -13107 && gesture.pressed <= -13107
            },
            "Measured stick gesture does not cross BizHawk's digital threshold"
        );
        match (gesture.output.as_str(), positive) {
            ("leftx", false) => "LStickLeft",
            ("leftx", true) => "LStickRight",
            ("lefty", false) => "LStickUp",
            ("lefty", true) => "LStickDown",
            ("rightx", false) => "RStickLeft",
            ("rightx", true) => "RStickRight",
            ("righty", false) => "RStickUp",
            ("righty", true) => "RStickDown",
            _ => anyhow::bail!("This SDL2 axis has no matching BizHawk digital getter"),
        }
    };
    Ok(format!(
        "{}{suffix}",
        sdl_gamepad_prefix(device.device_index, true)?
    ))
}

fn valid_gamepad_prefix(prefix: &str) -> bool {
    let Some(body) = prefix.strip_suffix(' ') else {
        return false;
    };
    let Some(number) = body.strip_prefix('X').or_else(|| body.strip_prefix('J')) else {
        return false;
    };
    !number.starts_with('0')
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && number.parse::<u32>().is_ok()
}

#[derive(Clone, Copy)]
pub(crate) enum RawDigitalInput {
    Button(u32),
    AxisDirection { index: u32, positive: bool },
    HatDirection { index: u32, mask: u8 },
}

/// Translate a physical calibration using a separately established SDL2 Linux
/// backend map. Classic and evdev retain their distinct numbering and axis
/// conversions; neither map may be substituted for an SDL HID backend.
pub(crate) fn raw_digital_from_physical(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    encoded: u32,
    measured: Option<lunchbox_controller_probe::linux_classic::AxisEndpoints>,
) -> Result<RawDigitalInput> {
    use lunchbox_controller_probe::duckstation::DigitalInput;
    ensure!(
        !device.is_game_controller,
        "Raw physical calibration requires SDL joystick mode"
    );
    let counts = device
        .controls
        .as_ref()
        .context("SDL2 control counts have not been inspected")?;
    physical.validate_counts(counts)?;
    let input = match physical.digital_input(encoded, measured)? {
        DigitalInput::Button(index) => RawDigitalInput::Button(index),
        DigitalInput::Hat { index, direction } => RawDigitalInput::HatDirection {
            index,
            mask: direction,
        },
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            // BizHawk's CreateJoystickButtonGetters uses integer thresholds
            // derived from +/-32768 / 2.5, after backend axis conversion.
            let positive = pressed > released;
            ensure!(
                if positive {
                    released < 13107 && pressed >= 13107
                } else {
                    released > -13107 && pressed <= -13107
                },
                "Measured axis gesture does not cross BizHawk's native digital threshold; normalized transport is required"
            );
            RawDigitalInput::AxisDirection { index, positive }
        }
    };
    // Apply the native codec's index and cardinal-hat validation as well.
    raw_digital_source(device, input)?;
    Ok(input)
}

fn logical_analog_source(
    device: &lunchbox_controller_probe::sdl2::Device,
    context: &lunchbox_controller_probe::sdl2_mapping::MappingContext,
    negative: &lunchbox_controller_probe::sdl2_mapping::OutputChange,
    positive: &lunchbox_controller_probe::sdl2_mapping::OutputChange,
    deadzone: f32,
) -> Result<AnalogBinding> {
    ensure!(
        device.is_game_controller
            && negative.analog
            && positive.analog
            && negative.output == positive.output,
        "Opposite stick gestures must use one SDL2 logical axis"
    );
    let axis = match negative.output.as_str() {
        "leftx" => "LeftThumbX",
        "lefty" => "LeftThumbY",
        "rightx" => "RightThumbX",
        "righty" => "RightThumbY",
        _ => anyhow::bail!("A full DualShock stick needs a signed SDL2 stick axis"),
    };
    let mapping = lunchbox_controller_probe::sdl2_mapping::parse(&context.mapping)?;
    let parts: Vec<_> = mapping
        .iter()
        .filter(|binding| binding.output == negative.output)
        .collect();
    ensure!(
        parts.len() == 1
            && parts[0].output_range == Some((-32768, 32767))
            && matches!(
                parts[0].input,
                lunchbox_controller_probe::sdl2_mapping::Input::Axis {
                    minimum: -32768,
                    maximum: 32767,
                    ..
                } | lunchbox_controller_probe::sdl2_mapping::Input::Axis {
                    minimum: 32767,
                    maximum: -32768,
                    ..
                }
            ),
        "Composite or half-axis SDL2 mappings need normalized transport for full stick fidelity"
    );
    ensure!(
        deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
        "Invalid logical stick deadzone"
    );
    ensure!(
        [
            negative.released,
            positive.released,
            negative.pressed,
            positive.pressed
        ]
        .iter()
        .all(|value| (-32768..=32767).contains(value)),
        "Invalid logical stick samples"
    );
    let convert = |value: i32| (value as f32 / (32768.0f32 / 10000.0)) as i32;
    ensure!(
        [negative.released, positive.released]
            .iter()
            .all(|value| (convert(*value) as f32 / 10000.0).abs() <= deadzone),
        "Logical stick neutral needs recentering through normalized transport"
    );
    let low = convert(negative.pressed);
    let high = convert(positive.pressed);
    ensure!(
        low != 0 && high != 0 && low.signum() != high.signum(),
        "Stick gestures must reach opposite sides of neutral"
    );
    ensure!(
        (low.abs() - high.abs()).abs() <= 1,
        "Asymmetric stick travel needs independent half-axis normalization"
    );
    let magnitude = low.abs().min(high.abs()) as f32 / 10000.0;
    ensure!(
        magnitude > deadzone,
        "Logical stick travel is inside its deadzone"
    );
    let mult = high.signum() as f32 * (1.0 - deadzone) / (magnitude - deadzone);
    ensure!(mult.is_finite(), "Invalid logical stick multiplier");
    Ok(AnalogBinding {
        value: format!(
            "{}{axis} Axis",
            sdl_gamepad_prefix(device.device_index, true)?
        ),
        mult,
        deadzone,
        button_bind_positive: String::new(),
        button_bind_negative: String::new(),
    })
}

/// A zero-neutral Nyma pressure axis can scale either signed half of a host
/// axis, but cannot offset its rest position. Require a measured continuous
/// axis; SDL button-to-axis mappings are not proportional pressure sources.
fn logical_pressure_source(
    device: &lunchbox_controller_probe::sdl2::Device,
    context: &lunchbox_controller_probe::sdl2_mapping::MappingContext,
    gesture: &lunchbox_controller_probe::sdl2_mapping::OutputChange,
    physical_measurement: &crate::controller_axis::AxisMeasurement,
    deadzone: f32,
) -> Result<AnalogBinding> {
    crate::controller_axis::PressureAxis::from_measurement(physical_measurement)?;
    ensure!(
        device.is_game_controller && gesture.analog,
        "Pressure requires an SDL2 analog gesture"
    );
    let axis = match gesture.output.as_str() {
        "leftx" => "LeftThumbX",
        "lefty" => "LeftThumbY",
        "rightx" => "RightThumbX",
        "righty" => "RightThumbY",
        "lefttrigger" => "LeftTrigger",
        "righttrigger" => "RightTrigger",
        _ => anyhow::bail!("Unsupported SDL2 pressure axis"),
    };
    let mapping = lunchbox_controller_probe::sdl2_mapping::parse(&context.mapping)?;
    let parts: Vec<_> = mapping
        .iter()
        .filter(|binding| binding.output == gesture.output)
        .collect();
    ensure!(
        parts.len() == 1
            && matches!(parts[0].input, lunchbox_controller_probe::sdl2_mapping::Input::Axis { minimum, maximum, .. } if (i64::from(maximum) - i64::from(minimum)).abs() > 2),
        "Pressure requires one continuous physical SDL axis"
    );
    ensure!(
        deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
        "Invalid pressure deadzone"
    );
    ensure!(
        [gesture.released, gesture.pressed]
            .into_iter()
            .all(|value| (-32768..=32767).contains(&value)),
        "Invalid pressure samples"
    );
    let convert = |value: i32| (value as f32 / (32768.0f32 / 10000.0)) as i32;
    let released = convert(gesture.released) as f32 / 10000.0;
    let pressed = convert(gesture.pressed) as f32 / 10000.0;
    ensure!(
        released.abs() <= deadzone,
        "Pressure rest position requires normalized transport"
    );
    ensure!(
        pressed.abs() > deadzone
            && (i64::from(gesture.pressed) - i64::from(gesture.released)).abs() > 2,
        "Pressure gesture has no proportional travel outside the deadzone"
    );
    let mult = pressed.signum() * (1.0 - deadzone) / (pressed.abs() - deadzone);
    ensure!(mult.is_finite(), "Invalid pressure scale");
    Ok(AnalogBinding {
        value: format!(
            "{}{axis} Axis",
            sdl_gamepad_prefix(device.device_index, true)?
        ),
        mult,
        deadzone,
        button_bind_positive: String::new(),
        button_bind_negative: String::new(),
    })
}

fn raw_axis_name(index: u32) -> Result<String> {
    ensure!(index < 1024, "Invalid SDL2 raw axis index");
    Ok(["X", "Y", "Z", "W", "V", "S", "Q", "P", "N"]
        .get(index as usize)
        .map(|name| (*name).to_owned())
        .unwrap_or_else(|| format!("Axis{index}")))
}

/// Full-stick calibration for an established SDL2 physical backend. A single
/// BizHawk multiplier cannot recenter or independently scale the two half-axes;
/// report those cases for normalized transport instead of distorting the stick.
fn raw_pressure_from_physical(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    encoded: u32,
    measurement: &crate::controller_axis::AxisMeasurement,
    deadzone: f32,
) -> Result<RawAnalogInput> {
    crate::controller_axis::PressureAxis::from_measurement(measurement)?;
    ensure!(
        deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
        "Invalid pressure deadzone"
    );
    let lunchbox_controller_probe::linux_classic::Control::Axis(axis_index) =
        physical.control(encoded)?
    else {
        anyhow::bail!("Proportional pressure requires a physical analog axis, not a button or hat");
    };
    physical.validate_counts(device.controls.as_ref().context("SDL2 counts are absent")?)?;
    let convert = |value| -> Result<f32> {
        Ok(
            (f32::from(physical.axis_value((encoded & 0xffff) as u8, value)?)
                / (32768.0f32 / 10000.0)) as i32 as f32
                / 10000.0,
        )
    };
    let released = convert(measurement.released)?;
    let pressed = convert(measurement.pressed)?;
    ensure!(
        released.abs() <= deadzone,
        "Pressure rest position requires normalized transport"
    );
    ensure!(
        pressed.abs() > deadzone,
        "Pressure travel is inside the configured deadzone"
    );
    let multiplier = pressed.signum() * (1.0 - deadzone) / (pressed.abs() - deadzone);
    raw_analog_source(device, axis_index, multiplier, deadzone)?;
    Ok(RawAnalogInput {
        axis_index,
        multiplier,
        deadzone,
    })
}

pub(crate) fn raw_analog_from_physical(
    device: &lunchbox_controller_probe::sdl2::Device,
    physical: PhysicalMap<'_>,
    encoded: u32,
    negative: &crate::controller_axis::AxisMeasurement,
    positive: &crate::controller_axis::AxisMeasurement,
    deadzone: f32,
) -> Result<RawAnalogInput> {
    use lunchbox_controller_probe::linux_classic::Control;
    negative.validate()?;
    positive.validate()?;
    ensure!(
        negative.minimum == positive.minimum
            && negative.maximum == positive.maximum
            && negative.released == positive.released
            && negative.flat == positive.flat
            && negative.fuzz == positive.fuzz
            && negative.resolution == positive.resolution,
        "Stick directions have inconsistent physical measurements"
    );
    ensure!(
        deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
        "Invalid stick deadzone"
    );
    let Control::Axis(axis_index) = physical.control(encoded)? else {
        anyhow::bail!(
            "Full stick calibration requires a physical analog axis, not a button or hat"
        );
    };
    let counts = device
        .controls
        .as_ref()
        .context("SDL2 control counts have not been inspected")?;
    physical.validate_counts(counts)?;
    // Match SDL2Gamepad.GetAxes's truncating conversion before Controller.cs's
    // deadzone and multiplier. Calibration uses physical, pre-correction units.
    let convert = |value| -> Result<i32> {
        Ok(
            (f32::from(physical.axis_value((encoded & 0xffff) as u8, value)?)
                / (32768.0f32 / 10000.0)) as i32,
        )
    };
    let neutral = convert(negative.released)?;
    let low = convert(negative.pressed)?;
    let high = convert(positive.pressed)?;
    ensure!(
        (neutral as f32 / 10000.0).abs() <= deadzone,
        "Stick neutral needs recentering through normalized transport"
    );
    ensure!(
        low != 0 && high != 0 && low.signum() != high.signum(),
        "Stick calibration must reach both sides of SDL neutral"
    );
    ensure!(
        (low.abs() - high.abs()).abs() <= 1,
        "Asymmetric stick travel needs independent half-axis normalization"
    );
    let magnitude = low.abs().min(high.abs()) as f32 / 10000.0;
    ensure!(
        magnitude > deadzone,
        "Stick travel is inside the configured deadzone"
    );
    let multiplier = high.signum() as f32 * (1.0 - deadzone) / (magnitude - deadzone);
    raw_analog_source(device, axis_index, multiplier, deadzone)?;
    Ok(RawAnalogInput {
        axis_index,
        multiplier,
        deadzone,
    })
}

/// Encode a measured SDL joystick index, not an evdev code or a joydev index.
/// Caller checks that the physical control exists and that any axis direction
/// reaches BizHawk's fixed raw-stick digital threshold (~40% of full scale).
pub(crate) fn raw_digital_source(
    device: &lunchbox_controller_probe::sdl2::Device,
    input: RawDigitalInput,
) -> Result<String> {
    ensure!(
        !device.is_game_controller,
        "Raw BizHawk bindings require SDL joystick mode"
    );
    let counts = device
        .controls
        .as_ref()
        .context("Inspect the raw SDL2 device's control counts before mapping it")?;
    let prefix = sdl_gamepad_prefix(device.device_index, false)?;
    let suffix = match input {
        RawDigitalInput::Button(index) => {
            ensure!(
                index < counts.buttons && index < 1024,
                "Invalid SDL2 raw button index"
            );
            format!("B{}", index + 1)
        }
        RawDigitalInput::AxisDirection { index, positive } => {
            ensure!(
                index < counts.axes,
                "SDL2 raw axis is not present on the selected controller"
            );
            format!(
                "{}{}",
                raw_axis_name(index)?,
                if positive { '+' } else { '-' }
            )
        }
        RawDigitalInput::HatDirection { index, mask } => {
            ensure!(
                index < counts.hats && index < 1024,
                "Invalid SDL2 raw hat index"
            );
            let direction = match mask {
                1 => 'U',
                2 => 'R',
                4 => 'D',
                8 => 'L',
                _ => anyhow::bail!("BizHawk raw hat binding requires one cardinal direction"),
            };
            format!("POV{index}{direction}")
        }
    };
    Ok(format!("{prefix}{suffix}"))
}

pub(crate) fn raw_analog_source(
    device: &lunchbox_controller_probe::sdl2::Device,
    axis_index: u32,
    multiplier: f32,
    deadzone: f32,
) -> Result<AnalogBinding> {
    ensure!(
        !device.is_game_controller,
        "Raw BizHawk axes require SDL joystick mode"
    );
    let counts = device
        .controls
        .as_ref()
        .context("Inspect the raw SDL2 device's control counts before mapping it")?;
    ensure!(
        axis_index < counts.axes,
        "SDL2 raw axis is not present on the selected controller"
    );
    ensure!(
        multiplier.is_finite() && deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
        "Invalid raw BizHawk analog calibration"
    );
    Ok(AnalogBinding {
        value: format!(
            "{}{} Axis",
            sdl_gamepad_prefix(device.device_index, false)?,
            raw_axis_name(axis_index)?
        ),
        mult: multiplier,
        deadzone,
        button_bind_positive: String::new(),
        button_bind_negative: String::new(),
    })
}

/// Complete ownership of one deck's bindings, not a patch over stale controls.
/// Port keys are zero-based Nyma virtual ports, not physical SDL device indices.
#[derive(Default)]
pub(crate) struct NymashockBindings {
    /// Physical-port multitaps, explicitly selected for this launch.
    pub multitaps: [bool; 2],
    pub buttons: BTreeMap<String, String>,
    pub axes: BTreeMap<String, AnalogBinding>,
    pub feedback: BTreeMap<String, FeedbackBinding>,
    pub port_devices: BTreeMap<u8, String>,
}

impl NymashockBindings {
    /// Pointer-device contracts are separate from ordinary pad axes. WMouse
    /// supplies transformed absolute position, never relative mouse movement.
    pub(crate) fn add_pointer_peripheral(
        &mut self,
        port: u8,
        kind: PointerPeripheral,
        digital: &BTreeMap<String, String>,
        analog: &BTreeMap<String, AnalogBinding>,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        let (device, buttons, axes): (&str, &[&str], &[&str]) = match kind {
            PointerPeripheral::Mouse => (
                "mouse",
                &["Left Button", "Right Button"],
                &["Motion Left / Right", "Motion Up / Down"],
            ),
            PointerPeripheral::GunCon => (
                "guncon",
                &["Trigger", "A", "B", "Offscreen Shot"],
                &["X Axis", "Y Axis"],
            ),
            PointerPeripheral::Justifier => (
                "justifier",
                &["Trigger", "O", "Start", "Offscreen Shot"],
                &["X Axis", "Y Axis"],
            ),
        };
        if kind == PointerPeripheral::Justifier {
            let second_port = if self.multitaps[0] { 4 } else { 1 };
            ensure!(
                port == 0 || port == second_port,
                "The pinned Justifier core does not work properly on multitap B-D slots"
            );
        }
        ensure!(
            digital.len() == buttons.len()
                && buttons.iter().all(|name| digital.contains_key(*name)),
            "Incomplete or unexpected {device} buttons"
        );
        ensure!(
            analog.len() == axes.len() && axes.iter().all(|name| analog.contains_key(*name)),
            "Incomplete or unexpected {device} axes"
        );
        ensure!(
            digital
                .values()
                .all(|value| !value.trim().is_empty() && !value.chars().any(char::is_control)),
            "Invalid pointer-device button binding"
        );
        for axis in analog.values() {
            let absolute_mouse = matches!(axis.value.as_str(), "WMouse X" | "WMouse Y");
            ensure!(
                !axis.value.trim().is_empty()
                    && !axis.value.chars().any(char::is_control)
                    && (axis.value.ends_with(" Axis")
                        || (kind != PointerPeripheral::Mouse && absolute_mouse))
                    && axis.mult.is_finite()
                    && axis.mult != 0.0
                    && axis.deadzone.is_finite()
                    && (0.0..1.0).contains(&axis.deadzone)
                    && axis.button_bind_positive.is_empty()
                    && axis.button_bind_negative.is_empty(),
                "Invalid {device} axis source; absolute cursor position cannot supply relative movement"
            );
        }
        let player = format!("P{} ", port + 1);
        self.buttons.extend(
            digital
                .iter()
                .map(|(name, source)| (format!("{player}{name}"), source.clone())),
        );
        self.axes.extend(
            analog
                .iter()
                .map(|(name, axis)| (format!("{player}{name}"), axis.clone())),
        );
        self.port_devices.insert(port, device.into());
        Ok(())
    }

    /// neGcon's twist is centered, but I/II/L are zero-neutral pressure axes.
    /// Callers must supply calibrated sources: this contract never substitutes
    /// digital on/off buttons for pressure or guesses a third trigger mapping.
    pub(crate) fn add_negcon(
        &mut self,
        port: u8,
        digital: &BTreeMap<String, String>,
        analog: &BTreeMap<String, AnalogBinding>,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        let buttons = [
            "Start",
            "D-Pad Up",
            "D-Pad Down",
            "D-Pad Left",
            "D-Pad Right",
            "R",
            "A",
            "B",
        ];
        let axes = ["Twist Ccwise / Cwise", "I", "II", "L"];
        ensure!(
            digital.len() == buttons.len()
                && buttons.iter().all(|name| digital.contains_key(*name)),
            "neGcon requires its exact eight digital controls"
        );
        ensure!(
            analog.len() == axes.len() && axes.iter().all(|name| analog.contains_key(*name)),
            "neGcon requires twist and all three pressure axes"
        );
        ensure!(
            digital
                .values()
                .all(|value| !value.trim().is_empty() && !value.chars().any(char::is_control)),
            "Invalid neGcon digital binding"
        );
        for axis in analog.values() {
            ensure!(
                !axis.value.trim().is_empty()
                    && axis.value.ends_with(" Axis")
                    && !axis.value.chars().any(char::is_control)
                    && axis.mult.is_finite()
                    && axis.mult != 0.0
                    && axis.deadzone.is_finite()
                    && (0.0..1.0).contains(&axis.deadzone)
                    && axis.button_bind_positive.is_empty()
                    && axis.button_bind_negative.is_empty(),
                "neGcon requires calibrated analog sources without digital substitutes"
            );
        }
        let player = format!("P{} ", port + 1);
        self.buttons.extend(
            digital
                .iter()
                .map(|(name, source)| (format!("{player}{name}"), source.clone())),
        );
        self.axes.extend(
            analog
                .iter()
                .map(|(name, axis)| (format!("{player}{name}"), axis.clone())),
        );
        self.port_devices.insert(port, "negcon".into());
        Ok(())
    }

    /// Use explicitly translated host inputs; no implicit gamepad aliases or
    /// contradictory-direction suppression are introduced for rhythm controls.
    pub(crate) fn add_digital_peripheral(
        &mut self,
        port: u8,
        kind: DigitalPeripheral,
        digital: &BTreeMap<String, String>,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        let required = kind.buttons();
        ensure!(
            digital.len() == required.len()
                && required.iter().all(|name| digital.contains_key(*name)),
            "Incomplete or unexpected {} controller buttons",
            kind.device_id()
        );
        ensure!(
            digital
                .values()
                .all(|source| !source.trim().is_empty() && !source.chars().any(char::is_control)),
            "Invalid native rhythm-controller binding"
        );
        let player = format!("P{} ", port + 1);
        self.buttons.extend(
            digital
                .iter()
                .map(|(name, source)| (format!("{player}{name}"), source.clone())),
        );
        self.port_devices.insert(port, kind.device_id().into());
        Ok(())
    }

    /// SCPH-1110, Mednafen's `analogjoy`: fourteen buttons and four axes.
    /// Host assignments follow PSX wire-button identities, not name similarity.
    /// There are no stick-click, analog-toggle, or rumble controls.
    pub(crate) fn add_analog_joystick(
        &mut self,
        port: u8,
        sdl_index: u32,
        deadzone: f32,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        ensure!(
            deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
            "Invalid Analog Joystick deadzone"
        );
        let host = sdl_gamepad_prefix(sdl_index, true)?;
        let player = format!("P{} ", port + 1);
        for (target, source) in [
            ("Select", "Back"),
            ("Start", "Start"),
            ("Thumbstick Up", "DpadUp"),
            ("Thumbstick Down", "DpadDown"),
            ("Thumbstick Left", "DpadLeft"),
            ("Thumbstick Right", "DpadRight"),
            ("Left Stick, Trigger", "LeftTrigger"),
            ("Left Stick, Pinky", "RightTrigger"),
            ("Left Stick, L-Thumb", "LeftShoulder"),
            ("Left Stick, R-Thumb", "RightShoulder"),
            ("Right Stick, Pinky", "Y"),
            ("Right Stick, R-Thumb", "B"),
            ("Right Stick, L-Thumb", "A"),
            ("Right Stick, Trigger", "X"),
        ] {
            self.buttons
                .insert(format!("{player}{target}"), format!("{host}{source}"));
        }
        for (target, source) in [
            ("Left Stick, Left / Right", "LeftThumbX"),
            ("Left Stick, Up / Down", "LeftThumbY"),
            ("Right Stick, Left / Right", "RightThumbX"),
            ("Right Stick, Up / Down", "RightThumbY"),
        ] {
            // The core marks all four axes co_invert=false.
            self.axes.insert(
                format!("{player}{target}"),
                AnalogBinding {
                    value: format!("{host}{source} Axis"),
                    mult: 1.0,
                    deadzone,
                    button_bind_positive: String::new(),
                    button_bind_negative: String::new(),
                },
            );
        }
        self.port_devices.insert(port, "analogjoy".into());
        Ok(())
    }

    /// Already translated/calibrated SDL raw controls, keyed by Nyma's exact
    /// per-player suffixes. Build locally and merge only after all checks pass.
    pub(crate) fn add_raw_pad(
        &mut self,
        port: u8,
        device: &lunchbox_controller_probe::sdl2::Device,
        digital: &BTreeMap<String, RawDigitalInput>,
        analog: &BTreeMap<String, AnalogBinding>,
        dualshock: bool,
        dualanalog: bool,
        analog_joystick: bool,
        rumble: bool,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        ensure!(
            !device.is_game_controller,
            "Raw pad mapping requires SDL joystick mode"
        );
        ensure!(
            [dualshock, dualanalog, analog_joystick]
                .into_iter()
                .filter(|mode| *mode)
                .count()
                <= 1
                && (dualshock || !rumble),
            "Only DualShock has rumble motors; select one pad mode"
        );
        let mut required = vec![
            "Select", "Start", "L1", "R1", "L2", "R2", "△", "○", "X", "□",
        ];
        if dualshock || dualanalog || analog_joystick {
            required.extend([
                "D-Pad Up",
                "D-Pad Down",
                "D-Pad Left",
                "D-Pad Right",
                "Left Stick, Button",
                "Right Stick, Button",
            ]);
            if dualshock {
                required.push("Analog");
            }
            if analog_joystick {
                required
                    .retain(|name| !matches!(*name, "Left Stick, Button" | "Right Stick, Button"));
                required = required.into_iter().map(analog_joystick_name).collect();
            }
        } else {
            required.extend(["Up", "Down", "Left", "Right"]);
        }
        ensure!(
            digital.len() == required.len()
                && required.iter().all(|key| digital.contains_key(*key)),
            "Raw Nymashock mapping does not contain exactly the selected pad's digital controls"
        );
        let required_axes = [
            "Left Stick Left / Right",
            "Left Stick Up / Down",
            "Right Stick Left / Right",
            "Right Stick Up / Down",
        ]
        .map(|name| {
            if analog_joystick {
                analog_joystick_name(name)
            } else {
                name
            }
        });
        ensure!(
            if dualshock || dualanalog || analog_joystick {
                analog.len() == 4 && required_axes.iter().all(|key| analog.contains_key(*key))
            } else {
                analog.is_empty()
            },
            "Raw Nymashock mapping has incomplete or unexpected stick axes"
        );
        let player = format!("P{} ", port + 1);
        let host = sdl_gamepad_prefix(device.device_index, false)?;
        let mut buttons = BTreeMap::new();
        let mut axes = BTreeMap::new();
        for (name, input) in digital {
            buttons.insert(
                format!("{player}{name}"),
                raw_digital_source(device, *input)?,
            );
        }
        for (name, input) in analog {
            ensure!(
                input.value.starts_with(&host) && input.value.ends_with(" Axis"),
                "Raw Nymashock axis points to another controller"
            );
            ensure!(
                input.mult.is_finite()
                    && input.deadzone.is_finite()
                    && (0.0..1.0).contains(&input.deadzone),
                "Invalid raw Nymashock axis scaling"
            );
            ensure!(
                input.button_bind_positive.is_empty() && input.button_bind_negative.is_empty(),
                "Raw stick mapping must not inherit unverified alternate buttons"
            );
            axes.insert(
                format!("{player}{name}"),
                AnalogBinding {
                    value: input.value.clone(),
                    mult: input.mult,
                    deadzone: input.deadzone,
                    button_bind_positive: String::new(),
                    button_bind_negative: String::new(),
                },
            );
        }
        self.buttons.extend(buttons);
        self.axes.extend(axes);
        if rumble {
            for (target, channel) in [("Left (strong)", "Left"), ("Right (weak)", "Right")] {
                self.feedback.insert(
                    format!("{player}Rumble {target}"),
                    FeedbackBinding {
                        channels: channel.into(),
                        gamepad_prefix: host.clone(),
                        prescale: 1.0,
                    },
                );
            }
        }
        self.port_devices.insert(
            port,
            if dualshock {
                "dualshock"
            } else if dualanalog {
                "dualanalog"
            } else if analog_joystick {
                "analogjoy"
            } else {
                "gamepad"
            }
            .into(),
        );
        Ok(())
    }

    /// SCPH-1080 digital pad. Unlike DualShock, this device names its directional
    /// inputs Up/Down/Left/Right, without the "D-Pad" prefix, and has no sticks.
    pub(crate) fn add_digital_pad(&mut self, port: u8, sdl_index: u32) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        let host = sdl_gamepad_prefix(sdl_index, true)?;
        let player = format!("P{} ", port + 1);
        for (target, source) in [
            ("Select", "Back"),
            ("Start", "Start"),
            ("Up", "DpadUp"),
            ("Down", "DpadDown"),
            ("Left", "DpadLeft"),
            ("Right", "DpadRight"),
            ("L1", "LeftShoulder"),
            ("R1", "RightShoulder"),
            ("L2", "LeftTrigger"),
            ("R2", "RightTrigger"),
            ("△", "Y"),
            ("○", "B"),
            ("X", "A"),
            ("□", "X"),
        ] {
            self.buttons
                .insert(format!("{player}{target}"), format!("{host}{source}"));
        }
        self.port_devices.insert(port, "gamepad".into());
        Ok(())
    }

    /// Map an SDL-recognized, calibrated GameController onto a Nyma DualShock.
    /// The mode-toggle source is explicit: never steal a gameplay button for it.
    /// Source: pinned Mednafen 382ff1b8d293c9a862497706808cbb79b2cecbfb,
    /// src/psx/input/dualshock.cpp and Nyma's OverrideButtonName conversion.
    pub(crate) fn add_dualshock(
        &mut self,
        port: u8,
        sdl_index: u32,
        analog_toggle: &str,
        deadzone: f32,
        rumble: bool,
    ) -> Result<()> {
        ensure!(
            !analog_toggle.trim().is_empty(),
            "DualShock analog toggle needs an explicit binding"
        );
        self.add_dual_stick_pad(port, sdl_index, Some(analog_toggle), deadzone, rumble)
    }

    /// SCPH-1180 uses the same controls but is permanently analog and has no
    /// mode-toggle or force-feedback inputs in Mednafen's dualanalog.cpp.
    pub(crate) fn add_dualanalog(&mut self, port: u8, sdl_index: u32, deadzone: f32) -> Result<()> {
        self.add_dual_stick_pad(port, sdl_index, None, deadzone, false)
    }

    fn add_dual_stick_pad(
        &mut self,
        port: u8,
        sdl_index: u32,
        analog_toggle: Option<&str>,
        deadzone: f32,
        rumble: bool,
    ) -> Result<()> {
        ensure!(
            port < 8 && !self.port_devices.contains_key(&port),
            "Duplicate or invalid Nymashock port {port}"
        );
        ensure!(
            deadzone.is_finite() && (0.0..1.0).contains(&deadzone),
            "Invalid dual-stick pad deadzone"
        );
        let host = sdl_gamepad_prefix(sdl_index, true)?;
        let player = format!("P{} ", port + 1);
        for (target, source) in [
            ("Select", "Back"),
            ("Start", "Start"),
            ("D-Pad Up", "DpadUp"),
            ("D-Pad Down", "DpadDown"),
            ("D-Pad Left", "DpadLeft"),
            ("D-Pad Right", "DpadRight"),
            ("L1", "LeftShoulder"),
            ("R1", "RightShoulder"),
            ("L2", "LeftTrigger"),
            ("R2", "RightTrigger"),
            ("Left Stick, Button", "LeftThumb"),
            ("Right Stick, Button", "RightThumb"),
            ("△", "Y"),
            ("○", "B"),
            ("X", "A"),
            ("□", "X"),
        ] {
            self.buttons
                .insert(format!("{player}{target}"), format!("{host}{source}"));
        }
        if let Some(toggle) = analog_toggle {
            self.buttons
                .insert(format!("{player}Analog"), toggle.to_owned());
        }
        for (target, source) in [
            ("Left Stick Left / Right", "LeftThumbX"),
            ("Left Stick Up / Down", "LeftThumbY"),
            ("Right Stick Left / Right", "RightThumbX"),
            ("Right Stick Up / Down", "RightThumbY"),
        ] {
            // All four DualShock axes have co_invert=false; SDL and the core
            // both use negative-up Y, so no multiplier inversion is needed.
            self.axes.insert(
                format!("{player}{target}"),
                AnalogBinding {
                    value: format!("{host}{source} Axis"),
                    mult: 1.0,
                    deadzone,
                    button_bind_positive: String::new(),
                    button_bind_negative: String::new(),
                },
            );
        }
        if rumble {
            for (target, channel) in [("Left (strong)", "Left"), ("Right (weak)", "Right")] {
                self.feedback.insert(
                    format!("{player}Rumble {target}"),
                    FeedbackBinding {
                        channels: channel.into(),
                        gamepad_prefix: host.clone(),
                        prescale: 1.0,
                    },
                );
            }
        }
        self.port_devices.insert(
            port,
            if analog_toggle.is_some() {
                "dualshock"
            } else {
                "dualanalog"
            }
            .into(),
        );
        Ok(())
    }
}

fn object_child<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    parent
        .entry(key.to_owned())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .with_context(|| format!("BizHawk {key} must be a JSON object"))
}

/// Return a private launch config; never mutate the user's source config.
/// Unknown config keys, other controller decks, and unrelated sync settings survive.
/// Caller must resolve native controller names and device identity before encoding.
pub(crate) fn encode_nymashock_config(
    source: &str,
    bindings: &NymashockBindings,
) -> Result<String> {
    ensure!(
        !bindings.buttons.is_empty(),
        "Nymashock requires explicit digital bindings"
    );
    for (name, axis) in &bindings.axes {
        ensure!(!name.trim().is_empty(), "Empty BizHawk axis name");
        ensure!(
            axis.mult.is_finite()
                && axis.deadzone.is_finite()
                && (0.0..1.0).contains(&axis.deadzone),
            "Invalid BizHawk analog scaling for {name}"
        );
        ensure!(
            axis.value.is_empty()
                || axis.value.ends_with(" Axis")
                || matches!(axis.value.as_str(), "WMouse X" | "WMouse Y"),
            "BizHawk analog source must name an SDL axis or an absolute WMouse coordinate: {name}"
        );
        ensure!(
            !axis.value.is_empty()
                || !axis.button_bind_positive.is_empty()
                || !axis.button_bind_negative.is_empty(),
            "Unbound BizHawk axis: {name}"
        );
    }
    for name in bindings.buttons.keys() {
        ensure!(!name.trim().is_empty(), "Empty BizHawk button name");
    }
    for (name, feedback) in &bindings.feedback {
        ensure!(!name.trim().is_empty(), "Empty BizHawk feedback name");
        ensure!(
            valid_gamepad_prefix(&feedback.gamepad_prefix),
            "Invalid BizHawk feedback device prefix for {name}"
        );
        ensure!(
            feedback.prescale.is_finite() && feedback.prescale >= 0.0,
            "Invalid BizHawk feedback scaling for {name}"
        );
        let mut channels = std::collections::BTreeSet::new();
        for channel in feedback.channels.split('+') {
            ensure!(
                matches!(channel, "Left" | "Right") && channels.insert(channel),
                "Invalid or repeated SDL feedback channel for {name}"
            );
        }
    }
    ensure!(
        !bindings.port_devices.is_empty(),
        "Nymashock requires explicit port devices"
    );
    for (port, device) in &bindings.port_devices {
        ensure!(
            *port < 8 && !device.trim().is_empty(),
            "Invalid Nymashock virtual port {port}"
        );
        let available = 2 + 3 * bindings
            .multitaps
            .iter()
            .filter(|enabled| **enabled)
            .count();
        ensure!(
            (*port as usize) < available,
            "Nymashock virtual port {port} is not connected by the selected multitap topology"
        );
    }
    let mut config: Value = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .context("Parsing the source BizHawk config")?;
    let root = config
        .as_object_mut()
        .context("BizHawk config must be a JSON object")?;
    // RomLoader otherwise tries the user's preferred core and can silently fall
    // back to Octoshock, whose control names differ from this Nyma contract.
    object_child(root, "PreferredCores")?.insert("PSX".into(), Value::String("Nymashock".into()));
    root.insert("DontTryOtherCores".into(), Value::Bool(true));
    object_child(root, "AllTrollers")?
        .insert(PSX_DECK.into(), serde_json::to_value(&bindings.buttons)?);
    object_child(root, "AllTrollersAnalog")?
        .insert(PSX_DECK.into(), serde_json::to_value(&bindings.axes)?);
    object_child(root, "AllTrollersFeedbacks")?
        .insert(PSX_DECK.into(), serde_json::to_value(&bindings.feedback)?);
    // Do not inherit autofire as a second source of input for the owned deck.
    object_child(root, "AllTrollersAutoFire")?.insert(PSX_DECK.into(), Value::Object(Map::new()));
    let settings = object_child(object_child(root, "CoreSyncSettings")?, NYMASHOCK)?;
    let mednafen = object_child(settings, "MednafenValues")?;
    for (index, enabled) in bindings.multitaps.iter().enumerate() {
        mednafen.insert(
            format!("psx.input.pport{}.multitap", index + 1),
            Value::String(if *enabled { "1" } else { "0" }.into()),
        );
    }
    let ports = object_child(settings, "PortDevices")?;
    // The adapter owns this entire deck's bindings. Inherit no configured pad
    // on an unselected port: it would remain plugged in with no mapped inputs.
    // Keep unrelated serialized dictionary metadata intact.
    for port in 0..8u8 {
        let device = bindings
            .port_devices
            .get(&port)
            .map(String::as_str)
            .unwrap_or("none");
        ports.insert(port.to_string(), Value::String(device.into()));
    }
    // Preserve unrelated MednafenValues (memory cards and firmware choices)
    // and the serializer's existing $type metadata.
    let mut encoded = serde_json::to_string_pretty(&config)?;
    encoded.push('\n');
    Ok(encoded)
}
