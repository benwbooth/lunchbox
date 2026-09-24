//! Session-scoped, localhost-only RetroArch AI Service to Ollama bridge.
//! No screenshot is persisted or sent to a remote service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use fontdb::{Database, Family, Query};
use fontdue::{Font, FontSettings};
use image::RgbImage;
use ocrs_cjk::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

const OLLAMA_URL: &str = "http://127.0.0.1:11434";
const OCR_MODEL: &str = "glm-ocr:latest";
// PaddlePaddle's Apache-2.0 PP-OCRv6 tiny text detector, pinned to this model
// revision and SHA-256. We download it only when the user installs models.
const DETECTOR_URL: &str = "https://huggingface.co/PaddlePaddle/PP-OCRv6_tiny_det_onnx/resolve/2ba1506c0380b8f0b03dd142459aac66d4421f6c/inference.onnx";
const DETECTOR_SHA256: &str = "193bab7a04fca699a6c82e6abb5b81bdb28177f0abd4062552b04908dafb19f8";
const MAX_DETECTOR_BYTES: u64 = 3 * 1024 * 1024;
// RetroArch uses F8 for screenshots by default. Keep that binding intact.
const TRANSLATION_HOTKEY: &str = "f10";
// A 5120×2160 24-bit BMP becomes roughly 44 MiB after base64 encoding.
const MAX_REQUEST_BYTES: usize = 80 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_FRAME_PIXELS: u64 = 16_000_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranslationSettings {
    pub enabled: bool,
    pub model: String,
    pub source_language: String,
}

impl Default for TranslationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "translategemma:12b".to_owned(),
            source_language: "auto".to_owned(),
        }
    }
}

impl TranslationSettings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            supported_model(&self.model),
            "unsupported local translation model"
        );
        ensure!(
            self.source_language == "auto"
                || (self.source_language.len() >= 2
                    && self.source_language.len() <= 16
                    && self
                        .source_language
                        .bytes()
                        .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')),
            "source language must be auto or a language code such as ja or fr"
        );
        Ok(())
    }
}

fn supported_model(model: &str) -> bool {
    matches!(
        model,
        "translategemma:4b" | "translategemma:12b" | "translategemma:27b"
    )
}

fn http_agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(2)))
        .timeout_global(Some(timeout))
        .proxy(None)
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}

fn detector_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("finding the local model cache")?;
    Ok(dirs
        .cache_dir()
        .join("translation")
        .join("pp-ocrv6-tiny-det-2ba1506c.onnx"))
}

fn detector_available() -> Result<bool> {
    let path = detector_path()?;
    if !path.is_file() {
        return Ok(false);
    }
    let bytes = std::fs::read(&path).context("reading local text detector")?;
    Ok(hex::encode(Sha256::digest(bytes)) == DETECTOR_SHA256)
}

fn download_detector(cancelled: &AtomicBool) -> Result<()> {
    if detector_available()? {
        return Ok(());
    }
    ensure!(
        !cancelled.load(Ordering::Relaxed),
        "text detector download cancelled"
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(120)))
        .max_redirects(5)
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .get(DETECTOR_URL)
        .call()
        .context("downloading Apache-licensed text detector")?;
    ensure!(
        response.status().as_u16() == 200,
        "text detector download failed"
    );
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_DETECTOR_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_DETECTOR_BYTES,
        "text detector download exceeded size limit"
    );
    ensure!(
        hex::encode(Sha256::digest(&bytes)) == DETECTOR_SHA256,
        "text detector checksum did not match the pinned model"
    );
    ensure!(
        !cancelled.load(Ordering::Relaxed),
        "text detector download cancelled"
    );
    let path = detector_path()?;
    let directory = path
        .parent()
        .context("text detector has no cache directory")?;
    std::fs::create_dir_all(directory)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(&bytes)?;
    temporary
        .persist(&path)
        .context("installing text detector")?;
    Ok(())
}

fn load_detector() -> Result<OcrEngine> {
    ensure!(
        detector_available()?,
        "text placement model is not installed; download models in Settings"
    );
    let model = Model::load_file(detector_path()?).context("loading local text detector")?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(model),
        ..Default::default()
    })
    .context("initializing local text detector")
}

pub fn model_available(model: &str) -> Result<bool> {
    ensure!(
        supported_model(model),
        "unsupported local translation model"
    );
    let mut response = http_agent(Duration::from_secs(5))
        .get(&format!("{OLLAMA_URL}/api/tags"))
        .call()
        .context("connecting to local Ollama")?;
    ensure!(
        response.status().as_u16() == 200,
        "Ollama model list failed"
    );
    let body = response
        .body_mut()
        .read_to_string()
        .context("reading Ollama model list")?;
    let body: Value = serde_json::from_str(&body).context("parsing Ollama model list")?;
    let ollama_ready = body["models"].as_array().is_some_and(|models| {
        [model, OCR_MODEL]
            .iter()
            .all(|required| models.iter().any(|entry| entry["name"] == *required))
    });
    Ok(ollama_ready && detector_available()?)
}

