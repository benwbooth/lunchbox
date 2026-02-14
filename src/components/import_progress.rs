//! Import progress component for Graboid game imports
//!
//! Shows real-time progress of a Graboid import job using SSE events.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use crate::tauri;

/// A single navigation step from the Graboid agent
#[derive(Clone, Debug)]
struct ImportStep {
    step_number: u32,
    action: String,
    observation: String,
    url: String,
    screenshot: Option<String>,
    notes: Vec<String>,
    is_error: bool,
}

/// A log entry from the Graboid agent
#[derive(Clone, Debug)]
struct LogEntry {
    level: String,
    source: String,
    message: String,
    timestamp: String,
}

/// Import progress component that manages the lifecycle of a single import job
#[component]
pub fn ImportProgress(
    /// The Graboid job ID
    #[prop(into)]
    job_id: String,
    /// Signal set when import completes (set to file_path)
    #[prop(optional)]
    on_complete: Option<WriteSignal<Option<String>>>,
    /// Signal set when import fails (set to error message)
    #[prop(optional)]
    on_failed: Option<WriteSignal<Option<String>>>,
) -> impl IntoView {
    let progress = RwSignal::new(0.0_f64);
    let status_message = RwSignal::new(String::from("Starting import..."));
    let status = RwSignal::new(String::from("pending"));
    let expanded = RwSignal::new(false);
    let steps: RwSignal<Vec<ImportStep>> = RwSignal::new(Vec::new());
    let logs: RwSignal<Vec<LogEntry>> = RwSignal::new(Vec::new());
    let logs_expanded = RwSignal::new(false);
    let selected_screenshot: RwSignal<Option<String>> = RwSignal::new(None);

    let job_id_short = if job_id.len() >= 8 {
        job_id[..8].to_string()
    } else {
        job_id.clone()
    };

    // Set up SSE connection
    let job_id_clone = job_id.clone();
    Effect::new(move |_| {
        let url = tauri::graboid_sse_url(&job_id_clone);
        let event_source = web_sys::EventSource::new(&url).ok();

        if let Some(ref es) = event_source {

            let es_for_message = es.clone();
            let onmessage = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                        let event_type = json["type"].as_str().unwrap_or("");

                        web_sys::console::log_1(&format!("SSE event type={} data_len={}", event_type, data.len()).into());

                        match event_type {
                            "progress" => {
                                if let Some(p) = json["progress"].as_f64() {
                                    progress.set(p);
                                }
                                if let Some(msg) = json["message"].as_str() {
                                    status_message.set(msg.to_string());
                                }
                                status.set("in_progress".to_string());
                            }
                            "step" => {
                                let step = ImportStep {
                                    step_number: json["step_number"].as_u64().unwrap_or(0) as u32,
                                    action: json["action"].as_str().unwrap_or("").to_string(),
                                    observation: json["observation"].as_str().unwrap_or("").to_string(),
                                    url: json["url"].as_str().unwrap_or("").to_string(),
                                    screenshot: json["screenshot"].as_str().map(|s| {
                                        if s.starts_with("data:") {
                                            s.to_string()
                                        } else {
                                            format!("data:image/png;base64,{}", s)
                                        }
                                    }),
                                    notes: json["notes"].as_array()
                                        .map(|arr| arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect())
                                        .or_else(|| json["notes"].as_str().map(|s| vec![s.to_string()]))
                                        .unwrap_or_default(),
                                    is_error: json["is_error"].as_bool().unwrap_or(false),
                                };
                                let step_num = step.step_number;
                                steps.update(|s| {
                                    // Replace existing step with same number, or append
                                    if let Some(existing) = s.iter_mut().find(|e| e.step_number == step_num) {
                                        *existing = step;
                                    } else {
                                        s.push(step);
                                        s.sort_by_key(|e| e.step_number);
                                    }
                                });
                                status.set("in_progress".to_string());
                            }
                            "screenshot" => {
                                let step_num = json["step_number"].as_u64().unwrap_or(0) as u32;
                                let data_b64 = json["data_base64"].as_str().unwrap_or("");
                                if !data_b64.is_empty() && step_num > 0 {
                                    let screenshot = if data_b64.starts_with("data:") {
                                        data_b64.to_string()
                                    } else {
                                        format!("data:image/png;base64,{}", data_b64)
                                    };
                                    steps.update(|s| {
                                        if let Some(existing) = s.iter_mut().find(|e| e.step_number == step_num) {
                                            existing.screenshot = Some(screenshot);
                                        }
                                    });
                                }
                            }
                            "log" => {
                                let entry = LogEntry {
                                    level: json["level"].as_str().unwrap_or("info").to_string(),
                                    source: json["source"].as_str().unwrap_or("").to_string(),
                                    message: json["message"].as_str().unwrap_or("").to_string(),
                                    timestamp: json["timestamp"].as_str().unwrap_or("").to_string(),
                                };
                                logs.update(|l| l.push(entry));
                            }
                            "complete" => {
                                progress.set(100.0);
                                status_message.set("Import complete!".to_string());
                                status.set("completed".to_string());
                                es_for_message.close();
                                if let Some(file_path) = json["file_path"].as_str() {
                                    if let Some(setter) = on_complete {
                                        setter.set(Some(file_path.to_string()));
                                    }
                                }
                            }
                            "error" | "failed" => {
                                let msg = json["message"].as_str().unwrap_or("Import failed");
                                status_message.set(msg.to_string());
                                status.set("failed".to_string());
                                es_for_message.close();
                                if let Some(setter) = on_failed {
                                    setter.set(Some(msg.to_string()));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });

            es.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            let es_for_error = es.clone();
            let onerror = Closure::<dyn Fn()>::new(move || {
                let s = status.get();
                if s == "completed" || s == "failed" {
                    es_for_error.close();
                } else if s == "pending" {
                    es_for_error.close();
                    status_message.set("Failed to connect to import service".to_string());
                    status.set("failed".to_string());
                    if let Some(setter) = on_failed {
                        setter.set(Some("Failed to connect to import service".to_string()));
                    }
                } else {
                    status_message.set("Connection lost. Reconnecting...".to_string());
                }
            });
            es.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();
        }
    });

    let progress_width = move || format!("{}%", progress.get());
    let is_done = move || {
        let s = status.get();
        s == "completed" || s == "failed" || s == "cancelled"
    };

    view! {
        <div class="import-progress" class:import-done=is_done>
            <div class="import-progress-header">
                <span class="import-job-id">{job_id_short}</span>
                <div class="import-progress-bar-container">
                    <div
                        class="import-progress-bar"
                        class:import-progress-complete=move || status.get() == "completed"
                        class:import-progress-failed=move || status.get() == "failed"
                        style:width=progress_width
                    />
                </div>
                <span class="import-progress-percent">
                    {move || format!("{:.0}%", progress.get())}
                </span>
                <span class="import-progress-status">
                    {move || status_message.get()}
                </span>
                <button
                    class="import-progress-expand"
                    on:click=move |_| expanded.update(|e| *e = !*e)
                >
                    {move || if expanded.get() { "\u{25B2}" } else { "\u{25BC}" }}
                </button>
            </div>

            <Show when=move || expanded.get()>
                <div class="import-progress-details">
                    // Screenshot lightbox
                    <Show when=move || selected_screenshot.get().is_some()>
                        <div
                            class="import-screenshot-lightbox"
                            on:click=move |_| selected_screenshot.set(None)
                        >
                            <img src=move || selected_screenshot.get().unwrap_or_default() />
                        </div>
                    </Show>

                    <Show when=move || !steps.get().is_empty()>
                        <div class="import-steps">
                            <For
                                each=move || steps.get()
                                key=|step| step.step_number
                                let:step
                            >
                                <div class="import-step" class:import-step-error=step.is_error>
                                    <div class="import-step-header">
                                        <span class="import-step-number">{step.step_number}</span>
                                        <span class="import-step-action">{step.action.clone()}</span>
                                    </div>
                                    {(!step.url.is_empty()).then(|| view! {
                                        <div class="import-step-url">{step.url.clone()}</div>
                                    })}
                                    {(!step.observation.is_empty()).then(|| view! {
                                        <div class="import-step-observation">{step.observation.clone()}</div>
                                    })}
                                    {(!step.notes.is_empty()).then(|| view! {
                                        <div class="import-step-notes">
                                            {step.notes.iter().map(|note| view! {
                                                <div class="import-step-note">{note.clone()}</div>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    })}
                                    {step.screenshot.clone().map(|src| {
                                        let src_for_click = src.clone();
                                        view! {
                                            <img
                                                class="import-step-screenshot"
                                                src=src
                                                on:click=move |e| {
                                                    e.stop_propagation();
                                                    selected_screenshot.set(Some(src_for_click.clone()));
                                                }
                                            />
                                        }
                                    })}
                                </div>
                            </For>
                        </div>
                    </Show>

                    <Show when=move || steps.get().is_empty()>
                        <div class="import-no-steps">
                            "Waiting for agent steps..."
                        </div>
                    </Show>

                    // Collapsible log viewer
                    <Show when=move || !logs.get().is_empty()>
                        <div class="import-logs-section">
                            <button
                                class="import-logs-toggle"
                                on:click=move |_| logs_expanded.update(|e| *e = !*e)
                            >
                                {move || if logs_expanded.get() { "\u{25B2} Logs" } else { "\u{25BC} Logs" }}
                                <span class="import-logs-count">{move || format!("({})", logs.get().len())}</span>
                            </button>
                            <Show when=move || logs_expanded.get()>
                                <div class="import-logs">
                                    <For
                                        each=move || logs.get()
                                        key=|log| format!("{}-{}-{}", log.timestamp, log.source, log.message)
                                        let:log
                                    >
                                        <div class=format!("import-log-entry import-log-{}", log.level)>
                                            {(!log.timestamp.is_empty()).then(|| view! {
                                                <span class="import-log-timestamp">{log.timestamp.clone()}</span>
                                            })}
                                            {(!log.source.is_empty()).then(|| view! {
                                                <span class="import-log-source">{log.source.clone()}</span>
                                            })}
                                            <span class="import-log-message">{log.message.clone()}</span>
                                        </div>
                                    </For>
                                </div>
                            </Show>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
