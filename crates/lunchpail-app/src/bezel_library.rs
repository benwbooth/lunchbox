//! User-selected artwork. Downloads and imports never execute overlay configs.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

const MAX_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const ARCADE_ART: &[(&str, &str, &str)] = &[
    (
        "arcade-duimon-vertical",
        "Duimon · vertical arcade · 21:9",
        "Arcade_Vertical.png",
    ),
    (
        "arcade-duimon-cab-vertical",
        "Duimon · vertical cabinet · 21:9",
        "Arcade_CAB_Vertical.png",
    ),
    (
        "arcade-duimon-horizontal",
        "Duimon · horizontal arcade · 21:9",
        "Arcade.png",
    ),
    (
        "arcade-duimon-cab-horizontal",
        "Duimon · horizontal cabinet · 21:9",
        "Arcade_CAB.png",
    ),
];
const SOURCE: &str = "https://github.com/Duimon/Duimon-Mega-Bezel-Potato-21x9";
const RAW: &str = "https://raw.githubusercontent.com/Duimon/Duimon-Mega-Bezel-Potato-21x9/main/Graphics/_Potato-21x9/Arcade/";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Opening {
    pub image_width: u32,
    pub image_height: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Opening {
    pub(crate) fn viewport(self, output_width: u32, output_height: u32) -> (i32, i32, u32, u32) {
        let scale = (output_width as f64 / self.image_width as f64)
            .min(output_height as f64 / self.image_height as f64);
        // RetroArch positions custom viewports relative to the screen center.
        let x = (self.x as f64 + self.width as f64 / 2.0 - self.image_width as f64 / 2.0) * scale;
        let y = (self.y as f64 + self.height as f64 / 2.0 - self.image_height as f64 / 2.0) * scale;
        (
            x.round() as i32,
            y.round() as i32,
            (self.width as f64 * scale).round().max(1.0) as u32,
            (self.height as f64 * scale).round().max(1.0) as u32,
        )
    }
}

#[derive(Serialize, Deserialize)]
struct Artwork {
    id: String,
    label: String,
    source: String,
    opening: Opening,
}

pub(crate) fn is_choice(id: &str) -> bool {
    ARCADE_ART.iter().any(|entry| entry.0 == id) || custom_hash(id).is_some()
}

fn custom_hash(id: &str) -> Option<&str> {
    id.strip_prefix("custom:").filter(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn root() -> Result<PathBuf> {
    crate::app_paths::project_dirs()
        .map(|dirs| dirs.data_local_dir().join("bezels/library"))
        .context("Could not find the artwork library")
}

fn directory(root: &Path, id: &str) -> Result<PathBuf> {
    ensure!(is_choice(id), "Unknown artwork selection");
    Ok(root.join(custom_hash(id).unwrap_or(id)))
}

pub(crate) fn label(id: &str) -> String {
    if let Some(entry) = ARCADE_ART.iter().find(|entry| entry.0 == id) {
        return entry.1.into();
    }
    root()
        .and_then(|root| metadata(&directory(&root, id)?))
        .map(|art| art.label)
        .unwrap_or_else(|_| "Custom artwork (unavailable)".into())
}

fn metadata(directory: &Path) -> Result<Artwork> {
    let bytes = fs::read(directory.join("bezel.json"))?;
    ensure!(bytes.len() < 16 * 1024, "Invalid artwork metadata");
    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) fn choices(platform: &str) -> Result<Vec<serde_json::Value>> {
    let mut rows = vec![
        serde_json::json!({"id":"", "label":"Use inherited artwork", "source":""}),
        serde_json::json!({"id":"off", "label":"No artwork", "source":""}),
    ];
    for choice in crate::display_setup::bezel_choices(platform) {
        rows.push(serde_json::json!({"id":choice.id, "label":choice.label,
            "source": if choice.id.starts_with("arcade-duimon-") { SOURCE } else { "" }}));
    }
    let root = root()?;
    if root.is_dir() {
        let mut custom = Vec::new();
        for entry in fs::read_dir(root)?.take(1000).flatten() {
            if let Ok(art) = metadata(&entry.path()) {
                if custom_hash(&art.id).is_some_and(|hash| Some(hash) == entry.file_name().to_str())
                {
                    custom.push(serde_json::json!({"id":art.id, "label":art.label, "source":"Imported PNG"}));
                }
            }
        }
        custom.sort_by_key(|row| row["label"].as_str().unwrap_or("").to_lowercase());
        rows.extend(custom);
    }
    Ok(rows)
}

pub(crate) fn overlay(id: &str) -> Result<Option<PathBuf>> {
    let dir = directory(&root()?, id)?;
    let path = dir.join("bezel.cfg");
    if path.is_file() && dir.join("artwork.png").is_file() && metadata(&dir).is_ok() {
        return Ok(Some(path));
    }
    let entry = ARCADE_ART
        .iter()
        .find(|entry| entry.0 == id)
        .context("Imported artwork is missing. Import the PNG again.")?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent("Lunchpail bezel picker")
        .build()
        .into();
    let mut response = agent.get(format!("{RAW}{}", entry.2)).call()?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    store_art(&dir, id, entry.1, SOURCE, &bytes)?;
    Ok(Some(path))
}

pub(crate) fn import(path: &Path) -> Result<String> {
    import_at(&root()?, path)
}

fn import_at(root: &Path, path: &Path) -> Result<String> {
    ensure!(path.is_file(), "Choose a PNG image file");
    ensure!(
        fs::metadata(path)?.len() <= MAX_BYTES,
        "Artwork must be smaller than 16 MB"
    );
    let bytes = fs::read(path)?;
    use sha2::{Digest, Sha256};
    let id = format!("custom:{:x}", Sha256::digest(&bytes));
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom artwork");
    let name: String = name.chars().filter(|c| !c.is_control()).take(120).collect();
    store_art(&directory(root, &id)?, &id, &name, "Imported PNG", &bytes)?;
    Ok(id)
}

fn store_art(dir: &Path, id: &str, label: &str, source: &str, bytes: &[u8]) -> Result<()> {
    let opening = detect_opening(bytes)?;
    fs::create_dir_all(dir)?;
    // Complete each file before publishing it. No imported paths/configuration
    // survive; the source PNG remains unchanged and can subsequently be moved.
    let art = Artwork {
        id: id.into(),
        label: label.into(),
        source: source.into(),
        opening,
    };
    for (name, data) in [("artwork.png", bytes.to_vec()), ("bezel.json", serde_json::to_vec(&art)?),
        ("bezel.cfg", b"overlays = 1\noverlay0_overlay = \"artwork.png\"\noverlay0_full_screen = true\noverlay0_descs = 0\n".to_vec())] {
        let mut file = tempfile::NamedTempFile::new_in(dir)?;
        std::io::Write::write_all(&mut file, &data)?;
        file.persist(dir.join(name))?;
    }
    Ok(())
}

/// Artwork must have one enclosed transparent screen through its center.
/// Flood fill ignores decorative transparency and transparent outside margins.
fn detect_opening(bytes: &[u8]) -> Result<Opening> {
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "Artwork must be smaller than 16 MB"
    );
    let (w, h) =
        crate::bezel_orionsangel::png_dimensions(bytes).context("Choose a valid PNG image")?;
    ensure!(
        w >= 32 && h >= 32 && w <= 8192 && h <= 8192 && u64::from(w) * u64::from(h) <= 16_777_216,
        "Artwork dimensions must be 32–8192 pixels, at most 16 megapixels"
    );
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
    let transparent = |index: usize| image.as_raw()[index * 4 + 3] < 32;
    let center = (h / 2 * w + w / 2) as usize;
    ensure!(
        transparent(center),
        "The PNG needs a transparent screen opening through its center (not a black-filled screen)"
    );
    let mut visited = vec![false; (w * h) as usize];
    let mut queue = vec![center];
    visited[center] = true;
    let (mut left, mut top, mut right, mut bottom) = (w, h, 0, 0);
    let mut cursor = 0;
    while cursor < queue.len() {
        let index = queue[cursor];
        cursor += 1;
        let (x, y) = (index as u32 % w, index as u32 / w);
        ensure!(
            x > 0 && y > 0 && x < w - 1 && y < h - 1,
            "The transparent screen must be enclosed by artwork, not open to the image edge"
        );
        left = left.min(x);
        right = right.max(x);
        top = top.min(y);
        bottom = bottom.max(y);
        for next in [index - 1, index + 1, index - w as usize, index + w as usize] {
            if !visited[next] && transparent(next) {
                visited[next] = true;
                queue.push(next);
            }
        }
    }
    ensure!(
        right - left >= 16 && bottom - top >= 16,
        "The transparent screen opening is too small"
    );
    Ok(Opening {
        image_width: w,
        image_height: h,
        x: left,
        y: top,
        width: right - left + 1,
        height: bottom - top + 1,
    })
}

pub(crate) fn opening(path: &Path) -> Result<Opening> {
    let art = metadata(path.parent().context("Missing artwork directory")?)?;
    let o = art.opening;
    ensure!(
        o.width > 0
            && o.height > 0
            && o.image_width > 0
            && o.image_height > 0
            && o.x.checked_add(o.width).is_some_and(|v| v <= o.image_width)
            && o.y
                .checked_add(o.height)
                .is_some_and(|v| v <= o.image_height),
        "Invalid artwork opening"
    );
    Ok(o)
}

pub(crate) fn preview(platform: &str, rom: &str, id: &str) -> Result<String> {
    let cfg = crate::display_setup::resolve_bezel_overlay(platform, rom, id)?
        .context("This source has no matching artwork for this game")?;
    let text = fs::read_to_string(&cfg)?;
    let name = crate::display_setup::config_value_from_text(&text, "overlay0_overlay")
        .context("Artwork has no image")?;
    let image = cfg
        .parent()
        .context("Missing artwork directory")?
        .join(name);
    ensure!(image.is_file(), "Artwork image is missing");
    Ok(url::Url::from_file_path(image)
        .map_err(|_| anyhow::anyhow!("Invalid artwork path"))?
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png() -> Vec<u8> {
        let mut image = image::RgbaImage::from_pixel(160, 90, image::Rgba([40, 40, 40, 255]));
        for y in 10..80 {
            for x in 45..115 {
                image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }
    #[test]
    fn imports_are_owned_deduplicated_and_keep_original_pixels() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("My bezel.png");
        let bytes = png();
        fs::write(&source, &bytes).unwrap();
        let root = tmp.path().join("library");
        let id = import_at(&root, &source).unwrap();
        assert_eq!(import_at(&root, &source).unwrap(), id);
        fs::remove_file(source).unwrap();
        let dir = directory(&root, &id).unwrap();
        assert_eq!(fs::read(dir.join("artwork.png")).unwrap(), bytes);
        assert_eq!(metadata(&dir).unwrap().label, "My bezel");
        assert_eq!(
            opening(&dir.join("bezel.cfg")).unwrap(),
            Opening {
                image_width: 160,
                image_height: 90,
                x: 45,
                y: 10,
                width: 70,
                height: 70
            }
        );
    }
    #[test]
    fn rejects_unsafe_ids_opaque_images_and_unbounded_transparency() {
        for id in [
            "custom:../image",
            "custom:/tmp/thing",
            "arcade-duimon-unknown",
            "custom:abc",
        ] {
            assert!(!is_choice(id));
        }
        for alpha in [0, 255] {
            let image = image::RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, alpha]));
            let mut out = std::io::Cursor::new(Vec::new());
            image.write_to(&mut out, image::ImageFormat::Png).unwrap();
            assert!(detect_opening(&out.into_inner()).is_err());
        }
        assert!(detect_opening(b"not png").is_err());
    }
    #[test]
    #[ignore = "requires unmodified Duimon artwork downloaded for local verification"]
    fn inspected_duimon_artwork_has_enclosed_opening() {
        let root = PathBuf::from(std::env::var("LUNCHPAIL_BEZEL_FIXTURE_DIRECTORY").unwrap());
        for (_, _, file) in ARCADE_ART {
            let o = detect_opening(&fs::read(root.join(file)).unwrap()).unwrap();
            assert_eq!(o.image_width, 2560);
            assert_eq!(o.image_height, 1080);
            assert_eq!(o.height > o.width, file.contains("Vertical"));
            eprintln!("{file}: {o:?}");
            if let Ok(base) = std::env::var("LUNCHPAIL_BEZEL_KOKO_PRESET") {
                let preset = root.join(file).with_extension("slangp");
                fs::write(&preset, format!("#reference \"{base}\"\nDO_DYNZOOM = 0.0\nDO_CURVATURE = 1.0\nAUTOCROP_MAX = 0.0\nDO_GAME_GEOM_OVERRIDE = 0.0\nGLOBAL_ZOOM = 1.0\nMIN_LINES_INTERLACED = 0.0\nPIXELGRID_INTR_FLICK_MODE = 0.0\nDO_BEZEL = 0.0\n")).unwrap();
                crate::retrotube_artwork::install_with_opening(
                    Path::new(&base),
                    &preset,
                    &root.join(file),
                    false,
                    Some(o),
                )
                .unwrap();
                assert!(
                    fs::read_to_string(&preset)
                        .unwrap()
                        .contains("shaders = \"18\"")
                );
            }
        }
    }

    #[test]
    fn fitting_preserves_art_aspect_and_centers_viewport_offsets() {
        let o = Opening {
            image_width: 2560,
            image_height: 1080,
            x: 955,
            y: 96,
            width: 650,
            height: 888,
        };
        assert_eq!(o.viewport(2560, 1080), (0, 0, 650, 888));
        assert_eq!(o.viewport(5120, 1440), (0, 0, 867, 1184));
        let shifted = Opening { x: 905, y: 56, ..o };
        assert_eq!(shifted.viewport(2560, 1080), (-50, -40, 650, 888));
    }
}
