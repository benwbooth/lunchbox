//! Session-scoped, localhost-only RetroArch AI Service to Ollama bridge.
//! No screenshot is persisted or sent to a remote service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
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
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

const OLLAMA_URL: &str = "http://127.0.0.1:11434";
// RetroArch uses F8 for screenshots by default. Keep that binding intact.
const TRANSLATION_HOTKEY: &str = "f10";
const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 128 * 1024;

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
    Ok(body["models"]
        .as_array()
        .is_some_and(|models| models.iter().any(|entry| entry["name"] == model)))
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
    let mut response = http_agent(Duration::from_secs(60 * 60))
        .post(&format!("{OLLAMA_URL}/api/pull"))
        .send_json(json!({"model": model, "stream": true}))
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
            (Some(done), Some(total)) if total > 0 => (done * 100 / total).min(99) as u8,
            _ => 0,
        };
        progress(fraction, status.to_owned());
        if status == "success" {
            complete = true;
        }
    }
    ensure!(
        complete && model_available(model)?,
        "Ollama did not install the model"
    );
    progress(100, format!("{model} is ready"));
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
            "{} is not installed in local Ollama; download it in Settings",
            settings.model
        );
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
            .spawn(move || serve(listener, &secret, &settings, &worker_stop))
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

fn serve(listener: TcpListener, secret: &str, settings: &TranslationSettings, stop: &AtomicBool) {
    let mut cached: Option<([u8; 32], String)> = None;
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

fn handle_request(
    stream: &mut TcpStream,
    secret: &str,
    settings: &TranslationSettings,
    cached: &mut Option<([u8; 32], String)>,
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
    let overlay = if let Some((old_digest, old_overlay)) = cached.as_ref() {
        if *old_digest == digest {
            old_overlay.clone()
        } else {
            render_translation(settings, &model_image, width, height)?
        }
    } else {
        render_translation(settings, &model_image, width, height)?
    };
    *cached = Some((digest, overlay.clone()));
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
    ensure!(
        (64..=4096).contains(&width) && (64..=2160).contains(&height),
        "screenshot dimensions are unsupported"
    );
    Ok((width, height))
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
    ensure!(
        (64..=4096).contains(&width) && (64..=2160).contains(&height),
        "screenshot dimensions are unsupported"
    );
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

fn render_translation(
    settings: &TranslationSettings,
    image: &str,
    width: u32,
    height: u32,
) -> Result<String> {
    let text = translate_image(settings, image)?;
    let location = if text.is_empty() {
        None
    } else {
        locate_text(settings, image).unwrap_or_else(|error| {
            eprintln!("LUNCHBOX_TRANSLATION_LOCATION_SKIPPED: {error:#}");
            None
        })
    };
    let png = render_subtitle_png(width, height, &text, location)?;
    Ok(BASE64.encode(png))
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

fn render_subtitle_png(
    width: u32,
    height: u32,
    text: &str,
    source_box: Option<[f32; 4]>,
) -> Result<Vec<u8>> {
    let font_px = (width as f32 / 42.0).clamp(11.0, 23.0);
    let margin = (width / 40).clamp(5, 24);
    let max_columns = ((width - 2 * margin) as f32 / (font_px * 0.68)) as usize;
    let lines = wrap_caption(text, max_columns.max(1), 3);
    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    if lines.is_empty() {
        return encode_rgba_png(width, height, &pixels);
    }
    let line_height = (font_px * 1.22).ceil() as u32;
    let total_height = line_height * lines.len() as u32;
    let (center_x, top) = subtitle_position(width, height, total_height, source_box);
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
            let mut cursor_x =
                (center_x as f32 - line_width / 2.0).clamp(margin as f32, (width - margin) as f32);
            let baseline = top as f32 + row as f32 * line_height as f32 + ascent;
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
        // Platforms without a discoverable system font still get a legible
        // outlined subtitle, never the old full-width opaque caption strip.
        for (row, line) in lines.iter().enumerate() {
            let start_x = center_x.saturating_sub(line.len() as u32 * 4);
            for (column, ch) in line.chars().enumerate() {
                if let Some(glyph) = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?')) {
                    for (gy, bits) in glyph.iter().enumerate() {
                        for gx in 0..8u32 {
                            let x = start_x + column as u32 * 8 + gx;
                            let y = top + row as u32 * line_height + gy as u32;
                            if bits & (1 << gx) != 0 && x < width && y < height {
                                mask[(y * width + x) as usize] = 255;
                            }
                        }
                    }
                }
            }
        }
    }
    let outline = (font_px / 11.0).ceil() as i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            if mask[(y as u32 * width + x as u32) as usize] == 0 {
                continue;
            }
            for dy in -outline..=outline {
                for dx in -outline..=outline {
                    let px = x + dx;
                    let py = y + dy;
                    if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                        put_pixel(&mut pixels, width, px as u32, py as u32, [0, 0, 0, 230]);
                    }
                }
            }
        }
    }
    for (index, coverage) in mask.into_iter().enumerate() {
        if coverage > 0 {
            pixels[index * 4..index * 4 + 4].copy_from_slice(&[255, 255, 255, coverage]);
        }
    }
    encode_rgba_png(width, height, &pixels)
}