pub fn pull_model(
    model: &str,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u8, String),
) -> Result<()> {
    ensure!(
        supported_model(model),
        "unsupported local translation model"
    );
    for (index, required) in [OCR_MODEL, model].into_iter().enumerate() {
        let mut response = http_agent(Duration::from_secs(60 * 60))
            .post(&format!("{OLLAMA_URL}/api/pull"))
            .send_json(json!({"model": required, "stream": true}))
            .context("starting local Ollama model download")?;
        ensure!(
            response.status().as_u16() == 200,
            "Ollama refused the model download"
        );
        let mut complete = false;
        for line in BufReader::new(response.body_mut().as_reader()).lines() {
            if cancelled.load(Ordering::Relaxed) {
                bail!("translation model download cancelled");
            }
            let event: Value = serde_json::from_str(&line.context("reading download progress")?)
                .context("parsing Ollama download progress")?;
            if let Some(error) = event["error"].as_str() {
                bail!("Ollama model download failed: {error}");
            }
            let status = event["status"].as_str().unwrap_or("Downloading model");
            let fraction = match (event["completed"].as_u64(), event["total"].as_u64()) {
                (Some(done), Some(total)) if total > 0 => (done * 49 / total).min(49) as u8,
                _ => 0,
            };
            progress(index as u8 * 50 + fraction, format!("{required}: {status}"));
            if status == "success" {
                complete = true;
            }
        }
        ensure!(complete, "Ollama did not finish downloading {required}");
    }
    progress(99, "Installing precise text placement model".to_owned());
    download_detector(cancelled)?;
    ensure!(model_available(model)?, "local models are incomplete");
    progress(
        100,
        "OCR, translation, and text placement are ready".to_owned(),
    );
    Ok(())
}

pub struct TranslationSession {
    stop: Arc<AtomicBool>,
}

impl TranslationSession {
    pub fn attach(
        plan: &mut LaunchPlan,
        executable: &EmulatorExecutable,
        settings: &TranslationSettings,
    ) -> Result<Option<Self>> {
        if !settings.enabled || plan.retroarch_content.is_none() {
            return Ok(None);
        }
        settings.validate()?;
        ensure!(
            model_available(&settings.model)?,
            "{OCR_MODEL}, {}, and the text placement model are required; download them in Settings",
            settings.model
        );
        let detector = load_detector()?;
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
            .context("opening local translation bridge")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let secret = uuid::Uuid::new_v4().simple().to_string();
        let config = retroarch_session_config(port, &secret);
        let path = crate::display_setup::write_launch_display_config(&config)?;
        crate::controller_launch::attach_config(plan, executable, &path)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let settings = settings.clone();
        thread::Builder::new()
            .name("lunchbox-translation-bridge".into())
            .spawn(move || serve(listener, &secret, &settings, &detector, &worker_stop))
            .context("starting local translation bridge")?;
        Ok(Some(Self { stop }))
    }
}

fn retroarch_session_config(port: u16, secret: &str) -> String {
    format!(
        "ai_service_enable = \"true\"\nai_service_url = \"http://127.0.0.1:{port}/{secret}\"\nai_service_mode = \"0\"\nai_service_source_lang = \"0\"\nai_service_target_lang = \"1\"\nai_service_pause = \"false\"\nmenu_enable_widgets = \"true\"\ninput_ai_service = \"{TRANSLATION_HOTKEY}\"\n"
    )
}

impl Drop for TranslationSession {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn serve(
    listener: TcpListener,
    secret: &str,
    settings: &TranslationSettings,
    detector: &OcrEngine,
    stop: &AtomicBool,
) {
    let mut cached: Option<CachedTranslation> = None;
    let mut last_request_at = None;
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                if let Err(error) = handle_request(
                    &mut stream,
                    secret,
                    settings,
                    Some(detector),
                    &mut cached,
                    &mut last_request_at,
                ) {
                    eprintln!("LUNCHBOX_TRANSLATION_REQUEST_FAILED: {error:#}");
                    let _ = write_json(&mut stream, 200, &json!({"error": error.to_string()}));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                eprintln!("LUNCHBOX_TRANSLATION_BRIDGE_FAILED: {error}");
                break;
            }
        }
    }
}

struct CachedTranslation {
    digest: [u8; 32],
    regions: Vec<TranslatedRegion>,
    dimensions: (u32, u32),
    overlay: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TextRect {
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
}

impl TextRect {
    fn width(self) -> u32 {
        self.x2.saturating_sub(self.x1)
    }

    fn height(self) -> u32 {
        self.y2.saturating_sub(self.y1)
    }

