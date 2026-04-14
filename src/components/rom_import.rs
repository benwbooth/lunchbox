//! ROM Import component — scan directories, match ROMs to games, import

use crate::tauri;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashSet;

#[component]
pub fn RomImport(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (scan_results, set_scan_results) = signal::<Vec<tauri::ScannedRom>>(Vec::new());
    let (scanning, set_scanning) = signal(false);
    let (importing, set_importing) = signal(false);
    let (import_result, set_import_result) = signal::<Option<String>>(None);
    let (search_query, set_search_query) = signal(String::new());
    let (sort_column, set_sort_column) = signal("match_confidence".to_string());
    let (sort_ascending, set_sort_ascending) = signal(false);
    let (selected_rows, set_selected_rows) = signal::<HashSet<usize>>(HashSet::new());
    let (scan_dir, set_scan_dir) = signal(String::new());
    let (compute_checksums, set_compute_checksums) = signal(true);
    let (total_scanned, set_total_scanned) = signal(0usize);
    let (matched_count, set_matched_count) = signal(0usize);

    // Filtered and sorted results
    let filtered_results = move || {
        let query = search_query.get().to_lowercase();
        let col = sort_column.get();
        let asc = sort_ascending.get();

        let mut results: Vec<(usize, tauri::ScannedRom)> = scan_results
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, rom)| {
                if query.is_empty() {
                    return true;
                }
                rom.file_name.to_lowercase().contains(&query)
                    || rom
                        .matched_game_title
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || rom
                        .detected_platform
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();

        results.sort_by(|(_, a), (_, b)| {
            let ord = match col.as_str() {
                "file_name" => a.file_name.cmp(&b.file_name),
                "platform" => a.detected_platform.cmp(&b.detected_platform),
                "game" => a.matched_game_title.cmp(&b.matched_game_title),
                "size" => a.file_size.cmp(&b.file_size),
                _ => b
                    .match_confidence
                    .partial_cmp(&a.match_confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            };
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });

        results
    };

    let on_scan = move |_| {
        let dir = scan_dir.get();
        if dir.is_empty() {
            return;
        }
        let checksums = compute_checksums.get();
        set_scanning.set(true);
        set_scan_results.set(Vec::new());
        set_import_result.set(None);
        spawn_local(async move {
            match tauri::scan_and_match_roms(vec![dir], checksums, None).await {
                Ok(result) => {
                    set_total_scanned.set(result.total_scanned);
                    set_matched_count.set(result.matched_count);
                    // Pre-select all matched ROMs
                    let selected: HashSet<usize> = result
                        .roms
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.matched_launchbox_db_id.is_some())
                        .map(|(i, _)| i)
                        .collect();
                    set_selected_rows.set(selected);
                    set_scan_results.set(result.roms);
                }
                Err(e) => set_import_result.set(Some(format!("Scan failed: {e}"))),
            }
            set_scanning.set(false);
        });
    };

    let on_import_selected = move |_| {
        let selected = selected_rows.get();
        let results = scan_results.get();
        let entries: Vec<tauri::RomImportEntry> = results
            .iter()
            .enumerate()
            .filter(|(i, _)| selected.contains(i))
            .filter_map(|(_, rom)| {
                Some(tauri::RomImportEntry {
                    file_path: rom.file_path.clone(),
                    launchbox_db_id: rom.matched_launchbox_db_id?,
                    game_title: rom.matched_game_title.clone()?,
                    platform: rom.detected_platform.clone()?,
                    copy_to_library: false,
                })
            })
            .collect();

        if entries.is_empty() {
            return;
        }

        let count = entries.len();
        set_importing.set(true);
        spawn_local(async move {
            match tauri::confirm_rom_import(entries).await {
                Ok(imported) => {
                    set_import_result.set(Some(format!("Imported {imported} of {count} ROMs")));
                }
                Err(e) => set_import_result.set(Some(format!("Import failed: {e}"))),
            }
            set_importing.set(false);
        });
    };

    let toggle_sort = move |col: &str| {
        let col = col.to_string();
        if sort_column.get() == col {
            set_sort_ascending.update(|v| *v = !*v);
        } else {
            set_sort_column.set(col);
            set_sort_ascending.set(false);
        }
    };

    view! {
        <div class="rom-import-overlay">
            <div class="rom-import-dialog">
                <div class="rom-import-header">
                    <h2>"Import ROMs"</h2>
                    <button class="file-picker-close" on:click=move |_| on_close.run(())>"X"</button>
                </div>

                // Scan controls
                <div class="rom-import-controls">
                    <input
                        class="settings-input rom-import-dir-input"
                        type="text"
                        placeholder="Enter ROM directory path..."
                        prop:value=move || scan_dir.get()
                        on:input=move |ev| set_scan_dir.set(event_target_value(&ev))
                    />
                    <label class="rom-import-checkbox">
                        <input
                            type="checkbox"
                            prop:checked=move || compute_checksums.get()
                            on:change=move |ev| set_compute_checksums.set(event_target_checked(&ev))
                        />
                        " Checksums"
                    </label>
                    <button
                        class="import-btn-action"
                        disabled=move || scanning.get() || scan_dir.get().is_empty()
                        on:click=on_scan
                    >
                        {move || if scanning.get() { "Scanning..." } else { "Scan" }}
                    </button>
                </div>

                // Status bar
                <Show when=move || { total_scanned.get() > 0 }>
                    <div class="rom-import-status">
                        {move || format!(
                            "{} scanned, {} matched, {} unmatched",
                            total_scanned.get(),
                            matched_count.get(),
                            total_scanned.get() - matched_count.get(),
                        )}
                    </div>
                </Show>

                // Search
                <Show when=move || !scan_results.get().is_empty()>
                    <input
                        class="settings-input rom-import-search"
                        type="text"
                        placeholder="Search results..."
                        prop:value=move || search_query.get()
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))
                    />
                </Show>

                // Results table
                <Show when=move || !scan_results.get().is_empty()>
                    <div class="rom-import-table-wrapper">
                        <table class="rom-import-table">
                            <thead>
                                <tr>
                                    <th class="rom-import-th-check">
                                        <input
                                            type="checkbox"
                                            prop:checked=move || {
                                                let filtered = filtered_results();
                                                let sel = selected_rows.get();
                                                !filtered.is_empty() && filtered.iter().all(|(i, _)| sel.contains(i))
                                            }
                                            on:change=move |_| {
                                                let filtered = filtered_results();
                                                let mut sel = selected_rows.get();
                                                let all_selected = filtered.iter().all(|(i, _)| sel.contains(i));
                                                if all_selected {
                                                    for (i, _) in &filtered { sel.remove(i); }
                                                } else {
                                                    for (i, _) in &filtered { sel.insert(*i); }
                                                }
                                                set_selected_rows.set(sel);
                                            }
                                        />
                                    </th>
                                    <th class="rom-import-th sortable" on:click=move |_| toggle_sort("match_confidence")>"Status"</th>
                                    <th class="rom-import-th sortable" on:click=move |_| toggle_sort("file_name")>"ROM File"</th>
                                    <th class="rom-import-th sortable" on:click=move |_| toggle_sort("platform")>"Platform"</th>
                                    <th class="rom-import-th sortable" on:click=move |_| toggle_sort("game")>"Matched Game"</th>
                                    <th class="rom-import-th sortable" on:click=move |_| toggle_sort("size")>"Size"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || filtered_results().into_iter().map(|(idx, rom)| {
                                    let is_selected = move || selected_rows.get().contains(&idx);
                                    let status_class = if rom.match_confidence >= 0.9 {
                                        "match-high"
                                    } else if rom.match_confidence >= 0.5 {
                                        "match-medium"
                                    } else if rom.matched_game_id.is_some() {
                                        "match-low"
                                    } else {
                                        "match-none"
                                    };
                                    let status_icon = if rom.match_confidence >= 0.9 { "checkmark" } else if rom.match_confidence >= 0.5 { "tilde" } else { "cross" };
                                    let size_str = format_size(rom.file_size);
                                    let method = rom.match_method.clone().unwrap_or_default();
                                    let conf = rom.match_confidence;
                                    let game_title = rom.matched_game_title.clone().unwrap_or_else(|| "—".to_string());
                                    let platform = rom.detected_platform.clone().unwrap_or_else(|| "Unknown".to_string());
                                    let file_name = rom.file_name.clone();

                                    view! {
                                        <tr class=format!("rom-import-row {status_class}")>
                                            <td>
                                                <input
                                                    type="checkbox"
                                                    prop:checked=is_selected
                                                    on:change=move |_| {
                                                        let mut sel = selected_rows.get();
                                                        if sel.contains(&idx) {
                                                            sel.remove(&idx);
                                                        } else {
                                                            sel.insert(idx);
                                                        }
                                                        set_selected_rows.set(sel);
                                                    }
                                                />
                                            </td>
                                            <td class="rom-import-status-cell">
                                                <span class=format!("rom-import-status-icon {status_icon}")>
                                                    {match status_icon {
                                                        "checkmark" => "V",
                                                        "tilde" => "~",
                                                        _ => "X",
                                                    }}
                                                </span>
                                            </td>
                                            <td class="rom-import-filename">{file_name}</td>
                                            <td>{platform}</td>
                                            <td>
                                                {game_title}
                                                {(conf > 0.0).then(|| view! {
                                                    <span class="rom-import-confidence">
                                                        {format!(" {:.0}% {}", conf * 100.0, method)}
                                                    </span>
                                                })}
                                            </td>
                                            <td class="rom-import-size">{size_str}</td>
                                        </tr>
                                    }
                                }).collect::<Vec<_>>()}
                            </tbody>
                        </table>
                    </div>
                </Show>

                // Import result message
                <Show when=move || import_result.get().is_some()>
                    {move || import_result.get().map(|msg| view! {
                        <div class="rom-import-result">{msg}</div>
                    })}
                </Show>

                // Action buttons
                <Show when=move || !scan_results.get().is_empty()>
                    <div class="rom-import-actions">
                        <button
                            class="import-btn-action"
                            disabled=move || importing.get() || selected_rows.get().is_empty()
                            on:click=on_import_selected
                        >
                            {move || {
                                let count = selected_rows.get().len();
                                if importing.get() {
                                    "Importing...".to_string()
                                } else {
                                    format!("Import Selected ({count})")
                                }
                            }}
                        </button>
                        <button class="cancel-import-btn" on:click=move |_| on_close.run(())>"Close"</button>
                    </div>
                </Show>
            </div>
        </div>
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