fn subtitle_position(
    width: u32,
    height: u32,
    subtitle_height: u32,
    source_box: Option<[f32; 4]>,
) -> (u32, u32) {
    let margin = (height / 32).clamp(4, 28);
    if let Some([left, top, right, bottom]) = source_box {
        let center_x = (((left + right) / 2.0) * width as f32).round() as u32;
        let source_top = (top * height as f32).round() as u32;
        let source_bottom = (bottom * height as f32).round() as u32;
        if source_top >= subtitle_height + 2 * margin {
            return (center_x, source_top - subtitle_height - margin);
        }
        if source_bottom + subtitle_height + 2 * margin < height {
            return (center_x, source_bottom + margin);
        }
    }
    (width / 2, (height * 2 / 3).saturating_sub(subtitle_height))
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

fn translate_image(settings: &TranslationSettings, image: &str) -> Result<String> {
    translate_image_at(settings, image, OLLAMA_URL)
}

fn translate_image_at(
    settings: &TranslationSettings,
    image: &str,
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
        "You are a professional {source_name} ({source_code}) to English (en) translator. Your goal is to accurately convey the meaning and nuances of the original {source_name} text while adhering to English grammar, vocabulary, and cultural sensitivities. Produce only the English translation, without any additional explanations or commentary. Please translate the following {source_name} text into English:\n\n"
    );
    let raw = ollama_image_request(settings, image, &prompt, base_url, false)?;
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

fn locate_text(settings: &TranslationSettings, image: &str) -> Result<Option<[f32; 4]>> {
    let prompt = "Find the visible dialogue or menu text in this game screenshot. Return ONLY JSON: {\"box\":[left,top,right,bottom]}. Use coordinates from 0 to 1000 relative to the whole image, tightly covering the original source-language text. Return {\"box\":null} if no text is visible.";
    let response = ollama_image_request(settings, image, prompt, OLLAMA_URL, true)?;
    let value: Value = serde_json::from_str(&response).context("parsing text location")?;
    let Some(box_values) = value["box"].as_array() else {
        return Ok(None);
    };
    ensure!(box_values.len() == 4, "invalid text location");
    let mut box_coordinates = [0.0; 4];
    for (index, value) in box_values.iter().enumerate() {
        let coordinate = value.as_f64().context("invalid text coordinate")?;
        ensure!(
            (0.0..=1000.0).contains(&coordinate),
            "text coordinate outside screenshot"
        );
        box_coordinates[index] = (coordinate / 1000.0) as f32;
    }
    ensure!(
        box_coordinates[2] - box_coordinates[0] >= 0.02
            && box_coordinates[3] - box_coordinates[1] >= 0.01,
        "invalid text box"
    );
    Ok(Some(box_coordinates))
}

fn ollama_image_request(
    settings: &TranslationSettings,
    image: &str,
    prompt: &str,
    base_url: &str,
    structured: bool,
) -> Result<String> {
    let mut request = json!({
        "model": settings.model,
        "messages": [{"role": "user", "content": prompt, "images": [image]}],
        "stream": false,
        "keep_alive": "10m",
        "options": {"temperature": 0, "num_predict": 256},
    });
    if structured {
        request["format"] = json!("json");
    }
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
    fn subtitle_preserves_original_text_below_it() {
        let caption =
            render_subtitle_png(320, 240, "The door is locked.", Some([0.1, 0.75, 0.9, 0.9]))
                .unwrap();
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
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
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
    fn text_location_is_above_original_or_below_when_needed() {
        assert_eq!(
            subtitle_position(320, 240, 30, Some([0.2, 0.75, 0.8, 0.9])),
            (160, 143)
        );
        let (_, top) = subtitle_position(320, 240, 30, Some([0.2, 0.01, 0.8, 0.1]));
        assert!(top > 24);
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
    fn retroarch_request_without_format_field_returns_image_and_auto() {
        let screenshot = render_subtitle_png(64, 64, "", None).unwrap();
        let overlay = render_subtitle_png(64, 64, "OPEN", None).unwrap();
        let digest: [u8; 32] = Sha256::digest(&screenshot).into();
        let mut cached = Some((digest, BASE64.encode(overlay)));
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
    fn ollama_chat_receives_image_and_returns_translation() {
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
            assert_eq!(body["model"], "translategemma:12b");
            assert_eq!(body["stream"], false);
            assert_eq!(body["messages"][0]["images"][0], "image-data");
            write_json(
                &mut stream,
                200,
                &json!({"message": {"content": "The door is open."}}),
            )
            .unwrap();
        });
        let translated = translate_image_at(
            &TranslationSettings::default(),
            "image-data",
            &format!("http://127.0.0.1:{}", address.port()),
        )
        .unwrap();
        assert_eq!(translated, "The door is open.");
        server.join().unwrap();
    }
}