    fn union(self, other: Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    fn padded(self, width: u32, height: u32, padding: u32) -> Self {
        Self {
            x1: self.x1.saturating_sub(padding),
            y1: self.y1.saturating_sub(padding),
            x2: self.x2.saturating_add(padding).min(width),
            y2: self.y2.saturating_add(padding).min(height),
        }
    }

    fn near(self, other: Self) -> bool {
        self.x1.abs_diff(other.x1) <= 5
            && self.y1.abs_diff(other.y1) <= 5
            && self.x2.abs_diff(other.x2) <= 5
            && self.y2.abs_diff(other.y2) <= 5
    }
}

#[derive(Clone, Debug)]
struct TranslatedRegion {
    rect: TextRect,
    source: String,
    english: String,
    background: [u8; 3],
}

fn handle_request(
    stream: &mut TcpStream,
    secret: &str,
    settings: &TranslationSettings,
    detector: Option<&OcrEngine>,
    cached: &mut Option<CachedTranslation>,
    last_request_at: &mut Option<Instant>,
) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let request_line = read_http_line(&mut reader)?;
    let expected_path = format!("/{secret}");
    let fields: Vec<_> = request_line.split_whitespace().collect();
    ensure!(
        fields.len() == 3
            && fields[0] == "POST"
            && fields[1].split('?').next() == Some(expected_path.as_str()),
        "invalid translation request"
    );
    let mut content_length = None;
    loop {
        let header = read_http_line(&mut reader)?;
        if header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }
    let length = content_length.context("missing content length")?;
    ensure!(
        length <= MAX_REQUEST_BYTES,
        "translation screenshot too large"
    );
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let request: Value = serde_json::from_slice(&body).context("parsing RetroArch screenshot")?;
    let image = request["image"].as_str().context("missing screenshot")?;
    ensure!(
        image.len() <= MAX_REQUEST_BYTES,
        "translation screenshot too large"
    );
    let decoded = BASE64
        .decode(image)
        .context("decoding RetroArch screenshot")?;
    let (width, height, model_image) = if decoded.starts_with(b"\x89PNG\r\n\x1a\n") {
        let (width, height) = png_dimensions(&decoded)?;
        (width, height, image.to_owned())
    } else {
        let (width, height, png) = bmp_to_png(&decoded)?;
        (width, height, BASE64.encode(png))
    };
    if let Some(last) = last_request_at {
        let wait = Duration::from_millis(1500).saturating_sub(last.elapsed());
        if !wait.is_zero() {
            thread::sleep(wait);
        }
    }
    *last_request_at = Some(Instant::now());
    let digest: [u8; 32] = Sha256::digest(&decoded).into();
    let (regions, overlay) = if let Some(old) = cached.as_ref() {
        if old.digest == digest {
            (old.regions.clone(), old.overlay.clone())
        } else {
            render_translation(
                settings,
                &model_image,
                width,
                height,
                detector.context("text detector is not ready")?,
                cached.as_ref(),
            )?
        }
    } else {
        render_translation(
            settings,
            &model_image,
            width,
            height,
            detector.context("text detector is not ready")?,
            None,
        )?
    };
    *cached = Some(CachedTranslation {
        digest,
        regions,
        dimensions: (width, height),
        overlay: overlay.clone(),
    });
    write_json(stream, 200, &json!({"image": overlay, "auto": "auto"}))?;
    Ok(())
}

fn read_http_line(reader: &mut impl BufRead) -> Result<String> {
    let mut line = String::new();
    let bytes = reader.take(4097).read_line(&mut line)?;
    ensure!(
        bytes > 0 && bytes <= 4096 && line.ends_with('\n'),
        "invalid or oversized HTTP request line"
    );
    Ok(line)
}

fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    ensure!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n",
        "screenshot is not PNG"
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    validate_frame_dimensions(width, height)?;
    Ok((width, height))
}

fn validate_frame_dimensions(width: u32, height: u32) -> Result<()> {
    ensure!(
        (64..=7680).contains(&width)
            && (64..=4320).contains(&height)
            && u64::from(width) * u64::from(height) <= MAX_FRAME_PIXELS,
        "screenshot dimensions are unsupported"
    );
    Ok(())
}

fn bmp_to_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    ensure!(
        bytes.len() >= 54 && &bytes[..2] == b"BM",
        "screenshot is not PNG or BMP"
    );
    let data_offset = u32::from_le_bytes(bytes[10..14].try_into()?) as usize;
    let dib_size = u32::from_le_bytes(bytes[14..18].try_into()?);
    ensure!(dib_size >= 40, "unsupported BMP header");
    let width = i32::from_le_bytes(bytes[18..22].try_into()?);
    let signed_height = i32::from_le_bytes(bytes[22..26].try_into()?);
    let planes = u16::from_le_bytes(bytes[26..28].try_into()?);
    let bits_per_pixel = u16::from_le_bytes(bytes[28..30].try_into()?);
    let compression = u32::from_le_bytes(bytes[30..34].try_into()?);
    ensure!(width > 0 && signed_height != 0, "invalid BMP dimensions");
    let width = width as u32;
    let height = signed_height.unsigned_abs();
    validate_frame_dimensions(width, height)?;
    ensure!(
        planes == 1 && bits_per_pixel == 24 && compression == 0,
        "unsupported BMP pixel format"
    );
    let row_stride = (width as usize * 3 + 3) & !3;
    let pixel_bytes = row_stride
        .checked_mul(height as usize)
        .context("BMP size overflow")?;
    ensure!(
        data_offset >= 54
            && data_offset
                .checked_add(pixel_bytes)
                .is_some_and(|end| end <= bytes.len()),
        "truncated BMP screenshot"
    );
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height as usize {
        let source_y = if signed_height > 0 {
            height as usize - 1 - y
        } else {
            y
        };
        for x in 0..width as usize {
            let source = data_offset + source_y * row_stride + x * 3;
            let target = (y * width as usize + x) * 4;
            rgba[target..target + 4].copy_from_slice(&[
                bytes[source + 2],
                bytes[source + 1],
                bytes[source],
                255,
            ]);
        }
    }
    Ok((width, height, encode_rgba_png(width, height, &rgba)?))
}

fn axis_gap(a1: u32, a2: u32, b1: u32, b2: u32) -> u32 {
    if a2 < b1 {
        b1 - a2
    } else if b2 < a1 {
        a1 - b2
    } else {
        0
    }
}

