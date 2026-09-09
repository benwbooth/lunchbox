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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_of: Option<String>,
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
        db.validate()
            .expect("bundled controller catalog must validate");
        db
    })
}

impl Catalog {
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
        self.emulator_profiles
            .iter()
            .filter(|profile| {
                profile.core == core
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
                    "rectangle" | "handheld" | "dual-grip" | "three-grip"
                ),
                "unknown shape"
            );
            ensure!(
                matches!(
                    layout.family.as_str(),
                    "two-button"
                        | "horizontal-four"
                        | "diamond"
                        | "n64"
                        | "three-button"
                        | "six-button"
                        | "arcade-rows"
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
                    matches!(
                        control.group.as_str(),
                        "face"
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
                        "retropad" | "duckstation-settings" | "ares-settings"
                    ),
                "unsupported profile contract"
            );
            ensure!(
                profile.source.starts_with("https://") && !profile.conditions.is_empty(),
                "profile lacks provenance or assumptions"
            );
            if let Some(native) = &profile.native_launch {
                ensure!(
                    profile.transport == "ares-settings"
                        && profile.core == "ares"
                        && profile.retroarch_launch.is_none()
                        && !native.platforms.is_empty()
                        && (1..=5).contains(&native.max_players),
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
            if let Some(launch) = &profile.retroarch_launch {
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
                        || (profile.target_layout == "dualshock"
                            && matches!(
                                (profile.core.as_str(), launch.device),
                                ("swanstation", 261) | ("mednafen_psx" | "mednafen_psx_hw", 517)
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
                        launch_targets.insert((
                            profile.core.clone(),
                            alias.to_ascii_lowercase(),
                            launch.device
                        )),
                        "duplicate or ambiguous core/platform launch contract"
                    );
                }
            }
            let mut outputs = HashSet::new();
            for (target, output) in &profile.bindings {
                ensure!(
                    layout.controls.iter().any(|control| control.id == *target),
                    "unknown target control"
                );
                let known = match profile.transport.as_str() {
                    "ares-settings" => crate::controller_ares::valid_output(profile, output),
                    "retropad" => {
                        crate::settings::CONTROLLER_GAMEPAD_BUTTONS.contains(&output.as_str())
                    }
                    "duckstation-settings" => {
                        [
                            "Up", "Down", "Left", "Right", "Start", "Select", "Cross", "Circle",
                            "Square", "Triangle", "L1", "R1", "L2", "R2",
                        ]
                        .contains(&output.as_str())
                            || (profile.target_layout == "dualshock"
                                && [
                                    "L3", "R3", "LLeft", "LRight", "LUp", "LDown", "RLeft",
                                    "RRight", "RUp", "RDown",
                                ]
                                .contains(&output.as_str()))
                    }
                    _ => false,
                };
                ensure!(known, "unknown output control for this transport");
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
                ensure!(outputs.insert(output), "conflicting output controls");
            }
            for control in layout.controls.iter().filter(|control| !control.optional) {
                ensure!(
                    profile.bindings.contains_key(&control.id),
                    "incomplete emulator contract"
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
        }
        Ok(())
    }

    pub fn plan(&self, profile_id: &str) -> Result<MappingPlan> {
        self.validate()?;
        let db = catalog();
        let source = db.layout(&self.layout).unwrap();
        let profile = db
            .emulator_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown emulator profile"))?;
        let target = db.layout(&profile.target_layout).unwrap();
        let mut warnings = profile.conditions.clone();
        let adapter = (self.backend != "sdl3-gamepad" || profile.transport == "ares-settings")
            && crate::controller_launch::supports_profile(profile);
        if self.backend == "sdl3-gamepad" && profile.transport != "ares-settings" {
            warnings.push("SDL3 native input is ready for setup and mapping preview. This target still requires an SDL3-aware launch transport; SDL logical codes are not evdev codes.".into());
        }
        warnings.push(if profile.transport == "ares-settings" {
            "ares 148+: saved player assignments and button choices are applied to a private settings file. The emulator's SDL device identities are checked at launch."
        } else if adapter {
            "Native Linux RetroArch launch adapter available. Physical bindings and connected-device numbering are checked again at launch; automatic RetroArch remaps/overrides are suspended for that session."
        } else {
            "Preview only: an automatic launch adapter for this contract is not implemented."
        }.into());
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
            } else {
                "→"
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
    svg.push_str("<text x=\"450\" y=\"477\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"15\" fill=\"#a9bbca\">Front view · rear controls shown at bottom · schematic, not to scale</text></svg>");
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
            target_mappings: Default::default(),
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
                    (
                        c.id.clone(),
                        InputBinding {
                            code: i as u32,
                            kind: if c.analog { "axis" } else { "button" }.into(),
                            direction: if c.analog { 1 } else { 0 },
                            logical: c.label.clone(),
                            native: None,
                            axis: None,
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
