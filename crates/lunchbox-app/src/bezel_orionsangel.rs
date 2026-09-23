//! Duimon's Orionsangel console artwork and native 21:9 Potato artwork.
//! Only unmodified transparent PNGs are fetched at runtime and used as
//! RetroArch overlays, leaving the selected CRT shader independent. The
//! 21:9 pack is CC BY-NC-ND 4.0; no artwork is vendored or transformed.

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};

const RAW_BASE: &str =
    "https://raw.githubusercontent.com/Duimon/Orionsangel-Original-Console/main/Graphics/";
const ULTRAWIDE_RAW_BASE: &str = "https://raw.githubusercontent.com/Duimon/Duimon-Mega-Bezel-Potato-21x9/main/Graphics/_Potato-21x9/";
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
pub const ULTRAWIDE_DIMENSIONS: (u32, u32) = (2560, 1080);
/// All inspected default and night images use this transparent 4:3 opening.
/// The viewport is set to this rectangle at launch so the overlay cannot
/// silently cover the sides of the game.
pub const ULTRAWIDE_SCREEN_OPENING: (u32, u32, u32, u32) = (687, 96, 1186, 888);

#[derive(Clone, Copy)]
struct Art {
    folder: &'static str,
    main: &'static str,
    plain: &'static str,
}

fn art_for_platform(platform: &str) -> Option<Art> {
    use crate::bezel_project::theme_for_platform;
    if platform.eq_ignore_ascii_case("Famicom") {
        return Some(Art {
            folder: "Nintendo_Famicom",
            main: "Famicom.png",
            plain: "FamicomPlain.png",
        });
    }
    let theme = theme_for_platform(platform).or_else(|| {
        let lower = platform.to_ascii_lowercase();
        if lower.contains("gamecube") {
            Some("GameCube")
        } else if lower.contains("playstation 2") {
            Some("PS2")
        } else {
            None
        }
    })?;
    let art = match theme {
        "SNES" => Art {
            folder: "Nintendo_SNES",
            main: "snes.png",
            plain: "snesplain.png",
        },
        "SFC" => Art {
            folder: "Nintendo_Super_Famicom",
            main: "Superfamicom.png",
            plain: "Superfamplain.png",
        },
        "NES" => Art {
            folder: "Nintendo_NES",
            main: "NES.png",
            plain: "NESplain.png",
        },
        "N64" => Art {
            folder: "Nintendo_N64",
            main: "N64.png",
            plain: "N64plain.png",
        },
        "MegaDrive" => Art {
            folder: "SEGA_Genesis",
            main: "genesis.png",
            plain: "genesisplain.png",
        },
        "MasterSystem" => Art {
            folder: "SEGA_Master_System",
            main: "sms.png",
            plain: "smsplain.png",
        },
        "SegaCD" => Art {
            folder: "SEGA_CD",
            main: "SegaCD.png",
            plain: "SegaCDPlain.png",
        },
        "Sega32X" => Art {
            folder: "SEGA_32X",
            main: "32x.png",
            plain: "32xplain.png",
        },
        "Saturn" => Art {
            folder: "SEGA_Saturn",
            main: "SegaSaturn.png",
            plain: "SegaSaturnPlain.png",
        },
        "Dreamcast" => Art {
            folder: "SEGA_Dreamcast",
            main: "DC1.png",
            plain: "DCPLAIN1.png",
        },
        "PCEngine" => Art {
            folder: "NEC_PC_Engine",
            main: "PCEngine.png",
            plain: "PCEnginePlain.png",
        },
        "PCE-CD" => Art {
            folder: "NEC_PC_Engine_CD",
            main: "TurboCD2.png",
            plain: "TurboCD2Plain.png",
        },
        "TG16" => Art {
            folder: "NEC_TurboGrafx_16",
            main: "tg16.png",
            plain: "tg16plain.png",
        },
        "TG-CD" => Art {
            folder: "NEC_TurboGrafx_CD",
            main: "TurboCD.png",
            plain: "TurboCDPlain.png",
        },
        "Atari2600" => Art {
            folder: "Atari_2600",
            main: "atarivcs.png",
            plain: "atariplain.png",
        },
        "Atari5200" => Art {
            folder: "Atari_5200",
            main: "atari5200.png",
            plain: "atari5200plain.png",
        },
        "AtariJaguar" => Art {
            folder: "Atari_Jaguar",
            main: "Jaguar.png",
            plain: "JaguarPlain.png",
        },
        "ColecoVision" => Art {
            folder: "ColecoVision",
            main: "CV.png",
            plain: "CVplain.png",
        },
        "Intellivision" => Art {
            folder: "Mattel_Intellivision",
            main: "InTele.png",
            plain: "InTelePlain.png",
        },
        "PSX" => Art {
            folder: "SONY_Playstation",
            main: "Playstation.png",
            plain: "Playstationplain.png",
        },
        "GameCube" => Art {
            folder: "Nintendo_Gamecube",
            main: "Gamecube.png",
            plain: "GamecubePlain.png",
        },
        "PS2" => Art {
            folder: "SONY_Playstation_2",
            main: "PS2.png",
            plain: "PS2plain.png",
        },
        _ => return None,
    };
    Some(art)
}

