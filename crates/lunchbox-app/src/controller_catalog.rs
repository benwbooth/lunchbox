//! Physical layouts, device calibration and explicit emulator input contracts.
//! The checked-in facts and original schematic renderer are independent of raw
//! device numbering. Documented contracts are not a claim of runtime validation.
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub layouts: Vec<Layout>,
    pub emulator_profiles: Vec<EmulatorProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub family: String,
    pub shape: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub notes: String,
    pub controls: Vec<Control>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub group: String,
    pub optional: bool,
    pub analog: bool,
    /// Unipolar measured pressure independent of ergonomic group. Older
    /// shoulder-axis layouts retain their existing pressure interpretation.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pressure: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_of: Option<String>,
}

impl Control {
    pub(crate) fn is_pressure(&self) -> bool {
        self.analog && (self.pressure || self.group == "shoulder")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmulatorProfile {
    pub id: String,
    pub name: String,
    pub core: String,
    pub target_layout: String,
    pub transport: String,
    /// System and player limits for an implemented native configuration writer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_launch: Option<NativeLaunch>,
    pub status: String,
    pub source: String,
    pub conditions: Vec<String>,
    pub bindings: BTreeMap<String, String>,
    /// One-based frontend ports may request a subset of the contract controls.
    /// Console controls can belong to player one without making every other
    /// connected pad supply the same panel. Omitted ports use all bindings.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub port_controls: BTreeMap<usize, std::collections::BTreeSet<String>>,
    /// A fixed topology can expose different semantic controls on each port.
    /// IDs retain their transport bindings, while labels/layout rules follow
    /// the target actually selected for that port.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub port_layouts: BTreeMap<usize, String>,
    /// Complete binding replacement for a heterogeneous fixed-topology port.
    /// Must have a matching port_layouts entry; cannot also use a subset.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub port_bindings: BTreeMap<usize, BTreeMap<String, String>>,
    /// All frontend ports exposed by this core, including expansion ports that
    /// must be explicitly disconnected. Defaults to the active mode's capacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontend_ports: Option<usize>,
    /// One-based libretro port device overrides for a fixed, explicitly selected
    /// console topology. Attachments such as a multitap stay connected even if
    /// that port's first player slot has no physical controller assigned.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub port_devices: BTreeMap<usize, u32>,
    /// Some cores restore controller device types from save states. Until the
    /// state metadata is resolved, automatic state loading cannot preserve this
    /// launch-time device contract.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_fresh_start: bool,
    /// Option-defined/topology modes require an explicit choice rather than
    /// guessing from the number or capabilities of connected physical pads.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub explicit_selection: bool,
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub content_extensions: std::collections::BTreeSet<String>,
    /// Content-dependent peripheral selection must be checked before writing a
    /// standard-pad launch configuration; filenames are not game identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_guard: Option<ContentGuard>,
    #[serde(default)]
    pub core_options: BTreeMap<String, String>,
    /// Exact core-reported library name used for RetroArch override directories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retroarch_library: Option<String>,
    /// Explicit opt-in to the implemented Linux RetroArch launch writer.
    /// Documented preview profiles do not implicitly become launch contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retroarch_launch: Option<RetroArchLaunch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLaunch {
    pub platforms: Vec<String>,
    pub max_players: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentGuard {
    DolphinGamecubeRaw,
    ScummvmRetropadCursor,
    SteemsseEmbeddedSte,
    SimcpSamOne,
    SimcpSamTwo,
    SimcpKempston,
    HatariStJoystick,
    Ep128emuTvcDefaults,
    Ep128emuZxDefaults,
    Ep128emuEnterpriseDefaults,
    Ep128emuCpcDefaults,
    Quasi88Disk,
    PuaeFixedInput,
    AtariComputerMedia,
    Atari5200Cartridge,
    BkJoystick,
    CrocodsJoystick,
    GeolithStandardCartridge,
    ViceJoystickPort1,
    ViceJoystickPort2,
    StellaJoysticks,
    StellaGenesisPads,
    #[serde(rename = "stella_booster_joy2b")]
    StellaBoosterGripOrJoy2BPlus,
}

impl ContentGuard {
    pub(crate) fn simcp_interface(self) -> Option<crate::controller_simcp::JoystickInterface> {
        use crate::controller_simcp::JoystickInterface;
        match self {
            Self::SimcpSamOne => Some(JoystickInterface::SamOne),
            Self::SimcpSamTwo => Some(JoystickInterface::SamTwo),
            Self::SimcpKempston => Some(JoystickInterface::Kempston),
            _ => None,
        }
    }

    pub fn stella_contract(self) -> Option<crate::controller_stella::DigitalContract> {
        use crate::controller_stella::DigitalContract;
        match self {
            Self::StellaJoysticks => Some(DigitalContract::Joysticks),
            Self::StellaGenesisPads => Some(DigitalContract::GenesisPads),
            Self::StellaBoosterGripOrJoy2BPlus => Some(DigitalContract::BoosterGripOrJoy2BPlus),
            Self::HatariStJoystick
            | Self::DolphinGamecubeRaw
            | Self::ScummvmRetropadCursor
            | Self::SteemsseEmbeddedSte
            | Self::SimcpSamOne
            | Self::SimcpSamTwo
            | Self::SimcpKempston
            | Self::Ep128emuTvcDefaults
            | Self::Ep128emuZxDefaults
            | Self::Ep128emuEnterpriseDefaults
            | Self::Ep128emuCpcDefaults
            | Self::GeolithStandardCartridge
            | Self::ViceJoystickPort1
            | Self::ViceJoystickPort2
            | Self::CrocodsJoystick
            | Self::BkJoystick
            | Self::AtariComputerMedia
            | Self::Atari5200Cartridge
            | Self::PuaeFixedInput
            | Self::Quasi88Disk => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetroArchLaunch {
    /// Exact platform aliases; matching only trims whitespace and ignores case.
    pub platforms: Vec<String>,
    /// libretro device ID, including a reviewed joypad subclass when necessary.
    pub device: u32,
    /// Player count for this device mode, not every mode the core supports.
    pub max_players: usize,
    /// Some cores change their frontend port count through a core option.
    /// The selected value must be resolved from the effective options file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_topology: Option<PlayerTopology>,
}

impl EmulatorProfile {
    pub fn launch_device_for_port(&self, port: usize) -> Option<u32> {
        let launch = self.retroarch_launch.as_ref()?;
        if !(1..=self.frontend_port_count()).contains(&port) {
            return None;
        }
        Some(
            self.port_devices
                .get(&port)
                .copied()
                .unwrap_or(launch.device),
        )
    }

    pub fn frontend_port_count(&self) -> usize {
        self.frontend_ports.unwrap_or_else(|| {
            self.retroarch_launch
                .as_ref()
                .map_or(0, |launch| launch.max_players)
        })
    }

    pub fn for_port(&self, port: usize) -> std::borrow::Cow<'_, Self> {
        let controls = self.port_controls.get(&port);
        let layout = self.port_layouts.get(&port);
        let bindings = self.port_bindings.get(&port);
        if controls.is_none() && layout.is_none() && bindings.is_none() {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut profile = self.clone();
        if let Some(bindings) = bindings {
            profile.bindings.clone_from(bindings);
        }
        if let Some(controls) = controls {
            profile.bindings.retain(|id, _| controls.contains(id));
        }
        if let Some(layout) = layout {
            profile.target_layout.clone_from(layout);
        }
        std::borrow::Cow::Owned(profile)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerTopology {
    pub option: String,
    pub default: String,
    pub values: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputBinding {
    /// Opaque GilRs native code: it is intentionally scoped by OS/backend.
    pub code: u32,
    pub kind: String,
    /// Direction in GilRs' normalized coordinate system, not raw evdev sign.
    pub direction: i8,
    pub logical: String,
    /// Physical evdev input, captured before translating to emulator numbering.
    /// Older calibrations lack this and remain usable for preview only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeInput>,
    /// Measured physical rest/peak/bounds, separate from input identity so two
    /// observations of the same control cannot evade duplicate-input checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<crate::controller_axis::AxisMeasurement>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInput {
    pub code: u32,
    /// Raw kernel axis sign, or zero for a physical button.
    pub direction: i8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    /// Target-profile choices are separate from physical button measurements.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub target_mappings: BTreeMap<String, BTreeMap<String, String>>,
    pub layout: String,
    pub os: String,
    pub backend: String,
    pub bindings: BTreeMap<String, InputBinding>,
}

#[derive(Debug, Serialize)]
pub struct MappingPlan {
    pub mapping_policy_version: u32,
    pub profile: String,
    pub transport: String,
    pub status: String,
    pub automatic_launch_ready: bool,
    /// A guided bridge exists, but the mapping alone cannot establish the
    /// native runtime/content setup or backend compatibility required to launch.
    pub native_runtime_required: bool,
    pub rows: Vec<MappingRow>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MappingRow {
    pub target_id: String,
    pub target: String,
    pub physical_id: Option<String>,
    pub physical: String,
    pub input: Option<InputBinding>,
    pub output: String,
    pub reason: String,
}

pub fn catalog() -> &'static Catalog {
    static DB: OnceLock<Catalog> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db: Catalog =
            serde_json::from_str(include_str!("../data/controllers/catalog.json"))
                .expect("bundled controller catalog must parse");
        crate::controller_sdl3::add_layout(&mut db);
        crate::controller_ares::add_profiles(&mut db).expect("ares controller profiles");
        crate::controller_native_targets::add_profiles(&mut db)
            .expect("native controller profiles");
        crate::controller_target::add_native_metadata(&mut db).expect("native controller metadata");
        db.add_mame_analog_layouts()
            .expect("MAME combined layouts require their bundled source layouts");
        db.add_mame_directional_switch_layouts()
            .expect("MAME directional switch layouts require fixed-channel panels");
        db.add_fbneo_channel_layout()
            .expect("FBNeo channel layout requires reference geometry");
        db.validate()
            .expect("bundled controller catalog must validate");
        db
    })
}

impl Catalog {
    fn add_mame_directional_switch_layouts(&mut self) -> Result<()> {
        for base in [
            "mame-fixed-digital",
            "mame-twin-digital",
            "mame-fixed-digital-analog",
            "mame-twin-digital-analog",
        ] {
            let mut layout = self
                .layout(base)
                .context("Missing MAME switch base")?
                .clone();
            layout.id = format!("{base}-switches");
            ensure!(
                self.layout(&layout.id).is_none(),
                "Duplicate MAME switch panel"
            );
            layout.name.push_str(" + axis switches");
            layout.shape = "grid".to_owned();
            layout.notes.push_str(" Lower band: explicit threshold switches for four bipolar axes. Each negative/positive pair is coupled, not independent simultaneous buttons. These are frontend channels, not cabinet geometry.");
            for control in &mut layout.controls {
                control.y *= 0.64;
            }
            for (index, (id, label)) in [
                ("axis_lx_negative_switch", "LX − switch"),
                ("axis_lx_positive_switch", "LX + switch"),
                ("axis_ly_negative_switch", "LY − switch"),
                ("axis_ly_positive_switch", "LY + switch"),
                ("axis_rx_negative_switch", "RX − switch"),
                ("axis_rx_positive_switch", "RX + switch"),
                ("axis_ry_negative_switch", "RY − switch"),
                ("axis_ry_positive_switch", "RY + switch"),
            ]
            .into_iter()
            .enumerate()
            {
                ensure!(
                    !layout.controls.iter().any(|control| control.id == id),
                    "Duplicate MAME directional switch identity"
                );
                layout
                    .controls
                    .push(serde_json::from_value(serde_json::json!({
                        "id":id,"label":label,"x":([15.0,38.0,62.0,85.0][index % 4]),
                        "y":if index < 4 {78.0} else {94.0},
                        "group":"stick","optional":true,"analog":false
                    }))?);
            }
            self.layouts.push(layout);
        }
        Ok(())
    }

    fn add_fbneo_channel_layout(&mut self) -> Result<()> {
        let mouse_controls = [
            ("mouse_x", "X delta"),
            ("mouse_y", "Y delta"),
            ("mouse_left", "Left button"),
            ("mouse_right", "Right button"),
            ("mouse_middle", "Middle button"),
            ("mouse_button4", "Button 4"),
            ("mouse_button5", "Button 5"),
            ("wheel_up", "Wheel up"),
            ("wheel_down", "Wheel down"),
            ("horizontal_wheel_up", "Horizontal wheel up"),
            ("horizontal_wheel_down", "Horizontal wheel down"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (id, label))| {
            serde_json::json!({
                "id":id,"label":label,"x":15.0 + (index % 3) as f64 * 35.0,
                "y":15.0 + (index / 3) as f64 * 20.0,
            "group":"pointer","optional":true,"analog":false
            })
        })
        .collect::<Vec<_>>();
        ensure!(
            self.layout("fbneo-mouse-channels").is_none(),
            "Duplicate FBNeo mouse panel"
        );
        self.layouts.push(serde_json::from_value(serde_json::json!({
            "id":"fbneo-mouse-channels","name":"FBNeo — relative mouse channels",
            "family":"pointer","shape":"grid","source":"libretro mouse callback identities",
            "notes":"Logical event panel, not physical cabinet geometry. Delta axes are relative events, not proportional joystick controls. Wheel conversion and live relative routing remain separate requirements; diagram availability does not establish mapping support.",
            "controls":mouse_controls
        }))?);
        let mut layout = self
            .layout("xbox")
            .context("Missing gamepad reference geometry")?
            .clone();
        layout.id = "fbneo-retropad-channels".into();
        layout.name = "FBNeo frontend RetroPad channels".into();
        layout.source = "FBNeo native address translation and libretro RetroPad channels; original schematic geometry".into();
        layout.notes = "Virtual frontend channels, not a physical Xbox controller or an original arcade cabinet. Input part distinguishes digital buttons, pressure and axis directions; native action labels remain per-game.".into();
        for control in &mut layout.controls {
            control.optional = true;
            // This virtual channel panel supports both digital and analog
            // button values, not a claim about pressure sensors in an Xbox pad.
            control.analog = true;
            control.label = match control.id.as_str() {
                "b" => "B / South",
                "a" => "A / East",
                "y" => "Y / West",
                "x" => "X / North",
                _ => continue,
            }
            .into();
        }
        ensure!(
            self.layout(&layout.id).is_none(),
            "Duplicate FBNeo channel layout"
        );
        self.layouts.push(layout);
        let id = "fbneo-lightgun-buttons";
        ensure!(self.layout(id).is_none(), "Duplicate FBNeo lightgun panel");
        let controls = [
            ("gun_trigger", "Trigger"),
            ("gun_offscreen_shot", "Offscreen shot"),
            ("gun_aux_a", "Aux A"),
            ("gun_aux_b", "Aux B"),
            ("gun_aux_c", "Aux C"),
            ("gun_start", "Start / Pause"),
            ("gun_select", "Select"),
            ("gun_dpad_up", "Up"),
            ("gun_dpad_down", "Down"),
            ("gun_dpad_left", "Left"),
            ("gun_dpad_right", "Right"),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (id, label))| Control {
            id: id.into(),
            label: label.into(),
            x: 12.0 + (index % 4) as f64 * 25.0,
            y: 20.0 + (index / 4) as f64 * 30.0,
            group: "auxiliary".into(),
            optional: true,
            analog: false,
            pressure: false,
            repeat_of: None,
        })
        .collect();
        self.layouts.push(Layout {
            id: id.into(), name: "FBNeo frontend lightgun buttons".into(),
            family: "pointer".into(), shape: "grid".into(),
            source: "RetroArch lightgun binding fields used by the pinned FBNeo adapter".into(),
            notes: "Logical button panel, not a physical gun. Coordinate capture and offscreen status are separate inputs; the offscreen-shot button alone does not establish either.".into(),
            controls,
        });
        Ok(())
    }

    /// Derive combined panels from the existing button geometries and measured
    /// analog control definitions. No extra emulator coverage is implied.
    fn add_mame_analog_layouts(&mut self) -> Result<()> {
        // These are frontend switch channels, not guessed cabinet buttons.
        // Dedicated IDs keep switch-only calibration digital even when the
        // derived analog panel also exposes proportional L2/R2 controls.
        for id in ["mame-fixed-digital", "mame-twin-digital"] {
            let layout = self
                .layouts
                .iter_mut()
                .find(|layout| layout.id == id)
                .ok_or_else(|| anyhow::anyhow!("Missing MAME switch layout {id}"))?;
            let twin = id == "mame-twin-digital";
            // Optional controls reach y=95; the gamepad rectangle ends above
            // them. These are logical channel panels, not physical shells.
            layout.shape = "grid".to_owned();
            if twin {
                // Preserve native action identities even when numbering is
                // sparse. These are logical positions, not a cabinet replica.
                for number in 1..=16 {
                    let control_id = format!("button{number}");
                    let x = [15.0, 35.0, 65.0, 85.0][(number - 1) % 4];
                    let y = [5.0, 20.0, 80.0, 95.0][(number - 1) / 4];
                    if let Some(control) = layout
                        .controls
                        .iter_mut()
                        .find(|control| control.id == control_id)
                    {
                        control.x = x;
                        control.y = y;
                    } else {
                        layout
                            .controls
                            .push(serde_json::from_value(serde_json::json!({
                                "id":control_id,"label":format!("Button {number}"),
                                "x":x,"y":y,"group":"face","optional":true,"analog":false
                            }))?);
                    }
                }
            }
            for (id, label, x) in [
                (
                    "trigger_left_switch",
                    "L2 switch",
                    if twin { 43.0 } else { 38.0 },
                ),
                (
                    "trigger_right_switch",
                    "R2 switch",
                    if twin { 57.0 } else { 60.0 },
                ),
            ] {
                ensure!(
                    !layout.controls.iter().any(|control| control.id == id),
                    "Duplicate MAME trigger switch control"
                );
                layout
                    .controls
                    .push(serde_json::from_value(serde_json::json!({
                    "id":id,"label":label,"x":x,"y":if twin { 70.0 } else { 95.0 },"group":"shoulder",
                        "optional":true,"analog":false
                    }))?);
            }
            layout.notes.push_str(" Optional L2/R2 switch positions are frontend axis-threshold channels, not native numbered buttons or cabinet geometry.");
        }
        let mut analog: Vec<_> = self
            .layout("xbox")
            .ok_or_else(|| anyhow::anyhow!("Missing analog reference layout"))?
            .controls
            .iter()
            .filter(|control| control.analog)
            .cloned()
            .collect();
        analog.extend(
            self.layout("dualshock2-pressure")
                .ok_or_else(|| anyhow::anyhow!("Missing pressure reference layout"))?
                .controls
                .iter()
                .filter(|control| matches!(control.id.as_str(), "l2" | "r2"))
                .cloned(),
        );
        ensure!(analog.len() == 10, "Unexpected analog reference controls");
        ensure!(
            analog.iter().all(|control| control.analog),
            "Analog reference contains a digital substitute"
        );
        for base in [
            "mame-fixed-digital",
            "mame-twin-digital",
            "arcade-six-button",
            "arcade-eight-button",
            "neogeo",
        ] {
            let mut layout = self
                .layout(base)
                .ok_or_else(|| anyhow::anyhow!("Missing MAME base layout {base}"))?
                .clone();
            layout.id = format!("{base}-analog");
            ensure!(
                self.layout(&layout.id).is_none(),
                "Duplicate generated MAME layout"
            );
            layout.name.push_str(" + analog channels");
            layout.shape = "grid".to_owned();
            layout.notes.push_str(" Combined schematic: original digital arrangement above; native analog channels below. Only requested controls need calibration.");
            // Preserve horizontal ordering and relative digital geometry while
            // reserving a separate lower band for the analog channel controls.
            for control in &mut layout.controls {
                control.y *= 0.62;
            }
            for mut control in analog.clone() {
                let (x, y) = match control.id.as_str() {
                    "stick_left" => (12.0, 80.0),
                    "stick_right" => (28.0, 80.0),
                    "stick_up" => (20.0, 68.0),
                    "stick_down" => (20.0, 92.0),
                    "right_stick_left" => (47.0, 80.0),
                    "right_stick_right" => (63.0, 80.0),
                    "right_stick_up" => (55.0, 68.0),
                    "right_stick_down" => (55.0, 92.0),
                    "l2" => (80.0, 74.0),
                    "r2" => (92.0, 88.0),
                    _ => anyhow::bail!("Unexpected analog reference control"),
                };
                ensure!(
                    !layout
                        .controls
                        .iter()
                        .any(|existing| existing.id == control.id),
                    "Combined MAME control ID collision"
                );
                control.x = x;
                control.y = y;
                control.optional = true;
                layout.controls.push(control);
            }
            self.layouts.push(layout);
        }
        Ok(())
    }

    pub fn layout(&self, id: &str) -> Option<&Layout> {
        self.layouts.iter().find(|layout| layout.id == id)
    }

    pub fn launch_profile(&self, core: &str, platform: &str) -> Option<&EmulatorProfile> {
        let matches = self.launch_modes(core, platform);
        if matches.len() == 1 {
            return Some(matches[0]);
        }
        self.launch_mode(core, platform, 1)
    }

    pub fn launch_modes(&self, core: &str, platform: &str) -> Vec<&EmulatorProfile> {
        self.platform_profiles(core, platform)
            .into_iter()
            .filter(|profile| !profile.explicit_selection)
            .collect()
    }

    pub fn platform_profiles(&self, core: &str, platform: &str) -> Vec<&EmulatorProfile> {
        self.emulator_profiles
            .iter()
            .filter(|profile| {
                crate::emulator::canonical_retroarch_core_name(&profile.core)
                    == crate::emulator::canonical_retroarch_core_name(core)
                    && profile.retroarch_launch.as_ref().is_some_and(|launch| {
                        launch
                            .platforms
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(platform.trim()))
                    })
            })
            .collect()
    }

    pub fn launch_mode(&self, core: &str, platform: &str, device: u32) -> Option<&EmulatorProfile> {
        let mut matches = self
            .launch_modes(core, platform)
            .into_iter()
            .filter(|profile| profile.retroarch_launch.as_ref().unwrap().device == device);
        let first = matches.next()?;
        // Refuse ambiguity even if a caller has not validated its catalog yet.
        matches.next().is_none().then_some(first)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "unsupported controller catalog schema"
        );
        let mut ids = HashSet::new();
        for layout in &self.layouts {
            ensure!(
                valid_id(&layout.id) && ids.insert(&layout.id),
                "duplicate or invalid layout ID"
            );
            ensure!(
                !layout.name.is_empty() && !layout.controls.is_empty(),
                "empty layout"
            );
            ensure!(
                matches!(
                    layout.shape.as_str(),
                    "rectangle" | "handheld" | "dual-grip" | "three-grip" | "grid"
                ),
                "unknown shape"
            );
            ensure!(
                matches!(
                    layout.family.as_str(),
                    "one-button"
                        | "pointer"
                        | "atari-console"
                        | "channel-f"
                        | "keypad"
                        | "two-button"
                        | "horizontal-four"
                        | "diamond"
                        | "n64"
                        | "three-button"
                        | "six-button"
                        | "arcade-rows"
                        | "four-button-row"
                        | "dual-direction"
                        | "twist-controller"
                        | "dance-pad"
                        | "rhythm-nine"
                ),
                "unknown layout-rule family"
            );
            let mut controls = HashSet::new();
            ensure!(
                layout.controls.len() <= 64,
                "Layout exceeds the 64-control assignment resource bound"
            );
            for control in &layout.controls {
                ensure!(
                    valid_id(&control.id) && controls.insert(&control.id),
                    "duplicate control ID"
                );
                ensure!(!control.label.is_empty(), "empty control label");
                ensure!(
                    !control.pressure
                        || (control.analog
                            && matches!(
                                control.group.as_str(),
                                "face" | "dpad" | "shoulder" | "rear"
                            )
                            && control.repeat_of.is_none()),
                    "Pressure requires an analog face, direction or shoulder control"
                );
                ensure!(
                    matches!(
                        control.group.as_str(),
                        "face"
                            | "pointer"
                            | "shoulder"
                            | "rear"
                            | "menu"
                            | "dpad"
                            | "stick"
                            | "turbo"
                            | "auxiliary"
                    ),
                    "unknown control semantic group"
                );
                ensure!(
                    control.group != "turbo" || control.repeat_of.is_some(),
                    "Hardware turbo needs its repeated control"
                );
                if let Some(base) = &control.repeat_of {
                    ensure!(
                        control.group == "turbo"
                            && control.optional
                            && !control.analog
                            && layout.controls.iter().any(|c| c.id == *base
                                && c.id != control.id
                                && c.repeat_of.is_none()),
                        "Invalid hardware turbo control"
                    );
                }
                ensure!(
                    control.x.is_finite()
                        && control.y.is_finite()
                        && (0.0..=100.0).contains(&control.x)
                        && (0.0..=100.0).contains(&control.y),
                    "invalid control coordinates"
                );
            }
        }
        let mut profiles = HashSet::new();
        let mut launch_targets = HashSet::new();
        for profile in &self.emulator_profiles {
            if profile.core == "pcsx2" && profile.retroarch_launch.is_some() {
                let target = self
                    .layout(&profile.target_layout)
                    .context("Unknown LRPS2 controller layout")?;
                crate::controller_lrps2::validate_profile(profile, target)?;
            }
            if profile.content_guard == Some(ContentGuard::PuaeFixedInput) {
                ensure!(
                    profile.core == "puae"
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.retroarch_library.as_deref() == Some("PUAE")
                        && profile.core_options.contains_key("puae_model")
                        && profile.frontend_port_count() == 6,
                    "PUAE configuration guard requires an explicit fresh-start machine profile and all six frontend ports"
                );
                if profile.target_layout == "puae-cd32" {
                    ensure!(
                        matches!(
                            profile.core_options.get("puae_model").map(String::as_str),
                            Some("CD32" | "CD32FR")
                        ) && profile
                            .core_options
                            .get("puae_cd32pad_options")
                            .map(String::as_str)
                            == Some("disabled")
                            && profile
                                .core_options
                                .get("puae_mapper_start")
                                .map(String::as_str)
                                == Some("---")
                            && profile
                                .retroarch_launch
                                .as_ref()
                                .is_some_and(
                                    |launch| launch.device == 517 && launch.max_players == 2
                                ),
                        "PUAE CD32 layout requires fixed normal serial pads without a Return binding on Play/Pause"
                    );
                }
            }
            if matches!(
                profile.content_guard,
                Some(ContentGuard::AtariComputerMedia | ContentGuard::Atari5200Cartridge)
            ) {
                let console = profile.content_guard == Some(ContentGuard::Atari5200Cartridge);
                ensure!(
                    profile.core == "atari800"
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.core_options.get("atari800_cfg").map(String::as_str)
                            == Some("disabled")
                        && profile
                            .core_options
                            .get("atari800_opt2")
                            .map(String::as_str)
                            == Some("none")
                        && profile
                            .core_options
                            .get("paddle_active")
                            .map(String::as_str)
                            == Some("disabled")
                        && profile
                            .core_options
                            .get("atari800_xep80")
                            .map(String::as_str)
                            == Some("disabled")
                        && profile
                            .core_options
                            .get("atari800_system")
                            .is_some_and(|model| (model == "5200") == console)
                        && profile.frontend_port_count() == 4
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == if console { 769 } else { 513 }),
                    "Atari800 input contracts require fixed system, port and legacy-configuration state"
                );
            }
            if profile.content_guard == Some(ContentGuard::BkJoystick) {
                ensure!(
                    profile.core == "bk"
                        && profile.target_layout == "bk-four-button-joystick"
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile
                            .core_options
                            .get("bk_peripheral")
                            .map(String::as_str)
                            == Some("joystick")
                        && matches!(
                            profile.core_options.get("bk_model").map(String::as_str),
                            Some(
                                "BK-0010"
                                    | "BK-0010.01"
                                    | "BK-0010.01 + FDD"
                                    | "BK-0011M + FDD"
                                    | "Slow BK-0011M"
                            )
                        )
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 1)
                        && profile.frontend_port_count() == 2,
                    "BK joystick guard requires an explicit BK model and one shared emulated joystick"
                );
            }
            if profile
                .content_guard
                .and_then(ContentGuard::simcp_interface)
                .is_some()
            {
                ensure!(
                    profile.core == "simcp"
                        && profile.target_layout == "simcp-patched-joystick"
                        && profile.retroarch_library.as_deref() == Some("SimCoupe")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.frontend_port_count() == 1
                        && profile.core_options.is_empty()
                        && profile.content_extensions.len() == 3
                        && ["sad", "dsk", "mgt"]
                            .iter()
                            .all(|ext| profile.content_extensions.contains(*ext))
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 1),
                    "SimCoupe interface guards require the explicit patched one-player disk contract"
                );
            }
            if profile.content_guard == Some(ContentGuard::SteemsseEmbeddedSte) {
                ensure!(
                    profile.core == "steemsse"
                        && profile.target_layout == "steemsse-st-joystick"
                        && profile.retroarch_library.as_deref() == Some("SteemSSE")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.frontend_port_count() == 2
                        && profile.core_options.get("sse_st_type").map(String::as_str)
                            == Some("STE")
                        && profile.core_options.get("sse_st_os").map(String::as_str) == Some("Emu")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 2),
                    "Steem SSE embedded-ROM joystick mode requires explicit fresh STE/EmuTOS with two frontend ports"
                );
            }
            if profile.content_guard == Some(ContentGuard::ScummvmRetropadCursor) {
                ensure!(
                    profile.core == "scummvm"
                        && profile.target_layout == "scummvm-retropad-cursor"
                        && profile.retroarch_library.as_deref() == Some("ScummVM")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.frontend_port_count() == 1
                        && profile.content_extensions.len() == 1
                        && profile.content_extensions.contains("scummvm")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 1),
                    "ScummVM fixed cursor guard requires explicit fresh one-port RetroPad mode"
                );
                crate::controller_scummvm::validate_options(&profile.core_options)?;
            }
            if profile.content_guard == Some(ContentGuard::DolphinGamecubeRaw) {
                ensure!(
                    profile.core == "dolphin"
                        && profile.target_layout == "dolphin-gamecube-mixed-triggers"
                        && profile.retroarch_library.as_deref() == Some("dolphin-emu")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.frontend_port_count() == 4
                        && profile.content_extensions
                            == std::collections::BTreeSet::from(["iso".into(), "gcm".into()])
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 4),
                    "Dolphin raw GameCube guard requires explicit fresh four-port pad mode"
                );
                crate::controller_dolphin::validate_gamecube_options(&profile.core_options)?;
            }
            if profile.content_guard == Some(ContentGuard::HatariStJoystick) {
                let jump = match profile.target_layout.as_str() {
                    "hatari-st-joystick-jump" => "enabled",
                    "hatari-st-joystick-space" => "disabled",
                    _ => bail!("Hatari ST guard requires the matching jump or Space layout"),
                };
                ensure!(
                    profile.core == "hatari"
                        && profile.retroarch_library.as_deref() == Some("hatari")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.frontend_port_count() == 6
                        && profile
                            .core_options
                            .get("hatari_machinetype")
                            .map(String::as_str)
                            == Some("st")
                        && profile
                            .core_options
                            .get("hatari_joystick_port1")
                            .map(String::as_str)
                            == Some("real")
                        && profile
                            .core_options
                            .get("hatari_joystick_autofire")
                            .map(String::as_str)
                            == Some("disabled")
                        && profile
                            .core_options
                            .get("hatari_joystick_jump_fire2")
                            .map(String::as_str)
                            == Some(jump)
                        && profile.retroarch_launch.as_ref().is_some_and(|launch| {
                            launch.device == 1
                                && matches!(launch.max_players, 1 | 2)
                                && profile
                                    .core_options
                                    .get("hatari_joystick_port0")
                                    .map(String::as_str)
                                    == Some(if launch.max_players == 2 {
                                        "real"
                                    } else {
                                        "none"
                                    })
                        }),
                    "Hatari ST guard requires fixed machine, reversed ST ports and matching shortcut options"
                );
            }
            if profile.content_guard == Some(ContentGuard::CrocodsJoystick) {
                ensure!(
                    profile.core == "crocods"
                        && profile.target_layout == "crocods-joystick-keyboard"
                        && profile.retroarch_library.as_deref() == Some("crocods")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.content_extensions.len() == 1
                        && profile.content_extensions.contains("dsk")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 1)
                        && profile.core_options.is_empty(),
                    "CrocoDS guard requires its explicit fresh disk/joystick contract"
                );
            }
            if profile.content_guard == Some(ContentGuard::Ep128emuCpcDefaults) {
                ensure!(
                    profile.core == "ep128emu-core"
                        && profile.target_layout == "ep128emu-two-fire-shortcuts"
                        && profile.retroarch_library.as_deref() == Some("ep128emu")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.content_extensions.len() == 2
                        && profile.content_extensions.contains("dsk")
                        && profile.content_extensions.contains("cdt")
                        && profile.frontend_port_count() == 2
                        && profile.for_port(2).target_layout == "ep128emu-cpc-joystick-two"
                        && profile
                            .core_options
                            .get("ep128emu_zoom")
                            .map(String::as_str)
                            == Some("R3")
                        && profile
                            .core_options
                            .get("ep128emu_info")
                            .map(String::as_str)
                            == Some("L3")
                        && profile
                            .core_options
                            .get("ep128emu_afbt")
                            .map(String::as_str)
                            == Some("None")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 2),
                    "ep128emu CPC guard requires fresh default two-port joysticks and fixed shortcut/autofire options"
                );
            }
            if profile.content_guard == Some(ContentGuard::Ep128emuEnterpriseDefaults) {
                ensure!(
                    profile.core == "ep128emu-core"
                        && profile.target_layout == "ep128emu-one-fire-shortcuts"
                        && profile.retroarch_library.as_deref() == Some("ep128emu")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.content_extensions.len() == 1
                        && profile.content_extensions.contains("tap")
                        && profile.frontend_port_count() == 6
                        && (2..=6)
                            .all(|port| profile.for_port(port).target_layout == "ep128emu-one-fire")
                        && profile
                            .core_options
                            .get("ep128emu_zoom")
                            .map(String::as_str)
                            == Some("R3")
                        && profile
                            .core_options
                            .get("ep128emu_info")
                            .map(String::as_str)
                            == Some("L3")
                        && profile
                            .core_options
                            .get("ep128emu_afbt")
                            .map(String::as_str)
                            == Some("None")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 6),
                    "ep128emu Enterprise guard requires fresh default six-port topology and fixed shortcut/autofire options"
                );
            }
            if profile.content_guard == Some(ContentGuard::Ep128emuZxDefaults) {
                ensure!(
                    profile.core == "ep128emu-core"
                        && profile.target_layout == "ep128emu-one-fire-shortcuts"
                        && profile.retroarch_library.as_deref() == Some("ep128emu")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.content_extensions.len() == 2
                        && profile.content_extensions.contains("tzx")
                        && profile.content_extensions.contains("tap")
                        && profile.frontend_port_count() == 4
                        && profile.for_port(2).target_layout == "ep128emu-zx-sinclair-one"
                        && profile.for_port(3).target_layout == "ep128emu-zx-sinclair-two"
                        && profile.for_port(4).target_layout == "ep128emu-zx-protek"
                        && profile
                            .core_options
                            .get("ep128emu_zoom")
                            .map(String::as_str)
                            == Some("R3")
                        && profile
                            .core_options
                            .get("ep128emu_info")
                            .map(String::as_str)
                            == Some("L3")
                        && profile
                            .core_options
                            .get("ep128emu_afbt")
                            .map(String::as_str)
                            == Some("None")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 4),
                    "ep128emu ZX guard requires fresh four-adapter defaults and fixed shortcut/autofire options"
                );
            }
            if profile.content_guard == Some(ContentGuard::Ep128emuTvcDefaults) {
                ensure!(
                    profile.core == "ep128emu-core"
                        && profile.target_layout == "ep128emu-one-fire-shortcuts"
                        && profile.retroarch_library.as_deref() == Some("ep128emu")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.content_extensions.len() == 2
                        && profile.content_extensions.contains("tvcwav")
                        && profile.content_extensions.contains("crt")
                        && profile.frontend_port_count() == 5
                        && (2..=5)
                            .all(|port| profile.for_port(port).target_layout == "ep128emu-one-fire")
                        && profile
                            .core_options
                            .get("ep128emu_zoom")
                            .map(String::as_str)
                            == Some("R3")
                        && profile
                            .core_options
                            .get("ep128emu_info")
                            .map(String::as_str)
                            == Some("L3")
                        && profile
                            .core_options
                            .get("ep128emu_afbt")
                            .map(String::as_str)
                            == Some("None")
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1 && launch.max_players == 5),
                    "ep128emu TVC guard requires fresh five-port defaults and fixed shortcut/autofire options"
                );
            }
            if let Some(
                guard @ (ContentGuard::ViceJoystickPort1 | ContentGuard::ViceJoystickPort2),
            ) = profile.content_guard
            {
                let port = if guard == ContentGuard::ViceJoystickPort1 {
                    "1"
                } else {
                    "2"
                };
                ensure!(
                    matches!(
                        profile.core.as_str(),
                        "vice_x64" | "vice_x64sc" | "vice_x128" | "vice_xplus4"
                    ) && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.core_options.get("vice_joyport").map(String::as_str)
                            == Some(port)
                        && profile
                            .core_options
                            .get("vice_read_vicerc")
                            .map(String::as_str)
                            == Some("disabled")
                        && !profile.content_extensions.is_empty()
                        && profile.content_extensions.iter().all(|extension| !matches!(
                            extension.as_str(),
                            "cmd" | "m3u" | "vfl" | "vsf" | "zip" | "7z" | "gz"
                        )),
                    "VICE joystick guard requires explicit fresh direct-content routing with vicerc disabled"
                );
            }
            if profile
                .content_guard
                .and_then(|guard| guard.stella_contract())
                .is_some()
            {
                ensure!(
                    profile.core == "stella"
                        && profile.target_layout == "atari2600-stella-panel"
                        && profile.retroarch_library.as_deref() == Some("Stella 2023")
                        && profile.explicit_selection
                        && profile.requires_fresh_start
                        && profile.core_options.is_empty()
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.device == 1
                                && launch.max_players == 2
                                && launch.player_topology.is_none())
                        && profile.frontend_port_count() == 2,
                    "Stella runtime guard requires an explicit fresh two-controller detection contract"
                );
            }
            ensure!(
                profile
                    .content_extensions
                    .iter()
                    .all(|extension| !extension.is_empty()
                        && extension.len() <= 16
                        && extension
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())),
                "Invalid controller profile content extension"
            );
            ensure!(
                !profile.explicit_selection
                    || profile
                        .retroarch_launch
                        .as_ref()
                        .is_some_and(|launch| launch.player_topology.is_none()),
                "Explicit controller modes must declare a fixed topology"
            );
            ensure!(
                profile.frontend_ports.is_none() || profile.retroarch_launch.is_some(),
                "Frontend port count requires a launch contract"
            );
            ensure!(
                !profile.requires_fresh_start || profile.retroarch_launch.is_some(),
                "Fresh-start guard requires a launch contract"
            );
            ensure!(
                profile.port_devices.is_empty()
                    || (profile.explicit_selection
                        && profile.retroarch_launch.as_ref().is_some_and(|launch| {
                            launch.player_topology.is_none()
                                && profile.port_devices.iter().all(|(port, device)| {
                                    (1..=profile.frontend_port_count()).contains(port)
                                        && *device <= u16::MAX.into()
                                        && (*device & 0xff == 1
                                            || (*port > launch.max_players && *device & 0xff == 3))
                                })
                        })),
                "Per-port attachments require an explicit fixed topology; native keyboard pass-through must be outside player slots"
            );
            ensure!(
                profile.port_controls.iter().all(|(port, controls)| {
                    profile
                        .retroarch_launch
                        .as_ref()
                        .is_some_and(|launch| (1..=launch.max_players).contains(port))
                        && !controls.is_empty()
                        && controls.iter().all(|id| profile.bindings.contains_key(id))
                }),
                "Invalid per-port controller control subset"
            );
            for (port, layout_id) in &profile.port_layouts {
                ensure!(
                    profile.explicit_selection
                        && profile
                            .retroarch_launch
                            .as_ref()
                            .is_some_and(|launch| launch.player_topology.is_none()
                                && (1..=launch.max_players).contains(port)),
                    "Per-port target layouts require an explicit fixed topology"
                );
                let layout = self
                    .layout(layout_id)
                    .context("Unknown per-port target layout")?;
                let resolved = profile.for_port(*port);
                ensure!(
                    resolved
                        .bindings
                        .keys()
                        .all(|id| layout.controls.iter().any(|control| control.id == *id))
                        && layout
                            .controls
                            .iter()
                            .filter(|control| !control.optional && control.repeat_of.is_none())
                            .all(|control| resolved.bindings.contains_key(&control.id)),
                    "Per-port target layout and active bindings disagree"
                );
            }
            for (port, bindings) in &profile.port_bindings {
                ensure!(
                    profile.transport == "retropad"
                        && profile.port_layouts.contains_key(port)
                        && !profile.port_controls.contains_key(port)
                        && !bindings.is_empty(),
                    "Per-port binding replacement requires an explicit RetroPad layout and cannot also use a subset"
                );
                let mut outputs = HashSet::new();
                ensure!(
                    bindings
                        .values()
                        .all(|output| crate::settings::CONTROLLER_GAMEPAD_BUTTONS
                            .contains(&output.as_str())
                            && outputs.insert(output)),
                    "Unknown or conflicting per-port output bindings"
                );
            }
            ensure!(
                profiles.insert(&profile.id),
                "duplicate emulator profile ID"
            );
            let layout = self
                .layout(&profile.target_layout)
                .ok_or_else(|| anyhow::anyhow!("unknown target layout"))?;
            ensure!(
                profile.status == "documented"
                    && matches!(
                        profile.transport.as_str(),
                        "retropad"
                            | "ares-settings"
                            | "duckstation-settings"
                            | "ppsspp-settings"
                            | "mgba-settings"
                            | "snes9x-gtk-settings"
                            | "fceux-qt-settings"
                            | "sameboy-sdl-settings"
                            | "mednafen-settings"
                            | "dolphin-settings"
                            | "pcsx2-native-settings"
                            | "rpcs3-native-settings"
                            | "melonds-native-settings"
                            | "flycast-native-settings"
                            | "mame-native-settings"
                            | "bsnes-native-settings"
                            | "stella-native-settings"
                            | "vice-native-settings"
                            | "hatari-native-settings"
                            | "desmume-native-settings"
                            | "openmsx-native-settings"
                            | "mesen2-native-settings"
                            | "blastem-native-settings"
                            | "xemu-native-settings"
                            | "scummvm-native-settings"
                            | "jgenesis-native-settings"
                            | "gopher64-native-settings"
                            | "rmg-native-settings"
                            | "simple64-native-settings"
                            | "bizhawk-native-settings"
                            | "yaba-sanshiro-native-settings"
                    ),
                "unsupported profile contract"
            );
            ensure!(
                profile.source.starts_with("https://") && !profile.conditions.is_empty(),
                "profile lacks provenance or assumptions"
            );
            if let Some(native) = &profile.native_launch {
                ensure!(
                    profile.transport != "retropad"
                        && profile.retroarch_launch.is_none()
                        && !native.platforms.is_empty()
                        && (1..=16).contains(&native.max_players),
                    "Invalid native launch metadata"
                );
            }
            if profile.transport == "duckstation-settings" {
                ensure!(
                    profile.core == "duckstation"
                        && matches!(
                            profile.target_layout.as_str(),
                            "playstation-digital" | "dualshock"
                        ),
                    "unreviewed DuckStation input mode"
                );
            }
            if profile.transport.ends_with("-native-settings") {
                crate::controller_native_targets::validate(profile)?;
            }
            if profile.transport == "ppsspp-settings" {
                ensure!(
                    profile.core == "ppsspp"
                        && profile.target_layout == "psp"
                        && profile.retroarch_launch.is_none(),
                    "Standalone PPSSPP profile must not be dispatched through RetroArch"
                );
            }
            if profile.transport == "mgba-settings" {
                ensure!(
                    profile.core == "mgba"
                        && matches!(profile.target_layout.as_str(), "gba" | "gameboy")
                        && profile.retroarch_launch.is_none(),
                    "Native mGBA profile must not use RetroArch dispatch"
                );
            }
            if profile.transport == "snes9x-gtk-settings" {
                ensure!(
                    profile.core == "snes9x"
                        && profile.target_layout == "snes"
                        && profile.retroarch_launch.is_none(),
                    "Native Snes9x GTK profile cannot use RetroArch dispatch"
                );
            }
            if profile.transport == "fceux-qt-settings" {
                ensure!(
                    profile.core == "fceux"
                        && profile.target_layout == "nes"
                        && profile.retroarch_launch.is_none(),
                    "Native FCEUX Qt profile cannot use RetroArch dispatch"
                );
            }
            if profile.transport == "sameboy-sdl-settings" {
                ensure!(
                    profile.core == "sameboy"
                        && profile.target_layout == "gameboy"
                        && profile.retroarch_launch.is_none(),
                    "Native SameBoy SDL profile cannot use RetroArch dispatch"
                );
            }
            if profile.transport == "mednafen-settings" {
                ensure!(
                    profile.core == "mednafen"
                        && matches!(
                            profile.target_layout.as_str(),
                            "gameboy"
                                | "gba"
                                | "lynx"
                                | "ngp"
                                | "wonderswan"
                                | "virtualboy"
                                | "gamegear"
                                | "mednafen-master-system"
                                | "snes"
                                | "saturn-digital"
                                | "playstation-digital"
                                | "dualshock"
                                | "genesis-3"
                                | "genesis-6"
                                | "pce-2"
                                | "pce-6"
                                | "nes"
                        )
                        && profile.retroarch_launch.is_none(),
                    "Native Mednafen profile cannot use RetroArch dispatch"
                );
            }
            if profile.transport == "dolphin-settings" {
                ensure!(
                    profile.core == "dolphin"
                        && profile.target_layout == "dolphin-native-gamecube"
                        && profile.retroarch_launch.is_none(),
                    "Native Dolphin GameCube mapping cannot use RetroArch dispatch"
                );
            }
            if let Some(launch) = &profile.retroarch_launch {
                if profile.content_guard == Some(ContentGuard::GeolithStandardCartridge) {
                    ensure!(
                        profile.core == "geolith"
                            && profile.explicit_selection
                            && profile.content_extensions.len() == 1
                            && profile.content_extensions.contains("neo")
                            && profile
                                .core_options
                                .get("geolith_4player")
                                .map(String::as_str)
                                == Some("off")
                            && matches!(
                                profile
                                    .core_options
                                    .get("geolith_system_type")
                                    .map(String::as_str),
                                Some("aes" | "mvs")
                            ),
                        "Geolith cartridge guard requires an explicit standard AES/MVS NEO mode with four-player disabled"
                    );
                }
                ensure!(
                    (launch.max_players..=16).contains(&profile.frontend_port_count()),
                    "Frontend port count cannot be smaller than the mode's active player count"
                );
                if let Some(library) = &profile.retroarch_library {
                    ensure!(
                        !library.is_empty()
                            && library != "."
                            && library != ".."
                            && !library.chars().any(|c| c.is_control() || "/\\".contains(c)),
                        "invalid RetroArch library directory name"
                    );
                }
                ensure!(
                    profile.transport == "retropad",
                    "non-RetroPad profile cannot use RetroArch launch adapter"
                );
                ensure!(valid_id(&profile.core), "invalid launched core ID");
                ensure!(
                    !launch.platforms.is_empty() && (1..=16).contains(&launch.max_players),
                    "launch contract requires platform aliases and 1-16 player ports"
                );
                if let Some(topology) = &launch.player_topology {
                    ensure!(
                        valid_id(&topology.option)
                            && profile.retroarch_library.is_some()
                            && !profile.core_options.contains_key(&topology.option)
                            && topology.values.contains_key(&topology.default)
                            && topology.values.values().copied().max() == Some(launch.max_players)
                            && topology.values.iter().all(|(value, ports)| {
                                !value.is_empty()
                                    && !value.chars().any(|c| c.is_control() || "\"\\".contains(c))
                                    && (1..=launch.max_players).contains(ports)
                            }),
                        "invalid option-dependent controller topology"
                    );
                }
                ensure!(
                    (launch.device & 0xff == 1
                        || (profile.core == "fbneo"
                            && matches!(
                                profile.target_layout.as_str(),
                                "arcade-six-button" | "arcade-eight-button"
                            )
                            && launch.device == 261)
                        || (profile.core == "puae"
                            && profile.target_layout == "puae-cd32"
                            && profile.content_guard == Some(ContentGuard::PuaeFixedInput)
                            && launch.device == 517)
                        || (profile.target_layout == "dualshock"
                            && matches!(
                                (profile.core.as_str(), launch.device),
                                ("swanstation", 261)
                                    | ("mednafen_psx" | "mednafen_psx_hw" | "pcsx_rearmed", 517)
                            )))
                        && launch.device <= u16::MAX.into(),
                    "launch writer requires a reviewed joypad or analog device mode"
                );
                for alias in &launch.platforms {
                    ensure!(
                        !alias.is_empty()
                            && alias.trim() == alias
                            && !alias.chars().any(char::is_control),
                        "invalid launch platform alias"
                    );
                    ensure!(
                        profile.explicit_selection
                            || launch_targets.insert((
                                profile.core.clone(),
                                alias.to_ascii_lowercase(),
                                launch.device
                            )),
                        "duplicate or ambiguous core/platform launch contract"
                    );
                }
            }
            if profile.core == "dolphin"
                && profile.target_layout == "dolphin-gamecube-mixed-triggers"
            {
                crate::controller_dolphin::validate_gamecube_options(&profile.core_options)?;
                ensure!(
                    profile.explicit_selection,
                    "Dolphin mixed-trigger mode requires explicit selection"
                );
            }
            let mut outputs = HashSet::new();
            if profile.core == "same_cdi" {
                crate::controller_same_cdi::mode_for_layout(&profile.target_layout)?;
                crate::controller_same_cdi::validate_options(&profile.core_options)?;
                ensure!(
                    profile.explicit_selection,
                    "SAME CD-i pointer mode requires explicit selection"
                );
                ensure!(
                    profile.retroarch_library.as_deref() == Some("SAME_CDI")
                        && profile.requires_fresh_start
                        && profile.frontend_ports == Some(6),
                    "SAME CD-i pointer mode requires native library identity, fresh start and six frontend ports"
                );
                if let Some(launch) = &profile.retroarch_launch {
                    ensure!(
                        launch.device == 1
                            && launch.max_players == 1
                            && launch.player_topology.is_none()
                            && profile.port_devices.is_empty(),
                        "SAME CD-i pointer modes use one calibrated RetroPad and clear the other native input ports"
                    );
                }
            }
            for (target, output) in &profile.bindings {
                ensure!(
                    layout.controls.iter().any(|control| control.id == *target),
                    "unknown target control"
                );
                let known =
                    match profile.transport.as_str() {
                        "ares-settings" => crate::controller_ares::valid_output(profile, output),
                        "pcsx2-native-settings"
                        | "rpcs3-native-settings"
                        | "melonds-native-settings"
                        | "flycast-native-settings"
                        | "mame-native-settings"
                        | "bizhawk-native-settings"
                        | "bsnes-native-settings"
                        | "stella-native-settings"
                        | "vice-native-settings"
                        | "hatari-native-settings"
                        | "desmume-native-settings"
                        | "openmsx-native-settings"
                        | "mesen2-native-settings"
                        | "blastem-native-settings"
                        | "xemu-native-settings"
                        | "scummvm-native-settings"
                        | "jgenesis-native-settings"
                        | "gopher64-native-settings"
                        | "rmg-native-settings"
                        | "simple64-native-settings"
                        | "yaba-sanshiro-native-settings" => {
                            crate::controller_native_targets::valid_output(profile, target, output)
                        }
                        "retropad" => {
                            crate::settings::CONTROLLER_GAMEPAD_BUTTONS.contains(&output.as_str())
                        }
                        "duckstation-settings" => {
                            [
                                "Up", "Down", "Left", "Right", "Start", "Select", "Cross",
                                "Circle", "Square", "Triangle", "L1", "R1", "L2", "R2",
                            ]
                            .contains(&output.as_str())
                                || (profile.target_layout == "dualshock"
                                    && [
                                        "L3", "R3", "LLeft", "LRight", "LUp", "LDown", "RLeft",
                                        "RRight", "RUp", "RDown",
                                    ]
                                    .contains(&output.as_str()))
                        }
                        "ppsspp-settings" => {
                            crate::controller_ppsspp::GAMEPLAY_KEYS.contains(&output.as_str())
                        }
                        "mgba-settings" => {
                            crate::controller_mgba::KEYS.contains(&output.as_str())
                                && (profile.target_layout == "gba"
                                    || !matches!(output.as_str(), "L" | "R"))
                        }
                        "snes9x-gtk-settings" => crate::controller_snes9x::configuration::CONTROLS
                            .contains(&output.as_str()),
                        "dolphin-settings" => crate::controller_dolphin::standalone::CONTROLS
                            .contains(&output.as_str()),
                        "fceux-qt-settings" => {
                            crate::controller_fceux::profile::CONTROLS.contains(&output.as_str())
                        }
                        "sameboy-sdl-settings" => {
                            crate::controller_sameboy::CONTROLS.contains(&output.as_str())
                        }
                        "mednafen-settings" => {
                            use crate::controller_mednafen::profiles::Gamepad;
                            let gamepad = match profile.target_layout.as_str() {
                                "gba" => Gamepad::GameBoyAdvance,
                                "lynx" => Gamepad::Lynx,
                                "ngp" => Gamepad::NeoGeoPocket,
                                "wonderswan" => Gamepad::WonderSwan,
                                "virtualboy" => Gamepad::VirtualBoy,
                                "gamegear" => Gamepad::GameGear,
                                "mednafen-master-system" => Gamepad::MasterSystem,
                                "pce-2" => Gamepad::PceTwo,
                                "pce-6" => Gamepad::PceSix,
                                "nes" => Gamepad::NesTwo,
                                "snes" => Gamepad::Snes,
                                "saturn-digital" => Gamepad::SaturnDigital,
                                "playstation-digital" => Gamepad::PlayStationDigital,
                                "dualshock" => Gamepad::PlayStationDualAnalog,
                                "genesis-3" => Gamepad::MdThree,
                                "genesis-6" => Gamepad::MdSix,
                                _ => Gamepad::GameBoy,
                            };
                            gamepad.controls().contains(&output.as_str())
                        }
                        _ => false,
                    };
                ensure!(known, "unknown output control for this transport");
                if profile.transport == "dolphin-settings" {
                    ensure!(
                        layout
                            .controls
                            .iter()
                            .find(|control| control.id == *target)
                            .unwrap()
                            .analog
                            == crate::controller_dolphin::standalone::analog_target(output),
                        "Dolphin native analog setting differs from the visual target capability"
                    );
                }
                if profile.transport == "ppsspp-settings" {
                    ensure!(
                        layout
                            .controls
                            .iter()
                            .find(|control| control.id == *target)
                            .unwrap()
                            .analog
                            == output.starts_with("An."),
                        "PPSSPP analog setting does not match the target control capability"
                    );
                }
                if profile.transport == "duckstation-settings" {
                    let analog_output = [
                        "LLeft", "LRight", "LUp", "LDown", "RLeft", "RRight", "RUp", "RDown",
                    ]
                    .contains(&output.as_str());
                    ensure!(
                        layout
                            .controls
                            .iter()
                            .find(|c| c.id == *target)
                            .unwrap()
                            .analog
                            == analog_output,
                        "DuckStation control and output disagree on analog capability"
                    );
                }
                let mixed_trigger_pair = profile.transport == "retropad"
                    && profile.core == "dolphin"
                    && profile.target_layout == "dolphin-gamecube-mixed-triggers"
                    && match output.as_str() {
                        "LeftTrigger" | "RightTrigger" => {
                            let (pressure, fallback) = if output == "LeftTrigger" {
                                ("trigger_left", "l2")
                            } else {
                                ("trigger_right", "r2")
                            };
                            let assigned: HashSet<_> = profile
                                .bindings
                                .iter()
                                .filter(|(_, value)| *value == output)
                                .map(|(id, _)| id.as_str())
                                .collect();
                            assigned == HashSet::from([pressure, fallback])
                                && layout.controls.iter().any(|c| {
                                    c.id == pressure
                                        && c.analog
                                        && c.group == "shoulder"
                                        && !c.optional
                                })
                                && layout
                                    .controls
                                    .iter()
                                    .any(|c| c.id == fallback && !c.analog && c.optional)
                        }
                        _ => false,
                    };
                ensure!(
                    outputs.insert(output) || mixed_trigger_pair,
                    "conflicting output controls"
                );
            }
            for control in layout.controls.iter().filter(|control| !control.optional) {
                // Native PCE saves the two/six-button mode as a device setting
                // and deliberately clears the mode-toggle input binding.
                let fixed_native_pce_mode = control.id == "mode"
                    && layout.id == "pce-6"
                    && profile.transport == "mednafen-settings"
                    && matches!(
                        profile.id.as_str(),
                        "mednafen:standalone-pce6" | "mednafen:standalone-pce-fast6"
                    );
                ensure!(
                    profile.bindings.contains_key(&control.id) || fixed_native_pce_mode,
                    "incomplete emulator contract: {} lacks {}",
                    profile.id,
                    control.id
                );
            }
        }
        Ok(())
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

