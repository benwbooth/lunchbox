#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");
        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, api_key_saved)]
        #[qproperty(QString, message)]
        #[qproperty(QString, query)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(QString, selected_game_name)]
        #[qproperty(i32, database_id)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, artwork_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, published_revision)]
        type SteamGridDbModel = super::SteamGridDbModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut SteamGridDbModel>);

        #[qinvokable]
        fn save_and_test_api_key(self: Pin<&mut SteamGridDbModel>, api_key: QString);

        #[qinvokable]
        fn test_connection(self: Pin<&mut SteamGridDbModel>, api_key: QString);

        #[qinvokable]
        fn clear_api_key(self: Pin<&mut SteamGridDbModel>);

        #[qinvokable]
        fn begin_selection(
            self: Pin<&mut SteamGridDbModel>,
            database_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn search_games(self: Pin<&mut SteamGridDbModel>, query: QString);

        #[qinvokable]
        fn choose_game(self: Pin<&mut SteamGridDbModel>, index: i32);

        #[qinvokable]
        fn choose_artwork_kind(self: Pin<&mut SteamGridDbModel>, artwork_type: QString);

        #[qinvokable]
        fn publish_artwork(self: Pin<&mut SteamGridDbModel>, index: i32);

        #[qinvokable]
        fn cancel(self: Pin<&mut SteamGridDbModel>);

        #[qinvokable]
        fn game_name_at(self: &SteamGridDbModel, index: i32) -> QString;

        #[qinvokable]
        fn game_detail_at(self: &SteamGridDbModel, index: i32) -> QString;

        #[qinvokable]
        fn artwork_thumbnail_at(self: &SteamGridDbModel, index: i32) -> QUrl;

        #[qinvokable]
        fn artwork_detail_at(self: &SteamGridDbModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for SteamGridDbModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::media::ArtworkKind;
use crate::settings::{MediaProviderGameLink, SettingsStore};
use crate::steamgriddb::{ArtworkCandidate, Client, GameCandidate};

const PROVIDER: &str = "steamgriddb";

pub struct SteamGridDbModelRust {
    initialized: bool,
    busy: bool,
    api_key_saved: bool,
    message: QString,
    query: QString,
    artwork_type: QString,
    selected_game_name: QString,
    database_id: i32,
    game_count: i32,
    artwork_count: i32,
    revision: i32,
    published_revision: i32,
    title: String,
    platform: String,
    games: Vec<GameCandidate>,
    artwork: Vec<ArtworkCandidate>,
    selected_game_id: Option<i64>,
    generation: u64,
}

impl Default for SteamGridDbModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            api_key_saved: false,
            message: qstring("Add a SteamGridDB API key in Settings to browse reviewed artwork."),
            query: QString::default(),
            artwork_type: qstring("fanart"),
            selected_game_name: QString::default(),
            database_id: 0,
            game_count: 0,
            artwork_count: 0,
            revision: 0,
            published_revision: 0,
            title: String::new(),
            platform: String::new(),
            games: Vec::new(),
            artwork: Vec::new(),
            selected_game_id: None,
            generation: 0,
        }
    }
}

