//! Download queue status component for sidebar
//!
//! Shows real-time stats on active/pending downloads, speed, and per-source timing.

use leptos::prelude::*;
use super::lazy_image::{queue_stats_signal, SourceStats};

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format bytes per second as human-readable speed
fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// Format milliseconds as human-readable duration
fn format_duration_ms(ms: u64) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

/// Queue status display component
#[component]
pub fn QueueStatus() -> impl IntoView {
    let (stats, _) = queue_stats_signal();

    // Get sorted sources for display
    let sources_sorted = move || {
        let s = stats.get();
        let mut sources: Vec<(String, SourceStats)> = s.by_source.into_iter().collect();
        sources.sort_by(|a, b| b.1.completed.cmp(&a.1.completed)); // Sort by completed count
        sources
    };

    view! {
        <div class="queue-status">
            <div class="queue-status-header">
                <h3>"Media Downloads"</h3>
            </div>

            <div class="queue-status-summary">
                <div class="queue-stat">
                    <span class="stat-label">"Active:"</span>
                    <span class="stat-value">{move || stats.get().active}</span>
                </div>
                <div class="queue-stat">
                    <span class="stat-label">"Queued:"</span>
                    <span class="stat-value">{move || stats.get().pending}</span>
                </div>
                <div class="queue-stat">
                    <span class="stat-label">"Done:"</span>
                    <span class="stat-value">{move || stats.get().total_completed}</span>
                </div>
                <div class="queue-stat">
                    <span class="stat-label">"Failed:"</span>
                    <span class="stat-value">{move || stats.get().total_failed}</span>
                </div>
            </div>

            <div class="queue-status-speed">
                <span class="stat-label">"Speed:"</span>
                <span class="stat-value">{move || format_speed(stats.get().bytes_per_sec())}</span>
                <span class="stat-label" style="margin-left: 8px">"Total:"</span>
                <span class="stat-value">{move || format_bytes(stats.get().total_bytes)}</span>
            </div>

            // Per-source stats table
            <Show when=move || !sources_sorted().is_empty()>
                <div class="queue-source-stats">
                    <div class="source-stats-header">
                        <span class="source-col">"Src"</span>
                        <span class="count-col">"OK"</span>
                        <span class="count-col">"Fail"</span>
                        <span class="time-col">"Avg"</span>
                    </div>
                    <For
                        each=move || sources_sorted()
                        key=|(name, _)| name.clone()
                        let:source
                    >
                        <div class="source-stats-row">
                            <span class="source-col">{source.0}</span>
                            <span class="count-col">{source.1.completed}</span>
                            <span class="count-col">{source.1.failed}</span>
                            <span class="time-col">{format_duration_ms(source.1.avg_time_ms())}</span>
                        </div>
                    </For>
                </div>
            </Show>
        </div>
    }
}