impl Calibration {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.target_mappings.len() <= catalog().emulator_profiles.len(),
            "Too many target mappings"
        );
        for (profile_id, choices) in &self.target_mappings {
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|p| p.id == *profile_id)
                .context("Saved mapping references an unknown target")?;
            let mut used = HashSet::new();
            for (target, source) in choices {
                ensure!(
                    profile.bindings.contains_key(target),
                    "Unknown saved target control"
                );
                ensure!(
                    self.bindings.contains_key(source),
                    "Saved choice requires an unrecorded control"
                );
                ensure!(used.insert(source), "Saved choices reuse a physical input");
            }
        }
        let layout = catalog()
            .layout(&self.layout)
            .ok_or_else(|| anyhow::anyhow!("Unknown controller layout"))?;
        ensure!(
            matches!(self.os.as_str(), "linux" | "windows" | "macos"),
            "Unsupported calibration OS"
        );
        ensure!(
            matches!(self.backend.as_str(), "gilrs-0.11" | "sdl3-gamepad"),
            "Unsupported calibration input backend"
        );
        ensure!(
            !self.bindings.is_empty() && self.bindings.len() <= layout.controls.len(),
            "Record at least one control"
        );
        let mut inputs = HashSet::new();
        let mut native_inputs = HashSet::new();
        for (id, input) in &self.bindings {
            ensure!(
                crate::controller_sdl3::is_binding(input) == (self.backend == "sdl3-gamepad"),
                "Mixed SDL3 and GilRs calibration inputs"
            );
            if self.backend == "sdl3-gamepad" {
                ensure!(
                    crate::controller_sdl3::valid_binding(input),
                    "Invalid SDL3 logical control"
                );
                ensure!(
                    input.native.is_none() && input.axis.is_none(),
                    "SDL3 input cannot be treated as measured evdev input"
                );
            }
            let control = layout
                .controls
                .iter()
                .find(|control| control.id == *id)
                .ok_or_else(|| anyhow::anyhow!("Unknown control {id}"))?;
            ensure!(
                matches!(input.kind.as_str(), "button" | "axis"),
                "Unsupported input type"
            );
            ensure!(
                (input.kind == "button" && input.direction == 0)
                    || (input.kind == "axis" && matches!(input.direction, -1 | 1)),
                "Invalid axis direction"
            );
            ensure!(
                !control.analog || input.kind == "axis",
                "{} needs an analog axis",
                control.label
            );
            ensure!(
                !input.logical.is_empty() && input.logical.len() <= 100,
                "Invalid logical input label"
            );
            ensure!(
                inputs.insert((input.code, input.kind.as_str(), input.direction)),
                "One input was assigned to multiple controls; skip duplicate hardware buttons"
            );
            if let Some(native) = &input.native {
                ensure!(self.os == "linux", "Physical evdev bindings require Linux");
                ensure!(
                    (native.code >> 16 == 1 && native.direction == 0)
                        || (native.code >> 16 == 3 && matches!(native.direction, -1 | 1)),
                    "Invalid physical controller input"
                );
                ensure!(native_inputs.insert(native), "Duplicate physical input");
            }
            if let Some(axis) = &input.axis {
                axis.validate()?;
                ensure!(
                    input.native.as_ref().is_some_and(
                        |native| native.code >> 16 == 3 && native.direction == axis.direction()
                    ),
                    "Measured axis does not match the physical input direction"
                );
            }
            // Pressure measurement is an evdev gesture; other OSes have no
            // measured release/full-press source wired to this validation yet.
            if control.is_pressure() && self.os == "linux" {
                let measured = input.axis.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "{} requires measured release and full-press values; recalibrate this pressure control",
                        control.label
                    )
                })?;
                crate::controller_axis::PressureAxis::from_measurement(measured)?;
            }
        }
        Ok(())
    }

    pub fn plan(&self, profile_id: &str) -> Result<MappingPlan> {
        let db = catalog();
        let profile = db
            .emulator_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown emulator profile"))?;
        self.plan_profile(profile)
    }

    pub(crate) fn plan_profile(&self, profile: &EmulatorProfile) -> Result<MappingPlan> {
        self.validate()?;
        let db = catalog();
        let mut measured_source = db.layout(&self.layout).unwrap().clone();
        let target = db.layout(&profile.target_layout).unwrap();
        let mut warnings = profile.conditions.clone();
        // Catalog buttons describe a layout, not the capabilities of every
        // device sold in that shape. Promote only the requested pressure role
        // when this calibration includes a real proportional evdev gesture.
        // Keep digital-target planning unchanged (including N64 Z presets).
        for control in &mut measured_source.controls {
            if control.analog {
                continue;
            }
            let Some(requested) = target.controls.iter().find(|requested| {
                (requested.id == control.id
                    || (crate::controller_layout::pressure_role(&control.id).is_some()
                        && crate::controller_layout::pressure_role(&control.id)
                            == crate::controller_layout::pressure_role(&requested.id)))
                    && (requested.group == control.group
                        || (requested.group == "shoulder" && control.group == "rear"))
                    && requested.is_pressure()
            }) else {
                continue;
            };
            let Some(binding) = self.bindings.get(&control.id) else {
                continue;
            };
            let Some(measured) = binding.axis.as_ref() else {
                continue;
            };
            if binding
                .native
                .as_ref()
                .is_some_and(|native| native.code >> 16 == 3)
                && crate::controller_axis::PressureAxis::from_measurement(measured).is_ok()
            {
                control.analog = true;
                control.pressure = true;
                control.group = requested.group.clone();
                warnings.push(format!(
                    "{} supplies measured proportional pressure for this profile; launch requires a session-local normalized gamepad and writable /dev/uinput.",
                    control.label
                ));
            }
        }
        let source = &measured_source;
        let adapter = (self.backend != "sdl3-gamepad" || profile.transport == "ares-settings")
            && crate::controller_launch::supports_profile(profile);
        if self.backend == "sdl3-gamepad" && profile.transport != "ares-settings" {
            warnings.push("SDL3 native input is ready for setup and mapping preview. This target still requires an SDL3-aware launch transport; SDL logical codes are not evdev codes.".into());
        }
        warnings.push(if profile.transport == "ares-settings" {
            "ares 148+: saved player assignments and button choices are applied to a private settings file. The emulator's SDL device identities are checked at launch."
        } else if profile.transport == "mgba-settings" {
            "Native Linux mGBA SDL discovers runtime paths at launch and applies guided Player 1. An existing config.ini and physical calibration are required; this plan does not establish runtime readiness."
        } else if crate::controller_guided_native::supports(profile) {
            "Guided players and target choices are connected to this native adapter. A matching native runtime setup and physical calibration are still required; the preview does not establish runtime readiness."
        } else if adapter {
            "Native Linux RetroArch launch adapter available. Physical bindings and connected-device numbering are checked again at launch; automatic RetroArch remaps/overrides are suspended for that session."
        } else {
            "Preview only: an automatic launch adapter for this contract is not implemented."
        }.into());
        if profile.explicit_selection {
            warnings.push("This input/topology mode must be explicitly selected in Controller setup or Controller coverage; connected pads do not select it automatically.".into());
        }
        if self.os != std::env::consts::OS {
            warnings.push(
                "This calibration was recorded on another OS; recalibrate before use.".into(),
            );
        }
        if source.family != target.family {
            warnings.push(
                "Cross-layout conversion is a suggested preset; review the physical assignments."
                    .into(),
            );
        }
        let resolution = crate::controller_layout::resolve_with_choices(
            source,
            target,
            &self.bindings.keys().map(String::as_str).collect(),
            &profile.bindings.keys().map(String::as_str).collect(),
            self.target_mappings
                .get(&profile.id)
                .unwrap_or(&BTreeMap::new()),
        )?;
        let rows: Vec<MappingRow> = profile
            .bindings
            .iter()
            .map(|(target_id, output)| {
                let physical = resolution
                    .assignments
                    .get(target_id)
                    .and_then(|source_id| source.controls.iter().find(|c| c.id == *source_id));
                let input = physical.and_then(|c| self.bindings.get(&c.id)).cloned();
                if input.is_none() {
                    warnings.push(format!(
                        "Missing physical input for {target_id}: {}",
                        resolution.missing[target_id].description()
                    ));
                }
                if let (Some(physical), Some(rule)) = (physical, resolution.rules.get(target_id)) {
                    if matches!(
                        rule,
                        crate::controller_layout::Rule::FacePosition
                            | crate::controller_layout::Rule::SameHandShoulder
                            | crate::controller_layout::Rule::DigitalOverflow
                    ) {
                        warnings.push(format!(
                            "{target_id} uses {}: {}",
                            physical.label,
                            rule.description()
                        ));
                    }
                }
                MappingRow {
                    target_id: target_id.clone(),
                    physical_id: physical.map(|c| c.id.clone()),
                    target: target
                        .controls
                        .iter()
                        .find(|c| c.id == *target_id)
                        .unwrap()
                        .label
                        .clone(),
                    physical: physical
                        .map(|c| c.label.clone())
                        .unwrap_or_else(|| "Not available".into()),
                    input,
                    output: output.clone(),
                    reason: resolution
                        .rules
                        .get(target_id)
                        .map(|rule| rule.description())
                        .unwrap_or_else(|| resolution.missing[target_id].description())
                        .to_string(),
                }
            })
            .collect();
        if profile.core == "mame"
            && matches!(
                profile.target_layout.as_str(),
                "mame-twin-digital" | "mame-twin-digital-analog"
            )
        {
            // Each row is an observed native input. Optional catalog controls
            // describe absent actions, not permission to omit an observed one.
            for (index, row) in rows.iter().enumerate() {
                let native = row
                    .input
                    .as_ref()
                    .and_then(|input| input.native.as_ref())
                    .with_context(|| {
                        format!(
                            "MAME twin-stick input {} needs a physical calibration",
                            row.target
                        )
                    })?;
                for previous in &rows[..index] {
                    let Some(other) = previous
                        .input
                        .as_ref()
                        .and_then(|input| input.native.as_ref())
                    else {
                        continue;
                    };
                    if native.code != other.code {
                        continue;
                    }
                    let opposite = matches!(
                        (previous.target_id.as_str(), row.target_id.as_str()),
                        ("up", "down")
                            | ("down", "up")
                            | ("left", "right")
                            | ("right", "left")
                            | ("right_up", "right_down")
                            | ("right_down", "right_up")
                            | ("right_left", "right_right")
                            | ("right_right", "right_left")
                    );
                    ensure!(
                        native.code >> 16 == 3
                            && opposite
                            && native.direction != 0
                            && native.direction == -other.direction,
                        "MAME twin-stick controls {} and {} share a physical input; calibrate independent controls",
                        previous.target,
                        row.target
                    );
                }
            }
        }
        let automatic_launch_ready = adapter
            && (self.os == "linux"
                || (profile.transport == "ares-settings" && self.backend == "sdl3-gamepad"))
            && self.os == std::env::consts::OS
            && rows.iter().all(|row| {
                row.input.as_ref().is_some_and(|input| {
                    input.native.is_some()
                        || (profile.transport == "ares-settings"
                            && crate::controller_sdl3::valid_binding(input))
                }) || target
                    .controls
                    .iter()
                    .any(|c| c.id == row.target_id && c.optional)
            });
        Ok(MappingPlan {
            mapping_policy_version: resolution.policy_version,
            profile: profile.name.clone(),
            transport: profile.transport.clone(),
            status: profile.status.clone(),
            automatic_launch_ready,
            native_runtime_required: profile.transport != "ares-settings"
                && crate::controller_guided_native::supports(profile),
            rows,
            warnings,
        })
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Original vector schematics, generated from the same IDs as calibration.
pub fn svg(layout: &Layout, active: &str) -> String {
    let outline = match layout.shape.as_str() {
        "grid" => "M45 25H855Q875 25 875 45V430Q875 450 855 450H45Q25 450 25 430V45Q25 25 45 25Z",
        "rectangle" => {
            "M70 60H830Q850 60 850 85V365Q850 390 820 390H80Q50 390 50 360V90Q50 60 70 60Z"
        }
        "handheld" => "M70 40H830V355Q830 420 760 420H70Z",
        "three-grip" => {
            "M90 80Q70 45 190 50L710 50Q830 45 820 120L845 395Q840 455 775 400L650 270L550 300L540 430Q500 485 460 430L445 300L310 270L160 405Q75 455 75 390Z"
        }
        _ => {
            "M110 80Q140 40 280 70L620 70Q760 40 790 80L855 365Q860 440 790 400L650 295H250L110 400Q40 435 45 365Z"
        }
    };
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 900 500\"><title>{}</title><rect width=\"900\" height=\"500\" rx=\"24\" fill=\"#101822\"/><path d=\"{outline}\" fill=\"#273646\" stroke=\"#54697e\" stroke-width=\"3\"/>",
        xml(&layout.name)
    );
    for control in &layout.controls {
        let x = control.x * 8.0 + 50.0;
        let y = control.y * 4.0 + 35.0;
        let lit = control.id == active;
        let fill = if lit {
            "#ffb454"
        } else if control.id.starts_with("c_") {
            "#776529"
        } else {
            "#18232f"
        };
        let stroke = if lit { "#fff1d5" } else { "#8395a7" };
        let radius = if control.group == "dpad" || control.group == "stick" {
            15
        } else {
            25
        };
        let label = if control.group == "dpad" || control.analog {
            if control.id.ends_with("up") {
                "↑"
            } else if control.id.ends_with("down") {
                "↓"
            } else if control.id.ends_with("left") {
                "←"
            } else if control.id.ends_with("right") {
                "→"
            } else {
                &control.label
            }
        } else {
            &control.label
        };
        let _ = write!(
            svg,
            "<g id=\"{}\"><title>{}</title><circle cx=\"{x}\" cy=\"{y}\" r=\"{radius}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{}\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"{}\" fill=\"{}\">{}</text></g>",
            xml(&control.id),
            xml(&control.label),
            if lit { 4 } else { 2 },
            y + 5.0,
            if label.len() > 7 { 11 } else { 15 },
            if lit { "#16212b" } else { "#f0f4f8" },
            xml(label)
        );
    }
    let caption = if layout.id.starts_with("mame-")
        || matches!(
            layout.id.as_str(),
            "arcade-six-button-analog" | "arcade-eight-button-analog" | "neogeo-analog"
        ) {
        "Logical frontend channels · not cabinet geometry or physical button placement"
    } else {
        "Front view · rear controls shown at bottom · schematic, not to scale"
    };
    let _ = write!(
        svg,
        "<text x=\"450\" y=\"477\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"15\" fill=\"#a9bbca\">{}</text></svg>",
        xml(caption)
    );
    svg
}