pub fn supported(platform: &str) -> bool {
    art_for_platform(platform).is_some()
}

fn ultrawide_basename(folder: &str) -> Option<&'static str> {
    Some(match folder {
        "Nintendo_SNES" => "SNES",
        "Nintendo_Super_Famicom" => "Super_Famicom",
        "Nintendo_NES" => "NES",
        "Nintendo_Famicom" => "Famicom",
        "Nintendo_N64" => "N64",
        "Nintendo_Gamecube" => "Gamecube",
        "SEGA_Genesis" => "Genesis",
        "SEGA_Master_System" => "SMS",
        "SEGA_CD" => "SEGACD",
        "SEGA_32X" => "SEGA32X",
        "SEGA_Saturn" => "Saturn",
        "SEGA_Dreamcast" => "Dreamcast",
        "NEC_PC_Engine" => "PC_Engine",
        "NEC_PC_Engine_CD" => "PC_Engine_CD",
        "NEC_TurboGrafx_16" => "TurboGrafx16",
        "NEC_TurboGrafx_CD" => "TurboGrafx_CD",
        "Atari_2600" => "2600",
        "Atari_5200" => "5200",
        "Atari_Jaguar" => "Jaguar",
        "ColecoVision" => "ColecoVision",
        "Mattel_Intellivision" => "Intellivision",
        "SONY_Playstation" => "Playstation",
        "SONY_Playstation_2" => "Playstation2",
        _ => return None,
    })
}

pub fn ultrawide_supported(platform: &str) -> bool {
    art_for_platform(platform)
        .and_then(|art| ultrawide_basename(art.folder))
        .is_some()
}

pub fn overlay(platform: &str, plain: bool) -> Result<Option<PathBuf>> {
    let Some(art) = art_for_platform(platform) else {
        return Ok(None);
    };
    let file_name = if plain { art.plain } else { art.main };
    let root = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("bezels/orionsangel"))
        .context("could not determine the Lunchbox bezel directory")?;
    let directory = root.join(art.folder);
    let url = format!("{RAW_BASE}{}/{}", art.folder, file_name);
    cached_overlay(directory, file_name, &url, None).map(Some)
}

