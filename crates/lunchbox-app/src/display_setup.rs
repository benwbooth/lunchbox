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

/// The curated CRT list, best first. Koko-AIO's configured Base preset is the
/// "RetroTube TV" look; its raw engine preset leaves the CRT effects disabled.
pub const RETROARCH_SHADER_PRESETS: &[ShaderPresetChoice] = &[
    ShaderPresetChoice {
        id: "retrotube-tv",
        label: "RetroTube TV · bezel + ambient light (Koko-AIO)",
        relative_paths: &["bezel/koko-aio/Presets-ng/Base.slangp"],
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
    runtime_kind == "retroarch" && !bezel_choices(platform).is_empty()
}

pub struct BezelChoice {
    pub id: &'static str,
    pub label: &'static str,
}

/// Artwork choices are explicit sources, rather than an opaque "pack" whose
/// per-game fallback can silently change its appearance.
pub fn bezel_choices(platform: &str) -> Vec<BezelChoice> {
    let mut choices = Vec::new();
    if crate::bezel_project::theme_for_platform(platform).is_some() {
        choices.push(BezelChoice {
            id: "system",
            label: "Bezel Project · system art",
        });
        choices.push(BezelChoice {
            id: "themed",
            label: "Bezel Project · game art",
        });
    }
    if crate::bezel_orionsangel::supported(platform) {
        choices.push(BezelChoice {
            id: "orionsangel",
            label: "Orionsangel · console",
        });
        choices.push(BezelChoice {
            id: "orionsangel-plain",
            label: "Orionsangel · plain console",
        });
    }
    choices
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

/// Koko-AIO's configured Base preset includes its own bezel. When a separate
/// Bezel Project overlay is active, reference that preset with only its built-in
/// bezel disabled; scanlines, phosphor mask, curvature, and ambient light stay on.
fn install_retrotube_system_bezel_variant(root: &Path, base: &Path) -> Result<PathBuf> {
    let relative = base
        .strip_prefix(root)
        .context("RetroTube TV preset is outside the shader directory")?;
    let directory = root.join("lunchbox");
    fs::create_dir_all(&directory)?;
    let path = directory.join("retrotube-tv-system-bezel.slangp");
    let reference = Path::new("..")
        .join(relative)
        .to_string_lossy()
        .replace('\\', "/");
    fs::write(
        &path,
        format!("#reference \"{reference}\"\nDO_BEZEL = \"0.0\"\n"),
    )
    .with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
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
    output_aspect: Option<f64>,
) -> Option<String> {
    if plan.retroarch_content.is_none() {
        return None;
    }
    let mut warnings = Vec::new();
    let mut lines = String::new();
    let mut shader_preset_path = None;
    let mut external_bezel_active = false;
    match customization.display_fullscreen.as_str() {
        "true" | "false" => {
            lines.push_str(&format!(
                "video_fullscreen = \"{}\"\n",
                customization.display_fullscreen
            ));
        }
        _ => {}
    }
    if customization.display_bezel == "off" {
        lines.push_str("input_overlay_enable = \"false\"\n");
    } else if !customization.display_bezel.is_empty() {
        let selected = match customization.display_bezel.as_str() {
            "system" => crate::bezel_project::system_bezel_overlay(platform, rom_stem),
            "themed" => crate::bezel_project::bezel_overlay(
                platform,
                rom_stem,
                crate::bezel_project::PackStyle::GameArt,
            ),
            "orionsangel" => crate::bezel_orionsangel::overlay(platform, false),
            "orionsangel-plain" => crate::bezel_orionsangel::overlay(platform, true),
            other => Err(anyhow::anyhow!("Unknown bezel choice {other}")),
        };
        match selected.and_then(|overlay| {
            overlay
                .map(|path| aspect_fitted_overlay(&path, output_aspect))
                .transpose()
        }) {
            Ok(Some(overlay_path)) => {
                external_bezel_active = true;
                lines.push_str("input_overlay_enable = \"true\"\n");
                lines.push_str(&format!("input_overlay = \"{}\"\n", overlay_path.display()));
                lines.push_str("input_overlay_opacity = \"1.000000\"\n");
                lines.push_str("input_overlay_auto_scale = \"false\"\n");
                lines.push_str("input_overlay_scale_landscape = \"1.000000\"\n");
                lines.push_str("input_overlay_aspect_adjust_landscape = \"0.000000\"\n");
            }
            Ok(None) => {
                lines.push_str("input_overlay_enable = \"false\"\n");
                warnings.push("The selected bezel source has no artwork for this game or system, so the game started without one".to_owned());
            }
            Err(error) => {
                lines.push_str("input_overlay_enable = \"false\"\n");
                warnings.push(format!(
                    "The selected bezel could not be prepared: {error:#}"
                ));
            }
        }
    }
    if !customization.display_shader.is_empty() {
        match resolve_shader_preset(executable, &customization.display_shader) {
            Some(mut preset_path) => {
                if customization.display_shader == "retrotube-tv"
                    && external_bezel_active
                    && let Some(root) = shader_root(executable)
                {
                    match install_retrotube_system_bezel_variant(&root, &preset_path) {
                        Ok(path) => preset_path = path,
                        Err(error) => warnings.push(format!(
                            "RetroTube TV could not disable its built-in bezel: {error:#}"
                        )),
                    }
                }
                lines.push_str("video_shader_enable = \"true\"\n");
                lines.push_str(&format!("video_shader = \"{}\"\n", preset_path.display()));
                shader_preset_path = Some(preset_path);
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
            match crate::controller_launch::attach_config(plan, executable, &path) {
                Ok(()) => {
                    // Apply the chosen preset when content loads. Never add
                    // this override unless the matching session config also
                    // attached (it selects a slang-capable video driver).
                    if let Some(preset_path) = shader_preset_path {
                        attach_shader_argument(plan, executable, &preset_path);
                    }
                }
                Err(error) => warnings.push(format!(
                    "The display settings could not be attached to the launch: {error:#}"
                )),
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

/// Place fixed-aspect artwork inside a wider (or taller) output without
/// stretching the console artwork. The overlay itself remains full-screen;
/// its rectangle is centered within that screen. RetroArch's documented
/// overlay0_rect coordinates are normalized to the full-screen rectangle.
fn aspect_fitted_overlay(path: &Path, output_aspect: Option<f64>) -> Result<PathBuf> {
    let Some(output_aspect) = output_aspect.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok(path.to_path_buf());
    };
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading selected bezel {}", path.display()))?;
    let image_name = contents
        .lines()
        .find_map(|line| line.trim().strip_prefix("overlay0_overlay"))
        .and_then(|value| value.trim().strip_prefix('='))
        .map(|value| value.trim().trim_matches('"'))
        .filter(|value| !value.is_empty())
        .context("selected bezel has no image")?;
    let image = path
        .parent()
        .context("selected bezel has no directory")?
        .join(image_name);
    let bytes = fs::read(&image)
        .with_context(|| format!("reading selected bezel image {}", image.display()))?;
    let (width, height) = crate::bezel_orionsangel::png_dimensions(&bytes)
        .context("selected bezel image has no valid PNG dimensions")?;
    let Some((x, y, w, h)) = fitted_overlay_rect(width, height, output_aspect) else {
        return Ok(path.to_path_buf());
    };
    let mut fitted = String::new();
    for line in contents.lines() {
        if line.trim_start().starts_with("overlay0_overlay") {
            fitted.push_str(&format!("overlay0_overlay = \"{}\"\n", image.display()));
        } else if !line.trim_start().starts_with("overlay0_rect") {
            fitted.push_str(line);
            fitted.push('\n');
        }
    }
    fitted.push_str(&format!(
        "overlay0_rect = \"{x:.6},{y:.6},{w:.6},{h:.6}\"\n"
    ));
    write_launch_display_config(&fitted)
}

fn fitted_overlay_rect(
    image_width: u32,
    image_height: u32,
    output_aspect: f64,
) -> Option<(f64, f64, f64, f64)> {
    if image_width == 0 || image_height == 0 || !output_aspect.is_finite() || output_aspect <= 0.0 {
        return None;
    }
    let source_aspect = f64::from(image_width) / f64::from(image_height);
    if (output_aspect - source_aspect).abs() < 0.001 {
        return None;
    }
    Some(if output_aspect > source_aspect {
        let w = source_aspect / output_aspect;
        ((1.0 - w) / 2.0, 0.0, w, 1.0)
    } else {
        let h = output_aspect / source_aspect;
        (0.0, (1.0 - h) / 2.0, 1.0, h)
    })
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
    fn sixteen_nine_art_is_centered_without_stretching_on_ultrawide() {
        let (x, y, width, height) = fitted_overlay_rect(1920, 1080, 5120.0 / 2160.0).unwrap();
        assert!((x - 0.125).abs() < 0.000001);
        assert_eq!(y, 0.0);
        assert!((width - 0.75).abs() < 0.000001);
        assert_eq!(height, 1.0);
        assert!(fitted_overlay_rect(1920, 1080, 16.0 / 9.0).is_none());
    }

    #[test]
    fn bezel_choices_include_both_sources_for_snes() {
        let choices = bezel_choices("Super Nintendo Entertainment System");
        assert_eq!(
            choices.iter().map(|choice| choice.id).collect::<Vec<_>>(),
            ["system", "themed", "orionsangel", "orionsangel-plain"]
        );
    }

    #[test]
    fn retrotube_uses_configured_crt_and_keeps_external_bezel_separate() {
        let choice = RETROARCH_SHADER_PRESETS
            .iter()
            .find(|choice| choice.id == "retrotube-tv")
            .unwrap();
        assert_eq!(
            choice.relative_paths,
            &["bezel/koko-aio/Presets-ng/Base.slangp"]
        );

        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        for relative in [
            "bezel/koko-aio/Presets-ng/Base.slangp",
            "shaders_slang/bezel/koko-aio/Presets-ng/Base.slangp",
        ] {
            let base = root.join(relative);
            fs::create_dir_all(base.parent().unwrap()).unwrap();
            fs::write(
                &base,
                "#reference \"../koko-aio-ng.slangp\"\nDO_PIXELGRID = \"1.0\"\n",
            )
            .unwrap();
            let variant = install_retrotube_system_bezel_variant(root, &base).unwrap();
            let contents = fs::read_to_string(&variant).unwrap();
            assert!(contents.contains("DO_BEZEL = \"0.0\""));
            assert!(!contents.contains("DO_PIXELGRID = \"0.0\""));
            let reference = contents
                .lines()
                .next()
                .unwrap()
                .strip_prefix("#reference \"")
                .unwrap()
                .strip_suffix('"')
                .unwrap();
            assert_eq!(
                variant
                    .parent()
                    .unwrap()
                    .join(reference)
                    .canonicalize()
                    .unwrap(),
                base.canonicalize().unwrap()
            );
        }
    }

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