fn axis_overlap(a1: u32, a2: u32, b1: u32, b2: u32) -> u32 {
    a2.min(b2).saturating_sub(a1.max(b1))
}

fn nearby_text(a: TextRect, b: TextRect) -> bool {
    let line_height = a.height().max(b.height()).max(8);
    let vertical_gap = axis_gap(a.y1, a.y2, b.y1, b.y2);
    let horizontal_gap = axis_gap(a.x1, a.x2, b.x1, b.x2);
    let horizontal_overlap = axis_overlap(a.x1, a.x2, b.x1, b.x2);
    (vertical_gap <= line_height * 2 && horizontal_overlap >= a.width().min(b.width()) / 4)
        || (vertical_gap <= line_height / 2 && horizontal_gap <= line_height * 2)
}

fn group_text_boxes(mut boxes: Vec<TextRect>) -> Vec<TextRect> {
    boxes.sort_by_key(|rect| (rect.y1, rect.x1));
    let mut groups = Vec::new();
    for mut rect in boxes {
        while let Some(index) = groups.iter().position(|group| nearby_text(*group, rect)) {
            rect = rect.union(groups.swap_remove(index));
        }
        groups.push(rect);
    }
    groups.sort_by_key(|rect: &TextRect| std::cmp::Reverse(rect.width() * rect.height()));
    groups.truncate(8);
    groups.sort_by_key(|rect| (rect.y1, rect.x1));
    groups
}

fn detect_text_regions(detector: &OcrEngine, image: &RgbImage) -> Result<Vec<TextRect>> {
    let (width, height) = image.dimensions();
    let source = ImageSource::from_bytes(image.as_raw(), (width, height))?;
    let prepared = detector.prepare_input(source)?;
    let boxes = detector.detect_words(&prepared)?;
    let mut rectangles = Vec::new();
    for detected in boxes {
        let corners = detected.corners();
        let x1 = corners
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .clamp(0.0, width as f32) as u32;
        let y1 = corners
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .clamp(0.0, height as f32) as u32;
        let x2 = corners
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, width as f32) as u32;
        let y2 = corners
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, height as f32) as u32;
        let rect = TextRect { x1, y1, x2, y2 };
        if rect.width() >= 8 && rect.height() >= 5 {
            rectangles.push(rect);
        }
    }
    Ok(group_text_boxes(rectangles))
}

fn crop_region_png(image: &RgbImage, rect: TextRect) -> Result<String> {
    let cropped =
        image::imageops::crop_imm(image, rect.x1, rect.y1, rect.width(), rect.height()).to_image();
    let mut pixels = Vec::with_capacity((cropped.width() * cropped.height() * 4) as usize);
    for pixel in cropped.pixels() {
        pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
    }
    Ok(BASE64.encode(encode_rgba_png(cropped.width(), cropped.height(), &pixels)?))
}

fn sample_text_background(image: &RgbImage, rect: TextRect) -> [u8; 3] {
    let (width, height) = image.dimensions();
    let ring = rect.padded(width, height, 3);
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for y in ring.y1..ring.y2 {
        for x in ring.x1..ring.x2 {
            if x >= rect.x1 && x < rect.x2 && y >= rect.y1 && y < rect.y2 {
                continue;
            }
            let color = image.get_pixel(x, y);
            for (channel, value) in channels.iter_mut().zip(color.0) {
                channel.push(value);
            }
        }
    }
    if channels[0].is_empty() {
        return [9, 14, 22];
    }
    for channel in &mut channels {
        channel.sort_unstable();
    }
    let middle = channels[0].len() / 2;
    [
        channels[0][middle],
        channels[1][middle],
        channels[2][middle],
    ]
}

fn render_translation(
    settings: &TranslationSettings,
    image: &str,
    width: u32,
    height: u32,
    detector: &OcrEngine,
    cached: Option<&CachedTranslation>,
) -> Result<(Vec<TranslatedRegion>, String)> {
    let bytes = BASE64
        .decode(image)
        .context("decoding screenshot for text placement")?;
    let screenshot = image::load_from_memory(&bytes)
        .context("reading screenshot for text placement")?
        .into_rgb8();
    ensure!(
        screenshot.dimensions() == (width, height),
        "screenshot dimensions changed"
    );
    let boxes = detect_text_regions(detector, &screenshot)?;
    let mut regions = Vec::new();
    for rect in boxes {
        let crop = crop_region_png(&screenshot, rect.padded(width, height, 8))?;
        let source = recognize_text_at(&crop, OLLAMA_URL)?;
        if source.chars().filter(|ch| ch.is_alphabetic()).count() < 2 {
            continue;
        }
        let previous = cached
            .filter(|old| old.dimensions == (width, height))
            .and_then(|old| {
                old.regions
                    .iter()
                    .find(|region| region.source == source && region.rect.near(rect))
            });
        let english = if let Some(previous) = previous {
            previous.english.clone()
        } else {
            translate_text_at(settings, &source, OLLAMA_URL)?
        };
        if english.is_empty() {
            continue;
        }
        regions.push(TranslatedRegion {
            rect: previous.map_or(rect, |old| old.rect),
            source,
            english,
            background: sample_text_background(&screenshot, rect),
        });
    }
    let overlay = render_regions_png(width, height, &regions)?;
    Ok((regions, BASE64.encode(overlay)))
}

