//! Game details panel

use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::tauri::{self, file_to_asset_url, Game, PlayStats};

#[component]
pub fn GameDetails(
    game: ReadSignal<Option<Game>>,
    on_close: WriteSignal<Option<Game>>,
) -> impl IntoView {
    let (play_stats, set_play_stats) = signal::<Option<PlayStats>>(None);
    let (is_fav, set_is_fav) = signal(false);

    // Load play stats and favorite status when game changes
    Effect::new(move || {
        if let Some(g) = game.get() {
            let db_id = g.database_id;
            spawn_local(async move {
                // Load play stats
                if let Ok(stats) = tauri::get_play_stats(db_id).await {
                    set_play_stats.set(stats);
                }
                // Check favorite status
                if let Ok(fav) = tauri::is_favorite(db_id).await {
                    set_is_fav.set(fav);
                }
            });
        } else {
            set_play_stats.set(None);
            set_is_fav.set(false);
        }
    });

    view! {
        <Show when=move || game.get().is_some()>
            {move || {
                game.get().map(|g| {
                    let title = g.title.clone();
                    let title_for_play = g.title.clone();
                    let first_char = title.chars().next().unwrap_or('?').to_string();
                    let platform = g.platform.clone();
                    let platform_for_play = g.platform.clone();
                    let description = g.description.clone().unwrap_or_else(|| "No description available.".to_string());
                    let developer = g.developer.clone().unwrap_or_else(|| "Unknown".to_string());
                    let publisher = g.publisher.clone().unwrap_or_else(|| "Unknown".to_string());
                    let genres = g.genres.clone().unwrap_or_else(|| "Unknown".to_string());
                    let year = g.release_year.map(|y| y.to_string()).unwrap_or_else(|| "Unknown".to_string());
                    let rating = g.rating.map(|r| format!("{:.1}", r)).unwrap_or_else(|| "-".to_string());
                    let box_front = g.box_front_path.clone();
                    let db_id = g.database_id;

                    let title_for_fav = g.title.clone();
                    let platform_for_fav = g.platform.clone();

                    let on_play = move |_| {
                        let title = title_for_play.clone();
                        let platform = platform_for_play.clone();
                        spawn_local(async move {
                            // Record the play session
                            let _ = tauri::record_play_session(db_id, title, platform).await;
                        });
                    };

                    let on_toggle_favorite = move |_| {
                        let title = title_for_fav.clone();
                        let platform = platform_for_fav.clone();
                        let currently_fav = is_fav.get();
                        spawn_local(async move {
                            if currently_fav {
                                if tauri::remove_favorite(db_id).await.is_ok() {
                                    set_is_fav.set(false);
                                }
                            } else {
                                if tauri::add_favorite(db_id, title, platform).await.is_ok() {
                                    set_is_fav.set(true);
                                }
                            }
                        });
                    };

                    view! {
                        <div class="game-details-overlay" on:click=move |_| on_close.set(None)>
                            <div class="game-details-panel" on:click=|e| e.stop_propagation()>
                                <button class="close-btn" on:click=move |_| on_close.set(None)>"×"</button>

                                <div class="game-details-header">
                                    <div class="game-details-cover">
                                        {match box_front {
                                            Some(path) => {
                                                let url = file_to_asset_url(&path);
                                                view! {
                                                    <img
                                                        src=url
                                                        alt=title.clone()
                                                        class="cover-image-large"
                                                    />
                                                }.into_any()
                                            }
                                            None => view! {
                                                <div class="cover-placeholder-large">
                                                    {first_char}
                                                </div>
                                            }.into_any()
                                        }}
                                    </div>
                                    <div class="game-details-info">
                                        <h1 class="game-details-title">{title}</h1>
                                        <p class="game-details-platform">{platform}</p>

                                        <div class="game-details-meta">
                                            <div class="meta-item">
                                                <span class="meta-label">"Developer"</span>
                                                <span class="meta-value">{developer}</span>
                                            </div>
                                            <div class="meta-item">
                                                <span class="meta-label">"Publisher"</span>
                                                <span class="meta-value">{publisher}</span>
                                            </div>
                                            <div class="meta-item">
                                                <span class="meta-label">"Year"</span>
                                                <span class="meta-value">{year}</span>
                                            </div>
                                            <div class="meta-item">
                                                <span class="meta-label">"Genre"</span>
                                                <span class="meta-value">{genres}</span>
                                            </div>
                                            <div class="meta-item">
                                                <span class="meta-label">"Rating"</span>
                                                <span class="meta-value">{rating}</span>
                                            </div>
                                        </div>

                                        // Play statistics
                                        <Show when=move || play_stats.get().is_some()>
                                            {move || play_stats.get().map(|stats| {
                                                let play_count = stats.play_count;
                                                let last_played = stats.last_played
                                                    .map(|s| format_date(&s))
                                                    .unwrap_or_else(|| "Never".to_string());
                                                view! {
                                                    <div class="play-stats">
                                                        <span class="play-stat">
                                                            <span class="stat-value">{play_count}</span>
                                                            " plays"
                                                        </span>
                                                        <span class="play-stat">
                                                            "Last: "
                                                            <span class="stat-value">{last_played}</span>
                                                        </span>
                                                    </div>
                                                }
                                            })}
                                        </Show>

                                        <div class="game-actions">
                                            <button class="play-btn" on:click=on_play>"Play Game"</button>
                                            <button
                                                class="favorite-btn"
                                                class:is-favorite=move || is_fav.get()
                                                on:click=on_toggle_favorite
                                            >
                                                {move || if is_fav.get() { "Unfavorite" } else { "Favorite" }}
                                            </button>
                                        </div>
                                    </div>
                                </div>

                                <div class="game-details-description">
                                    <h2>"Description"</h2>
                                    <p>{description}</p>
                                </div>
                            </div>
                        </div>
                    }
                })
            }}
        </Show>
    }
}

fn format_date(date_str: &str) -> String {
    // Input format: "2026-01-11 23:21:43"
    // Output: "Jan 11, 2026"
    if let Some((date_part, _)) = date_str.split_once(' ') {
        let parts: Vec<&str> = date_part.split('-').collect();
        if parts.len() == 3 {
            let month = match parts[1] {
                "01" => "Jan", "02" => "Feb", "03" => "Mar", "04" => "Apr",
                "05" => "May", "06" => "Jun", "07" => "Jul", "08" => "Aug",
                "09" => "Sep", "10" => "Oct", "11" => "Nov", "12" => "Dec",
                _ => parts[1],
            };
            return format!("{} {}, {}", month, parts[2], parts[0]);
        }
    }
    date_str.to_string()
}