pub fn export_svg(directory: &std::path::Path) -> Result<()> {
    if directory.as_os_str().is_empty() {
        bail!("Specify an export directory");
    }
    std::fs::create_dir_all(directory)?;
    for layout in &catalog().layouts {
        let path = directory.join(format!("{}.svg", layout.id));
        let contents = svg(layout, "");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(&contents) {
            std::fs::write(path, contents)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duckstation_preview_uses_configuration_keys_and_stable_control_ids() {
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "duckstation:digital-controller")
            .unwrap();
        assert!(!crate::controller_launch::supports_profile(profile));
        assert_eq!(profile.bindings.len(), 14);
        let plan = calibration("dualshock").plan(&profile.id).unwrap();
        assert_eq!(plan.transport, "duckstation-settings");
        assert!(!plan.automatic_launch_ready);
        assert!(plan.rows.iter().all(|row| row.input.is_some()));
        let cross = plan.rows.iter().find(|row| row.target_id == "b").unwrap();
        assert_eq!(cross.output, "Cross");
        assert_eq!(cross.physical_id.as_deref(), Some("b"));
        let digital = catalog().layout("playstation-digital").unwrap();
        assert_eq!(digital.controls.len(), 14);
        assert!(digital.controls.iter().all(|control| !control.analog));

        let mut brawler = calibration("brawler64");
        let plan = brawler.plan(&profile.id).unwrap();
        let control = |id: &str| plan.rows.iter().find(|r| r.target_id == id).unwrap();
        assert_eq!(control("b").physical_id.as_deref(), Some("a"));
        assert_eq!(control("y").physical_id.as_deref(), Some("b"));
        assert_eq!(control("l2").physical_id.as_deref(), Some("z"));
        assert_eq!(control("r2").physical_id.as_deref(), Some("z_right"));
        brawler.bindings.remove("z_right");
        let partial = brawler.plan(&profile.id).unwrap();
        let r2 = partial.rows.iter().find(|r| r.target_id == "r2").unwrap();
        assert!(r2.input.is_some(), "an unused C button can supply R2");
        assert_ne!(r2.physical_id.as_deref(), Some("z_right"));
        assert!(r2.reason.contains("Spare gameplay button"));
        let standard_n64 = calibration("n64").plan(&profile.id).unwrap();
        for id in ["r2", "select"] {
            assert!(
                standard_n64
                    .rows
                    .iter()
                    .find(|r| r.target_id == id)
                    .unwrap()
                    .input
                    .is_some()
            );
        }
    }

    #[test]
    fn duckstation_analog_preview_respects_each_physical_layouts_capabilities() {
        let profile = catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "duckstation:analog-controller")
            .unwrap();
        assert_eq!(profile.bindings.len(), 24);
        assert!(!crate::controller_launch::supports_profile(profile));
        for source in ["dualshock", "xbox"] {
            let plan = calibration(source).plan(&profile.id).unwrap();
            assert!(!plan.automatic_launch_ready);
            assert!(plan.rows.iter().all(|row| row.input.is_some()));
            for (target, output) in [("stick_up", "LUp"), ("right_stick_right", "RRight")] {
                let row = plan.rows.iter().find(|r| r.target_id == target).unwrap();
                assert_eq!(row.output, output);
                assert_eq!(row.physical_id.as_deref(), Some(target));
                assert_eq!(row.input.as_ref().unwrap().kind, "axis");
            }
        }
        for source in ["brawler64", "n64", "horizontal-four", "n30-turbo"] {
            let plan = calibration(source).plan(&profile.id).unwrap();
            for row in plan
                .rows
                .iter()
                .filter(|r| r.target_id.starts_with("right_stick_"))
            {
                assert!(
                    row.input.is_none(),
                    "{source} cannot provide {}",
                    row.target_id
                );
            }
        }
        let mut db = catalog().clone();
        let profile = db
            .emulator_profiles
            .iter_mut()
            .find(|p| p.id == "duckstation:analog-controller")
            .unwrap();
        // Both setting names exist, but swapping their physical roles is invalid.
        profile.bindings.insert("stick_up".into(), "Cross".into());
        profile.bindings.insert("b".into(), "LUp".into());
        assert!(db.validate().is_err());
    }

    #[test]
    fn option_topology_requires_bounded_values_default_and_unmodified_model() {
        let original = catalog().clone();
        let index = original
            .emulator_profiles
            .iter()
            .position(|p| p.id == "retroarch:sameboy:gameboy")
            .unwrap();
        for invalid in 0..8 {
            let mut db = original.clone();
            let profile = &mut db.emulator_profiles[index];
            let topology = profile
                .retroarch_launch
                .as_mut()
                .unwrap()
                .player_topology
                .as_mut()
                .unwrap();
            match invalid {
                0 => topology.default = "unknown".into(),
                1 => {
                    topology.values.insert("Auto".into(), 0);
                }
                2 => {
                    topology.values.insert("Auto".into(), 5);
                }
                3 => topology.option = "bad=key".into(),
                4 => {
                    topology.values.insert("bad\nvalue".into(), 1);
                }
                5 => topology.values.values_mut().for_each(|ports| *ports = 1),
                6 => profile.retroarch_library = None,
                7 => {
                    profile
                        .core_options
                        .insert("sameboy_model".into(), "Auto".into());
                }
                _ => unreachable!(),
            }
            assert!(db.validate().is_err(), "invalid topology case {invalid}");
        }
        original.validate().unwrap();
    }

    #[test]
    fn transports_cannot_mix_retropad_with_standalone_setting_names() {
        let mut db = catalog().clone();
        let index = db
            .emulator_profiles
            .iter()
            .position(|p| p.id == "duckstation:digital-controller")
            .unwrap();
        db.emulator_profiles[index]
            .bindings
            .insert("b".into(), "South".into());
        assert!(db.validate().is_err());
        db.emulator_profiles[index]
            .bindings
            .insert("b".into(), "Cross".into());
        db.emulator_profiles[index].retroarch_launch = Some(RetroArchLaunch {
            platforms: vec!["Sony Playstation".into()],
            device: 1,
            max_players: 2,
            player_topology: None,
        });
        assert!(db.validate().is_err());
        db.emulator_profiles[index].retroarch_launch = None;
        db.emulator_profiles[index].transport = "retropad".into();
        assert!(db.validate().is_err());
    }
    #[test]
    fn launch_catalog_requires_unambiguous_reviewed_device_modes() {
        let original = catalog().clone();
        let index = original
            .emulator_profiles
            .iter()
            .position(|p| p.id == "retroarch:snes9x:snes")
            .unwrap();
        let mut duplicate = original.clone();
        let mut profile = duplicate.emulator_profiles[index].clone();
        profile.id.push_str("-duplicate");
        duplicate.emulator_profiles.push(profile);
        assert!(duplicate.validate().is_err());
        assert!(duplicate.launch_profile("snes9x", "SNES").is_none());
        for max_players in [0, 17] {
            let mut bad = original.clone();
            bad.emulator_profiles[index]
                .retroarch_launch
                .as_mut()
                .unwrap()
                .max_players = max_players;
            assert!(bad.validate().is_err());
        }
        for device in [0, 2, 5, 65537] {
            let mut bad = original.clone();
            bad.emulator_profiles[index]
                .retroarch_launch
                .as_mut()
                .unwrap()
                .device = device;
            assert!(bad.validate().is_err());
        }
        for aliases in [
            vec![],
            vec![" SNES".into()],
            vec!["SNES".into(), "snes".into()],
        ] {
            let mut bad = original.clone();
            bad.emulator_profiles[index]
                .retroarch_launch
                .as_mut()
                .unwrap()
                .platforms = aliases;
            assert!(bad.validate().is_err());
        }
        assert_eq!(
            original.launch_profile("snes9x", "  sNeS ").unwrap().id,
            "retroarch:snes9x:snes"
        );
        assert!(original.launch_profile("snes9x", "SNES Mouse").is_none());
        assert!(original.launch_profile("snes9x2010", "SNES").is_none());
        assert!(original.launch_profile("bsnes", "Game Boy").is_none());
    }

    #[test]
    fn psx_mode_ids_are_core_specific_and_defaults_remain_unambiguous() {
        let db = catalog();
        let digital = db.launch_profile("swanstation", "PSX").unwrap();
        assert_eq!(digital.retroarch_launch.as_ref().unwrap().device, 1);
        let analog = db
            .launch_mode("swanstation", "Sony - PlayStation", 261)
            .unwrap();
        assert_eq!(analog.target_layout, "dualshock");
        assert_eq!(analog.retroarch_library.as_deref(), Some("SwanStation"));
        assert!(db.launch_mode("swanstation", "PSX", 517).is_none());
        let mut invalid = db.clone();
        invalid
            .emulator_profiles
            .iter_mut()
            .find(|profile| profile.id == analog.id)
            .unwrap()
            .retroarch_launch
            .as_mut()
            .unwrap()
            .device = 517;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn catalog_ids_contracts_and_svg_controls_are_consistent() {
        catalog().validate().unwrap();
        for layout in &catalog().layouts {
            let image = svg(layout, &layout.controls[0].id);
            for control in &layout.controls {
                assert!(image.contains(&format!("id=\"{}\"", control.id)));
            }
            assert!(image.contains("#ffb454"));
        }
    }
    fn calibration(layout: &str) -> Calibration {
        Calibration {
            target_mappings: BTreeMap::new(),
            layout: layout.into(),
            os: std::env::consts::OS.into(),
            backend: "gilrs-0.11".into(),
            bindings: catalog()
                .layout(layout)
                .unwrap()
                .controls
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    // Pressure controls measure a real evdev gesture on Linux.
                    let (native, axis) = if c.is_pressure() && std::env::consts::OS == "linux" {
                        (
                            Some(NativeInput {
                                code: 0x30000 + i as u32,
                                direction: 1,
                            }),
                            Some(crate::controller_axis::AxisMeasurement {
                                minimum: 0,
                                maximum: 255,
                                flat: 0,
                                fuzz: 0,
                                resolution: 0,
                                released: 0,
                                pressed: 255,
                            }),
                        )
                    } else {
                        (None, None)
                    };
                    (
                        c.id.clone(),
                        InputBinding {
                            code: i as u32,
                            kind: if c.analog { "axis" } else { "button" }.into(),
                            direction: if c.analog { 1 } else { 0 },
                            logical: c.label.clone(),
                            native,
                            axis,
                        },
                    )
                })
                .collect(),
        }
    }
    #[test]
    fn calibration_rejects_duplicate_inputs_and_keeps_missing_controls_explicit() {
        let mut cal = calibration("nes");
        cal.bindings.insert("b".into(), cal.bindings["a"].clone());
        assert!(cal.validate().is_err());
        cal.bindings.remove("b");
        let plan = cal.plan("retroarch:fceumm:nes").unwrap();
        assert!(
            plan.warnings
                .iter()
                .any(|w| w.contains("Missing physical input for b"))
        );
        assert!(!plan.automatic_launch_ready);
    }
    #[test]
    fn brawler_calibration_reuses_controls_without_inventing_sega_mode() {
        let cal = calibration("brawler64");
        let plan = cal.plan("retroarch:genesis_plus_gx:md6").unwrap();
        assert_eq!(
            plan.rows.iter().find(|r| r.target == "X").unwrap().physical,
            "B"
        );
        assert_eq!(
            plan.rows.iter().find(|r| r.target == "B").unwrap().physical,
            "C ↓"
        );
        assert!(!plan.automatic_launch_ready);
        assert!(plan.warnings.iter().any(|w| w.contains("suggested")));
    }
    #[test]
    fn diamond_two_button_profile_preserves_left_run_bottom_jump() {
        let cal = calibration("xbox");
        let plan = cal.plan("retroarch:fceumm:nes").unwrap();
        assert_eq!(
            plan.rows.iter().find(|r| r.target == "B").unwrap().physical,
            "X / West"
        );
        assert_eq!(
            plan.rows.iter().find(|r| r.target == "A").unwrap().physical,
            "A / South"
        );
    }
    #[test]
    fn partial_calibration_is_resolved_before_assignment_not_afterwards() {
        let mut cal = calibration("xbox");
        cal.bindings.remove("y");
        let plan = cal.plan("retroarch:fceumm:nes").unwrap();
        assert!(plan.rows.iter().all(|row| row.input.is_some()));
        assert!(
            plan.rows
                .iter()
                .all(|row| row.physical_id.as_deref() != Some("y"))
        );
        assert_eq!(
            plan.mapping_policy_version,
            crate::controller_layout::POLICY_VERSION
        );
        assert!(plan.rows.iter().all(|row| !row.reason.is_empty()));
        let primary = plan.rows.iter().find(|row| row.target_id == "a").unwrap();
        assert_eq!(primary.physical_id.as_deref(), Some("b"));
        assert!(plan.rows.iter().any(|row| row.reason.contains("position")));
    }

    #[test]
    fn every_source_composes_with_every_adapter_without_changing_the_contract() {
        let db = catalog();
        for source in &db.layouts {
            let cal = calibration(&source.id);
            for profile in &db.emulator_profiles {
                let plan = cal.plan(&profile.id).unwrap();
                assert_eq!(plan.rows.len(), profile.bindings.len());
                let mut used = HashSet::new();
                for row in &plan.rows {
                    assert_eq!(row.output, profile.bindings[&row.target_id]);
                    assert!(!row.reason.is_empty());
                    if let Some(physical) = &row.physical_id {
                        assert!(used.insert(physical));
                        assert_eq!(row.input.as_ref(), cal.bindings.get(physical));
                    } else {
                        assert!(row.input.is_none());
                    }
                }
            }
        }
    }

    #[test]
    fn layout_rule_schema_rejects_typoes_and_bounds_polynomial_solver_resources() {
        let mut db = catalog().clone();
        db.layouts[0].family = "diamondd".into();
        assert!(db.validate().is_err());
        db = catalog().clone();
        db.layouts[0].controls[0].group = "facce".into();
        assert!(db.validate().is_err());
        db = catalog().clone();
        let mut layout = db.layouts[0].clone();
        layout.id = "large-pad".into();
        let template = layout.controls[0].clone();
        layout.controls = (0..64)
            .map(|index| {
                let mut c = template.clone();
                c.id = format!("button_{index}");
                c
            })
            .collect();
        db.layouts.push(layout);
        assert!(
            db.validate().is_ok(),
            "solver no longer has an eight-face limit"
        );
        let layout = db.layouts.last_mut().unwrap();
        let mut extra = template;
        extra.id = "button_64".into();
        layout.controls.push(extra);
        assert!(db.validate().is_err());
    }
    #[test]
    fn svg_export_is_reproducible() {
        let dir = tempfile::tempdir().unwrap();
        export_svg(dir.path()).unwrap();
        let before = std::fs::read(dir.path().join("brawler64.svg")).unwrap();
        export_svg(dir.path()).unwrap();
        assert_eq!(
            before,
            std::fs::read(dir.path().join("brawler64.svg")).unwrap()
        );
    }

    #[test]
    fn measured_axis_roundtrips_and_cannot_change_physical_identity() {
        use crate::controller_axis::AxisMeasurement;
        let mut cal = calibration("nes");
        let b = cal.bindings.get_mut("b").unwrap();
        b.native = Some(NativeInput {
            code: 0x30000,
            direction: -1,
        });
        b.axis = Some(AxisMeasurement {
            minimum: 0,
            maximum: 255,
            flat: 0,
            fuzz: 0,
            resolution: 0,
            released: 128,
            pressed: 0,
        });
        cal.validate().unwrap();
        let decoded: Calibration =
            serde_json::from_str(&serde_json::to_string(&cal).unwrap()).unwrap();
        assert_eq!(cal, decoded);
        cal.bindings
            .get_mut("b")
            .unwrap()
            .native
            .as_mut()
            .unwrap()
            .direction = 1;
        assert!(cal.validate().is_err());
        cal.bindings.get_mut("b").unwrap().native = None;
        assert!(cal.validate().is_err());
        cal = decoded;
        let mut alias = cal.bindings["b"].clone();
        alias.code += 1; // Different normalized code must not hide a physical alias.
        alias.axis.as_mut().unwrap().pressed = 1;
        cal.bindings.insert("a".into(), alias);
        assert!(cal.validate().is_err());
    }
}
