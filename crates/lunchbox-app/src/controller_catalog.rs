//! Physical layouts, device calibration and explicit emulator input contracts.
//! The checked-in facts and original schematic renderer are independent of raw
//! device numbering. Documented contracts are not a claim of runtime validation.
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;
use std::sync::OnceLock;

use anyhow::{Result, bail, ensure};
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
    pub status: String,
    pub source: String,
    pub conditions: Vec<String>,
    pub bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub core_options: BTreeMap<String, String>,
    /// Explicit opt-in to the implemented Linux RetroArch launch writer.
    /// Documented preview profiles do not implicitly become launch contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retroarch_launch: Option<RetroArchLaunch>,
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
    pub layout: String,
    pub os: String,
    pub backend: String,
    pub bindings: BTreeMap<String, InputBinding>,
}

#[derive(Debug, Serialize)]
pub struct MappingPlan {
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
}

pub fn catalog() -> &'static Catalog {
    static DB: OnceLock<Catalog> = OnceLock::new();
    DB.get_or_init(|| {
        let db: Catalog = serde_json::from_str(include_str!("../data/controllers/catalog.json"))
            .expect("bundled controller catalog must parse");
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
        let mut matches = self.emulator_profiles.iter().filter(|profile| {
            profile.core == core
                && profile.retroarch_launch.as_ref().is_some_and(|launch| {
                    launch
                        .platforms
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(platform.trim()))
                })
        });
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
            let mut controls = HashSet::new();
            ensure!(
                layout.controls.iter().filter(|c| c.group == "face").count() <= 8,
                "Face group exceeds assignment solver bound"
            );
            for control in &layout.controls {
                ensure!(
                    valid_id(&control.id) && controls.insert(&control.id),
                    "duplicate control ID"
                );
                ensure!(!control.label.is_empty(), "empty control label");
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
                        "retropad" | "duckstation-settings"
                    ),
                "unsupported profile contract"
            );
            ensure!(
                profile.source.starts_with("https://") && !profile.conditions.is_empty(),
                "profile lacks provenance or assumptions"
            );
            if profile.transport == "duckstation-settings" {
                ensure!(
                    profile.core == "duckstation" && profile.target_layout == "playstation-digital",
                    "unreviewed DuckStation input mode"
                );
            }
            if let Some(launch) = &profile.retroarch_launch {
                ensure!(
                    profile.transport == "retropad",
                    "non-RetroPad profile cannot use RetroArch launch adapter"
                );
                ensure!(valid_id(&profile.core), "invalid launched core ID");
                ensure!(
                    !launch.platforms.is_empty() && (1..=16).contains(&launch.max_players),
                    "launch contract requires platform aliases and 1-16 player ports"
                );
                ensure!(
                    launch.device & 0xff == 1 && launch.device <= u16::MAX.into(),
                    "launch writer only supports joypads and their subclasses"
                );
                for alias in &launch.platforms {
                    ensure!(
                        !alias.is_empty()
                            && alias.trim() == alias
                            && !alias.chars().any(char::is_control),
                        "invalid launch platform alias"
                    );
                    ensure!(
                        launch_targets.insert((profile.core.clone(), alias.to_ascii_lowercase())),
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
                    "retropad" => {
                        crate::settings::CONTROLLER_GAMEPAD_BUTTONS.contains(&output.as_str())
                    }
                    "duckstation-settings" => [
                        "Up", "Down", "Left", "Right", "Start", "Select", "Cross", "Circle",
                        "Square", "Triangle", "L1", "R1", "L2", "R2",
                    ]
                    .contains(&output.as_str()),
                    _ => false,
                };
                ensure!(known, "unknown output control for this transport");
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
        let layout = catalog()
            .layout(&self.layout)
            .ok_or_else(|| anyhow::anyhow!("Unknown controller layout"))?;
        ensure!(
            matches!(self.os.as_str(), "linux" | "windows" | "macos"),
            "Unsupported calibration OS"
        );
        ensure!(
            self.backend == "gilrs-0.11",
            "Unsupported calibration input backend"
        );
        ensure!(
            !self.bindings.is_empty() && self.bindings.len() <= layout.controls.len(),
            "Record at least one control"
        );
        let mut inputs = HashSet::new();
        let mut native_inputs = HashSet::new();
        for (id, input) in &self.bindings {
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
        let adapter = crate::controller_launch::supports_profile(profile);
        warnings.push(if adapter {
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
        let assignments = crate::controller_layout::assignments(source, target);
        let rows: Vec<MappingRow> = profile
            .bindings
            .iter()
            .map(|(target_id, output)| {
                let physical = assignments
                    .get(target_id)
                    .and_then(|source_id| source.controls.iter().find(|c| c.id == *source_id));
                let input = physical.and_then(|c| self.bindings.get(&c.id)).cloned();
                if input.is_none() {
                    warnings.push(format!("Missing physical input for {target_id}"));
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
                }
            })
            .collect();
        let automatic_launch_ready = adapter
            && self.os == "linux"
            && self.os == std::env::consts::OS
            && rows.iter().all(|row| {
                row.input
                    .as_ref()
                    .is_some_and(|input| input.native.is_some())
                    || target
                        .controls
                        .iter()
                        .any(|c| c.id == row.target_id && c.optional)
            });
        Ok(MappingPlan {
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
        assert!(
            brawler
                .plan(&profile.id)
                .unwrap()
                .rows
                .iter()
                .find(|r| r.target_id == "r2")
                .unwrap()
                .input
                .is_none()
        );
        let standard_n64 = calibration("n64").plan(&profile.id).unwrap();
        for id in ["r2", "select"] {
            assert!(
                standard_n64
                    .rows
                    .iter()
                    .find(|r| r.target_id == id)
                    .unwrap()
                    .input
                    .is_none()
            );
        }
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
}