fn subtitle_font() -> Option<&'static Font> {
    static FONT: OnceLock<Option<Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        let mut database = Database::new();
        database.load_system_fonts();
        let id = database.query(&Query {
            families: &[Family::SansSerif],
            weight: fontdb::Weight::SEMIBOLD,
            ..Query::default()
        })?;
        database.with_face_data(id, |bytes, index| {
            Font::from_bytes(
                bytes.to_vec(),
                FontSettings {
                    collection_index: index,
                    ..FontSettings::default()
                },
            )
            .ok()
        })?
    })
    .as_ref()
}

fn render_regions_png(width: u32, height: u32, regions: &[TranslatedRegion]) -> Result<Vec<u8>> {
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    for region in regions {
        draw_region(&mut pixels, width, height, region);
    }
    encode_rgba_png(width, height, &pixels)
}

fn draw_region(pixels: &mut [u8], width: u32, height: u32, region: &TranslatedRegion) {
    if region.english.is_empty() {
        return;
    }
    // Keep short game-text regions legible without expanding a translation
    // panel over nearby dialogue-box borders or unrelated artwork.
    let font_px = (height as f32 / 17.0)
        .clamp(11.0, 52.0)
        .min((region.rect.height() as f32 / 3.5).max(11.0));
    let line_height = (font_px * 1.35).ceil() as u32;
    let margin = (width / 80).clamp(2, 24);
    let padding = (font_px / 1.5).ceil() as u32;
    let max_width = width.saturating_sub(2 * margin).max(1);
    let desired_width =
        (region.english.chars().count() as f32 * font_px * 0.56 / 2.0).ceil() as u32;
    let panel_width = region
        .rect
        .width()
        .saturating_add(2 * padding)
        .max(desired_width)
        .min(max_width);
    let columns = ((panel_width.saturating_sub(2 * padding)) as f32 / (font_px * 0.56))
        .floor()
        .max(1.0) as usize;
    let max_lines =
        ((height.saturating_sub(2 * (margin + padding))) / line_height).clamp(1, 6) as usize;
    let lines = wrap_caption(&region.english, columns, max_lines);
    if lines.is_empty() {
        return;
    }
    let total_height = line_height * lines.len() as u32;
    let panel_height = region
        .rect
        .height()
        .saturating_add(2 * padding)
        .max(total_height + 2 * padding)
        .min(height.saturating_sub(2 * margin));
    let center_x = region.rect.x1.saturating_add(region.rect.width() / 2);
    let panel_left = center_x
        .saturating_sub(panel_width / 2)
        .min(width.saturating_sub(margin + panel_width))
        .max(margin);
    let panel_top = region
        .rect
        .y1
        .saturating_sub(padding)
        .min(height.saturating_sub(margin + panel_height))
        .max(margin);
    let text_top = panel_top + panel_height.saturating_sub(total_height) / 2;
    let text_center_x = panel_left + panel_width / 2;
    for y in panel_top..panel_top + panel_height {
        for x in panel_left..panel_left + panel_width {
            put_pixel(
                pixels,
                width,
                x,
                y,
                [
                    region.background[0],
                    region.background[1],
                    region.background[2],
                    255,
                ],
            );
        }
    }
    let mut mask = vec![0u8; width as usize * height as usize];
    if let Some(font) = subtitle_font() {
        let ascent = font
            .horizontal_line_metrics(font_px)
            .map_or(font_px, |metrics| metrics.ascent);
        for (row, line) in lines.iter().enumerate() {
            let line_width: f32 = line
                .chars()
                .map(|ch| font.metrics(ch, font_px).advance_width)
                .sum();
            let mut cursor_x = (text_center_x as f32 - line_width / 2.0).clamp(
                (panel_left + padding) as f32,
                (panel_left + panel_width.saturating_sub(padding)) as f32,
            );
            let baseline = text_top as f32 + row as f32 * line_height as f32 + ascent;
            for ch in line.chars() {
                let (metrics, bitmap) = font.rasterize(ch, font_px);
                let glyph_x = cursor_x.round() as i32 + metrics.xmin;
                let glyph_y = baseline.round() as i32 - metrics.ymin - metrics.height as i32;
                for gy in 0..metrics.height {
                    for gx in 0..metrics.width {
                        let x = glyph_x + gx as i32;
                        let y = glyph_y + gy as i32;
                        if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                            let index = y as usize * width as usize + x as usize;
                            mask[index] = mask[index].max(bitmap[gy * metrics.width + gx]);
                        }
                    }
                }
                cursor_x += metrics.advance_width;
            }
        }
    } else {
        // The built-in bitmap font keeps captions available on minimal systems.
        for (row, line) in lines.iter().enumerate() {
            let start_x = text_center_x.saturating_sub(line.len() as u32 * 4);
            for (column, ch) in line.chars().enumerate() {
                if let Some(glyph) = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?')) {
                    for (gy, bits) in glyph.iter().enumerate() {
                        for gx in 0..8u32 {
                            let x = start_x + column as u32 * 8 + gx;
                            let y = text_top + row as u32 * line_height + gy as u32;
                            if bits & (1 << gx) != 0 && x < width && y < height {
                                mask[(y * width + x) as usize] = 255;
                            }
                        }
                    }
                }
            }
        }
    }
    let brightness = u32::from(region.background[0]) * 2126
        + u32::from(region.background[1]) * 7152
        + u32::from(region.background[2]) * 722;
    let foreground = if brightness < 1_400_000 { 255 } else { 0 };
    for (index, coverage) in mask.into_iter().enumerate() {
        if coverage > 0 {
            let pixel = &mut pixels[index * 4..index * 4 + 4];
            for channel in &mut pixel[..3] {
                *channel = (((u32::from(*channel) * u32::from(255 - coverage))
                    + foreground * u32::from(coverage))
                    / 255) as u8;
            }
            pixel[3] = pixel[3].max(coverage);
        }
    }
}

