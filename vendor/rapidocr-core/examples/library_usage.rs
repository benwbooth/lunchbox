use std::{env, path::PathBuf, time::Instant};

use rapidocr_core::{
    config::{ExecutionProvider, InferenceOptions, PipelineConfig},
    model::{model_set_by_name, ModelCache, ModelDownloadMode, DEFAULT_MODEL_SET_NAME},
    RapidOcr,
};

fn main() -> anyhow::Result<()> {
    let image_path = env::args().nth(1).map(PathBuf::from).ok_or_else(|| {
        anyhow::anyhow!("usage: cargo run -p rapidocr-core --example library_usage -- <image>")
    })?;
    let model_dir = env::var_os("RAPIDOCR_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models"));
    let model_set_name =
        env::var("RAPIDOCR_MODEL_SET").unwrap_or_else(|_| DEFAULT_MODEL_SET_NAME.to_string());
    let model_set = model_set_by_name(&model_set_name)
        .ok_or_else(|| anyhow::anyhow!("unknown model set {model_set_name:?}"))?;
    let pipeline = match env::var("RAPIDOCR_PIPELINE").as_deref() {
        Ok("full") | Err(_) => PipelineConfig::full(),
        Ok("no-cls") => PipelineConfig::without_cls(),
        Ok("det-only") => PipelineConfig::detection_only(),
        Ok("rec-only") => PipelineConfig::recognition_only(),
        Ok(value) => {
            return Err(anyhow::anyhow!(
                "unsupported RAPIDOCR_PIPELINE={value:?}; use full, no-cls, det-only, or rec-only"
            ));
        }
    };

    let cache = ModelCache::new(model_dir);
    cache.ensure_model_set_for_pipeline(model_set, pipeline, ModelDownloadMode::Missing)?;

    let provider = match env::var("RAPIDOCR_PROVIDER").as_deref() {
        Ok("migraphx") => ExecutionProvider::Migraphx,
        Ok("coreml") => ExecutionProvider::CoreMl,
        Ok("directml") => ExecutionProvider::DirectMl,
        Ok("cpu") | Err(_) => ExecutionProvider::Cpu,
        Ok(value) => anyhow::bail!("unsupported RAPIDOCR_PROVIDER={value:?}"),
    };
    let cfg = cache
        .config_for(model_set)
        .with_pipeline(pipeline)
        .with_inference_options(InferenceOptions {
            execution_provider: provider,
            ..Default::default()
        });
    let loading = Instant::now();
    let mut ocr = RapidOcr::from_config(cfg)?;
    eprintln!(
        "RAPIDOCR_PROVIDER={provider:?} load_ms={}",
        loading.elapsed().as_millis()
    );
    let running = Instant::now();
    let output = ocr.run_path(image_path)?;
    eprintln!("RAPIDOCR_RUN_MS={}", running.elapsed().as_millis());

    for line in output.lines {
        println!("{:.5}\t{}", line.score, line.text);
    }

    Ok(())
}
