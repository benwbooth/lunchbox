use leptos::prelude::*;
use crate::app::ViewMode;

#[component]
pub fn Toolbar(
    view_mode: ReadSignal<ViewMode>,
    set_view_mode: WriteSignal<ViewMode>,
    search_query: ReadSignal<String>,
    set_search_query: WriteSignal<String>,
    set_show_settings: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <header class="toolbar">
            <div class="toolbar-left">
                <h1 class="app-title">"Lunchbox"</h1>
            </div>
            <div class="toolbar-center">
                <div class="search-box">
                    <input
                        type="text"
                        placeholder="Search games..."
                        prop:value=move || search_query.get()
                        on:input=move |ev| {
                            set_search_query.set(event_target_value(&ev));
                        }
                    />
                    <Show when=move || !search_query.get().is_empty()>
                        <button
                            class="search-clear"
                            on:click=move |_| set_search_query.set(String::new())
                            title="Clear search"
                        >
                            "×"
                        </button>
                    </Show>
                </div>
            </div>
            <div class="toolbar-right">
                <div class="view-toggle">
                    <button
                        class="view-btn"
                        class:active=move || view_mode.get() == ViewMode::Grid
                        on:click=move |_| set_view_mode.set(ViewMode::Grid)
                        title="Grid View"
                    >
                        "Grid"
                    </button>
                    <button
                        class="view-btn"
                        class:active=move || view_mode.get() == ViewMode::List
                        on:click=move |_| set_view_mode.set(ViewMode::List)
                        title="List View"
                    >
                        "List"
                    </button>
                </div>
                <button class="import-btn" title="Import ROMs">
                    "Import"
                </button>
                <button
                    class="settings-btn"
                    title="Settings"
                    on:click=move |_| set_show_settings.set(true)
                >
                    "Settings"
                </button>
            </div>
        </header>
    }
}
