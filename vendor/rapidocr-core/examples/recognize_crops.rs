//! Compare recognition on explicit crops without rerunning the detector.
//! Usage: recognize_crops IMAGE X,Y,W,H [X,Y,W,H ...]
use rapidocr_core::{
    config::{ExecutionProvider, InferenceOptions, PipelineConfig},
    model::{model_set_by_name, ModelCache},
    RapidOcr,
};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    anyhow::ensure!(
        args.len() >= 2,
        "expected image path and x,y,width,height crops"
    );
    let image = image::open(&args[0])?.into_rgb8();
    let cache = ModelCache::new(std::env::var("RAPIDOCR_MODEL_DIR")?);
    let model = model_set_by_name("ppocrv6-small").unwrap();
    let provider = match std::env::var("RAPIDOCR_PROVIDER").as_deref() {
        Ok("migraphx") => ExecutionProvider::Migraphx,
        Ok("coreml") => ExecutionProvider::CoreMl,
        Ok("directml") => ExecutionProvider::DirectMl,
        _ => ExecutionProvider::Cpu,
    };
    let mut ocr = RapidOcr::from_config(
        cache
            .config_for(model)
            .with_pipeline(PipelineConfig::recognition_only())
            .with_inference_options(InferenceOptions {
                execution_provider: provider,
                ..Default::default()
            }),
    )?;
    for spec in &args[1..] {
        let xywh = spec
            .split(',')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        anyhow::ensure!(xywh.len() == 4, "crop needs four numbers");
        let crop = image::imageops::crop_imm(&image, xywh[0], xywh[1], xywh[2], xywh[3]).to_image();
        let output = ocr.run_image_timed(&crop)?;
        println!(
            "{spec}: {:?} ({:.0}ms)",
            output.output.lines, output.timings.total_ms
        );
    }
    Ok(())
}