fn encode_rgba_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut result, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&pixels)?;
    }
    Ok(result)
}

fn put_pixel(pixels: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let offset = ((y * width + x) * 4) as usize;
    pixels[offset..offset + 4].copy_from_slice(&color);
}

fn wrap_caption(text: &str, columns: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.len() + usize::from(!current.is_empty()) + word.len() > columns
            && !current.is_empty()
        {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                break;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.extend(word.chars().take(columns));
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines
}

fn recognize_text_at(image: &str, base_url: &str) -> Result<String> {
    let raw = ollama_chat(
        OCR_MODEL,
        "Text Recognition: Read only visible game dialogue, menu labels, and instructions in reading order. Return only the recognized text, without markdown, code fences, or commentary.",
        Some(image),
        base_url,
    )?;
    Ok(clean_ocr_text(&raw))
}

fn clean_ocr_text(raw: &str) -> String {
    // GLM-OCR sometimes appends repeated Markdown fences after a correct OCR
    // result. Never send those hallucinated tokens on to the translator.
    let before_fence = raw.split("```").next().unwrap_or("");
    let mut lines = Vec::new();
    for line in before_fence.lines() {
        let line = line.trim().trim_matches('"').trim();
        if line.is_empty() || lines.last().is_some_and(|previous| *previous == line) {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").chars().take(1200).collect()
}

fn translate_text_at(
    settings: &TranslationSettings,
    source_text: &str,
    base_url: &str,
) -> Result<String> {
    let (source_name, source_code) = if settings.source_language == "auto" {
        ("source language".to_owned(), "auto".to_owned())
    } else {
        let name = match settings.source_language.as_str() {
            "ja" => "Japanese",
            "zh" => "Chinese",
            "ko" => "Korean",
            "fr" => "French",
            "de" => "German",
            "es" => "Spanish",
            _ => "source language",
        };
        (name.to_owned(), settings.source_language.clone())
    };
    let prompt = format!(
        "You are a professional {source_name} ({source_code}) to English (en) translator. Translate the following game dialogue or menu text accurately into natural English. Preserve names and the order of lines. Return only the English translation, without explanations or commentary.\n\n{source_text}"
    );
    let raw = ollama_chat(&settings.model, &prompt, None, base_url)?;
    Ok(english_only_translation(&raw))
}

fn english_only_translation(raw: &str) -> String {
    let lines: Vec<_> = raw
        .lines()
        .map(|line| line.trim().trim_matches('"').trim())
        .filter(|line| line.chars().any(|ch| ch.is_ascii_alphabetic()))
        .collect();
    lines.join(" ").chars().take(2000).collect()
}

fn ollama_chat(model: &str, prompt: &str, image: Option<&str>, base_url: &str) -> Result<String> {
    let mut message = json!({"role": "user", "content": prompt});
    if let Some(image) = image {
        message["images"] = json!([image]);
    }
    let request = json!({
        "model": model,
        "messages": [message],
        "stream": false,
        "keep_alive": "10m",
        "options": {"temperature": 0, "num_predict": 256},
    });
    let mut response = http_agent(Duration::from_secs(60))
        .post(&format!("{base_url}/api/chat"))
        .send_json(request)
        .context("requesting local translation")?;
    ensure!(
        response.status().as_u16() == 200,
        "local Ollama translation failed"
    );
    let mut response_bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut response_bytes)?;
    ensure!(
        response_bytes.len() <= MAX_RESPONSE_BYTES,
        "Ollama response too large"
    );
    let result: Value = serde_json::from_slice(&response_bytes)?;
    let text = result["message"]["content"]
        .as_str()
        .context("Ollama returned no translation")?;
    Ok(text.trim().to_owned())
}

fn write_json(stream: &mut TcpStream, status: u16, body: &Value) -> Result<()> {
    let data = serde_json::to_vec(body)?;
    write!(
        stream,
        "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        data.len()
    )?;
    stream.write_all(&data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_hotkey_preserves_retroarch_screenshots() {
        let config = retroarch_session_config(41769, "session-token");
        assert!(config.contains("input_ai_service = \"f10\""));
        assert!(!config.contains("input_ai_service = \"f8\""));
        assert!(!config.contains("input_screenshot ="));
    }

    #[test]
    fn translation_defaults_to_disabled_and_validates_local_models() {
        let mut settings = TranslationSettings::default();
        assert!(!settings.enabled);
        settings.validate().unwrap();
        settings.model = "evil/remote".to_owned();
        assert!(settings.validate().is_err());
        settings.model = "translategemma:27b".to_owned();
        settings.source_language = "ja".to_owned();
        settings.validate().unwrap();
    }

    #[test]
    fn translation_panel_tracks_detected_text_instead_of_bottom_of_screen() {
        let regions = [TranslatedRegion {
            rect: TextRect {
                x1: 28,
                y1: 20,
                x2: 190,
                y2: 48,
            },
            source: "扉を開けてください。".to_owned(),
            english: "Open the door.".to_owned(),
            background: [9, 14, 22],
        }];
        let caption = render_regions_png(320, 240, &regions).unwrap();
        if let Ok(path) = std::env::var("LUNCHBOX_TRANSLATION_CAPTION_PROBE") {
            std::fs::write(path, &caption).unwrap();
        }
        assert_eq!(png_dimensions(&caption).unwrap(), (320, 240));
        assert!(caption.len() < 100_000);
        let decoder = png::Decoder::new(std::io::Cursor::new(&caption));
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels[3], 0);
        assert_eq!(pixels[(239 * 320 * 4) + 3], 0);
        assert_eq!(pixels[((30 * 320 + 40) * 4) + 3], 255);
        assert_eq!(pixels[((170 * 320 + 40) * 4) + 3], 0);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[0] == 255));
    }

    #[test]
    fn separated_text_blocks_keep_separate_transparent_regions() {
        let regions = [
            TranslatedRegion {
                rect: TextRect {
                    x1: 20,
                    y1: 20,
                    x2: 110,
                    y2: 40,
                },
                source: "北へ".to_owned(),
                english: "North".to_owned(),
                background: [9, 14, 22],
            },
            TranslatedRegion {
                rect: TextRect {
                    x1: 20,
                    y1: 180,
                    x2: 140,
                    y2: 200,
                },
                source: "南へ".to_owned(),
                english: "South".to_owned(),
                background: [9, 14, 22],
            },
        ];
        let png = render_regions_png(320, 240, &regions).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&png))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        decoder.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels[((30 * 320 + 40) * 4) + 3], 255);
        assert_eq!(pixels[((190 * 320 + 40) * 4) + 3], 255);
        assert_eq!(pixels[((110 * 320 + 40) * 4) + 3], 0);
    }

    #[test]
    fn dialogue_translation_stays_inside_the_text_box_border() {
        let region = TranslatedRegion {
            rect: TextRect {
                x1: 25,
                y1: 160,
                x2: 238,
                y2: 201,
            },
            source: "ここはマナの聖地です。".to_owned(),
            english: "This is a sacred place of mana. Hero, please open the gate.".to_owned(),
            background: [24, 22, 32],
        };
        let png = render_regions_png(320, 240, &[region]).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&png))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        decoder.next_frame(&mut pixels).unwrap();
        assert_eq!(pixels[((146 * 320 + 100) * 4) + 3], 0);
        assert_eq!(pixels[((170 * 320 + 100) * 4) + 3], 255);
        assert_eq!(pixels[((225 * 320 + 100) * 4) + 3], 0);
    }

    #[test]
    fn text_background_comes_from_the_neighboring_game_pixels() {
        let mut image = RgbImage::from_pixel(64, 64, image::Rgb([21, 35, 49]));
        let rect = TextRect {
            x1: 10,
            y1: 20,
            x2: 50,
            y2: 40,
        };
        for y in rect.y1..rect.y2 {
            for x in rect.x1..rect.x2 {
                image.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
        assert_eq!(sample_text_background(&image, rect), [21, 35, 49]);
    }

    #[test]
    fn neighboring_lines_group_without_joining_distant_ui_text() {
        let boxes = vec![
            TextRect {
                x1: 25,
                y1: 160,
                x2: 188,
                y2: 174,
            },
            TextRect {
                x1: 25,
                y1: 185,
                x2: 238,
                y2: 200,
            },
            TextRect {
                x1: 10,
                y1: 15,
                x2: 90,
                y2: 29,
            },
        ];
        assert_eq!(
            group_text_boxes(boxes),
            vec![
                TextRect {
                    x1: 10,
                    y1: 15,
                    x2: 90,
                    y2: 29,
                },
                TextRect {
                    x1: 25,
                    y1: 160,
                    x2: 238,
                    y2: 200,
                },
            ]
        );
    }

    #[test]
    fn translation_keeps_english_and_drops_quoted_source_line() {
        assert_eq!(
            english_only_translation("\"どうもありがとうございます。\"\n\"Thank you very much.\""),
            "Thank you very much."
        );
        assert_eq!(english_only_translation("日本語だけ"), "");
    }

    #[test]
    fn ocr_strips_repeated_markdown_from_model() {
        assert_eq!(
            clean_ocr_text(
                "ここはマナの聖地です。\n勇者よ、扉を開けてください。\n```markdown\n```"
            ),
            "ここはマナの聖地です。\n勇者よ、扉を開けてください。"
        );
    }

    #[test]
    fn older_bmp_screenshot_is_converted_to_png_for_ollama() {
        let width = 64u32;
        let height = 64u32;
        let pixels = vec![80u8; (width * height * 3) as usize];
        let mut bmp = vec![0u8; 54];
        bmp[0..2].copy_from_slice(b"BM");
        bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
        bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
        bmp[18..22].copy_from_slice(&width.to_le_bytes());
        bmp[22..26].copy_from_slice(&height.to_le_bytes());
        bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
        bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&pixels);
        let (converted_width, converted_height, png) = bmp_to_png(&bmp).unwrap();
        assert_eq!((converted_width, converted_height), (width, height));
        assert_eq!(png_dimensions(&png).unwrap(), (width, height));
    }

    #[test]
    fn ultrawide_monitor_screenshots_are_supported_with_a_pixel_cap() {
        validate_frame_dimensions(5120, 2160).unwrap();
        validate_frame_dimensions(7680, 4320).unwrap_err();
        validate_frame_dimensions(5120, 4320).unwrap_err();
        let mut png_header = vec![0u8; 24];
        png_header[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png_header[16..20].copy_from_slice(&5120u32.to_be_bytes());
        png_header[20..24].copy_from_slice(&2160u32.to_be_bytes());
        assert_eq!(png_dimensions(&png_header).unwrap(), (5120, 2160));
    }

    #[test]
    fn retroarch_request_without_format_field_returns_image_and_auto() {
        let screenshot = render_regions_png(64, 64, &[]).unwrap();
        let overlay = render_regions_png(
            64,
            64,
            &[TranslatedRegion {
                rect: TextRect {
                    x1: 10,
                    y1: 20,
                    x2: 54,
                    y2: 40,
                },
                source: "開く".to_owned(),
                english: "OPEN".to_owned(),
                background: [9, 14, 22],
            }],
        )
        .unwrap();
        let digest: [u8; 32] = Sha256::digest(&screenshot).into();
        let mut cached = Some(CachedTranslation {
            digest,
            regions: vec![],
            dimensions: (64, 64),
            overlay: BASE64.encode(overlay),
        });
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            let body = serde_json::to_vec(&json!({"image": BASE64.encode(screenshot)})).unwrap();
            write!(
                stream,
                "POST /secret?output=image,png,png-a HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            stream.flush().unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });
        let (mut stream, _) = listener.accept().unwrap();
        handle_request(
            &mut stream,
            "secret",
            &TranslationSettings::default(),
            None,
            &mut cached,
            &mut None,
        )
        .unwrap();
        drop(stream);
        let response = client.join().unwrap();
        let body = response.split("\r\n\r\n").nth(1).unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["auto"], "auto");
        let image = BASE64.decode(body["image"].as_str().unwrap()).unwrap();
        assert_eq!(png_dimensions(&image).unwrap(), (64, 64));
    }

    #[test]
    fn ollama_chat_sends_image_to_ocr_model() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            assert!(request_line.starts_with("POST /api/chat HTTP/1.1"));
            let mut length = None;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = Some(value.trim().parse::<usize>().unwrap());
                }
            }
            let mut body = vec![0; length.unwrap()];
            reader.read_exact(&mut body).unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["model"], OCR_MODEL);
            assert_eq!(body["stream"], false);
            assert_eq!(body["messages"][0]["images"][0], "image-data");
            write_json(
                &mut stream,
                200,
                &json!({"message": {"content": "扉が開いている。\n```"}}),
            )
            .unwrap();
        });
        let recognized = recognize_text_at(
            "image-data",
            &format!("http://127.0.0.1:{}", address.port()),
        )
        .unwrap();
        assert_eq!(recognized, "扉が開いている。");
        server.join().unwrap();
    }

    #[test]
    fn ollama_chat_sends_text_to_translation_model() {
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("POST /api/chat HTTP/1.1"));
            let mut length = None;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = Some(value.trim().parse::<usize>().unwrap());
                }
            }
            let mut body = vec![0; length.unwrap()];
            reader.read_exact(&mut body).unwrap();
            let body: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["model"], "translategemma:12b");
            assert!(body["messages"][0]["images"].is_null());
            assert!(
                body["messages"][0]["content"]
                    .as_str()
                    .unwrap()
                    .contains("扉が開いている。")
            );
            write_json(
                &mut stream,
                200,
                &json!({"message": {"content": "The door is open."}}),
            )
            .unwrap();
        });
        let translated = translate_text_at(
            &TranslationSettings::default(),
            "扉が開いている。",
            &format!("http://127.0.0.1:{}", address.port()),
        )
        .unwrap();
        assert_eq!(translated, "The door is open.");
        server.join().unwrap();
    }

    #[test]
    #[ignore = "requires local Ollama models and explicit image paths"]
    fn live_ollama_overlay_probe() {
        let source_path = std::env::var("LUNCHBOX_TRANSLATION_SOURCE_IMAGE").unwrap();
        let overlay_path = std::env::var("LUNCHBOX_TRANSLATION_OVERLAY_IMAGE").unwrap();
        let source = std::fs::read(source_path).unwrap();
        let (width, height) = png_dimensions(&source).unwrap();
        let image = BASE64.encode(source);
        download_detector(&AtomicBool::new(false)).unwrap();
        let detector = load_detector().unwrap();
        let (regions, overlay) = render_translation(
            &TranslationSettings::default(),
            &image,
            width,
            height,
            &detector,
            None,
        )
        .unwrap();
        assert!(!regions.is_empty());
        let png = BASE64.decode(overlay).unwrap();
        std::fs::write(overlay_path, png).unwrap();
        for region in regions {
            eprintln!(
                "LUNCHBOX_TRANSLATION_REGION: {:?} {:?} => {:?}",
                region.rect, region.source, region.english
            );
        }
    }
}
