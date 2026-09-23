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
use std::process::Command;
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
    if crate::bezel_orionsangel::ultrawide_supported(platform) {
        choices.push(BezelChoice {
            id: "ultrawide",
            label: "Duimon · ultrawide 21:9",
        });
        choices.push(BezelChoice {
            id: "ultrawide-night",
            label: "Duimon · ultrawide 21:9 night",
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
/// artwork overlay is active, disable the shader's built-in bezel. Keep its
/// ambient light except when it would illuminate the blank sidebars beside
/// aspect-fitted artwork.
fn install_retrotube_system_bezel_variant(
    root: &Path,
    base: &Path,
    black_sidebars: bool,
) -> Result<PathBuf> {
    let relative = base
        .strip_prefix(root)
        .context("RetroTube TV preset is outside the shader directory")?;
    let directory = root.join("lunchbox");
    fs::create_dir_all(&directory)?;
    let name = if black_sidebars {
        "retrotube-tv-black-sidebars.slangp"
    } else {
        "retrotube-tv-system-bezel.slangp"
    };
    let path = directory.join(name);
    let reference = Path::new("..")
        .join(relative)
        .to_string_lossy()
        .replace('\\', "/");
    let ambient = if black_sidebars {
        // Koko's ambient light otherwise paints into the transparent space
        // outside a centered 16:9 overlay on an ultrawide display.
        "DO_AMBILIGHT = \"0.0\"\n"
    } else {
        ""
    };
    fs::write(
        &path,
        format!("#reference \"{reference}\"\nDO_BEZEL = \"0.0\"\n{ambient}"),
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

/// RetroArch can retain a custom viewport from a previous bezel session.
/// When its inherited overlay is one of Lunchbox's Duimon 21:9 overlays,
/// rebuild that viewport for this launch instead of inheriting stale pixels.
pub(crate) fn inherited_ultrawide_bezel(executable: &EmulatorExecutable) -> Option<&'static str> {
    let config =
        fs::read_to_string(retroarch_config_base(executable)?.join("retroarch.cfg")).ok()?;
    ultrawide_bezel_from_retroarch_config(&config)
}

fn ultrawide_bezel_from_retroarch_config(config: &str) -> Option<&'static str> {
    if config_value_from_text(config, "input_overlay_enable").as_deref() != Some("true") {
        return None;
    }
    let overlay = config_value_from_text(config, "input_overlay")?;
    let path = Path::new(&overlay);
    if !path
        .components()
        .any(|part| part.as_os_str() == "duimon-ultrawide")
        || path.extension().and_then(|part| part.to_str()) != Some("cfg")
    {
        return None;
    }
    let name = path.file_stem()?.to_str()?.to_ascii_lowercase();
    Some(if name.contains("night") {
        "ultrawide-night"
    } else {
        "ultrawide"
    })
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

/// Keep RetroArch's optional Qt desktop companion out of content launches.
/// Qt 6 builds initialize it whenever the user's desktop menu is enabled,
/// even when ui_companion_enable is false. A crash in that initialization
/// prevents the game and its display settings from starting. This private
/// appendconfig does not change the user's RetroArch configuration.
pub fn attach_launch_desktop_menu_override(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
) -> Result<()> {
    if plan.retroarch_content.is_none() {
        return Ok(());
    }
    let path = write_launch_display_config(
        "desktop_menu_enable = \"false\"\nconfig_save_on_exit = \"false\"\n",
    )?;
    crate::controller_launch::attach_config(plan, executable, &path)
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
    output_dimensions: Option<(u32, u32)>,
) -> Option<String> {
    if plan.retroarch_content.is_none() {
        return None;
    }
    let mut warnings = Vec::new();
    let mut lines = String::new();
    let mut shader_preset_path = None;
    let mut external_bezel_active = false;
    let mut black_sidebars = false;
    let inherited_bezel = customization
        .display_bezel
        .is_empty()
        .then(|| inherited_ultrawide_bezel(executable))
        .flatten();
    let display_bezel = inherited_bezel.unwrap_or(&customization.display_bezel);
    match customization.display_fullscreen.as_str() {
        "true" | "false" => {
            lines.push_str(&format!(
                "video_fullscreen = \"{}\"\n",
                customization.display_fullscreen
            ));
        }
        _ => {}
    }
    if display_bezel == "off" {
        lines.push_str("input_overlay_enable = \"false\"\n");
        lines.push_str("aspect_ratio_index = \"22\"\n");
    } else if !display_bezel.is_empty() {
        let ultrawide = matches!(display_bezel, "ultrawide" | "ultrawide-night");
        let selected = match display_bezel {
            "system" => crate::bezel_project::system_bezel_overlay(platform, rom_stem),
            "themed" => crate::bezel_project::bezel_overlay(
                platform,
                rom_stem,
                crate::bezel_project::PackStyle::GameArt,
            ),
            "orionsangel" => crate::bezel_orionsangel::overlay(platform, false),
            "orionsangel-plain" => crate::bezel_orionsangel::overlay(platform, true),
            "ultrawide" | "ultrawide-night" if customization.display_fullscreen == "false" => {
                Err(anyhow::anyhow!(
                    "21:9 artwork needs fullscreen; change Display fullscreen to On or Inherit"
                ))
            }
            "ultrawide" => crate::bezel_orionsangel::ultrawide_overlay(platform, false),
            "ultrawide-night" => crate::bezel_orionsangel::ultrawide_overlay(platform, true),
            other => Err(anyhow::anyhow!("Unknown bezel choice {other}")),
        };
        match selected.and_then(|overlay| {
            overlay
                .map(|path| {
                    // Custom viewports use RetroArch's render-buffer pixels;
                    // this can differ from the monitor mode under fractional
                    // scaling. The overlay itself remains full-screen.
                    let dimensions = if ultrawide {
                        Some(probe_retroarch_output_dimensions(
                            executable,
                            output_dimensions,
                        )?)
                    } else {
                        output_dimensions
                    };
                    let output_aspect = dimensions
                        .filter(|(_, height)| *height > 0)
                        .map(|(width, height)| f64::from(width) / f64::from(height));
                    let (prepared, pillarboxed) = aspect_fitted_overlay(&path, output_aspect)?;
                    let viewport =
                        if ultrawide {
                            let (width, height) = dimensions
                                .context("the output resolution is needed for 21:9 artwork")?;
                            Some(ultrawide_viewport(width, height).context(
                                "the output resolution cannot fit the 21:9 game opening",
                            )?)
                        } else {
                            None
                        };
                    Ok((prepared, viewport, pillarboxed))
                })
                .transpose()
        }) {
            Ok(Some((overlay_path, viewport, pillarboxed))) => {
                external_bezel_active = true;
                black_sidebars = pillarboxed;
                lines.push_str("input_overlay_enable = \"true\"\n");
                lines.push_str(&format!("input_overlay = \"{}\"\n", overlay_path.display()));
                lines.push_str("input_overlay_opacity = \"1.000000\"\n");
                lines.push_str("input_overlay_auto_scale = \"false\"\n");
                lines.push_str("input_overlay_scale_landscape = \"1.000000\"\n");
                lines.push_str("input_overlay_aspect_adjust_landscape = \"0.000000\"\n");
                if let Some((x, y, width, height)) = viewport {
                    // RetroArch's current custom-aspect index is 23. The
                    // Duimon's transparent opening is centered and exactly
                    // 4:3. RetroArch centers custom viewport dimensions;
                    // custom_viewport_x/y are additional offsets, not the
                    // opening's absolute screen coordinates.
                    if customization.display_fullscreen.is_empty() {
                        lines.push_str("video_fullscreen = \"true\"\n");
                    }
                    lines.push_str("aspect_ratio_index = \"23\"\n");
                    lines.push_str("video_scale_integer = \"false\"\n");
                    lines.push_str(&format!("custom_viewport_x = \"{x}\"\n"));
                    lines.push_str(&format!("custom_viewport_y = \"{y}\"\n"));
                    lines.push_str(&format!("custom_viewport_width = \"{width}\"\n"));
                    lines.push_str(&format!("custom_viewport_height = \"{height}\"\n"));
                }
            }
            Ok(None) => {
                lines.push_str("input_overlay_enable = \"false\"\n");
                lines.push_str("aspect_ratio_index = \"22\"\n");
                warnings.push("The selected bezel source has no artwork for this game or system, so the game started without one".to_owned());
            }
            Err(error) => {
                lines.push_str("input_overlay_enable = \"false\"\n");
                lines.push_str("aspect_ratio_index = \"22\"\n");
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
                    match install_retrotube_system_bezel_variant(
                        &root,
                        &preset_path,
                        black_sidebars,
                    ) {
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
    // These are launch-only overrides. RetroArch must not save the temporary
    // custom viewport (or input/display settings) back into retroarch.cfg.
    lines.push_str("config_save_on_exit = \"false\"\n");
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

/// SDL3 reads the native mode and content scale without creating a window.
/// RetroArch's Wayland context currently applies that scale once more to its
/// GL backing surface, so custom viewport pixels must use the same units.
/// Other contexts use the native mode directly. No host-specific DPI is baked
/// into the launch profile or the artwork.
fn probe_retroarch_output_dimensions(
    executable: &EmulatorExecutable,
    qt_dimensions: Option<(u32, u32)>,
) -> Result<(u32, u32)> {
    let output = Command::new(std::env::current_exe()?)
        .arg("--sdl3-display-inspect")
        .env(
            "LUNCHBOX_SDL3_LIBRARY",
            crate::controller_sdl3::runtime_path(),
        )
        .output()
        .context("starting windowless SDL3 display query")?;
    anyhow::ensure!(
        output.status.success(),
        "SDL3 display query failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let metrics = parse_sdl3_display_metrics(&String::from_utf8(output.stdout)?)
        .context("SDL3 did not report its display mode")?;
    let dimensions = (metrics.width, metrics.height);
    if let Some((width, height)) = qt_dimensions {
        let qt_aspect = f64::from(width) / f64::from(height);
        let sdl_aspect = f64::from(dimensions.0) / f64::from(dimensions.1);
        anyhow::ensure!(
            (qt_aspect - sdl_aspect).abs() < 0.02,
            "The primary display differs from Lunchbox's current screen; 21:9 artwork was skipped"
        );
    }
    let context = retroarch_config_value(executable, "video_context_driver");
    let scale = retroarch_render_scale(
        &metrics.video_driver,
        context.as_deref(),
        metrics.pixel_density,
    );
    scaled_display_dimensions(dimensions, scale)
        .context("RetroArch's render size could not be determined")
}

struct Sdl3DisplayMetrics {
    width: u32,
    height: u32,
    pixel_density: f32,
    video_driver: String,
}

fn parse_sdl3_display_metrics(report: &str) -> Option<Sdl3DisplayMetrics> {
    let (dimensions, remainder) = report.trim().split_once('@')?;
    let (density, video_driver) = remainder.split_once('@')?;
    let (width, height) = dimensions.split_once('x')?;
    let width: u32 = width.parse().ok()?;
    let height: u32 = height.parse().ok()?;
    let pixel_density: f32 = density.parse().ok()?;
    (width > 0 && height > 0 && pixel_density.is_finite() && pixel_density > 0.0).then(|| {
        Sdl3DisplayMetrics {
            width,
            height,
            pixel_density,
            video_driver: video_driver.to_owned(),
        }
    })
}

fn scaled_display_dimensions(dimensions: (u32, u32), scale: f32) -> Option<(u32, u32)> {
    let width = (f64::from(dimensions.0) * f64::from(scale)).round();
    let height = (f64::from(dimensions.1) * f64::from(scale)).round();
    (width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
        && width <= f64::from(u32::MAX)
        && height <= f64::from(u32::MAX))
    .then_some((width as u32, height as u32))
}

fn retroarch_render_scale(video_driver: &str, context: Option<&str>, density: f32) -> f32 {
    if video_driver == "wayland"
        && context.is_none_or(|value| value.is_empty() || value == "wayland")
    {
        density
    } else {
        1.0
    }
}

/// Place fixed-aspect artwork inside a wider (or taller) output without
/// stretching the console artwork. The overlay itself remains full-screen;
/// its rectangle is centered within that screen. RetroArch's documented
/// overlay0_rect coordinates are normalized to the full-screen rectangle.
fn aspect_fitted_overlay(path: &Path, output_aspect: Option<f64>) -> Result<(PathBuf, bool)> {
    let Some(output_aspect) = output_aspect.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return Ok((path.to_path_buf(), false));
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
        return Ok((path.to_path_buf(), false));
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
    Ok((write_launch_display_config(&fitted)?, w < 1.0))
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

/// Custom viewport dimensions corresponding to Duimon's centered transparent
/// 4:3 opening after fitting its unmodified 2560x1080 artwork to the output.
/// RetroArch centers these dimensions, so x and y must remain zero offsets.
fn ultrawide_viewport(output_width: u32, output_height: u32) -> Option<(u32, u32, u32, u32)> {
    if output_width == 0 || output_height == 0 {
        return None;
    }
    let (image_width, image_height) = crate::bezel_orionsangel::ULTRAWIDE_DIMENSIONS;
    let (hole_x, hole_y, hole_width, hole_height) =
        crate::bezel_orionsangel::ULTRAWIDE_SCREEN_OPENING;
    if hole_x.checked_mul(2)?.checked_add(hole_width)? != image_width
        || hole_y.checked_mul(2)?.checked_add(hole_height)? != image_height
    {
        return None;
    }
    let output_aspect = f64::from(output_width) / f64::from(output_height);
    let (_, _, outer_width, outer_height) =
        fitted_overlay_rect(image_width, image_height, output_aspect)
            .unwrap_or((0.0, 0.0, 1.0, 1.0));
    let width = (outer_width * f64::from(hole_width) / f64::from(image_width)
        * f64::from(output_width))
    .round() as u32;
    let height = (outer_height * f64::from(hole_height) / f64::from(image_height)
        * f64::from(output_height))
    .round() as u32;
    (width > 0 && height > 0 && width <= output_width && height <= output_height)
        .then_some((0, 0, width, height))
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

pub(crate) fn write_launch_display_config(contents: &str) -> Result<PathBuf> {
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
    fn retroarch_content_launch_disables_only_the_optional_desktop_menu() {
        let temporary = tempfile::tempdir().unwrap();
        let content = temporary.path().join("game.sfc");
        let executable = EmulatorExecutable::Native(PathBuf::from("retroarch"));
        let mut plan = LaunchPlan {
            emulator_name: "RetroArch".into(),
            program: PathBuf::from("retroarch"),
            arguments: vec![content.as_os_str().to_owned()],
            current_directory: temporary.path().to_path_buf(),
            environment: Vec::new(),
            cleanup_paths: Vec::new(),
            retroarch_content: Some(crate::emulator::PreparedRetroarchContent {
                core: PathBuf::from("snes9x_libretro.so"),
                content,
            }),
        };

        attach_launch_desktop_menu_override(&mut plan, &executable).unwrap();
        assert_eq!(plan.arguments[0], "--appendconfig");
        let config = fs::read_to_string(PathBuf::from(&plan.arguments[1])).unwrap();
        assert_eq!(
            config,
            "desktop_menu_enable = \"false\"\nconfig_save_on_exit = \"false\"\n"
        );
        assert_eq!(
            plan.arguments[2].as_os_str(),
            plan.retroarch_content.as_ref().unwrap().content.as_os_str()
        );
    }

    #[test]
    fn windowless_display_query_uses_native_pixels() {
        let metrics = parse_sdl3_display_metrics("5120x2160@1.3@wayland\n").unwrap();
        assert_eq!((metrics.width, metrics.height), (5120, 2160));
        assert_eq!(metrics.video_driver, "wayland");
        assert_eq!(
            scaled_display_dimensions((5120, 2160), metrics.pixel_density),
            Some((6656, 2808))
        );
        assert_eq!(ultrawide_viewport(5120, 2160), Some((0, 0, 2372, 1776)));
        assert_eq!(ultrawide_viewport(6656, 2808), Some((0, 0, 3084, 2309)));
        assert_eq!(
            scaled_display_dimensions((1920, 1080), 1.0),
            Some((1920, 1080))
        );
        assert_eq!(retroarch_render_scale("wayland", Some(""), 1.3), 1.3);
        assert_eq!(retroarch_render_scale("wayland", Some("x"), 1.3), 1.0);
        assert_eq!(retroarch_render_scale("x11", Some(""), 1.3), 1.0);
        assert_eq!(retroarch_render_scale("windows", None, 1.5), 1.0);
        assert!(parse_sdl3_display_metrics("no video output").is_none());
    }

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
    fn fitted_overlay_config_preserves_source_art_and_leaves_sidebars() {
        let temporary = tempfile::tempdir().unwrap();
        let image = temporary.path().join("art.png");
        let mut header = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        header.extend_from_slice(&1920_u32.to_be_bytes());
        header.extend_from_slice(&1080_u32.to_be_bytes());
        fs::write(&image, header).unwrap();
        let source = temporary.path().join("art.cfg");
        let original = "overlays = 1\noverlay0_overlay = \"art.png\"\noverlay0_full_screen = true\noverlay0_descs = 0\n";
        fs::write(&source, original).unwrap();

        let (prepared, black_sidebars) =
            aspect_fitted_overlay(&source, Some(5120.0 / 2160.0)).unwrap();
        assert!(black_sidebars);
        let fitted = fs::read_to_string(prepared).unwrap();
        assert!(fitted.contains("overlay0_rect = \"0.125000,0.000000,0.750000,1.000000\""));
        assert!(fitted.contains("overlay0_full_screen = true"));
        assert!(fitted.contains(&format!("overlay0_overlay = \"{}\"", image.display())));
        assert_eq!(fs::read_to_string(&source).unwrap(), original);

        let (same_aspect, no_sidebars) = aspect_fitted_overlay(&source, Some(16.0 / 9.0)).unwrap();
        assert_eq!(same_aspect, source);
        assert!(!no_sidebars);
    }

    #[test]
    fn bezel_choices_include_both_sources_for_snes() {
        let choices = bezel_choices("Super Nintendo Entertainment System");
        assert_eq!(
            choices.iter().map(|choice| choice.id).collect::<Vec<_>>(),
            [
                "system",
                "themed",
                "orionsangel",
                "orionsangel-plain",
                "ultrawide",
                "ultrawide-night"
            ]
        );
    }

    #[test]
    fn native_ultrawide_art_places_game_in_its_transparent_opening() {
        assert_eq!(ultrawide_viewport(5120, 2160), Some((0, 0, 2372, 1776)));
        assert_eq!(ultrawide_viewport(2560, 1080), Some((0, 0, 1186, 888)));
        assert_eq!(ultrawide_viewport(0, 1080), None);
        let (x, y, width, height) = ultrawide_viewport(1920, 1080).unwrap();
        assert_eq!((x, y), (0, 0));
        assert!(width < 1920 && height < 1080);
    }

    #[test]
    fn inherited_duimon_overlay_rebuilds_the_ultrawide_viewport() {
        let config = "input_overlay_enable = \"true\"\ninput_overlay = \"~/.local/share/lunchbox/bezels/duimon-ultrawide/Nintendo_SNES/SNES.cfg\"\ncustom_viewport_x = \"2114\"\n";
        assert_eq!(
            ultrawide_bezel_from_retroarch_config(config),
            Some("ultrawide")
        );
        assert_eq!(ultrawide_viewport(5120, 2160), Some((0, 0, 2372, 1776)));
        assert_eq!(
            ultrawide_bezel_from_retroarch_config(&config.replace("SNES.cfg", "NES_Night.cfg")),
            Some("ultrawide-night")
        );
        assert_eq!(
            ultrawide_bezel_from_retroarch_config(&config.replace("= \"true\"", "= \"false\"")),
            None
        );
        assert_eq!(
            ultrawide_bezel_from_retroarch_config(
                &config.replace("duimon-ultrawide", "other-pack")
            ),
            None
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
            let variant = install_retrotube_system_bezel_variant(root, &base, false).unwrap();
            let contents = fs::read_to_string(&variant).unwrap();
            assert!(contents.contains("DO_BEZEL = \"0.0\""));
            assert!(!contents.contains("DO_AMBILIGHT"));
            assert!(!contents.contains("DO_PIXELGRID = \"0.0\""));
            let pillarbox_variant =
                install_retrotube_system_bezel_variant(root, &base, true).unwrap();
            assert_ne!(variant, pillarbox_variant);
            let pillarbox_contents = fs::read_to_string(pillarbox_variant).unwrap();
            assert!(pillarbox_contents.contains("DO_AMBILIGHT = \"0.0\""));
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
