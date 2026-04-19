use crate::backend_api::{self, EmulatorUpdate};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, PartialEq, Eq)]
enum UpdateRowStatus {
    Pending,
    Updating,
    Success,
    Failed(String),
}

#[derive(Clone, PartialEq, Eq)]
struct UpdateRow {
    update: EmulatorUpdate,
    status: UpdateRowStatus,
}

#[component]
pub fn EmulatorUpdates(
    show: ReadSignal<bool>,
    on_close: WriteSignal<bool>,
    set_update_count: WriteSignal<Option<usize>>,
) -> impl IntoView {
    let rows = RwSignal::new(Vec::<UpdateRow>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (updating, set_updating) = signal(false);

    let load_updates = Callback::new(move |_| {
        set_loading.set(true);
        set_error.set(None);
        set_update_count.set(None);

        spawn_local(async move {
            match backend_api::get_emulator_updates().await {
                Ok(updates) => {
                    set_update_count.set(Some(updates.len()));
                    rows.set(
                        updates
                            .into_iter()
                            .map(|update| UpdateRow {
                                update,
                                status: UpdateRowStatus::Pending,
                            })
                            .collect(),
                    );
                }
                Err(err) => {
                    set_update_count.set(Some(0));
                    rows.set(Vec::new());
                    set_error.set(Some(err));
                }
            }
            set_loading.set(false);
        });
    });

    Effect::new(move || {
        if show.get() {
            load_updates.run(());
        }
    });

    let update_all = move |_| {
        if loading.get() || updating.get() {
            return;
        }

        let pending = rows.with_untracked(|current| {
            current
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| match row.status {
                    UpdateRowStatus::Pending | UpdateRowStatus::Failed(_) => {
                        Some((idx, row.update.key.clone()))
                    }
                    UpdateRowStatus::Updating | UpdateRowStatus::Success => None,
                })
                .collect::<Vec<_>>()
        });

        if pending.is_empty() {
            set_update_count.set(Some(0));
            return;
        }

        set_updating.set(true);
        set_error.set(None);

        spawn_local(async move {
            for (idx, update_key) in pending {
                rows.update(|current| {
                    if let Some(row) = current.get_mut(idx) {
                        row.status = UpdateRowStatus::Updating;
                    }
                });

                match backend_api::update_emulator(update_key).await {
                    Ok(()) => {
                        rows.update(|current| {
                            if let Some(row) = current.get_mut(idx) {
                                row.status = UpdateRowStatus::Success;
                            }
                        });
                    }
                    Err(err) => {
                        rows.update(|current| {
                            if let Some(row) = current.get_mut(idx) {
                                row.status = UpdateRowStatus::Failed(err);
                            }
                        });
                    }
                }
            }

            let remaining = rows.with_untracked(|current| {
                current
                    .iter()
                    .filter(|row| {
                        matches!(
                            row.status,
                            UpdateRowStatus::Pending | UpdateRowStatus::Failed(_)
                        )
                    })
                    .count()
            });
            set_update_count.set(Some(remaining));
            set_updating.set(false);
        });
    };

    view! {
        <Show when=move || show.get()>
            <div class="settings-overlay" on:click=move |_| {
                if !updating.get() {
                    on_close.set(false);
                }
            }>
                <div class="emulator-updates-panel" on:click=|ev| ev.stop_propagation()>
                    <button
                        class="close-btn"
                        on:click=move |_| {
                            if !updating.get() {
                                on_close.set(false);
                            }
                        }
                        disabled=move || updating.get()
                    >
                        "×"
                    </button>
                    <h2 class="settings-title">"Emulator Updates"</h2>

                    <p class="emulator-updates-intro">
                        "Lunchbox will only update emulators when you explicitly request it here."
                    </p>

                    <Show
                        when=move || !loading.get()
                        fallback=|| view! { <div class="loading">"Checking installed emulators for updates..."</div> }
                    >
                        {move || {
                            if let Some(err) = error.get() {
                                view! {
                                    <div class="emulator-updates-error">
                                        {format!("Update check failed: {}", err)}
                                    </div>
                                }
                                    .into_any()
                            } else if rows.get().is_empty() {
                                view! {
                                    <div class="emulator-updates-empty">
                                        "No managed emulator updates are currently available."
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div class="emulator-updates-list">
                                        <For
                                            each=move || rows.get()
                                            key=|row| row.update.key.clone()
                                            children=move |row| {
                                                let version_summary = match (
                                                    row.update.current_version.clone(),
                                                    row.update.available_version.clone(),
                                                ) {
                                                    (Some(current), Some(available)) => {
                                                        format!("{} -> {}", current, available)
                                                    }
                                                    (None, Some(available)) => {
                                                        format!("Update available: {}", available)
                                                    }
                                                    (Some(current), None) => {
                                                        format!("Installed: {}", current)
                                                    }
                                                    (None, None) => "Version change available".to_string(),
                                                };

                                                let (status_text, progress_class) = match row.status {
                                                    UpdateRowStatus::Pending => ("Ready to update".to_string(), "pending"),
                                                    UpdateRowStatus::Updating => ("Updating...".to_string(), "updating"),
                                                    UpdateRowStatus::Success => ("Updated".to_string(), "success"),
                                                    UpdateRowStatus::Failed(err) => (format!("Update failed: {}", err), "failed"),
                                                };

                                                view! {
                                                    <div class="emulator-update-row">
                                                        <div class="emulator-update-header">
                                                            <div>
                                                                <div class="emulator-update-name">{row.update.display_name}</div>
                                                                <div class="emulator-update-source">{row.update.source_label}</div>
                                                            </div>
                                                            <div class="emulator-update-version">{version_summary}</div>
                                                        </div>
                                                        <div class="emulator-update-status">{status_text}</div>
                                                        <div class="emulator-update-progress">
                                                            <div class=format!("emulator-update-progress-fill {}", progress_class) />
                                                        </div>
                                                    </div>
                                                }
                                            }
                                        />
                                    </div>
                                }
                                    .into_any()
                            }
                        }}
                    </Show>

                    <div class="emulator-updates-actions">
                        <button
                            class="dialog-cancel"
                            on:click=move |_| load_updates.run(())
                            disabled=move || loading.get() || updating.get()
                        >
                            "Refresh"
                        </button>
                        <button
                            class="dialog-confirm"
                            on:click=update_all
                            disabled=move || loading.get() || updating.get() || rows.get().is_empty()
                        >
                            {move || if updating.get() { "Updating..." } else { "Update All" }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
