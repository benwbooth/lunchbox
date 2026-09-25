//! Session-scoped, localhost-only RetroArch AI Service to Ollama bridge.
//! No screenshot is persisted or sent to a remote service.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use fontdb::{Database, Family, Query};
use fontdue::{Font, FontSettings};
use image::RgbImage;
use rapidocr_core::RapidOcr;
use rapidocr_core::cancellation::OcrCancellationToken;
use rapidocr_core::config::{ExecutionProvider, InferenceOptions, PipelineConfig};
use rapidocr_core::model::{ModelCache, ModelDownloadMode, model_set_by_name};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sysinfo::{ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};

use crate::emulator::{EmulatorExecutable, LaunchPlan};

const OLLAMA_URL: &str = "http://127.0.0.1:11434";
const OCR_MODEL_SET: &str = "ppocrv6-small";
// RetroArch uses F8 for screenshots by default. Keep that binding intact.
const TRANSLATION_HOTKEY: &str = "f10";
// A 5120×2160 24-bit BMP becomes roughly 44 MiB after base64 encoding.
const MAX_REQUEST_BYTES: usize = 80 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_FRAME_PIXELS: u64 = 16_000_000;
// RetroArch requests the next frame as soon as it receives `auto: "auto"`.
// One capture per second is enough for dialogue and limits interruption of the game.
const AUTO_REQUEST_INTERVAL: Duration = Duration::from_secs(1);
// Return a transparent frame promptly when inference is still running. The
// next auto-capture picks up the finished overlay without holding RetroArch's
// HTTP task on a spinner for the full model latency.
const RESULT_WAIT_TIMEOUT: Duration = Duration::from_millis(150);
const RESULT_RESPONSE_BUDGET: Duration = Duration::from_millis(1450);
const MAX_TRANSLATION_MEMORY: usize = 256;
// Keep the game legible beneath a translated region without letting the
// original glyphs compete with the English foreground.
const REGION_BACKGROUND_ALPHA: u8 = 224;

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

fn ocr_model_cache() -> Result<ModelCache> {
    let dirs = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("finding the local model cache")?;
    Ok(ModelCache::new(
        dirs.cache_dir().join("translation").join(OCR_MODEL_SET),
    ))
}

fn ocr_model_set() -> Result<&'static rapidocr_core::model::ModelSetSpec> {
    model_set_by_name(OCR_MODEL_SET).context("PP-OCRv6 small model set is unavailable")
}

#[cfg(all(target_os = "linux", feature = "rocm-ocr"))]
pub fn configure_gpu_cache() -> Result<()> {
    if std::env::var_os("ORT_MIGRAPHX_MODEL_CACHE_PATH").is_some() {
        return Ok(());
    }
    let dirs = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("finding the local model cache")?;
    let path = dirs
        .cache_dir()
        .join("translation")
        .join("migraphx-fixed-v1");
    std::fs::create_dir_all(&path).context("creating the MIGraphX model cache")?;
    // SAFETY: run() calls this before starting Qt, OCR, or any worker threads.
    unsafe { std::env::set_var("ORT_MIGRAPHX_MODEL_CACHE_PATH", &path) };
    Ok(())
}

fn ocr_models_available() -> Result<bool> {
    Ok(ocr_model_cache()?
        .missing_assets_for_pipeline(ocr_model_set()?, PipelineConfig::without_cls())
        .is_empty())
}

fn download_ocr_models(cancelled: &AtomicBool) -> Result<()> {
    ensure!(
        !cancelled.load(Ordering::Relaxed),
        "OCR model download cancelled"
    );
    ocr_model_cache()?
        .ensure_model_set_for_pipeline(
            ocr_model_set()?,
            PipelineConfig::without_cls(),
            ModelDownloadMode::Missing,
        )
        .context("installing Japanese-capable OCR models")?;
    ensure!(
        !cancelled.load(Ordering::Relaxed),
        "OCR model download cancelled"
    );
    Ok(())
}

fn gpu_ocr_provider() -> Result<ExecutionProvider> {
    #[cfg(all(target_os = "linux", feature = "rocm-ocr"))]
    let gpu_provider = std::env::var_os("ORT_MIGRAPHX_MODEL_CACHE_PATH")
        .filter(|path| Path::new(path).is_dir())
        .map(|_| ExecutionProvider::Migraphx);
    #[cfg(all(target_os = "linux", not(feature = "rocm-ocr")))]
    let gpu_provider: Option<ExecutionProvider> = None;
    #[cfg(target_os = "macos")]
    let gpu_provider = Some(ExecutionProvider::CoreMl);
    #[cfg(target_os = "windows")]
    let gpu_provider = Some(ExecutionProvider::DirectMl);
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let gpu_provider: Option<ExecutionProvider> = None;
    gpu_provider.context(
        "This Lunchbox build has no GPU OCR backend. Translation will not use CPU OCR; install a GPU-capable build.",
    )
}

pub fn preflight_gpu_ocr() -> Result<()> {
    let _ = gpu_ocr_provider()?;
    Ok(())
}

fn load_ocr() -> Result<RapidOcr> {
    let provider = gpu_ocr_provider()?;
    let cache = ocr_model_cache()?;
    let model_set = ocr_model_set()?;
    cache
        .ensure_model_set_for_pipeline(
            model_set,
            PipelineConfig::without_cls(),
            ModelDownloadMode::Never,
        )
        .context("OCR models are not installed; download models in Settings")?;
    let config = cache
        .config_for(model_set)
        .with_pipeline(PipelineConfig::without_cls());
    let options = InferenceOptions {
        intra_threads: std::thread::available_parallelism()
            .map(|threads| threads.get())
            .unwrap_or(2)
            .min(4),
        enable_cpu_mem_arena: true,
        ..Default::default()
    };
    let ocr = RapidOcr::from_config(config.with_inference_options(InferenceOptions {
        execution_provider: provider,
        enable_cpu_mem_arena: false,
        ..options
    }))
    .with_context(|| {
        format!("GPU OCR ({provider:?}) could not initialize; CPU fallback is disabled")
    })?;
    eprintln!("LUNCHBOX_TRANSLATION_OCR_PROVIDER={provider:?}");
    Ok(ocr)
}

fn loaded_model_gpu_fraction(body: &Value, model: &str) -> Result<Option<u64>> {
    let entry = body["models"]
        .as_array()
        .and_then(|models| models.iter().find(|entry| entry["name"] == model));
    let Some(entry) = entry else {
        return Ok(None);
    };
    let size = entry["size"]
        .as_u64()
        .context("Ollama did not report model size")?;
    let gpu = entry["size_vram"]
        .as_u64()
        .context("Ollama did not report GPU allocation")?;
    ensure!(size > 0, "Ollama reported an empty model");
    Ok(Some(gpu.saturating_mul(100) / size))
}

fn active_model_gpu_percent(model: &str) -> Result<Option<u64>> {
    let mut response = http_agent(Duration::from_secs(5))
        .get(&format!("{OLLAMA_URL}/api/ps"))
        .call()
        .context("checking translation model GPU allocation")?;
    ensure!(
        response.status().as_u16() == 200,
        "Ollama model status failed"
    );
    let body: Value = serde_json::from_str(&response.body_mut().read_to_string()?)?;
    loaded_model_gpu_fraction(&body, model)
}

fn verify_model_gpu(model: &str) -> Result<()> {
    // An empty prompt loads the model without generating text. Check its actual
    // VRAM allocation: seeing a device node alone does not prove GPU inference.
    let mut load = http_agent(Duration::from_secs(120))
        .post(&format!("{OLLAMA_URL}/api/generate"))
        .send_json(json!({"model": model, "prompt": "", "stream": false, "keep_alive": "10m"}))
        .context("loading the translation model")?;
    ensure!(
        load.status().as_u16() == 200,
        "Ollama could not load {model}"
    );
    // Drain the response before querying /api/ps, including on keep-alive HTTP.
    let _ = load.body_mut().read_to_string()?;
    let percent = active_model_gpu_percent(model)?.context("translation model did not load")?;
    ensure!(
        percent >= 95,
        "{model} is only {percent}% on GPU; CPU or partial-CPU inference is disabled. Free GPU memory or choose a smaller model."
    );
    eprintln!("LUNCHBOX_TRANSLATION_MODEL_GPU_PERCENT={percent}");
    Ok(())
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
    let ollama_ready = body["models"]
        .as_array()
        .is_some_and(|models| models.iter().any(|entry| entry["name"] == model));
    Ok(ollama_ready && ocr_models_available()?)
}

