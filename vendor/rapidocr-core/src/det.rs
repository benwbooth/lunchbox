use std::time::Instant;

use anyhow::{Context, Result};

use ndarray::Array4;

use crate::{
    cancellation::OcrCancellationToken,
    config::{DetConfig, ExecutionProvider, InferenceOptions, LimitType},
    db_postprocess::{DbPostProcess, DbPostProcessConfig},
    image_ops::{resize_to_multiple_for_det, rgb_to_nchw},
    inference::OnnxSession,
    types::{OcrTimings, Quad},
};

pub(crate) struct TextDetector {
    cfg: DetConfig,
    session: OnnxSession,
    postprocess: DbPostProcess,
    fixed_migraphx_shape: bool,
}

impl TextDetector {
    pub(crate) fn new(cfg: DetConfig, inference: InferenceOptions) -> Result<Self> {
        cfg.validate().context("invalid detection config")?;
        let session = OnnxSession::new(&cfg.model_path, inference, true).with_context(|| {
            format!(
                "failed to load detection model {}",
                cfg.model_path.display()
            )
        })?;
        let postprocess = DbPostProcess::new(DbPostProcessConfig::from(&cfg));
        Ok(Self {
            cfg,
            session,
            postprocess,
            fixed_migraphx_shape: inference.execution_provider == ExecutionProvider::Migraphx,
        })
    }

    pub(crate) fn detect_timed(
        &mut self,
        img: &image::RgbImage,
        cancellation: &OcrCancellationToken,
    ) -> Result<DetectResult> {
        let mut timings = OcrTimings::default();
        cancellation.checkpoint()?;

        let start = Instant::now();
        // The detector expects dimensions divisible by 32 and normalized NCHW
        // input. Box coordinates are mapped back by postprocess using the
        // original image size passed below.
        let input_img = if self.fixed_migraphx_shape {
            // MIGraphX caches a compiled graph by model, not input dimensions.
            // A stable tensor shape avoids reusing a graph for the wrong frame.
            image::imageops::resize(img, 1024, 1024, image::imageops::FilterType::Triangle)
        } else {
            resize_to_multiple_for_det(
                img,
                self.cfg.limit_side_len,
                matches!(self.cfg.limit_type, LimitType::Min),
                &self.cfg.input_limits,
            )?
        };
        let tensor = rgb_to_nchw(&input_img, self.cfg.mean, self.cfg.std, cancellation)?;
        timings.det_preprocess_ms = elapsed_ms(start);

        let start = Instant::now();
        let pred = self.session.run_f32(&tensor, cancellation)?;
        timings.det_inference_ms = elapsed_ms(start);

        let start = Instant::now();
        let boxes = self
            .postprocess
            .process_cancellable(pred, img.width(), img.height(), cancellation)?
            .into_iter()
            .map(|candidate| candidate.bbox)
            .collect();
        timings.det_postprocess_ms = elapsed_ms(start);

        Ok(DetectResult { boxes, timings })
    }

    pub(crate) fn warm_up(&mut self, cancellation: &OcrCancellationToken) -> Result<()> {
        if self.fixed_migraphx_shape {
            let input = Array4::<f32>::zeros((1, 3, 1024, 1024));
            self.session.run_f32(&input, cancellation)?;
        }
        Ok(())
    }
}

pub(crate) struct DetectResult {
    pub(crate) boxes: Vec<Quad>,
    pub(crate) timings: OcrTimings,
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}
