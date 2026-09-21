//! Per-adapter display capabilities: fullscreen, CRT shader presets, and
//! system bezels.
//!
//! RetroArch settings are applied through a private `--appendconfig` file
//! written at launch, so they override the user's own config for that
//! session only. Standalone fullscreen settings become ordinary launch
//! flags. Nothing in this module may block a launch: a missing preset or an
//! unavailable bezel degrades into a warning string.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::emulator::{EmulatorExecutable, LaunchPlan};
use crate::settings::ResolvedLaunchCustomization;

/// Standalone emulators with a trustworthy command-line fullscreen switch.
/// Everything else keeps its own window state; users can pass their own
/// flags through the launch profile's extra arguments.
const STANDALONE_FULLSCREEN_FLAGS: &[(&str, &str)] =
    &[("DuckStation", "--fullscreen"), ("PCSX2", "--fullscreen")];

pub struct ShaderPresetChoice {
    /// Stored in `emulator_launch_profiles.display_shader`.
    pub id: &'static str,
    pub label: &'static str,
    /// Paths relative to the RetroArch shader root, probed in order across
    /// both the managed (`shaders_slang/`) and legacy flat pack layouts.
    pub relative_paths: &'static [&'static str],
    /// Written by Lunchbox on demand instead of probed on disk.
    pub generated: bool,
}

/// The curated CRT list, best first. Koko-AIO doubles as the "RetroTube TV"
/// look: bezel artwork, ambient screen lighting, and aperture curvature in
/// one preset (GPL-3.0, installed alongside the libretro slang pack).
pub const RETROARCH_SHADER_PRESETS: &[ShaderPresetChoice] = &[
    ShaderPresetChoice {
        id: "retrotube-tv",
        label: "RetroTube TV · bezel + ambient light (Koko-AIO)",
        relative_paths: &["bezel/koko-aio/koko-aio-ng.slangp"],
        generated: false,
    },
    ShaderPresetChoice {
        id: "crt-guest-advanced",
        label: "CRT Guest Advanced",
        relative_paths: &[
            "crt/crt-guest-advanced.slangp",
            "crt/crt-guest-advanced-hd.slangp",
        ],
        generated: false,
    },
    ShaderPresetChoice {
        id: "crt-royale-fast",
        label: "CRT Royale Fast",
        relative_paths: &["crt/crt-royale-fast.slangp"],
        generated: false,
    },
    ShaderPresetChoice {
        id: "crt-easymode",
        label: "CRT Easy Mode",
        relative_paths: &["crt/crt-easymode.slangp"],
        generated: false,
    },
    ShaderPresetChoice {
        id: "zfast-crt",
        label: "ZFast CRT Geometry",
        relative_paths: &["crt/zfast-crt-geo.slangp"],
        generated: false,
    },
    ShaderPresetChoice {
        id: "retrotube-aperture-warp",
        label: "RetroTube aperture warp · geometry only",
        relative_paths: &[],
        generated: true,
    },
];

pub fn shader_preset_choices() -> &'static [ShaderPresetChoice] {
    RETROARCH_SHADER_PRESETS
}

pub fn fullscreen_flag_for(emulator_name: &str) -> Option<&'static str> {
    STANDALONE_FULLSCREEN_FLAGS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(emulator_name))
        .map(|(_, flag)| *flag)
}

pub fn fullscreen_supported(emulator_name: &str, runtime_kind: &str) -> bool {
    runtime_kind == "retroarch" || fullscreen_flag_for(emulator_name).is_some()
}

pub fn shader_presets_supported(runtime_kind: &str) -> bool {
    runtime_kind == "retroarch"
}

pub fn bezels_supported(platform: &str, runtime_kind: &str) -> bool {
    runtime_kind == "retroarch" && crate::bezel_project::theme_for_platform(platform).is_some()
}

/// The RetroArch configuration directory for this executable, mirroring the
/// calibrated-launch resolution (flatpak sandboxes keep their config under
/// ~/.var/app).
fn retroarch_config_base(executable: &EmulatorExecutable) -> Option<PathBuf> {
    let dirs = directories::BaseDirs::new()?;
    Some(match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => dirs
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/retroarch"),
        _ => dirs.config_dir().join("retroarch"),
    })
}

fn shader_root(executable: &EmulatorExecutable) -> Option<PathBuf> {
    Some(retroarch_config_base(executable)?.join("shaders"))
}