pub fn gpu_translation_ready(model: &str) -> Result<bool> {
    if !model_available(model)? {
        return Ok(false);
    }
    let mut ocr = load_ocr()?;
    ocr.warm_up_gpu(&OcrCancellationToken::new())
        .context("warming up GPU OCR for setup verification")?;
    verify_model_gpu(model)?;
    Ok(true)
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
    preflight_gpu_ocr()?;
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
            (Some(done), Some(total)) if total > 0 => (done * 89 / total).min(89) as u8,
            _ => 0,
        };
        progress(fraction, format!("{model}: {status}"));
        if status == "success" {
            complete = true;
        }
    }
    ensure!(complete, "Ollama did not finish downloading {model}");
    progress(90, "Installing local Japanese-capable OCR".to_owned());
    download_ocr_models(cancelled)?;
    let mut ocr = load_ocr()?;
    progress(
        95,
        "Compiling local GPU OCR (first setup may take a few minutes)".to_owned(),
    );
    ocr.warm_up_gpu(&OcrCancellationToken::new())
        .context("compiling local GPU OCR")?;
    verify_model_gpu(model)?;
    ensure!(
        !cancelled.load(Ordering::Relaxed),
        "OCR model setup cancelled"
    );
    ensure!(model_available(model)?, "local models are incomplete");
    progress(100, "OCR and translation are ready".to_owned());
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
        output_dimensions: Option<(u32, u32)>,
    ) -> Result<Option<Self>> {
        if !settings.enabled || plan.retroarch_content.is_none() {
            return Ok(None);
        }
        settings.validate()?;
        ensure!(
            model_available(&settings.model)?,
            "{}, and the local OCR models are required; download them in Settings",
            settings.model
        );
        let mut ocr = load_ocr()?;
        // Pay the one-time GPU graph initialization before emulation starts,
        // not on the first dialogue frame while the game is playing.
        ocr.warm_up_gpu(&OcrCancellationToken::new())
            .context("warming up local GPU OCR before launch")?;
        verify_model_gpu(&settings.model)?;
        let viewport = overlay_viewport(plan, executable, output_dimensions)?;
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
            .spawn(move || serve(listener, &secret, settings, ocr, viewport, &worker_stop))
            .context("starting local translation bridge")?;
        Ok(Some(Self { stop }))
    }

    /// Rebind the URL already embedded in a Lunchbox-owned, still-running
    /// RetroArch session after the UI process was restarted. The emulator
    /// keeps its launch config; assigning a new port would never reach it.
    pub fn recover_running(
        settings: &TranslationSettings,
        output_dimensions: Option<(u32, u32)>,
    ) -> Result<Option<Self>> {
        if !settings.enabled
            || !crate::emulator_session::active()?.is_some_and(|session| !session.preparing())
        {
            return Ok(None);
        }
        let Some(output) = output_dimensions.filter(|(width, height)| *width > 0 && *height > 0)
        else {
            return Ok(None);
        };
        let mut system = System::new();
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
        );
        for process in system.processes().values() {
            if !process
                .name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with("retroarch")
                || matches!(
                    process.status(),
                    ProcessStatus::Zombie | ProcessStatus::Dead
                )
            {
                continue;
            }
            let arguments = process.cmd();
            let Some((port, secret)) = retroarch_translation_endpoint(arguments)? else {
                continue;
            };
            let listener = match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)) {
                Ok(listener) => listener,
                Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => return Ok(None),
                Err(error) => return Err(error).context("restoring local translation bridge"),
            };
            settings.validate()?;
            ensure!(
                model_available(&settings.model)?,
                "translation model is unavailable"
            );
            let mut ocr = load_ocr()?;
            ocr.warm_up_gpu(&OcrCancellationToken::new())
                .context("warming recovered GPU OCR")?;
            verify_model_gpu(&settings.model)?;
            let viewport = overlay_viewport_from_arguments(arguments, output)?;
            listener.set_nonblocking(true)?;
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = Arc::clone(&stop);
            let settings = settings.clone();
            thread::Builder::new()
                .name("lunchbox-translation-bridge".into())
                .spawn(move || serve(listener, &secret, settings, ocr, viewport, &worker_stop))
                .context("restoring local translation bridge")?;
            eprintln!("LUNCHBOX_TRANSLATION_RECOVERED port={port}");
            return Ok(Some(Self { stop }));
        }
        Ok(None)
    }
}

/// RetroArch 1.22's AI widget stretches the returned PNG over the full video
/// output, even when the game uses a smaller custom viewport inside artwork.
/// Read the exact launch config and map source-frame rectangles into that
/// viewport before drawing the transparent response image.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OverlayViewport {
    output: (u32, u32),
    game: TextRect,
    content_zoom_percent: u32,
}

fn config_u32(contents: &str, key: &str) -> Option<u32> {
    contents.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key)
            .then(|| value.trim().trim_matches('"').parse::<u32>().ok())
            .flatten()
    })
}

fn overlay_viewport(
    plan: &LaunchPlan,
    executable: &EmulatorExecutable,
    output_dimensions: Option<(u32, u32)>,
) -> Result<Option<OverlayViewport>> {
    let arguments = retroarch_app_arguments(&plan.arguments, executable)?;
    let output =
        crate::display_setup::probe_retroarch_output_dimensions(executable, output_dimensions)?;
    overlay_viewport_from_arguments(arguments, output)
}

fn appended_config_paths(arguments: &[OsString]) -> Result<Vec<PathBuf>> {
    let Some(index) = crate::controller_launch_modes::append_config_index(arguments)? else {
        return Ok(Vec::new());
    };
    let value = if arguments[index] == "--appendconfig" {
        arguments.get(index + 1).and_then(|arg| arg.to_str())
    } else {
        arguments[index]
            .to_str()
            .and_then(|arg| arg.strip_prefix("--appendconfig="))
    };
    Ok(value
        .map(|value| value.split('|').map(PathBuf::from).collect())
        .unwrap_or_default())
}

fn retroarch_translation_endpoint(arguments: &[OsString]) -> Result<Option<(u16, String)>> {
    for config in appended_config_paths(arguments)? {
        // Never revive a user-owned AI service or an unrelated RetroArch.
        if !config
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("retroarch-") && name.ends_with(".cfg"))
            || !config
                .parent()
                .is_some_and(|parent| parent.ends_with("lunchbox/launch-display"))
        {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(config) else {
            continue;
        };
        for line in contents.lines() {
            let Some(value) = line.strip_prefix("ai_service_url = ") else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            let Some((port, secret)) = value
                .strip_prefix("http://127.0.0.1:")
                .and_then(|value| value.split_once('/'))
            else {
                continue;
            };
            if let Ok(port) = port.parse::<u16>()
                && port > 0
                && secret.len() == 32
                && secret.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Ok(Some((port, secret.to_owned())));
            }
        }
    }
    Ok(None)
}

fn overlay_viewport_from_arguments(
    arguments: &[OsString],
    output: (u32, u32),
) -> Result<Option<OverlayViewport>> {
    let mut viewport_width = None;
    let mut viewport_height = None;
    let mut viewport_x = 0;
    let mut viewport_y = 0;
    for config in appended_config_paths(arguments)? {
        let Ok(contents) = std::fs::read_to_string(&config) else {
            continue;
        };
        viewport_width = config_u32(&contents, "custom_viewport_width").or(viewport_width);
        viewport_height = config_u32(&contents, "custom_viewport_height").or(viewport_height);
        viewport_x = config_u32(&contents, "custom_viewport_x").unwrap_or(viewport_x);
        viewport_y = config_u32(&contents, "custom_viewport_y").unwrap_or(viewport_y);
    }
    let (Some(width), Some(height)) = (viewport_width, viewport_height) else {
        return Ok(None);
    };
    ensure!(
        width > 0 && height > 0 && width <= output.0 && height <= output.1,
        "translation game viewport does not fit the video output"
    );
    let x1 = (output.0 - width) / 2 + viewport_x;
    let y1 = (output.1 - height) / 2 + viewport_y;
    ensure!(
        x1.checked_add(width).is_some_and(|end| end <= output.0)
            && y1.checked_add(height).is_some_and(|end| end <= output.1),
        "translation game viewport is outside the video output"
    );
    Ok(Some(OverlayViewport {
        output,
        game: TextRect {
            x1,
            y1,
            x2: x1 + width,
            y2: y1 + height,
        },
        // Koko AIO's RetroTube content is visibly zoomed inside the custom
        // viewport. The AI widget is drawn *after* the shader, so its raw
        // core-frame coordinates need the same centered zoom. Use a
        // dimensionless preset correction, never monitor-specific pixels.
        content_zoom_percent: if retrotube_shader_active(arguments) {
            120
        } else {
            100
        },
    }))
}

fn retrotube_shader_active(arguments: &[OsString]) -> bool {
    arguments.iter().any(|argument| {
        argument
            .to_str()
            .and_then(|value| value.strip_prefix("--set-shader="))
            .and_then(|value| Path::new(value).file_stem())
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("retrotube-tv-"))
    })
}

fn retroarch_app_arguments<'a>(
    arguments: &'a [OsString],
    executable: &EmulatorExecutable,
) -> Result<&'a [OsString]> {
    if let EmulatorExecutable::Flatpak { app_id, .. } = executable {
        let boundary = arguments
            .iter()
            .position(|argument| argument.to_str() == Some(app_id))
            .context("Missing Flatpak RetroArch app boundary")?;
        Ok(&arguments[boundary + 1..])
    } else {
        Ok(arguments)
    }
}

