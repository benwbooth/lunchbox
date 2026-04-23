use crate::backend_api::{self, MinervaDownloadQueueItem};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

static MINERVA_DOWNLOADS_SIGNAL: OnceLock<(
    ReadSignal<Vec<MinervaDownloadQueueItem>>,
    WriteSignal<Vec<MinervaDownloadQueueItem>>,
)> = OnceLock::new();
static MINERVA_RECENT_SIGNAL: OnceLock<(
    ReadSignal<Vec<MinervaDownloadQueueItem>>,
    WriteSignal<Vec<MinervaDownloadQueueItem>>,
)> = OnceLock::new();
static MINERVA_DOWNLOAD_REFRESH_SIGNAL: OnceLock<(ReadSignal<u64>, WriteSignal<u64>)> =
    OnceLock::new();

pub fn minerva_downloads_signal() -> (
    ReadSignal<Vec<MinervaDownloadQueueItem>>,
    WriteSignal<Vec<MinervaDownloadQueueItem>>,
) {
    *MINERVA_DOWNLOADS_SIGNAL.get_or_init(|| signal(Vec::new()))
}

fn minerva_recent_signal() -> (
    ReadSignal<Vec<MinervaDownloadQueueItem>>,
    WriteSignal<Vec<MinervaDownloadQueueItem>>,
) {
    *MINERVA_RECENT_SIGNAL.get_or_init(|| signal(Vec::new()))
}

fn minerva_download_refresh_signal() -> (ReadSignal<u64>, WriteSignal<u64>) {
    *MINERVA_DOWNLOAD_REFRESH_SIGNAL.get_or_init(|| signal(0))
}

pub fn request_minerva_download_queue_refresh() {
    let (_, set_refresh) = minerva_download_refresh_signal();
    set_refresh.update(|value| *value = value.saturating_add(1));
}

pub async fn refresh_minerva_download_queue_now() {
    let (_, set_downloads) = minerva_downloads_signal();
    let (recent_downloads, set_recent_downloads) = minerva_recent_signal();
    match backend_api::list_minerva_downloads().await {
        Ok(downloads) => {
            let active_downloads = downloads
                .iter()
                .filter(|item| !is_terminal_download_status(&item.status))
                .cloned()
                .collect::<Vec<_>>();
            let mut recent_by_id = recent_downloads
                .get_untracked()
                .into_iter()
                .map(|item| (item.job_id.clone(), item))
                .collect::<HashMap<_, _>>();
            for item in downloads
                .iter()
                .filter(|item| is_terminal_download_status(&item.status))
            {
                recent_by_id.insert(item.job_id.clone(), item.clone());
            }
            let active_ids = active_downloads
                .iter()
                .map(|item| item.job_id.as_str())
                .collect::<HashSet<_>>();
            let mut recent = recent_by_id
                .into_values()
                .filter(|item| !active_ids.contains(item.job_id.as_str()))
                .collect::<Vec<_>>();
            recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            set_downloads.set(active_downloads);
            set_recent_downloads.set(recent);
        }
        Err(err) => {
            backend_api::log_to_backend(
                "warn",
                &format!("Failed to refresh Minerva download queue: {err}"),
            );
        }
    }
}

async fn delay_ms(ms: i32) {
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    }))
    .await
    .unwrap();
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1_000_000 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / 1_000_000.0)
    } else if bytes_per_sec >= 1_000 {
        format!("{:.1} KB/s", bytes_per_sec as f64 / 1_000.0)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

fn is_terminal_download_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

fn is_active_download_status(status: &str) -> bool {
    matches!(
        status,
        "fetching_torrent" | "downloading" | "extracting" | "in_progress" | "pending"
    )
}

fn status_label(status: &str) -> &'static str {
    match status {
        "fetching_torrent" => "Preparing",
        "downloading" | "in_progress" => "Downloading",
        "extracting" => "Finishing",
        "paused" => "Paused",
        "completed" => "Completed",
        "failed" => "Failed",
        "cancelled" => "Cancelled",
        "pending" => "Queued",
        _ => "Download",
    }
}

