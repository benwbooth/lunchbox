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

/// Adapters with a trustworthy automatic save-state mechanism. RetroArch
/// covers every core through config; MAME documents `-autosave` (save at
/// exit, restore at start); DuckStation pairs `-resume` with a settings
/// override that enables Save State on Shutdown.
pub fn save_states_supported(emulator_name: &str, runtime_kind: &str) -> bool {
    runtime_kind == "retroarch"
        || ["MAME", "DuckStation"]
            .iter()
            .any(|name| name.eq_ignore_ascii_case(emulator_name))
}

/// Launch arguments that turn automatic save states on (or explicitly off,
/// where the emulator supports a counter-signal) for standalone emulators.
pub fn save_state_arguments(
    emulator_name: &str,
    save_states: &str,
) -> Result<Vec<std::ffi::OsString>> {
    let mut arguments = Vec::new();
    if emulator_name.eq_ignore_ascii_case("MAME") {
        if save_states == "on" {
            arguments.push(std::ffi::OsString::from("-autosave"));
        }
        return Ok(arguments);
    }
    if emulator_name.eq_ignore_ascii_case("DuckStation") && !save_states.is_empty() {
        let settings_path = write_duckstation_session_settings(save_states == "on")?;
        if save_states == "on" {
            arguments.push(std::ffi::OsString::from("-resume"));
        }
        arguments.push(std::ffi::OsString::from("-settings"));
        arguments.push(std::ffi::OsString::from(settings_path.as_os_str()));
    }
    Ok(arguments)
}

const DUCKSTATION_SESSION_SETTINGS: &str = "[Main]\nSaveStateOnShutdown = {value}\n";

fn write_duckstation_session_settings(save_on_shutdown: bool) -> Result<PathBuf> {
    let directory = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("launch-display"))
        .context("could not determine the Lunchbox data directory")?;
    fs::create_dir_all(&directory)?;
    let path = directory.join("duckstation-session.ini");
    fs::write(
        &path,
        DUCKSTATION_SESSION_SETTINGS
            .replace("{value}", if save_on_shutdown { "true" } else { "false" }),
    )
    .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
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

/// Slang (`.slangp`) presets need a slang-capable video driver. When the
/// user's saved driver cannot run them, the session switches to `glcore`
/// so the chosen shader actually loads instead of silently falling back
/// to stock.
const SLANG_VIDEO_DRIVERS: &[&str] = &["vulkan", "glcore", "d3d11", "d3d12", "metal"];

fn user_video_driver(executable: &EmulatorExecutable) -> Option<String> {
    retroarch_config_value(executable, "video_driver")
}

fn video_driver_from_config(text: &str) -> Option<String> {
    config_value_from_text(text, "video_driver")
}

pub fn retroarch_config_value(executable: &EmulatorExecutable, key: &str) -> Option<String> {
    let path = retroarch_config_base(executable)?.join("retroarch.cfg");
    let text = fs::read_to_string(path).ok()?;
    config_value_from_text(&text, key)
}

fn config_value_from_text(text: &str, wanted_key: &str) -> Option<String> {
    let mut result = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == wanted_key {
            result = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    result
}

fn slang_driver_override(executable: &EmulatorExecutable) -> Option<&'static str> {
    slang_driver_override_for(user_video_driver(executable).as_deref())
}

fn slang_driver_override_for(configured: Option<&str>) -> Option<&'static str> {
    let configured = configured?;
    (!SLANG_VIDEO_DRIVERS
        .iter()
        .any(|capable| configured.eq_ignore_ascii_case(capable)))
    .then_some("glcore")
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
                // RetroArch's CLI applies this when content loads and takes
                // precedence over any automatic core/game shader preset.
                attach_shader_argument(plan, executable, &preset_path);
                if let Some(driver) = slang_driver_override(executable) {
                    lines.push_str(&format!("video_driver = \"{driver}\"\n"));
                }
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
                "The Bezel Project pack has no matching per-game or system bezel, so the game started without one"
                    .to_owned(),
            ),
            Err(error) => warnings.push(format!(
                "The system bezel could not be prepared: {error:#}"
            )),
        }
    }
    match customization.save_states.as_str() {
        "on" | "off" => {
            let value = if customization.save_states == "on" {
                "true"
            } else {
                "false"
            };
            // Save on exit, resume on launch. Explicit false keeps a
            // platform-level "on" from leaking into a game-level opt-out.
            lines.push_str(&format!("savestate_auto_save = \"{value}\"\n"));
            lines.push_str(&format!("savestate_auto_load = \"{value}\"\n"));
        }
        _ => {}
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

fn attach_shader_argument(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
    preset_path: &Path,
) {
    let insertion = match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => plan
            .arguments
            .iter()
            .position(|argument| argument.to_str() == Some(app_id))
            .map(|index| index + 1),
        EmulatorExecutable::Native(_) => Some(0),
        _ => None,
    };
    if let Some(index) = insertion {
        plan.arguments.insert(
            index,
            format!("--set-shader={}", preset_path.display()).into(),
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slang_shaders_switch_legacy_drivers_to_glcore() {
        assert_eq!(slang_driver_override_for(Some("gl")), Some("glcore"));
        assert_eq!(slang_driver_override_for(Some("GL")), Some("glcore"));
        assert_eq!(slang_driver_override_for(Some("gl1")), Some("glcore"));
        assert_eq!(slang_driver_override_for(Some("sdl2")), Some("glcore"));
        assert_eq!(slang_driver_override_for(Some("vulkan")), None);
        assert_eq!(slang_driver_override_for(Some("glcore")), None);
        assert_eq!(slang_driver_override_for(Some("d3d11")), None);
        assert_eq!(slang_driver_override_for(Some("metal")), None);
        assert_eq!(slang_driver_override_for(None), None);
    }

    #[test]
    fn video_driver_parser_skips_comments_and_blank_lines() {
        assert_eq!(
            video_driver_from_config("# RetroArch\n\nvideo_driver = \"gl\"\n"),
            Some("gl".to_owned())
        );
    }
}