fn retroarch_session_config(port: u16, secret: &str) -> String {
    format!(
        "ai_service_enable = \"true\"\nai_service_url = \"http://127.0.0.1:{port}/{secret}\"\nai_service_mode = \"0\"\nai_service_source_lang = \"0\"\nai_service_target_lang = \"1\"\nai_service_pause = \"false\"\naudio_latency = \"128\"\nmenu_enable_widgets = \"true\"\ninput_ai_service = \"{TRANSLATION_HOTKEY}\"\n"
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
    settings: TranslationSettings,
    ocr: RapidOcr,
    viewport: Option<OverlayViewport>,
    stop: &AtomicBool,
) {
    let (jobs, pending_jobs) = mpsc::sync_channel::<TranslationJob>(1);
    let (completed_jobs, results) = mpsc::channel();
    let worker_settings = settings;
    let worker = thread::Builder::new()
        .name("lunchbox-translation-worker".into())
        .spawn(move || {
            let mut cached: Option<CachedTranslation> = None;
            let mut memory = TranslationMemory::default();
            let mut ocr = ocr;
            // Empty-prompt loading primes the weights but not first-token
            // generation. Warm inference alongside RetroArch startup rather
            // than blocking the game launch or the first F10 capture.
            if let Err(error) = ollama_chat(
                &worker_settings.model,
                "Translate Japanese to English. Return only English: ありがとう",
                OLLAMA_URL,
            ) {
                eprintln!("LUNCHBOX_TRANSLATION_WARMUP_FAILED: {error:#}");
            }
            for job in pending_jobs {
                let result = render_translation(
                    &worker_settings,
                    &job.image,
                    job.dimensions.0,
                    job.dimensions.1,
                    &mut ocr,
                    viewport,
                    cached.as_ref(),
                    &mut memory,
                )
                .and_then(|(regions, overlay)| {
                    let screenshot =
                        image::load_from_memory(&BASE64.decode(&job.image)?)?.into_rgb8();
                    let region_fingerprints = regions
                        .iter()
                        .map(|region| fingerprint_region(&screenshot, region.rect))
                        .collect();
                    Ok(CachedTranslation {
                        digest: job.digest,
                        regions,
                        region_fingerprints,
                        dimensions: job.dimensions,
                        overlay,
                    })
                });
                if let Ok(translation) = &result
                    && !translation.regions.is_empty()
                {
                    cached = Some(translation.clone());
                }
                if completed_jobs.send((job.digest, result)).is_err() {
                    break;
                }
            }
        });
    let Ok(_worker) = worker else {
        eprintln!("LUNCHBOX_TRANSLATION_BRIDGE_FAILED: could not start translation worker");
        return;
    };
    let mut state = TranslationBridge {
        jobs,
        results,
        cached: None,
        in_flight: None,
        retry_after: None,
        last_started_at: None,
        last_response_at: None,
        blank_overlay: None,
    };
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                if let Err(error) = handle_request(&mut stream, secret, viewport, &mut state) {
                    eprintln!("LUNCHBOX_TRANSLATION_REQUEST_FAILED: {error:#}");
                    // Keep RetroArch's automatic capture loop alive after a
                    // transient worker or socket error.
                    let _ = write_json(
                        &mut stream,
                        200,
                        &json!({"error": error.to_string(), "auto": "auto"}),
                    );
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

struct TranslationJob {
    digest: [u8; 32],
    dimensions: (u32, u32),
    image: String,
}

struct TranslationBridge {
    jobs: mpsc::SyncSender<TranslationJob>,
    results: mpsc::Receiver<([u8; 32], Result<CachedTranslation>)>,
    cached: Option<CachedTranslation>,
    in_flight: Option<[u8; 32]>,
    retry_after: Option<([u8; 32], Instant)>,
    last_started_at: Option<Instant>,
    last_response_at: Option<Instant>,
    blank_overlay: Option<((u32, u32), String)>,
}

fn pace_auto_response(state: &mut TranslationBridge) {
    if let Some(last) = state.last_response_at
        && let Some(wait) = AUTO_REQUEST_INTERVAL.checked_sub(last.elapsed())
    {
        thread::sleep(wait);
    }
    state.last_response_at = Some(Instant::now());
}

#[derive(Clone)]
struct CachedTranslation {
    digest: [u8; 32],
    regions: Vec<TranslatedRegion>,
    region_fingerprints: Vec<[u8; 32]>,
    dimensions: (u32, u32),
    overlay: String,
}

#[derive(Default)]
struct TranslationMemory {
    entries: HashMap<String, String>,
    order: VecDeque<String>,
}

impl TranslationMemory {
    fn key(source: &str) -> String {
        source.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn get(&self, source: &str) -> Option<&str> {
        self.entries.get(&Self::key(source)).map(String::as_str)
    }

    fn insert(&mut self, source: &str, english: &str) {
        if english.trim().is_empty() {
            return;
        }
        let key = Self::key(source);
        if key.is_empty() || self.entries.contains_key(&key) {
            return;
        }
        if self.entries.len() == MAX_TRANSLATION_MEMORY
            && let Some(oldest) = self.order.pop_front()
        {
            self.entries.remove(&oldest);
        }
        self.entries.insert(key.clone(), english.to_owned());
        self.order.push_back(key);
    }
}

fn fingerprint_region(image: &RgbImage, rect: TextRect) -> [u8; 32] {
    let mut digest = Sha256::new();
    for y in rect.y1..rect.y2 {
        for x in rect.x1..rect.x2 {
            digest.update(image.get_pixel(x, y).0);
        }
    }
    digest.finalize().into()
}

fn cached_overlay_matches_frame(cached: &CachedTranslation, digest: [u8; 32]) -> bool {
    // Matching only the *old* text rectangles hid newly appearing text
    // elsewhere on a still-animated screen forever. Re-run OCR whenever the
    // frame changes, but reuse unchanged region translations in the worker.
    // Empty OCR results remain provisional even on an identical frame.
    !cached.regions.is_empty() && cached.digest == digest
}

fn cached_overlay_still_visible(
    cached: &CachedTranslation,
    screenshot: &[u8],
    dimensions: (u32, u32),
) -> bool {
    if cached.regions.is_empty()
        || cached.dimensions != dimensions
        || cached.regions.len() != cached.region_fingerprints.len()
    {
        return false;
    }
    let Ok(image) = image::load_from_memory(screenshot) else {
        return false;
    };
    let image = image.into_rgb8();
    if image.dimensions() != dimensions {
        return false;
    }
    cached
        .regions
        .iter()
        .zip(&cached.region_fingerprints)
        .all(|(region, fingerprint)| fingerprint_region(&image, region.rect) == *fingerprint)
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
    source_line_height: u32,
    source: String,
    english: String,
    background: [u8; 3],
}

fn handle_request(
    stream: &mut TcpStream,
    secret: &str,
    viewport: Option<OverlayViewport>,
    state: &mut TranslationBridge,
) -> Result<()> {
    let request_started = Instant::now();
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
    let digest: [u8; 32] = Sha256::digest(&decoded).into();
    while let Ok((completed_digest, result)) = state.results.try_recv() {
        accept_translation_result(state, completed_digest, result);
    }
    let same_text = state
        .cached
        .as_ref()
        .is_some_and(|cached| cached_overlay_matches_frame(cached, digest));
    if same_text {
        pace_auto_response(state);
        return write_json(
            stream,
            200,
            &json!({"image": state.cached.as_ref().unwrap().overlay, "auto": "auto"}),
        );
    }
    let can_retry = !state
        .retry_after
        .is_some_and(|(failed, until)| failed == digest && Instant::now() < until);
    let can_start = state.in_flight.is_none()
        && can_retry
        && state
            .last_started_at
            .is_none_or(|last| last.elapsed() >= AUTO_REQUEST_INTERVAL);
    if can_start {
        state.jobs.try_send(TranslationJob {
            digest,
            dimensions: (width, height),
            image: model_image,
        })?;
        state.in_flight = Some(digest);
        state.last_started_at = Some(Instant::now());
    }
    // RetroArch's HTTP task is asynchronous. Wait for the worker during this
    // request so a ready caption is returned with the captured frame, rather
    // than showing a placeholder and waiting for one more screenshot cycle.
    if state.in_flight.is_some() {
        match state.results.recv_timeout(RESULT_WAIT_TIMEOUT) {
            Ok((completed_digest, result)) => {
                accept_translation_result(state, completed_digest, result)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("translation worker stopped")
            }
        }
    }
    pace_auto_response(state);
    // Inference may have finished during the one-second pacing interval.
    // Pick it up before replying instead of making RetroArch wait for another
    // entire capture cycle to see the caption.
    while let Ok((completed_digest, result)) = state.results.try_recv() {
        accept_translation_result(state, completed_digest, result);
    }
    // A warm model often finishes just after the one-second capture interval.
    // Give this screenshot a bounded chance to receive its own caption instead
    // of discarding it and waiting for another capture of a changed scene.
    if state.in_flight.is_some()
        && let Some(remaining) = RESULT_RESPONSE_BUDGET.checked_sub(request_started.elapsed())
    {
        match state.results.recv_timeout(remaining) {
            Ok((completed_digest, result)) => {
                accept_translation_result(state, completed_digest, result)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("translation worker stopped")
            }
        }
        state.last_response_at = Some(Instant::now());
    }
    if let Some(cached) = &state.cached
        && (cached_overlay_matches_frame(cached, digest)
            || cached_overlay_still_visible(cached, &decoded, (width, height)))
    {
        return write_json(
            stream,
            200,
            &json!({"image": cached.overlay, "auto": "auto"}),
        );
    }
    let overlay = blank_overlay(state, viewport, (width, height))?;
    write_json(stream, 200, &json!({"image": overlay, "auto": "auto"}))?;
    Ok(())
}

fn accept_translation_result(
    state: &mut TranslationBridge,
    completed_digest: [u8; 32],
    result: Result<CachedTranslation>,
) {
    state.in_flight = None;
    match result {
        Ok(translation) => {
            // A single OCR miss must not erase a caption whose source pixels
            // are still visible. Empty reads remain provisional so the next
            // frame is still sent through OCR.
            if !translation.regions.is_empty() || state.cached.is_none() {
                state.cached = Some(translation);
            }
            state.retry_after = None;
        }
        Err(error) => {
            eprintln!("LUNCHBOX_TRANSLATION_REQUEST_FAILED: {error:#}");
            state.retry_after = Some((completed_digest, Instant::now() + Duration::from_secs(2)));
        }
    }
}

fn blank_overlay<'a>(
    state: &'a mut TranslationBridge,
    viewport: Option<OverlayViewport>,
    frame: (u32, u32),
) -> Result<&'a str> {
    let output = viewport.map(overlay_image_dimensions).unwrap_or_else(|| {
        let width = frame.0.min(2048);
        (
            width,
            (u64::from(width) * u64::from(frame.1) / u64::from(frame.0)) as u32,
        )
    });
    if state.blank_overlay.as_ref().map(|(size, _)| *size) != Some(output) {
        state.blank_overlay = Some((
            output,
            BASE64.encode(render_regions_png(output.0, output.1, &[])?),
        ));
    }
    Ok(&state.blank_overlay.as_ref().unwrap().1)
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

fn text_region_allowed(rect: TextRect, width: u32, height: u32) -> bool {
    // Dialogue often fills almost the entire width of a short text strip.
    // The old 95% width cap split those lines in two; keep rejecting broad
    // panels that also consume substantial vertical space.
    let short_dialogue_line = u64::from(rect.height()) * 100 <= u64::from(height) * 12;
    rect.x1 < rect.x2
        && rect.y1 < rect.y2
        && rect.x2 <= width
        && rect.y2 <= height
        && rect.width() >= 8
        && rect.height() >= 5
        && (u64::from(rect.width()) * 100 <= u64::from(width) * 95 || short_dialogue_line)
        && u64::from(rect.height()) * 100 <= u64::from(height) * 30
        && u64::from(rect.width()) * u64::from(rect.height()) * 100
            <= u64::from(width) * u64::from(height) * 20
}

#[derive(Clone, Copy, Debug)]
struct TextGroup {
    rect: TextRect,
    line_height: u32,
}

fn nearby_text(a: TextGroup, b: TextGroup, allow_multiline: bool) -> bool {
    let line_height = a.line_height.max(b.line_height).max(8);
    let vertical_gap = axis_gap(a.rect.y1, a.rect.y2, b.rect.y1, b.rect.y2);
    let horizontal_gap = axis_gap(a.rect.x1, a.rect.x2, b.rect.x1, b.rect.x2);
    let horizontal_overlap = axis_overlap(a.rect.x1, a.rect.x2, b.rect.x1, b.rect.x2);
    let same_line = axis_overlap(a.rect.y1, a.rect.y2, b.rect.y1, b.rect.y2)
        >= a.rect.height().min(b.rect.height()) / 2
        && horizontal_gap <= line_height.saturating_mul(3) / 2;
    same_line
        || (allow_multiline
            && vertical_gap <= line_height
            && horizontal_overlap >= a.rect.width().min(b.rect.width()) / 2)
}

fn group_text_regions(mut boxes: Vec<TextRect>, width: u32, height: u32) -> Vec<TextGroup> {
    if let Some((grid_top, line_height)) = dense_grid_start(&boxes, width, height) {
        // A character picker or similarly dense menu grid is not dialogue.
        // OCRing its full table produces speculative English and obscures the
        // controls. Keep the separate labels above it, not the grid itself.
        boxes.retain(|rect| {
            rect.y1 < grid_top
                && !(rect.width() > width / 2 && rect.y2.saturating_add(line_height) >= grid_top)
        });
    }
    // Dialogue boxes often have more than four OCR lines. The dense character
    // grid has already been removed above, so do not split longer dialogue
    // into isolated lines and discard its lower half.
    let allow_multiline = true;
    boxes.sort_by_key(|rect| (rect.y1, rect.x1));
    let mut groups: Vec<TextGroup> = Vec::new();
    for rect in boxes {
        if !text_region_allowed(rect, width, height) {
            continue;
        }
        let mut group = TextGroup {
            rect,
            line_height: rect.height(),
        };
        while let Some(index) = groups.iter().position(|other| {
            let merged = group.rect.union(other.rect);
            let line_height = group.line_height.max(other.line_height);
            nearby_text(*other, group, allow_multiline)
                && merged.height() <= line_height.saturating_mul(5)
                && text_region_allowed(merged, width, height)
        }) {
            let other = groups.swap_remove(index);
            group.rect = group.rect.union(other.rect);
            group.line_height = group.line_height.max(other.line_height);
        }
        groups.push(group);
    }
    groups.sort_by_key(|group| std::cmp::Reverse(group.rect.width() * group.rect.height()));
    groups.truncate(8);
    groups.sort_by_key(|group| (group.rect.y1, group.rect.x1));
    groups
}

#[cfg(test)]
fn group_text_boxes(boxes: Vec<TextRect>, width: u32, height: u32) -> Vec<TextRect> {
    group_text_regions(boxes, width, height)
        .into_iter()
        .map(|group| group.rect)
        .collect()
}

fn dense_grid_start(boxes: &[TextRect], width: u32, height: u32) -> Option<(u32, u32)> {
    if boxes.len() < 12 {
        return None;
    }
    let mut candidates = boxes
        .iter()
        .filter(|rect| rect.width() < width / 2 && rect.height() < height / 8)
        .copied()
        .collect::<Vec<_>>();
    if candidates.len() < 12 {
        return None;
    }
    let mut heights = candidates
        .iter()
        .map(|rect| rect.height())
        .collect::<Vec<_>>();
    heights.sort_unstable();
    let line_height = heights[heights.len() / 2].max(8);
    candidates.sort_by_key(|rect| rect.y1);
    let mut rows: Vec<(u32, u32)> = Vec::new();
    for rect in candidates {
        if let Some((row_y, count)) = rows.last_mut()
            && rect.y1.abs_diff(*row_y) <= line_height / 2
        {
            *count += 1;
        } else {
            rows.push((rect.y1, 1));
        }
    }
    let dense_rows = rows
        .into_iter()
        .filter(|(_, count)| *count >= 3)
        .map(|(y, _)| y)
        .collect::<Vec<_>>();
    dense_rows.windows(4).find_map(|window| {
        window
            .windows(2)
            .all(|pair| pair[1] - pair[0] <= line_height * 3)
            .then_some((window[0], line_height))
    })
}

fn detect_text_regions(ocr: &mut RapidOcr, image: &RgbImage) -> Result<Vec<(TextGroup, String)>> {
    let (width, height) = image.dimensions();
    let lines = ocr
        .run_image(image)
        .context("reading frame with local OCR")?
        .lines
        .into_iter()
        .filter_map(|line| {
            if line.score < 0.3 || line.text.trim().is_empty() {
                return None;
            }
            let corners = line.bbox.points;
            let x1 = corners
                .iter()
                .map(|point| point[0])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .clamp(0.0, width as f32) as u32;
            let y1 = corners
                .iter()
                .map(|point| point[1])
                .fold(f32::INFINITY, f32::min)
                .floor()
                .clamp(0.0, height as f32) as u32;
            let x2 = corners
                .iter()
                .map(|point| point[0])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .clamp(0.0, width as f32) as u32;
            let y2 = corners
                .iter()
                .map(|point| point[1])
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .clamp(0.0, height as f32) as u32;
            let rect = TextRect { x1, y1, x2, y2 };
            text_region_allowed(rect, width, height).then_some((rect, line.text))
        })
        .collect::<Vec<_>>();
    let groups = group_text_regions(lines.iter().map(|(rect, _)| *rect).collect(), width, height);
    Ok(groups
        .into_iter()
        .filter_map(|group| {
            let mut members = lines
                .iter()
                .filter(|(rect, _)| {
                    let center_x = (rect.x1 + rect.x2) / 2;
                    let center_y = (rect.y1 + rect.y2) / 2;
                    center_x >= group.rect.x1
                        && center_x <= group.rect.x2
                        && center_y >= group.rect.y1
                        && center_y <= group.rect.y2
                })
                .collect::<Vec<_>>();
            members.sort_by_key(|(rect, _)| (rect.y1, rect.x1));
            let source = members
                .into_iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (!source.is_empty()).then_some((group, source))
        })
        .collect())
}

fn sample_text_background(image: &RgbImage, rect: TextRect) -> [u8; 3] {
    let (width, height) = image.dimensions();
    let ring = rect.padded(width, height, 3);
    // The old ring-only median could pick scenery just outside a dialogue
    // panel. Text occupies fewer pixels than its background inside the OCR
    // box, so choose the dominant quantized color from both areas and give
    // the interior twice the weight of the surrounding ring.
    let mut buckets = vec![[0u64; 4]; 16 * 16 * 16];
    for y in ring.y1..ring.y2 {
        for x in ring.x1..ring.x2 {
            let [red, green, blue] = image.get_pixel(x, y).0;
            let index = (usize::from(red >> 4) << 8)
                | (usize::from(green >> 4) << 4)
                | usize::from(blue >> 4);
            let weight = u64::from(
                if x >= rect.x1 && x < rect.x2 && y >= rect.y1 && y < rect.y2 {
                    2u8
                } else {
                    1u8
                },
            );
            let bucket = &mut buckets[index];
            bucket[0] += weight;
            bucket[1] += weight * u64::from(red);
            bucket[2] += weight * u64::from(green);
            bucket[3] += weight * u64::from(blue);
        }
    }
    let Some(bucket) = buckets.iter().max_by_key(|bucket| bucket[0]) else {
        return [9, 14, 22];
    };
    if bucket[0] == 0 {
        return [9, 14, 22];
    }
    [
        (bucket[1] / bucket[0]) as u8,
        (bucket[2] / bucket[0]) as u8,
        (bucket[3] / bucket[0]) as u8,
    ]
}

fn cached_region<'a>(
    cached: Option<&'a CachedTranslation>,
    screenshot: &RgbImage,
    dimensions: (u32, u32),
    rect: TextRect,
) -> Option<&'a TranslatedRegion> {
    let old = cached.filter(|old| old.dimensions == dimensions)?;
    let fingerprint = fingerprint_region(screenshot, rect);
    old.regions
        .iter()
        .enumerate()
        .find(|(index, region)| {
            region.rect.near(rect) && old.region_fingerprints.get(*index) == Some(&fingerprint)
        })
        .map(|(_, region)| region)
}

fn is_numeric_hud_label(source: &str) -> bool {
    let source = source.trim();
    source.len() <= 12
        && source.bytes().any(|byte| byte.is_ascii_digit())
        && source
            .bytes()
            .filter(|byte| byte.is_ascii_alphabetic())
            .count()
            <= 4
        && source
            .bytes()
            .all(|byte| byte.is_ascii() && !byte.is_ascii_lowercase())
}

fn render_translation(
    settings: &TranslationSettings,
    image: &str,
    width: u32,
    height: u32,
    ocr: &mut RapidOcr,
    viewport: Option<OverlayViewport>,
    cached: Option<&CachedTranslation>,
    memory: &mut TranslationMemory,
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
    let started = Instant::now();
    let mut boxes = detect_text_regions(ocr, &screenshot)?
        .into_iter()
        .filter(|(group, _)| {
            group.line_height <= (height / 8).max(24)
                || group.rect.width() >= group.line_height.saturating_mul(2)
        })
        .collect::<Vec<_>>();
    let ocr_elapsed = started.elapsed();
    boxes.sort_by_key(|(group, _)| std::cmp::Reverse(group.rect.width()));
    boxes.truncate(8);
    boxes.sort_by_key(|(group, _)| (group.rect.y1, group.rect.x1));
    if std::env::var_os("LUNCHBOX_TRANSLATION_SOURCE_IMAGE").is_some() {
        eprintln!("LUNCHBOX_TRANSLATION_PROBE_CANDIDATES: {boxes:?}");
    }
    let candidate_count = boxes.len();
    let mut regions = Vec::new();
    let mut pixel_reused = 0;
    let mut text_reused = 0;
    let pending = boxes
        .iter()
        .enumerate()
        .filter(|(_, (detected, source))| {
            source.chars().filter(|ch| ch.is_alphabetic()).count() >= 2
                && !is_numeric_hud_label(source)
                && cached_region(cached, &screenshot, (width, height), detected.rect).is_none()
                && memory.get(source).is_none()
        })
        .map(|(index, (_, source))| (index, source.clone()))
        .collect::<Vec<_>>();
    let model_requested = pending.len();
    let translated = match translate_texts_at(
        settings,
        &pending
            .iter()
            .map(|(_, source)| source.clone())
            .collect::<Vec<_>>(),
        OLLAMA_URL,
    ) {
        Ok(english) => pending
            .iter()
            .map(|(index, _)| *index)
            .zip(english)
            .collect::<std::collections::HashMap<_, _>>(),
        Err(error) => {
            eprintln!("LUNCHBOX_TRANSLATION_MODEL_FAILED: {error:#}");
            std::collections::HashMap::new()
        }
    };
    for (index, (detected, recognized)) in boxes.into_iter().enumerate() {
        let rect = detected.rect;
        let previous = cached_region(cached, &screenshot, (width, height), rect);
        let (source, english) = if let Some(previous) = previous {
            pixel_reused += 1;
            (previous.source.clone(), previous.english.clone())
        } else {
            let source = recognized;
            if source.chars().filter(|ch| ch.is_alphabetic()).count() < 2
                || is_numeric_hud_label(&source)
            {
                continue;
            }
            let english = if let Some(english) = memory.get(&source) {
                text_reused += 1;
                english.to_owned()
            } else {
                translated.get(&index).cloned().unwrap_or_default()
            };
            (source, english)
        };
        if english.is_empty() {
            continue;
        }
        memory.insert(&source, &english);
        regions.push(TranslatedRegion {
            rect,
            source_line_height: detected.line_height,
            source,
            english,
            background: sample_text_background(&screenshot, rect),
        });
    }
    eprintln!(
        "LUNCHBOX_TRANSLATION_RENDER elapsed_ms={} ocr_ms={} candidates={} translated={} model_requested={} pixel_reused={} text_reused={}",
        started.elapsed().as_millis(),
        ocr_elapsed.as_millis(),
        candidate_count,
        regions.len(),
        model_requested,
        pixel_reused,
        text_reused,
    );
    if regions.is_empty() {
        let output = viewport
            .map(overlay_image_dimensions)
            .unwrap_or((width, height));
        return Ok((
            regions,
            BASE64.encode(render_regions_png(output.0, output.1, &[])?),
        ));
    }
    let overlay = if let Some(viewport) = viewport {
        let output = overlay_image_dimensions(viewport);
        let transformed = regions
            .iter()
            .map(|region| TranslatedRegion {
                rect: map_overlay_rect(region.rect, (width, height), viewport, output),
                source_line_height: map_overlay_line_height(
                    region.source_line_height,
                    (width, height),
                    viewport,
                    output,
                ),
                source: region.source.clone(),
                english: region.english.clone(),
                background: region.background,
            })
            .collect::<Vec<_>>();
        render_regions_png(output.0, output.1, &transformed)?
    } else {
        render_regions_png(width, height, &regions)?
    };
    Ok((regions, BASE64.encode(overlay)))
}

fn overlay_image_dimensions(viewport: OverlayViewport) -> (u32, u32) {
    let width = 2048;
    let height =
        (u64::from(width) * u64::from(viewport.output.1) / u64::from(viewport.output.0)) as u32;
    (width, height.max(64))
}

fn map_overlay_rect(
    rect: TextRect,
    source: (u32, u32),
    viewport: OverlayViewport,
    output: (u32, u32),
) -> TextRect {
    // CRT artwork is outside the game's opening. A zoom correction may move
    // text toward that edge, but captions must never paint over the bezel.
    let opening = TextRect {
        x1: (u64::from(viewport.game.x1) * u64::from(output.0) / u64::from(viewport.output.0))
            as u32,
        y1: (u64::from(viewport.game.y1) * u64::from(output.1) / u64::from(viewport.output.1))
            as u32,
        x2: (u64::from(viewport.game.x2) * u64::from(output.0) / u64::from(viewport.output.0))
            as u32,
        y2: (u64::from(viewport.game.y2) * u64::from(output.1) / u64::from(viewport.output.1))
            as u32,
    };
    let center_x = (u64::from(viewport.game.x1 + viewport.game.x2) * u64::from(output.0)
        / (2 * u64::from(viewport.output.0))) as u32;
    let center_y = (u64::from(viewport.game.y1 + viewport.game.y2) * u64::from(output.1)
        / (2 * u64::from(viewport.output.1))) as u32;
    let map_x = |x: u32| {
        let content_x = u64::from(viewport.game.x1)
            + u64::from(x) * u64::from(viewport.game.width()) / u64::from(source.0);
        let mapped = (content_x * u64::from(output.0) / u64::from(viewport.output.0)) as u32;
        zoom_overlay_coordinate(mapped, center_x, viewport.content_zoom_percent, output.0)
    };
    let map_y = |y: u32| {
        let content_y = u64::from(viewport.game.y1)
            + u64::from(y) * u64::from(viewport.game.height()) / u64::from(source.1);
        let mapped = (content_y * u64::from(output.1) / u64::from(viewport.output.1)) as u32;
        zoom_overlay_coordinate(mapped, center_y, viewport.content_zoom_percent, output.1)
    };
    TextRect {
        x1: map_x(rect.x1).clamp(opening.x1.saturating_add(12), opening.x2.saturating_sub(12)),
        y1: map_y(rect.y1).clamp(opening.y1.saturating_add(8), opening.y2.saturating_sub(8)),
        x2: map_x(rect.x2).clamp(opening.x1.saturating_add(12), opening.x2.saturating_sub(12)),
        y2: map_y(rect.y2).clamp(opening.y1.saturating_add(8), opening.y2.saturating_sub(8)),
    }
}

fn map_overlay_line_height(
    line_height: u32,
    source: (u32, u32),
    viewport: OverlayViewport,
    output: (u32, u32),
) -> u32 {
    let numerator = u64::from(line_height)
        * u64::from(viewport.game.height())
        * u64::from(output.1)
        * u64::from(viewport.content_zoom_percent);
    let denominator = u64::from(source.1) * u64::from(viewport.output.1) * 100;
    ((numerator + denominator / 2) / denominator).max(1) as u32
}

fn zoom_overlay_coordinate(value: u32, center: u32, percent: u32, limit: u32) -> u32 {
    let delta = i64::from(value) - i64::from(center);
    (i64::from(center) + delta * i64::from(percent) / 100).clamp(0, i64::from(limit)) as u32
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
    for region in merge_colliding_regions(regions, width, height) {
        draw_region(&mut pixels, width, height, &region);
    }
    encode_rgba_png(width, height, &pixels)
}

fn region_panel(region: &TranslatedRegion, width: u32, height: u32) -> TextRect {
    let line_height = region.source_line_height.max(1).min(region.rect.height());
    let vertical_padding = (line_height / 5).clamp(2, 8);
    let horizontal_padding = (line_height / 3).clamp(3, 12);
    TextRect {
        x1: region.rect.x1.saturating_sub(horizontal_padding),
        y1: region.rect.y1.saturating_sub(vertical_padding),
        x2: region.rect.x2.saturating_add(horizontal_padding).min(width),
        y2: region.rect.y2.saturating_add(vertical_padding).min(height),
    }
}

fn regions_collide(a: &TranslatedRegion, b: &TranslatedRegion, width: u32, height: u32) -> bool {
    let a = region_panel(a, width, height);
    let b = region_panel(b, width, height);
    if a.width() == 0 || a.height() == 0 || b.width() == 0 || b.height() == 0 {
        return false;
    }
    let overlap_x = axis_overlap(a.x1, a.x2, b.x1, b.x2);
    let overlap_y = axis_overlap(a.y1, a.y2, b.y1, b.y2);
    overlap_x * 4 >= a.width().min(b.width()) && overlap_y * 4 >= a.height().min(b.height())
}

fn merge_caption_text(first: &str, second: &str) -> String {
    let left = first.split_whitespace().collect::<Vec<_>>();
    let right = second.split_whitespace().collect::<Vec<_>>();
    for shared in (2..=left.len().min(right.len())).rev() {
        let matches = left[left.len() - shared..]
            .iter()
            .zip(&right[..shared])
            .all(|(a, b)| {
                a.trim_matches(|ch: char| !ch.is_alphanumeric())
                    .eq_ignore_ascii_case(b.trim_matches(|ch: char| !ch.is_alphanumeric()))
            });
        if matches {
            return format!("{} {}", first.trim(), right[shared..].join(" "))
                .trim()
                .to_owned();
        }
    }
    format!("{} {}", first.trim(), second.trim())
}

fn merge_colliding_regions(
    regions: &[TranslatedRegion],
    width: u32,
    height: u32,
) -> Vec<TranslatedRegion> {
    let mut ordered = regions.to_vec();
    ordered.sort_by_key(|region| (region.rect.y1, region.rect.x1));
    let mut merged: Vec<TranslatedRegion> = Vec::new();
    for mut region in ordered {
        while let Some(index) = merged
            .iter()
            .position(|other| regions_collide(other, &region, width, height))
        {
            let other = merged.remove(index);
            let (earlier, later) =
                if (other.rect.y1, other.rect.x1) <= (region.rect.y1, region.rect.x1) {
                    (&other, &region)
                } else {
                    (&region, &other)
                };
            region = TranslatedRegion {
                rect: other.rect.union(region.rect),
                source_line_height: other.source_line_height.min(region.source_line_height),
                source: format!("{} {}", earlier.source, later.source),
                english: merge_caption_text(&earlier.english, &later.english),
                background: if other
                    .background
                    .iter()
                    .map(|value| u32::from(*value))
                    .sum::<u32>()
                    <= region
                        .background
                        .iter()
                        .map(|value| u32::from(*value))
                        .sum::<u32>()
                {
                    other.background
                } else {
                    region.background
                },
            };
        }
        merged.push(region);
    }
    merged
}

fn draw_region(pixels: &mut [u8], width: u32, height: u32, region: &TranslatedRegion) {
    if region.english.is_empty()
        || region.rect.x1 >= region.rect.x2
        || region.rect.y1 >= region.rect.y2
        || region.rect.x2 > width
        || region.rect.y2 > height
        || u64::from(region.rect.width()) * u64::from(region.rect.height()) * 100
            > u64::from(width) * u64::from(height) * 40
    {
        return;
    }
    // Pad by the source glyph height, not the dimensions of a merged dialogue
    // block. This keeps a multi-line translation attached to the text instead
    // of growing into a large rectangle over the scene.
    let source_line_height = region.source_line_height.max(1).min(region.rect.height());
    let vertical_padding = (source_line_height / 5).clamp(2, 8);
    let horizontal_padding = (source_line_height / 3).clamp(3, 12);
    let panel = region_panel(region, width, height);
    let usable_width = panel.width().saturating_sub(2 * horizontal_padding);
    let usable_height = panel.height().saturating_sub(vertical_padding);
    let font = subtitle_font();
    let Some((font_px, line_height, lines)) = choose_caption_layout(
        &region.english,
        font,
        source_line_height,
        height,
        usable_width,
        usable_height,
    ) else {
        return;
    };
    let total_height = line_height * lines.len() as u32;
    let text_top = panel.y1 + panel.height().saturating_sub(total_height) / 2;
    let text_center_x = panel.x1 + panel.width() / 2;
    for y in panel.y1..panel.y2 {
        for x in panel.x1..panel.x2 {
            put_pixel(
                pixels,
                width,
                x,
                y,
                [
                    region.background[0],
                    region.background[1],
                    region.background[2],
                    REGION_BACKGROUND_ALPHA,
                ],
            );
        }
    }
    let mut mask = vec![0u8; panel.width() as usize * panel.height() as usize];
    if let Some(font) = font {
        let ascent = font
            .horizontal_line_metrics(font_px)
            .map_or(font_px, |metrics| metrics.ascent);
        for (row, line) in lines.iter().enumerate() {
            let line_width: f32 = line
                .chars()
                .map(|ch| font.metrics(ch, font_px).advance_width)
                .sum();
            let mut cursor_x = (text_center_x as f32 - line_width / 2.0).clamp(
                (panel.x1 + horizontal_padding) as f32,
                (panel.x2.saturating_sub(horizontal_padding)) as f32,
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
                        if x >= panel.x1 as i32
                            && y >= panel.y1 as i32
                            && x < panel.x2 as i32
                            && y < panel.y2 as i32
                        {
                            let index = (y as u32 - panel.y1) as usize * panel.width() as usize
                                + (x as u32 - panel.x1) as usize;
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
            let start_x = text_center_x.saturating_sub(line.chars().count() as u32 * 4);
            for (column, ch) in line.chars().enumerate() {
                if let Some(glyph) = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?')) {
                    for (gy, bits) in glyph.iter().enumerate() {
                        for gx in 0..8u32 {
                            let x = start_x + column as u32 * 8 + gx;
                            let y = text_top + row as u32 * line_height + gy as u32;
                            if bits & (1 << gx) != 0
                                && x >= panel.x1
                                && y >= panel.y1
                                && x < panel.x2
                                && y < panel.y2
                            {
                                mask[((y - panel.y1) * panel.width() + x - panel.x1) as usize] =
                                    255;
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
            let x = panel.x1 + index as u32 % panel.width();
            let y = panel.y1 + index as u32 / panel.width();
            let pixel_index = ((y * width + x) * 4) as usize;
            let pixel = &mut pixels[pixel_index..pixel_index + 4];
            let ink = u32::from(coverage);
            let background = u32::from(REGION_BACKGROUND_ALPHA);
            let output_alpha = ink * 255 + background * (255 - ink);
            for channel in &mut pixel[..3] {
                let output_color =
                    foreground * ink * 255 + u32::from(*channel) * background * (255 - ink);
                *channel = ((output_color + output_alpha / 2) / output_alpha) as u8;
            }
            pixel[3] = ((output_alpha + 127) / 255) as u8;
        }
    }
}

fn choose_caption_layout(
    text: &str,
    font: Option<&Font>,
    source_line_height: u32,
    frame_height: u32,
    usable_width: u32,
    usable_height: u32,
) -> Option<(f32, u32, Vec<String>)> {
    let preferred_font_px = preferred_font_size(source_line_height, frame_height);
    (8..=(preferred_font_px * 2.0).floor() as u32)
        .rev()
        .map(|half_px| half_px as f32 / 2.0)
        .find_map(|font_px| {
            let line_height = (font_px * 1.12).ceil() as u32;
            let max_lines = (usable_height / line_height).min(6) as usize;
            wrap_caption_pixels(text, font, font_px, usable_width, max_lines)
                .map(|lines| (font_px, line_height, lines))
        })
}

fn preferred_font_size(source_line_height: u32, frame_height: u32) -> f32 {
    // Latin cap height is typically about three quarters of the font's em
    // size, so a 1.3x em tracks the detected Japanese glyph height.
    (source_line_height as f32 * 1.3).clamp(8.0, (frame_height as f32 / 6.0).clamp(12.0, 72.0))
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

fn caption_width(text: &str, font: Option<&Font>, font_px: f32) -> f32 {
    if let Some(font) = font {
        text.chars()
            .map(|ch| font.metrics(ch, font_px).advance_width)
            .sum()
    } else {
        text.chars().count() as f32 * 8.0
    }
}

fn wrap_caption_pixels(
    text: &str,
    font: Option<&Font>,
    font_px: f32,
    max_width: u32,
    max_lines: usize,
) -> Option<Vec<String>> {
    if max_width == 0 || max_lines == 0 {
        return None;
    }
    let fits = |line: &str| caption_width(line, font, font_px) <= max_width as f32;
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let with_word = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if fits(&with_word) {
            current = with_word;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            if lines.len() >= max_lines {
                return None;
            }
        }
        if fits(word) {
            current.push_str(word);
            continue;
        }
        // Some models return a long token or omit spaces. Split that token
        // at glyph boundaries, retaining every character rather than clipping.
        for ch in word.chars() {
            let mut candidate = current.clone();
            candidate.push(ch);
            if fits(&candidate) {
                current = candidate;
                continue;
            }
            if current.is_empty() {
                return None;
            }
            lines.push(std::mem::take(&mut current));
            if lines.len() >= max_lines {
                return None;
            }
            current.push(ch);
            if !fits(&current) {
                return None;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    (!lines.is_empty() && lines.len() <= max_lines).then_some(lines)
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
        "You are a professional {source_name} ({source_code}) to English (en) translator. Translate the following game dialogue or menu text accurately into natural English. Preserve names and the order of lines. Return only the exact English text that should replace the source. Do not add labels such as Text or Translation, explanations, extra punctuation, or commentary.\n\n{source_text}"
    );
    let raw = ollama_chat(&settings.model, &prompt, base_url)?;
    Ok(english_only_translation(&raw))
}

fn translate_texts_at(
    settings: &TranslationSettings,
    sources: &[String],
    base_url: &str,
) -> Result<Vec<String>> {
    if sources.len() <= 1 {
        return sources
            .iter()
            .map(|source| translate_text_at(settings, source, base_url))
            .collect();
    }
    let source_language = if settings.source_language == "auto" {
        "Detect each item's language"
    } else {
        settings.source_language.as_str()
    };
    let prompt = format!(
        "Translate each game dialogue or menu item in this JSON array from {source_language} to natural English. Preserve names, array order, and array length. Return ONLY a valid JSON array of English strings, with no markdown or explanation: {}",
        serde_json::to_string(sources)?
    );
    let raw = ollama_chat(&settings.model, &prompt, base_url)?;
    if let Some(translated) = parse_translation_array(&raw, sources.len()) {
        return Ok(translated);
    }
    eprintln!("LUNCHBOX_TRANSLATION_BATCH_FALLBACK: model did not return the requested array");
    Ok(sources
        .iter()
        .map(
            |source| match translate_text_at(settings, source, base_url) {
                Ok(english) => english,
                Err(error) => {
                    eprintln!("LUNCHBOX_TRANSLATION_MODEL_FAILED: {error:#}");
                    String::new()
                }
            },
        )
        .collect())
}

fn parse_translation_array(raw: &str, count: usize) -> Option<Vec<String>> {
    let body = raw.trim();
    let body = body
        .strip_prefix("```json")
        .or_else(|| body.strip_prefix("```"))
        .unwrap_or(body)
        .trim();
    let body = body.strip_suffix("```").unwrap_or(body).trim();
    let parsed = serde_json::from_str::<Vec<String>>(body).ok().or_else(|| {
        // Some local models wrap a correct array in a sentence despite
        // the prompt. Recover it before paying for serial fallback calls.
        let start = body.find('[')?;
        let end = body.rfind(']')?;
        serde_json::from_str::<Vec<String>>(&body[start..=end]).ok()
    })?;
    if parsed.len() != count {
        return None;
    }
    let translated = parsed
        .iter()
        .map(|text| english_only_translation(text))
        .collect::<Vec<_>>();
    translated
        .iter()
        .all(|text| !text.is_empty())
        .then_some(translated)
}

fn english_only_translation(raw: &str) -> String {
    let lines: Vec<_> = raw
        .lines()
        .map(|line| line.trim().trim_matches('"').trim())
        .filter(|line| line.chars().any(|ch| ch.is_ascii_alphabetic()))
        .collect();
    lines.join(" ").chars().take(2000).collect()
}

fn ollama_chat(model: &str, prompt: &str, base_url: &str) -> Result<String> {
    if base_url == OLLAMA_URL {
        match active_model_gpu_percent(model)? {
            Some(percent) => ensure!(
                percent >= 95,
                "{model} is no longer fully on the GPU ({percent}% in VRAM)"
            ),
            // Ollama may unload an idle model after its keep-alive expires.
            // Reload it with an empty prompt, then verify placement *before*
            // submitting text, rather than permanently rejecting requests.
            None => verify_model_gpu(model)?,
        }
    }
    let request = json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "keep_alive": "10m",
        // A game dialogue request is short. Avoid Ollama's 32k default on a
        // 24 GB card so RetroArch and GPU OCR retain headroom beside the LLM.
        "options": {"temperature": 0, "num_ctx": 2048, "num_predict": 256},
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
    // GPU memory pressure can make a previously GPU-loaded model reload after
    // the game has started. Detect and reject the result if that happened.
    if base_url == OLLAMA_URL {
        let percent = active_model_gpu_percent(model)?
            .context("translation model unloaded during inference")?;
        ensure!(
            percent >= 95,
            "{model} left the GPU ({percent}% in VRAM); translation stopped"
        );
    }
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
    fn gpu_placement_requires_the_selected_model() {
        let status = json!({"models": [
            {"name": "translategemma:4b", "size": 100, "size_vram": 99},
            {"name": "translategemma:12b", "size": 100, "size_vram": 2}
        ]});
        assert_eq!(
            loaded_model_gpu_fraction(&status, "translategemma:4b").unwrap(),
            Some(99)
        );
        assert_eq!(
            loaded_model_gpu_fraction(&status, "translategemma:12b").unwrap(),
            Some(2)
        );
        assert_eq!(
            loaded_model_gpu_fraction(&status, "translategemma:27b").unwrap(),
            None
        );
    }

    #[test]
    fn translation_memory_reuses_dialogue_across_changed_frames() {
        let mut memory = TranslationMemory::default();
        memory.insert("ブリッツの攻撃を止める!", "Stop Blitzer's attack!");
        assert_eq!(
            memory.get("  ブリッツの攻撃を止める!\n"),
            Some("Stop Blitzer's attack!")
        );
        memory.insert("失敗", "");
        assert_eq!(memory.get("失敗"), None);
    }

    #[test]
    fn translation_memory_is_bounded_to_the_current_session() {
        let mut memory = TranslationMemory::default();
        for index in 0..=MAX_TRANSLATION_MEMORY {
            memory.insert(&format!("source {index}"), &format!("english {index}"));
        }
        assert_eq!(memory.entries.len(), MAX_TRANSLATION_MEMORY);
        assert_eq!(memory.get("source 0"), None);
        assert_eq!(
            memory.get(&format!("source {MAX_TRANSLATION_MEMORY}")),
            Some(format!("english {MAX_TRANSLATION_MEMORY}").as_str())
        );
    }

    #[test]
    fn new_pixels_outside_old_text_trigger_ocr_without_blank_caption() {
        let mut screenshot = RgbImage::from_pixel(64, 64, image::Rgb([10, 20, 30]));
        let digest: [u8; 32] = Sha256::digest(screenshot.as_raw()).into();
        let rect = TextRect {
            x1: 10,
            y1: 20,
            x2: 40,
            y2: 38,
        };
        let cached = CachedTranslation {
            digest,
            regions: vec![TranslatedRegion {
                rect,
                source_line_height: 18,
                source: "開く".to_owned(),
                english: "Open".to_owned(),
                background: [10, 20, 30],
            }],
            region_fingerprints: vec![fingerprint_region(&screenshot, rect)],
            dimensions: (64, 64),
            overlay: String::new(),
        };
        assert!(cached_overlay_matches_frame(&cached, digest));
        screenshot.put_pixel(50, 50, image::Rgb([255, 255, 255]));
        let changed: [u8; 32] = Sha256::digest(screenshot.as_raw()).into();
        assert!(!cached_overlay_matches_frame(&cached, changed));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(screenshot.clone())
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        assert!(cached_overlay_still_visible(
            &cached,
            encoded.get_ref(),
            (64, 64)
        ));
        screenshot.put_pixel(20, 25, image::Rgb([255, 255, 255]));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(screenshot)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        assert!(!cached_overlay_still_visible(
            &cached,
            encoded.get_ref(),
            (64, 64)
        ));
    }

    #[test]
    fn empty_ocr_result_does_not_freeze_an_unchanged_frame() {
        let screenshot = render_regions_png(64, 64, &[]).unwrap();
        let digest: [u8; 32] = Sha256::digest(&screenshot).into();
        let cached = CachedTranslation {
            digest,
            regions: Vec::new(),
            region_fingerprints: Vec::new(),
            dimensions: (64, 64),
            overlay: BASE64.encode(&screenshot),
        };
        assert!(!cached_overlay_matches_frame(&cached, digest));
    }

    #[test]
    fn one_empty_ocr_read_preserves_the_last_visible_caption() {
        let (jobs, _pending) = mpsc::sync_channel(1);
        let (_completed, results) = mpsc::channel();
        let old = CachedTranslation {
            digest: [1; 32],
            regions: vec![TranslatedRegion {
                rect: TextRect {
                    x1: 1,
                    y1: 1,
                    x2: 10,
                    y2: 10,
                },
                source_line_height: 9,
                source: "開く".to_owned(),
                english: "Open".to_owned(),
                background: [0, 0, 0],
            }],
            region_fingerprints: vec![[2; 32]],
            dimensions: (64, 64),
            overlay: "old-caption".to_owned(),
        };
        let mut state = TranslationBridge {
            jobs,
            results,
            cached: Some(old),
            in_flight: Some([3; 32]),
            retry_after: None,
            last_started_at: None,
            last_response_at: None,
            blank_overlay: None,
        };
        accept_translation_result(
            &mut state,
            [3; 32],
            Ok(CachedTranslation {
                digest: [3; 32],
                regions: vec![],
                region_fingerprints: vec![],
                dimensions: (64, 64),
                overlay: "empty-caption".to_owned(),
            }),
        );
        assert_eq!(state.cached.unwrap().overlay, "old-caption");
        assert_eq!(state.in_flight, None);
    }

    #[test]
    fn flatpak_filesystem_flags_are_not_parsed_as_retroarch_options() {
        let executable = EmulatorExecutable::Flatpak {
            command: PathBuf::from("/usr/bin/flatpak"),
            app_id: "org.libretro.RetroArch".to_owned(),
        };
        let arguments = [
            "run",
            "--filesystem=/tmp/lunchbox-session",
            "org.libretro.RetroArch",
            "--set-shader=/tmp/retrotube-tv-system-bezel.slangp",
            "--appendconfig",
            "/tmp/lunchbox-display.cfg",
            "-L",
            "/tmp/mesen-s_libretro.so",
            "/tmp/game.sfc",
        ]
        .map(OsString::from);
        let app_arguments = retroarch_app_arguments(&arguments, &executable).unwrap();
        assert_eq!(
            crate::controller_launch_modes::append_config_index(app_arguments).unwrap(),
            Some(1)
        );
        assert!(retrotube_shader_active(app_arguments));
    }

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
            source_line_height: 22,
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
        assert_eq!(pixels[((30 * 320 + 22) * 4) + 3], REGION_BACKGROUND_ALPHA);
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
                source_line_height: 16,
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
                source_line_height: 16,
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
        assert_eq!(pixels[((30 * 320 + 17) * 4) + 3], REGION_BACKGROUND_ALPHA);
        assert_eq!(pixels[((190 * 320 + 17) * 4) + 3], REGION_BACKGROUND_ALPHA);
        assert_eq!(pixels[((110 * 320 + 40) * 4) + 3], 0);
    }

    #[test]
    fn overlapping_dialogue_captions_become_one_panel_without_repeating_words() {
        let first = TranslatedRegion {
            rect: TextRect {
                x1: 20,
                y1: 20,
                x2: 300,
                y2: 70,
            },
            source_line_height: 20,
            source: "魔法使い".to_owned(),
            english: "Do not worry, we can easily".to_owned(),
            background: [3, 55, 8],
        };
        let second = TranslatedRegion {
            rect: TextRect {
                x1: 50,
                y1: 55,
                x2: 260,
                y2: 100,
            },
            source_line_height: 20,
            source: "勝てる".to_owned(),
            english: "We can easily win this battle".to_owned(),
            background: [3, 70, 8],
        };
        let merged = merge_colliding_regions(&[first, second], 320, 240);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].english,
            "Do not worry, we can easily win this battle"
        );
        assert_eq!(merged[0].rect.y2, 100);
        assert_eq!(merged[0].background, [3, 55, 8]);
    }

    #[test]
    fn numeric_english_hud_labels_are_not_translated() {
        assert!(is_numeric_hud_label("MP:6"));
        assert!(is_numeric_hud_label("HP 100/100"));
        assert!(!is_numeric_hud_label("魔法使い6"));
        assert!(!is_numeric_hud_label("Kukuku, you made a mess"));
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
            source_line_height: 17,
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
        assert_eq!(pixels[((170 * 320 + 21) * 4) + 3], REGION_BACKGROUND_ALPHA);
        assert_eq!(pixels[((225 * 320 + 100) * 4) + 3], 0);
    }

    #[test]
    fn text_background_prefers_the_dialogue_panel_over_outer_scenery_and_glyphs() {
        let mut image = RgbImage::from_pixel(64, 64, image::Rgb([21, 35, 49]));
        let rect = TextRect {
            x1: 10,
            y1: 20,
            x2: 50,
            y2: 40,
        };
        for y in rect.y1..rect.y1 + 3 {
            for x in rect.x1..rect.x2 {
                image.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
        for y in rect.y1 - 3..rect.y1 {
            for x in rect.x1 - 3..rect.x2 + 3 {
                image.put_pixel(x, y, image::Rgb([180, 70, 25]));
            }
        }
        assert_eq!(sample_text_background(&image, rect), [21, 35, 49]);
    }

    #[test]
    fn english_font_tracks_detected_source_glyph_height() {
        let font = subtitle_font();
        let small = choose_caption_layout("Open", font, 12, 1000, 240, 60).unwrap();
        let large = choose_caption_layout("Open", font, 24, 1000, 240, 60).unwrap();
        assert!(large.0 >= small.0 * 1.5, "small={small:?} large={large:?}");
        assert_eq!(large.2, vec!["Open"]);
    }

    #[test]
    fn caption_wrap_uses_rendered_glyph_width_without_losing_words() {
        let font = subtitle_font();
        let text = "A very wide translation in a narrow game window";
        let lines = wrap_caption_pixels(text, font, 20.0, 110, 8).unwrap();
        assert_eq!(lines.join(" "), text);
        assert!(
            lines
                .iter()
                .all(|line| caption_width(line, font, 20.0) <= 110.0)
        );
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
        let groups = group_text_regions(boxes, 320, 240);
        assert_eq!(groups[0].line_height, 14);
        assert_eq!(groups[1].line_height, 15);
        assert_eq!(
            groups
                .into_iter()
                .map(|group| group.rect)
                .collect::<Vec<_>>(),
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
    fn nearly_full_width_dialogue_line_is_one_region() {
        let left = TextRect {
            x1: 5,
            y1: 70,
            x2: 236,
            y2: 96,
        };
        let right = TextRect {
            x1: 262,
            y1: 71,
            x2: 507,
            y2: 97,
        };
        let groups = group_text_regions(vec![right, left], 512, 478);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].rect, left.union(right));
        assert!(text_region_allowed(groups[0].rect, 512, 478));

        let distant = TextRect {
            x1: 300,
            y1: 71,
            x2: 507,
            y2: 97,
        };
        assert_eq!(group_text_regions(vec![left, distant], 512, 478).len(), 2);
    }

    #[test]
    fn text_lines_cannot_chain_into_one_screen_sized_region() {
        let boxes = (0..12)
            .map(|line| TextRect {
                x1: 50,
                y1: 20 + line * 18,
                x2: 265,
                y2: 34 + line * 18,
            })
            .collect();
        let groups = group_text_boxes(boxes, 320, 240);
        assert!(!groups.is_empty());
        assert!(groups.iter().all(|rect| rect.height() <= 72));
        assert!(
            groups
                .iter()
                .all(|rect| text_region_allowed(*rect, 320, 240))
        );
        assert!(!text_region_allowed(
            TextRect {
                x1: 0,
                y1: 0,
                x2: 320,
                y2: 240,
            },
            320,
            240
        ));
    }

    #[test]
    fn dense_character_picker_leaves_only_header_labels() {
        let mut boxes = vec![
            TextRect {
                x1: 16,
                y1: 20,
                x2: 88,
                y2: 50,
            },
            TextRect {
                x1: 300,
                y1: 37,
                x2: 386,
                y2: 66,
            },
            TextRect {
                x1: 300,
                y1: 80,
                x2: 466,
                y2: 105,
            },
            TextRect {
                x1: 41,
                y1: 160,
                x2: 491,
                y2: 185,
            },
        ];
        for row in 0..6 {
            for column in 0..4 {
                let x = 40 + column * 115;
                let y = 200 + row * 40;
                boxes.push(TextRect {
                    x1: x,
                    y1: y,
                    x2: x + 90,
                    y2: y + 26,
                });
            }
        }
        assert_eq!(
            group_text_boxes(boxes, 512, 478),
            vec![
                TextRect {
                    x1: 16,
                    y1: 20,
                    x2: 88,
                    y2: 50
                },
                TextRect {
                    x1: 300,
                    y1: 37,
                    x2: 466,
                    y2: 105
                },
            ]
        );
    }

    #[test]
    fn overlay_maps_core_text_into_centered_ultrawide_game_opening() {
        let viewport = OverlayViewport {
            output: (6656, 2808),
            game: TextRect {
                x1: 1786,
                y1: 249,
                x2: 4870,
                y2: 2558,
            },
            content_zoom_percent: 100,
        };
        let output = overlay_image_dimensions(viewport);
        assert_eq!(output, (2048, 864));
        let full = map_overlay_rect(
            TextRect {
                x1: 0,
                y1: 0,
                x2: 512,
                y2: 478,
            },
            (512, 478),
            viewport,
            output,
        );
        assert!(full.x1 > 500 && full.x2 < 1550);
        assert!(full.y1 > 50 && full.y2 < 800);
        let label = map_overlay_rect(
            TextRect {
                x1: 300,
                y1: 80,
                x2: 466,
                y2: 105,
            },
            (512, 478),
            viewport,
            output,
        );
        assert!(label.x1 >= full.x1 && label.x2 <= full.x2);
        assert!(label.y1 >= full.y1 && label.y2 <= full.y2);
        assert!(label.width() < full.width() / 2);
    }

    #[test]
    fn retrotube_zoom_aligns_labels_to_the_rendered_sd3_menu() {
        let viewport = OverlayViewport {
            output: (6656, 2808),
            game: TextRect {
                x1: 1786,
                y1: 249,
                x2: 4870,
                y2: 2558,
            },
            content_zoom_percent: 120,
        };
        let output = overlay_image_dimensions(viewport);
        let duran = map_overlay_rect(
            TextRect {
                x1: 63,
                y1: 58,
                x2: 124,
                y2: 88,
            },
            (512, 478),
            viewport,
            output,
        );
        let fighter = map_overlay_rect(
            TextRect {
                x1: 293,
                y1: 77,
                x2: 362,
                y2: 100,
            },
            (512, 478),
            viewport,
            output,
        );
        assert!((575..=600).contains(&duran.x1));
        assert!((95..=115).contains(&duran.y1));
        assert!((1090..=1115).contains(&fighter.x1));
        assert!((130..=150).contains(&fighter.y1));
        let mapped_line_height = map_overlay_line_height(23, (512, 478), viewport, output);
        assert!(mapped_line_height.abs_diff(fighter.height()) <= 2);
        let full = map_overlay_rect(
            TextRect {
                x1: 0,
                y1: 0,
                x2: 512,
                y2: 478,
            },
            (512, 478),
            viewport,
            output,
        );
        let opening_left = viewport.game.x1 * output.0 / viewport.output.0;
        let opening_right = viewport.game.x2 * output.0 / viewport.output.0;
        assert!(full.x1 >= opening_left + 12);
        assert!(full.x2 <= opening_right - 12);
    }

    #[test]
    fn long_translation_cannot_expand_panel_or_truncate_into_a_caption() {
        let rect = TextRect {
            x1: 1300,
            y1: 1530,
            x2: 2050,
            y2: 1600,
        };
        let region = TranslatedRegion {
            rect,
            source_line_height: 28,
            source: "日本語".to_owned(),
            english: "The translation must never obscure the game. ".repeat(80),
            background: [10, 20, 30],
        };
        let mut pixels = vec![0; 5120 * 2160 * 4];
        draw_region(&mut pixels, 5120, 2160, &region);
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 0));

        let mut short = region;
        short.english = "Open the door.".to_owned();
        draw_region(&mut pixels, 5120, 2160, &short);
        let panel = TextRect {
            x1: 1291,
            y1: 1525,
            x2: 2059,
            y2: 1605,
        };
        for (index, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[3] != 0 {
                let x = index as u32 % 5120;
                let y = index as u32 / 5120;
                assert!(x >= panel.x1 && x < panel.x2);
                assert!(y >= panel.y1 && y < panel.y2);
            }
        }
        assert!(pixels[((1565 * 5120 + 1400) * 4 + 3) as usize] > 0);
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
    fn batched_translation_accepts_fenced_json_without_losing_box_order() {
        assert_eq!(
            parse_translation_array("```json\n[\"Normal\", \"Wide\"]\n```", 2),
            Some(vec!["Normal".to_owned(), "Wide".to_owned()])
        );
        assert_eq!(
            parse_translation_array("Translations: [\"Normal\", \"Wide\"]", 2),
            Some(vec!["Normal".to_owned(), "Wide".to_owned()])
        );
        assert_eq!(parse_translation_array("[\"Normal\"]", 2), None);
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
    fn retroarch_request_without_format_field_reuses_cached_image() {
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
                source_line_height: 16,
                source: "開く".to_owned(),
                english: "OPEN".to_owned(),
                background: [9, 14, 22],
            }],
        )
        .unwrap();
        let digest: [u8; 32] = Sha256::digest(&screenshot).into();
        let cached = Some(CachedTranslation {
            digest,
            regions: vec![],
            region_fingerprints: vec![],
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
        let (jobs, _pending) = mpsc::sync_channel(1);
        let (_completed, results) = mpsc::channel();
        let mut state = TranslationBridge {
            jobs,
            results,
            cached,
            in_flight: None,
            retry_after: None,
            last_started_at: None,
            last_response_at: None,
            blank_overlay: None,
        };
        handle_request(&mut stream, "secret", None, &mut state).unwrap();
        drop(stream);
        let response = client.join().unwrap();
        let body = response.split("\r\n\r\n").nth(1).unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["auto"], "auto");
        let image = BASE64.decode(body["image"].as_str().unwrap()).unwrap();
        assert_eq!(png_dimensions(&image).unwrap(), (64, 64));
    }

    #[test]
    fn slow_translation_returns_a_blank_overlay_without_a_status_card() {
        let screenshot = render_regions_png(512, 478, &[]).unwrap();
        let digest: [u8; 32] = Sha256::digest(&screenshot).into();
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
        let (jobs, pending) = mpsc::sync_channel(1);
        let (_completed, results) = mpsc::channel();
        let mut state = TranslationBridge {
            jobs,
            results,
            cached: None,
            in_flight: None,
            retry_after: None,
            last_started_at: None,
            last_response_at: None,
            blank_overlay: None,
        };
        let (mut stream, _) = listener.accept().unwrap();
        let start = Instant::now();
        handle_request(&mut stream, "secret", None, &mut state).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        drop(stream);
        let response = client.join().unwrap();
        let body: Value = serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["auto"], "auto");
        let overlay = BASE64.decode(body["image"].as_str().unwrap()).unwrap();
        assert!(
            image::load_from_memory(&overlay)
                .unwrap()
                .into_rgba8()
                .pixels()
                .all(|pixel| pixel.0[3] == 0)
        );
        assert_eq!(state.in_flight, Some(digest));
        assert_eq!(pending.try_recv().unwrap().digest, digest);
    }

    #[test]
    fn completed_translation_is_returned_on_the_same_capture() {
        let screenshot = render_regions_png(64, 64, &[]).unwrap();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            let body = serde_json::to_vec(&json!({"image": BASE64.encode(screenshot)})).unwrap();
            write!(
                stream,
                "POST /secret HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            stream.flush().unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            response
        });
        let (jobs, pending) = mpsc::sync_channel(1);
        let (completed, results) = mpsc::channel();
        let worker = thread::spawn(move || {
            let job: TranslationJob = pending.recv().unwrap();
            // Finish after the initial short wait, but before the bounded
            // response deadline, as a warm local model commonly does.
            thread::sleep(Duration::from_millis(1100));
            let rect = TextRect {
                x1: 1,
                y1: 1,
                x2: 10,
                y2: 10,
            };
            let screenshot = image::load_from_memory(&BASE64.decode(&job.image).unwrap())
                .unwrap()
                .into_rgb8();
            completed
                .send((
                    job.digest,
                    Ok(CachedTranslation {
                        digest: job.digest,
                        regions: vec![TranslatedRegion {
                            rect,
                            source_line_height: 9,
                            source: "開く".to_owned(),
                            english: "Open".to_owned(),
                            background: [0, 0, 0],
                        }],
                        region_fingerprints: vec![fingerprint_region(&screenshot, rect)],
                        dimensions: job.dimensions,
                        overlay: "ready-overlay".to_owned(),
                    }),
                ))
                .unwrap();
        });
        let mut state = TranslationBridge {
            jobs,
            results,
            cached: None,
            in_flight: None,
            retry_after: None,
            last_started_at: None,
            last_response_at: None,
            blank_overlay: None,
        };
        let (mut stream, _) = listener.accept().unwrap();
        handle_request(&mut stream, "secret", None, &mut state).unwrap();
        drop(stream);
        let response = client.join().unwrap();
        worker.join().unwrap();
        let body: Value = serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["image"], "ready-overlay");
        assert_eq!(body["auto"], "auto");
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
            assert_eq!(body["options"]["num_ctx"], 2048);
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
        let mut source = std::fs::read(source_path).unwrap();
        if let Ok(crop) = std::env::var("LUNCHBOX_TRANSLATION_SOURCE_CROP") {
            let values = crop
                .split(',')
                .map(|part| part.parse::<u32>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 4);
            let image = image::load_from_memory(&source).unwrap().into_rgb8();
            let cropped =
                image::imageops::crop_imm(&image, values[0], values[1], values[2], values[3])
                    .to_image();
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(cropped)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .unwrap();
            source = encoded.into_inner();
        }
        let (width, height) = png_dimensions(&source).unwrap();
        let image = BASE64.encode(source);
        download_ocr_models(&AtomicBool::new(false)).unwrap();
        let mut ocr = load_ocr().unwrap();
        let viewport =
            std::env::var_os("LUNCHBOX_TRANSLATION_ULTRAWIDE_PROBE").map(|_| OverlayViewport {
                output: (6656, 2808),
                game: TextRect {
                    x1: 1786,
                    y1: 249,
                    x2: 4870,
                    y2: 2558,
                },
                content_zoom_percent: 120,
            });
        let (regions, overlay) = render_translation(
            &TranslationSettings::default(),
            &image,
            width,
            height,
            &mut ocr,
            viewport,
            None,
            &mut TranslationMemory::default(),
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