#[component]
pub fn MinervaDownloadQueue() -> impl IntoView {
    let (downloads, _) = minerva_downloads_signal();
    let (recent_downloads, set_recent_downloads) = minerva_recent_signal();
    let (refresh_tick, _) = minerva_download_refresh_signal();
    let (expanded, set_expanded) = signal(false);
    let (dismissed_recent_ids, set_dismissed_recent_ids) =
        signal::<HashSet<String>>(HashSet::new());

    Effect::new(move || {
        let _ = refresh_tick.get();
        spawn_local(async move {
            refresh_minerva_download_queue_now().await;
        });
    });

    Effect::new(move || {
        spawn_local(async move {
            refresh_minerva_download_queue_now().await;
            loop {
                delay_ms(2000).await;
                refresh_minerva_download_queue_now().await;
            }
        });
    });

    let visible_recent_downloads = Memo::new(move |_| {
        let dismissed_ids = dismissed_recent_ids.get();
        recent_downloads
            .get()
            .into_iter()
            .filter(|item| !dismissed_ids.contains(&item.job_id))
            .collect::<Vec<_>>()
    });

    let visible_downloads = Memo::new(move |_| {
        let mut items = downloads.get();
        items.extend(visible_recent_downloads.get());
        items
    });

    let active_count = move || downloads.get().len();
    let total_speed = move || {
        downloads
            .get()
            .iter()
            .map(|item| item.download_speed)
            .sum::<u64>()
    };
    let active_downloaded_bytes = move || {
        downloads
            .get()
            .iter()
            .map(|item| item.downloaded_bytes)
            .sum::<u64>()
    };
    let active_total_bytes = move || {
        downloads
            .get()
            .iter()
            .map(|item| item.total_bytes)
            .sum::<u64>()
    };

    let dismiss_finished = move || {
        let visible_recent = visible_recent_downloads.get_untracked();
        if visible_recent.is_empty() {
            set_expanded.set(false);
            return;
        }
        let dismissed_ids = visible_recent
            .iter()
            .map(|item| item.job_id.clone())
            .collect::<HashSet<_>>();
        set_dismissed_recent_ids.update(|ids| ids.extend(dismissed_ids));
        set_recent_downloads.update(|items| {
            items.retain(|item| {
                !visible_recent
                    .iter()
                    .any(|recent| recent.job_id == item.job_id)
            })
        });
        if downloads.get_untracked().is_empty() {
            set_expanded.set(false);
        }
    };

    view! {
        <Show when=move || !visible_downloads.get().is_empty()>
            <div class="minerva-download-queue" class:expanded=move || expanded.get()>
                <button
                    class="minerva-download-queue-toggle"
                    on:click=move |_| set_expanded.update(|value| *value = !*value)
                >
                    <div class="minerva-download-queue-summary">
                        <div class="minerva-download-queue-title">
                            <span>"Downloads"</span>
                            <span class="minerva-download-queue-count">
                                {move || {
                                    let finished_count = visible_recent_downloads.get().len();
                                    if finished_count > 0 {
                                        format!("{} active • {} recent", active_count(), finished_count)
                                    } else {
                                        format!("{} active", active_count())
                                    }
                                }}
                            </span>
                        </div>
                        <div class="minerva-download-queue-meta">
                            <span>{move || format!("{} / {}", format_bytes(active_downloaded_bytes()), format_bytes(active_total_bytes()))}</span>
                            <span>{move || format_speed(total_speed())}</span>
                        </div>
                    </div>
                    <div class="minerva-download-queue-header-actions">
                        <Show when=move || !visible_recent_downloads.get().is_empty()>
                            <button
                                class="minerva-download-queue-dismiss"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    dismiss_finished();
                                }
                                title="Dismiss finished downloads"
                            >
                                "×"
                            </button>
                        </Show>
                        <span class="minerva-download-queue-chevron">
                            {move || if expanded.get() { "−" } else { "+" }}
                        </span>
                    </div>
                </button>

                <Show when=move || expanded.get()>
                    <div class="minerva-download-queue-list">
                        <Show when=move || !visible_recent_downloads.get().is_empty()>
                            <div class="minerva-download-queue-list-actions">
                                <button
                                    class="minerva-download-queue-clear"
                                    on:click=move |_| dismiss_finished()
                                >
                                    "Clear Finished"
                                </button>
                            </div>
                        </Show>
                        <For
                            each=move || visible_downloads.get()
                            key=|item| item.job_id.clone()
                            let:item
                        >
                            {
                                let job_id = item.job_id.clone();
                                let item_status = item.status.clone();
                                let pause_label = if item_status == "paused" {
                                    "Resume"
                                } else {
                                    "Pause"
                                };
                                let pause_action_job_id = job_id.clone();
                                let delete_action_job_id = job_id.clone();
                                let can_pause = item_status == "paused"
                                    || is_active_download_status(&item_status);
                                let show_progress = item.total_bytes > 0 || item.progress_percent > 0.0;
                                let progress_percent = item.progress_percent.clamp(0.0, 100.0);
                                let is_paused = item_status == "paused";
                                let is_terminal = is_terminal_download_status(&item_status);
                                let progress_text = if item.total_bytes > 0 {
                                    format!(
                                        "{:.0}% • {} / {}{}",
                                        item.progress_percent,
                                        format_bytes(item.downloaded_bytes),
                                        format_bytes(item.total_bytes),
                                        if item.download_speed > 0 {
                                            format!(" • {}", format_speed(item.download_speed))
                                        } else {
                                            String::new()
                                        }
                                    )
                                } else {
                                    format!("{:.0}%", item.progress_percent)
                                };
                                let platform_label =
                                    format!("{} • {}", item.platform, status_label(&item_status));
                                let game_title = item.game_title.clone();
                                let status_message = item.status_message.clone();
                                let pause_click_job_id = StoredValue::new(pause_action_job_id.clone());
                                let pause_click_status = StoredValue::new(item_status.clone());
                                let delete_click_job_id = StoredValue::new(delete_action_job_id.clone());
                                let progress_text_for_view = StoredValue::new(progress_text.clone());

                                view! {
                                    <div class="minerva-download-queue-item">
                                        <div class="minerva-download-queue-item-header">
                                            <div>
                                                <div class="minerva-download-queue-game">{game_title}</div>
                                                <div class="minerva-download-queue-platform">
                                                    {platform_label}
                                                </div>
                                            </div>
                                            <div class="minerva-download-queue-actions">
                                                <Show when=move || can_pause>
                                                    <button
                                                        class="minerva-download-queue-action"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            let job_id = pause_click_job_id.get_value();
                                                            let status = pause_click_status.get_value();
                                                            spawn_local(async move {
                                                                let result = if status == "paused" {
                                                                    backend_api::resume_minerva_download(job_id.clone()).await
                                                                } else {
                                                                    backend_api::pause_minerva_download(job_id.clone()).await
                                                                };
                                                                if let Err(err) = result {
                                                                    backend_api::log_to_backend(
                                                                        "warn",
                                                                        &format!("Failed to update Minerva download {job_id}: {err}"),
                                                                    );
                                                                }
                                                                refresh_minerva_download_queue_now().await;
                                                            });
                                                        }
                                                    >
                                                        {pause_label}
                                                    </button>
                                                </Show>
                                                <button
                                                    class="minerva-download-queue-action danger"
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        let job_id = delete_click_job_id.get_value();
                                                        spawn_local(async move {
                                                            if let Err(err) = backend_api::delete_minerva_download(job_id.clone()).await {
                                                                backend_api::log_to_backend(
                                                                    "warn",
                                                                    &format!("Failed to delete Minerva download {job_id}: {err}"),
                                                                );
                                                            }
                                                            refresh_minerva_download_queue_now().await;
                                                        });
                                                    }
                                                >
                                                    "Delete"
                                                </button>
                                            </div>
                                        </div>

                                        <div class="minerva-download-queue-status">{status_message}</div>

                                        <Show when=move || show_progress>
                                            <div class="minerva-download-queue-progress-row">
                                                <div class="minerva-download-queue-progress">
                                                    <div
                                                        class="minerva-download-queue-progress-fill"
                                                        class:is-paused=move || is_paused
                                                        class:is-terminal=move || is_terminal
                                                        style:width=move || format!("{progress_percent:.1}%")
                                                    ></div>
                                                </div>
                                                <div class="minerva-download-queue-progress-text">
                                                    {move || progress_text_for_view.get_value()}
                                                </div>
                                            </div>
                                        </Show>
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </div>
        </Show>
    }
}