enum BeginResult {
    Linked {
        link: MediaProviderGameLink,
        artwork: Vec<ArtworkCandidate>,
    },
    Search(Vec<GameCandidate>),
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn effective_api_key(entered: String) -> anyhow::Result<String> {
    let entered = entered.trim().to_owned();
    if !entered.is_empty() {
        return Ok(entered);
    }
    if let Some(api_key) = std::env::var("LUNCHBOX_STEAMGRIDDB_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(api_key);
    }
    crate::settings::load_steamgriddb_api_key()?
        .ok_or_else(|| anyhow::anyhow!("no SteamGridDB API key is saved; add one in Settings"))
}

fn requested_kind(value: &str) -> ArtworkKind {
    match ArtworkKind::parse(value) {
        Some(ArtworkKind::BoxFront) => ArtworkKind::BoxFront,
        Some(ArtworkKind::ClearLogo) => ArtworkKind::ClearLogo,
        _ => ArtworkKind::Fanart,
    }
}

impl qobject::SteamGridDbModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-credentials".into())
            .spawn(move || {
                let result = crate::settings::load_steamgriddb_api_key()
                    .map(|key| key.is_some())
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_initialize(result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not inspect the credential store: {error}"
            )));
        }
    }

    fn finish_initialize(mut self: Pin<&mut Self>, result: Result<bool, String>) {
        self.as_mut().set_busy(false);
        self.as_mut().set_initialized(true);
        match result {
            Ok(saved) => {
                self.as_mut().set_api_key_saved(saved);
                self.as_mut().set_message(qstring(if saved {
                    "SteamGridDB API key is stored in the operating system credential store."
                } else {
                    "No SteamGridDB API key is saved. Generate one in your SteamGridDB preferences."
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not inspect the operating system credential store: {error}"
            ))),
        }
    }

    pub fn save_and_test_api_key(mut self: Pin<&mut Self>, api_key: QString) {
        let api_key = api_key.to_string().trim().to_owned();
        if api_key.is_empty() {
            self.as_mut()
                .set_message(qstring("Enter a SteamGridDB API key first."));
            return;
        }
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Testing SteamGridDB before saving the key…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-save-key".into())
            .spawn(move || {
                let result = Client::new(api_key.clone())
                    .and_then(|client| client.test_connection())
                    .and_then(|()| crate::settings::save_steamgriddb_api_key(&api_key))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, true);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start API-key test: {error}")));
        }
    }

    pub fn test_connection(mut self: Pin<&mut Self>, api_key: QString) {
        if *self.as_ref().busy() {
            return;
        }
        let entered = api_key.to_string();
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Testing the SteamGridDB connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-test".into())
            .spawn(move || {
                let result = effective_api_key(entered)
                    .and_then(Client::new)
                    .and_then(|client| client.test_connection())
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, false);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start connection test: {error}")));
        }
    }

    fn finish_credential_action(mut self: Pin<&mut Self>, result: Result<(), String>, saved: bool) {
        self.as_mut().set_busy(false);
        match result {
            Ok(()) => {
                if saved {
                    self.as_mut().set_api_key_saved(true);
                }
                self.as_mut().set_message(qstring(if saved {
                    "SteamGridDB connection succeeded. The API key is saved securely."
                } else {
                    "SteamGridDB connection succeeded."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("SteamGridDB connection failed: {error}"))),
        }
    }

    pub fn clear_api_key(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-clear-key".into())
            .spawn(move || {
                let result = crate::settings::save_steamgriddb_api_key("")
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(()) => {
                            model.as_mut().set_api_key_saved(false);
                            model.as_mut().set_message(qstring(
                                "SteamGridDB API key removed from the credential store.",
                            ));
                        }
                        Err(error) => model.as_mut().set_message(qstring(format!(
                            "Could not remove SteamGridDB API key: {error}"
                        ))),
                    }
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start credential removal: {error}"
            )));
        }
    }

    pub fn begin_selection(
        mut self: Pin<&mut Self>,
        database_id: i32,
        title: QString,
        platform: QString,
        artwork_type: QString,
    ) {
        if database_id <= 0 {
            self.as_mut()
                .set_message(qstring("This game has no stable catalog database ID."));
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        let title = title.to_string().trim().to_owned();
        let platform = platform.to_string().trim().to_owned();
        let kind = requested_kind(&artwork_type.to_string());
        self.as_mut().rust_mut().title = title.clone();
        self.as_mut().rust_mut().platform = platform;
        self.as_mut().rust_mut().games.clear();
        self.as_mut().rust_mut().artwork.clear();
        self.as_mut().rust_mut().selected_game_id = None;
        self.as_mut().set_database_id(database_id);
        self.as_mut().set_query(qstring(&title));
        self.as_mut().set_artwork_type(qstring(kind.key()));
        self.as_mut().set_selected_game_name(QString::default());
        self.as_mut().set_game_count(0);
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(format!(
            "Checking reviewed SteamGridDB artwork for {title}…"
        )));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-begin".into())
            .spawn(move || {
                let result = (|| {
                    let client = Client::new(effective_api_key(String::new())?)?;
                    let store = SettingsStore::open_default()?;
                    if let Some(link) =
                        store.media_provider_game_link(PROVIDER, i64::from(database_id))?
                    {
                        let provider_game_id = link
                            .provider_game_id
                            .parse::<i64>()
                            .map_err(|_| anyhow::anyhow!("saved SteamGridDB game ID is invalid"))?;
                        let artwork = client.artwork_for_game(provider_game_id, kind)?;
                        Ok(BeginResult::Linked { link, artwork })
                    } else {
                        Ok(BeginResult::Search(client.search_games(&title)?))
                    }
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_begin(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start artwork search: {error}")));
        }
    }

    fn finish_begin(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<BeginResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(BeginResult::Linked { link, artwork }) => {
                let count = artwork.len();
                let selected_id = link.provider_game_id.parse::<i64>().ok();
                self.as_mut().rust_mut().selected_game_id = selected_id;
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut()
                    .set_selected_game_name(qstring(&link.provider_game_name));
                self.as_mut().set_artwork_count(saturating_i32(count));
                let artwork_type = self.as_ref().artwork_type().to_string();
                self.as_mut().set_message(qstring(if count == 0 {
                    format!(
                        "The reviewed SteamGridDB link is {}, but it has no safe static {} artwork.",
                        link.provider_game_name,
                        artwork_type
                    )
                } else {
                    format!(
                        "Showing {count} artwork choices for the reviewed link {}.",
                        link.provider_game_name
                    )
                }));
            }
            Ok(BeginResult::Search(games)) => {
                let count = games.len();
                self.as_mut().rust_mut().games = games;
                self.as_mut().set_game_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    "No SteamGridDB games matched. Change the search text and try again.".to_owned()
                } else {
                    format!(
                        "Found {count} possible games. Choose the exact record before browsing artwork."
                    )
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("SteamGridDB search failed: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn search_games(mut self: Pin<&mut Self>, query: QString) {
        if *self.as_ref().busy() || *self.as_ref().database_id() <= 0 {
            return;
        }
        let query = query.to_string().trim().to_owned();
        if query.chars().count() < 2 {
            self.as_mut()
                .set_message(qstring("Enter at least two characters to search."));
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().rust_mut().games.clear();
        self.as_mut().rust_mut().artwork.clear();
        self.as_mut().rust_mut().selected_game_id = None;
        self.as_mut().set_query(qstring(&query));
        self.as_mut().set_selected_game_name(QString::default());
        self.as_mut().set_game_count(0);
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring(format!("Searching SteamGridDB for “{query}”…")));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-search".into())
            .spawn(move || {
                let result = effective_api_key(String::new())
                    .and_then(Client::new)
                    .and_then(|client| client.search_games(&query))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_game_search(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start game search: {error}")));
        }
    }

    fn finish_game_search(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<GameCandidate>, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(games) => {
                let count = games.len();
                self.as_mut().rust_mut().games = games;
                self.as_mut().set_game_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    "No SteamGridDB games matched. Try another exact title.".to_owned()
                } else {
                    format!("Found {count} possible games. Nothing is linked until you choose one.")
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("SteamGridDB search failed: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn choose_game(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(game) = self.game_at(index).cloned() else {
            return;
        };
        let database_id = i64::from(*self.as_ref().database_id());
        let kind = requested_kind(&self.as_ref().artwork_type().to_string());
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(format!(
            "Saving the reviewed link to {} and loading artwork…",
            game.name
        )));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-link".into())
            .spawn(move || {
                let result = (|| {
                    let client = Client::new(effective_api_key(String::new())?)?;
                    let artwork = client.artwork_for_game(game.id, kind)?;
                    SettingsStore::open_default()?.save_media_provider_game_link(
                        &MediaProviderGameLink {
                            provider: PROVIDER.to_owned(),
                            launchbox_db_id: database_id,
                            provider_game_id: game.id.to_string(),
                            provider_game_name: game.name.clone(),
                            reviewed_at: unix_timestamp(),
                        },
                    )?;
                    Ok((game, artwork))
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_choose_game(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start provider link: {error}")));
        }
    }

    fn finish_choose_game(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<(GameCandidate, Vec<ArtworkCandidate>), String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok((game, artwork)) => {
                let count = artwork.len();
                self.as_mut().rust_mut().selected_game_id = Some(game.id);
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut().rust_mut().games.clear();
                self.as_mut().set_selected_game_name(qstring(&game.name));
                self.as_mut().set_game_count(0);
                self.as_mut().set_artwork_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    format!(
                        "Linked to {}, but no safe static artwork exists in this category.",
                        game.name
                    )
                } else {
                    format!(
                        "Linked to {} by explicit review. Choose one of {count} artwork files.",
                        game.name
                    )
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save reviewed link: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn choose_artwork_kind(mut self: Pin<&mut Self>, artwork_type: QString) {
        if *self.as_ref().busy() {
            return;
        }
        let kind = requested_kind(&artwork_type.to_string());
        self.as_mut().set_artwork_type(qstring(kind.key()));
        let Some(game_id) = self.as_ref().rust().selected_game_id else {
            self.as_mut().bump_revision();
            return;
        };
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().rust_mut().artwork.clear();
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Loading artwork in the selected category…"));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-artwork".into())
            .spawn(move || {
                let result = effective_api_key(String::new())
                    .and_then(Client::new)
                    .and_then(|client| client.artwork_for_game(game_id, kind))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_artwork_load(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start artwork loading: {error}")));
        }
    }

    fn finish_artwork_load(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<Vec<ArtworkCandidate>, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(artwork) => {
                let count = artwork.len();
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut().set_artwork_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    "No safe static artwork exists in this category.".to_owned()
                } else {
                    format!("Choose one of {count} reviewed artwork files.")
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load artwork: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn publish_artwork(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() {
            return;
        }
        let Some(candidate) = self.artwork_at(index).cloned() else {
            return;
        };
        let database_id = i64::from(*self.as_ref().database_id());
        let kind = requested_kind(&self.as_ref().artwork_type().to_string());
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Downloading and validating the selected artwork…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-steamgriddb-publish".into())
            .spawn(move || {
                let result = effective_api_key(String::new())
                    .and_then(Client::new)
                    .and_then(|client| {
                        client.download_and_publish(
                            &candidate,
                            &crate::media::requested_media_directory(),
                            database_id,
                            kind,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_publish(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start artwork download: {error}"
            )));
        }
    }

    fn finish_publish(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<std::path::PathBuf, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(path) => {
                eprintln!(
                    "LUNCHBOX_STEAMGRIDDB_PUBLISHED database_id={} game={} path={}",
                    self.as_ref().database_id(),
                    self.as_ref().selected_game_name(),
                    path.display()
                );
                self.as_mut().set_message(qstring(format!(
                    "Saved reviewed SteamGridDB artwork to {}.",
                    path.display()
                )));
                let revision = self.as_ref().published_revision().wrapping_add(1);
                self.as_mut().set_published_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save artwork: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().set_busy(false);
    }

    pub fn game_name_at(&self, index: i32) -> QString {
        self.game_at(index)
            .map(|game| qstring(&game.name))
            .unwrap_or_default()
    }

    pub fn game_detail_at(&self, index: i32) -> QString {
        self.game_at(index)
            .map(|game| {
                let mut detail = format!("SteamGridDB ID {}", game.id);
                if game.verified {
                    detail.push_str(" · verified");
                }
                if !game.types.is_empty() {
                    detail.push_str(" · ");
                    detail.push_str(&game.types.join(", "));
                }
                qstring(detail)
            })
            .unwrap_or_default()
    }

    pub fn artwork_thumbnail_at(&self, index: i32) -> QUrl {
        self.artwork_at(index)
            .map(|artwork| QUrl::from(artwork.thumb.as_str()))
            .unwrap_or_default()
    }

    pub fn artwork_detail_at(&self, index: i32) -> QString {
        self.artwork_at(index)
            .map(|artwork| {
                let author = if artwork.author.name.trim().is_empty() {
                    "unknown artist"
                } else {
                    artwork.author.name.as_str()
                };
                let style = if artwork.style.trim().is_empty() {
                    String::new()
                } else {
                    format!(" · {}", artwork.style)
                };
                qstring(format!(
                    "ID {} · score {} · {}{}",
                    artwork.id, artwork.score, author, style
                ))
            })
            .unwrap_or_default()
    }

    fn game_at(&self, index: i32) -> Option<&GameCandidate> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().games.get(index))
    }

    fn artwork_at(&self, index: i32) -> Option<&ArtworkCandidate> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().artwork.get(index))
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}