pub fn ultrawide_overlay(platform: &str, night: bool) -> Result<Option<PathBuf>> {
    let Some(art) = art_for_platform(platform) else {
        return Ok(None);
    };
    let Some(basename) = ultrawide_basename(art.folder) else {
        return Ok(None);
    };
    let file_name = if night {
        format!("{basename}_Night.png")
    } else {
        format!("{basename}.png")
    };
    let root = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("bezels/duimon-ultrawide"))
        .context("could not determine the Lunchbox bezel directory")?;
    let directory = root.join(art.folder);
    let url = format!("{ULTRAWIDE_RAW_BASE}{}/{}", art.folder, file_name);
    cached_overlay(directory, &file_name, &url, Some(ULTRAWIDE_DIMENSIONS)).map(Some)
}

fn cached_overlay(
    directory: PathBuf,
    file_name: &str,
    url: &str,
    expected_dimensions: Option<(u32, u32)>,
) -> Result<PathBuf> {
    fs::create_dir_all(&directory)?;
    let image = directory.join(file_name);
    if !image.is_file() {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .user_agent("Lunchbox/0.1 Duimon console artwork client")
            .build()
            .into();
        let mut response = agent
            .get(url)
            .call()
            .with_context(|| format!("downloading {url}"))?;
        if !response.status().is_success() {
            bail!(
                "console artwork download returned HTTP {}",
                response.status()
            );
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_IMAGE_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES
            || !image_dimensions_match(&bytes, expected_dimensions)
        {
            bail!("console artwork was not a supported PNG image");
        }
        let mut staged = tempfile::NamedTempFile::new_in(&directory)?;
        use std::io::Write;
        staged.write_all(&bytes)?;
        staged.persist(&image).map_err(|error| error.error)?;
    }
    let image_bytes = fs::read(&image)?;
    if !image_dimensions_match(&image_bytes, expected_dimensions) {
        bail!("cached console artwork is not a supported PNG image");
    }
    let config = directory.join(format!("{}.cfg", file_name.trim_end_matches(".png")));
    let contents = format!(
        "overlays = 1\noverlay0_overlay = \"{file_name}\"\noverlay0_full_screen = true\noverlay0_descs = 0\n"
    );
    fs::write(&config, contents)?;
    Ok(config)
}

fn image_dimensions_match(bytes: &[u8], expected: Option<(u32, u32)>) -> bool {
    png_dimensions(bytes).is_some_and(|dimensions| expected.is_none_or(|value| dimensions == value))
}

pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    (width > 0 && height > 0 && width <= 16384 && height <= 16384).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snes_console_variants_are_separate_from_bezel_project() {
        let art = art_for_platform("Super Nintendo Entertainment System").unwrap();
        assert_eq!(art.folder, "Nintendo_SNES");
        assert_eq!(art.main, "snes.png");
        assert_eq!(art.plain, "snesplain.png");
        assert_eq!(ultrawide_basename(art.folder), Some("SNES"));
        assert!(ultrawide_supported("Super Nintendo Entertainment System"));
        assert!(!supported("MAME"));
        assert!(!ultrawide_supported("MAME"));
    }

    #[test]
    fn png_dimensions_reject_invalid_or_implausible_headers() {
        assert_eq!(png_dimensions(b"not a PNG"), None);
        let mut bytes = [0u8; 24];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&2560u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&1440u32.to_be_bytes());
        assert_eq!(png_dimensions(&bytes), Some((2560, 1440)));
    }

    #[test]
    fn ultrawide_cache_generates_a_fullscreen_overlay_without_network() {
        let directory = tempfile::tempdir().unwrap();
        let mut bytes = [0u8; 24];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&2560u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&1080u32.to_be_bytes());
        fs::write(directory.path().join("SNES.png"), bytes).unwrap();
        let config = cached_overlay(
            directory.path().to_owned(),
            "SNES.png",
            "https://example.invalid/not-used",
            Some(ULTRAWIDE_DIMENSIONS),
        )
        .unwrap();
        let contents = fs::read_to_string(config).unwrap();
        assert!(contents.contains("overlay0_overlay = \"SNES.png\""));
        assert!(contents.contains("overlay0_full_screen = true"));
        assert!(contents.contains("overlay0_descs = 0"));
    }
}
