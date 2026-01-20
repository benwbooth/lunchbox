//! 3D Box Viewer component using Three.js
//!
//! Displays a rotatable 3D box with front and back cover textures.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "Box3DViewer"])]
    fn init(canvas_id: &str, front_url: &str, back_url: Option<&str>) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "Box3DViewer"])]
    fn destroy(canvas_id: &str);
}

/// 3D Box Viewer component
///
/// Renders an interactive 3D box that can be rotated with mouse controls.
#[component]
pub fn Box3DViewer(
    /// URL for the front cover image
    front_url: String,
    /// URL for the back cover image (None = use front as back)
    back_url: Option<String>,
    /// Unique ID for this viewer instance
    #[prop(default = "box3d-canvas".to_string())]
    canvas_id: String,
) -> impl IntoView {
    let canvas_id_clone = canvas_id.clone();
    let front_url_clone = front_url.clone();
    let back_url_clone = back_url.clone();

    // Initialize the viewer when component mounts
    Effect::new(move || {
        let id = canvas_id_clone.clone();
        let front = front_url_clone.clone();
        let back = back_url_clone.clone();

        // Small delay to ensure canvas is in DOM
        let _ = gloo_timers::callback::Timeout::new(100, move || {
            init(&id, &front, back.as_deref());
        });
    });

    // Clean up when component unmounts
    let cleanup_id = canvas_id.clone();
    on_cleanup(move || {
        destroy(&cleanup_id);
    });

    view! {
        <div class="box-3d-viewer">
            <canvas
                id=canvas_id
                class="box-3d-canvas"
            />
            <div class="box-3d-controls">
                <span class="box-3d-hint">"Drag to rotate"</span>
            </div>
        </div>
    }
}