fn probe_preset(root: &Path, relative_paths: &[&str]) -> Option<PathBuf> {
    for relative in relative_paths {
        for candidate in [
            root.join("shaders_slang").join(relative),
            root.join(relative),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Resolve a stored preset id to an on-disk preset path, writing Lunchbox
/// presets on demand. `None` means the preset is not available right now.
pub fn resolve_shader_preset(executable: &EmulatorExecutable, preset_id: &str) -> Option<PathBuf> {
    let root = shader_root(executable)?;
    let choice = RETROARCH_SHADER_PRESETS
        .iter()
        .find(|choice| choice.id == preset_id)?;
    if choice.generated {
        install_generated_preset(&root, choice)
    } else {
        probe_preset(&root, choice.relative_paths)
    }
}

/// Presets that exist on disk right now, for the settings UI.
pub fn installed_shader_preset_ids(executable: &EmulatorExecutable) -> Vec<&'static str> {
    let Some(root) = shader_root(executable) else {
        return Vec::new();
    };
    RETROARCH_SHADER_PRESETS
        .iter()
        .filter(|choice| choice.generated || probe_preset(&root, choice.relative_paths).is_some())
        .map(|choice| choice.id)
        .collect()
}

const APERTURE_WARP_SHADER: &str = include_str!("../shaders/retrotube-aperture-warp.slang");
const APERTURE_WARP_PRESET: &str = include_str!("../shaders/retrotube-aperture-warp.slangp");

fn install_generated_preset(root: &Path, choice: &ShaderPresetChoice) -> Option<PathBuf> {
    let directory = root.join("lunchbox");
    let preset_path = directory.join(format!("{}.slangp", choice.id));
    let shader_path = directory.join(format!("{}.slang", choice.id));
    let write = |path: &Path, contents: &str| -> Option<()> {
        fs::create_dir_all(path.parent()?).ok()?;
        fs::write(path, contents).ok()?;
        Some(())
    };
    write(&shader_path, APERTURE_WARP_SHADER)?;
    write(&preset_path, APERTURE_WARP_PRESET)?;
    Some(preset_path)
}

/// Apply the resolved display customization to a ready launch plan. Must be
/// called after calibrated-controller attachment so the display values win
/// RetroArch's appendconfig merge order. Returns a warning when the launch
/// continues with a degraded display setup.
pub fn attach_launch_display_configuration(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
    platform: &str,
    rom_stem: &str,
    customization: &ResolvedLaunchCustomization,
) -> Option<String> {
    if plan.retroarch_content.is_none() {
        return None;
    }
    let mut warnings = Vec::new();
    let mut lines = String::new();
    match customization.display_fullscreen.as_str() {
        "true" | "false" => {
            lines.push_str(&format!(
                "video_fullscreen = \"{}\"\n",
                customization.display_fullscreen
            ));
        }
        _ => {}
    }
    if !customization.display_shader.is_empty() {
        match resolve_shader_preset(executable, &customization.display_shader) {
            Some(preset_path) => {
                lines.push_str("video_shader_enable = \"true\"\n");
                lines.push_str(&format!("video_shader = \"{}\"\n", preset_path.display()));
            }
            None => warnings.push(format!(
                "The {} shader preset is not installed, so the game started without it",
                customization.display_shader
            )),
        }
    }
    if customization.display_bezel == "system" {
        match crate::bezel_project::system_bezel_overlay(platform, rom_stem) {
            Ok(Some(overlay_path)) => {
                lines.push_str("input_overlay_enable = \"true\"\n");
                lines.push_str(&format!(
                    "input_overlay = \"{}\"\n",
                    overlay_path.display()
                ));
                lines.push_str("input_overlay_opacity = \"1.000000\"\n");
            }
            Ok(None) => warnings.push(
                "No system bezel matched this game in The Bezel Project pack, so the game started without one"
                    .to_owned(),
            ),
            Err(error) => warnings.push(format!(
                "The system bezel could not be prepared: {error:#}"
            )),
        }
    }
    if lines.is_empty() {
        return if warnings.is_empty() {
            None
        } else {
            Some(warnings.join("; "))
        };
    }
    let config_path = write_launch_display_config(&lines);
    match config_path {
        Ok(path) => {
            if let Err(error) = crate::controller_launch::attach_config(plan, executable, &path) {
                warnings.push(format!(
                    "The display settings could not be attached to the launch: {error:#}"
                ));
            }
        }
        Err(error) => warnings.push(format!(
            "The display settings file could not be written: {error:#}"
        )),
    }
    if warnings.is_empty() {
        None
    } else {
        Some(warnings.join("; "))
    }
}

fn write_launch_display_config(contents: &str) -> Result<PathBuf> {
    let directory = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("launch-display"))
        .context("could not determine the Lunchbox data directory")?;
    fs::create_dir_all(&directory)?;
    prune_stale_launch_display_configs(&directory);
    let path = directory.join(format!("retroarch-{}.cfg", Uuid::new_v4().simple()));
    fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

fn prune_stale_launch_display_configs(directory: &Path) {
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| now.as_secs().saturating_sub(48 * 60 * 60))
        .unwrap_or_default();
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        if modified < cutoff {
            let _ = fs::remove_file(entry.path());
        }
    }
}
