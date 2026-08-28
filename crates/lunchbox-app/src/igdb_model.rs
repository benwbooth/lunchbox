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
        #[qproperty(bool, credentials_saved)]
        #[qproperty(QString, message)]
        #[qproperty(QString, query)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(QString, selected_game_name)]
        #[qproperty(i32, database_id)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, artwork_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, published_revision)]
        type IgdbModel = super::IgdbModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut IgdbModel>);

        #[qinvokable]
        fn save_and_test_credentials(
            self: Pin<&mut IgdbModel>,
            client_id: QString,
            client_secret: QString,
        );

        #[qinvokable]
        fn test_connection(self: Pin<&mut IgdbModel>, client_id: QString, client_secret: QString);

        #[qinvokable]
        fn clear_credentials(self: Pin<&mut IgdbModel>);

        #[qinvokable]
        fn begin_selection(
            self: Pin<&mut IgdbModel>,
            database_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn search_games(self: Pin<&mut IgdbModel>, query: QString);

        #[qinvokable]
        fn choose_game(self: Pin<&mut IgdbModel>, index: i32);

        #[qinvokable]
        fn choose_artwork_kind(self: Pin<&mut IgdbModel>, artwork_type: QString);

        #[qinvokable]
        fn publish_artwork(self: Pin<&mut IgdbModel>, index: i32);

        #[qinvokable]
        fn cancel(self: Pin<&mut IgdbModel>);

        #[qinvokable]
        fn game_name_at(self: &IgdbModel, index: i32) -> QString;

        #[qinvokable]
        fn game_detail_at(self: &IgdbModel, index: i32) -> QString;

        #[qinvokable]
        fn artwork_thumbnail_at(self: &IgdbModel, index: i32) -> QUrl;

        #[qinvokable]
        fn artwork_detail_at(self: &IgdbModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for IgdbModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::igdb::{ArtworkCandidate, Client, GameCandidate};
use crate::media::ArtworkKind;
use crate::settings::{MediaProviderGameLink, SettingsStore};

const PROVIDER: &str = "igdb";

pub struct IgdbModelRust {
    initialized: bool,
    busy: bool,
    credentials_saved: bool,
    message: QString,
    query: QString,
    artwork_type: QString,
    selected_game_name: QString,
    database_id: i32,
    game_count: i32,
    artwork_count: i32,
    revision: i32,
    published_revision: i32,
    games: Vec<GameCandidate>,
    artwork: Vec<ArtworkCandidate>,
    selected_game_id: Option<i64>,
    generation: u64,
}

impl Default for IgdbModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            credentials_saved: false,
            message: qstring(
                "Add IGDB Twitch application credentials in Settings to browse artwork.",
            ),
            query: QString::default(),
            artwork_type: qstring("fanart"),
            selected_game_name: QString::default(),
            database_id: 0,
            game_count: 0,
            artwork_count: 0,
            revision: 0,
            published_revision: 0,
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

fn effective_credentials(
    entered_client_id: String,
    entered_client_secret: String,
) -> anyhow::Result<(String, String)> {
    let entered_client_id = entered_client_id.trim().to_owned();
    let entered_client_secret = entered_client_secret.trim().to_owned();
    if !entered_client_id.is_empty() || !entered_client_secret.is_empty() {
        if entered_client_id.is_empty() || entered_client_secret.is_empty() {
            anyhow::bail!("enter both the Twitch client ID and client secret");
        }
        return Ok((entered_client_id, entered_client_secret));
    }
    let environment_client_id = std::env::var("LUNCHBOX_IGDB_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let environment_client_secret = std::env::var("LUNCHBOX_IGDB_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty());
    if environment_client_id.is_some() || environment_client_secret.is_some() {
        return match (environment_client_id, environment_client_secret) {
            (Some(client_id), Some(client_secret)) => Ok((client_id, client_secret)),
            _ => anyhow::bail!(
                "both LUNCHBOX_IGDB_CLIENT_ID and LUNCHBOX_IGDB_CLIENT_SECRET must be set"
            ),
        };
    }
    crate::settings::load_igdb_credentials()?.ok_or_else(|| {
        anyhow::anyhow!("no IGDB Twitch credentials are saved; add them in Settings")
    })
}

fn requested_kind(value: &str) -> ArtworkKind {
    match ArtworkKind::parse(value) {
        Some(ArtworkKind::BoxFront) => ArtworkKind::BoxFront,
        Some(ArtworkKind::Screenshot) => ArtworkKind::Screenshot,
        _ => ArtworkKind::Fanart,
    }
}

impl qobject::IgdbModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-credentials".into())
            .spawn(move || {
                let result = crate::settings::load_igdb_credentials()
                    .map(|credentials| credentials.is_some())
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
                self.as_mut().set_credentials_saved(saved);
                self.as_mut().set_message(qstring(if saved {
                    "IGDB Twitch credentials are stored in the operating system credential store."
                } else {
                    "No IGDB Twitch credentials are saved. Register a confidential Twitch application to connect."
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not inspect the operating system credential store: {error}"
            ))),
        }
    }

    pub fn save_and_test_credentials(
        mut self: Pin<&mut Self>,
        client_id: QString,
        client_secret: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let client_id = client_id.to_string().trim().to_owned();
        let client_secret = client_secret.to_string().trim().to_owned();
        if client_id.is_empty() || client_secret.is_empty() {
            self.as_mut().set_message(qstring(
                "Enter both the Twitch client ID and client secret.",
            ));
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Testing IGDB before saving the credentials…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-save-credentials".into())
            .spawn(move || {
                let result = Client::new(client_id.clone(), client_secret.clone())
                    .and_then(|client| client.test_connection())
                    .and_then(|()| {
                        crate::settings::save_igdb_credentials(&client_id, &client_secret)
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, true);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the IGDB credential test: {error}"
            )));
        }
    }

    pub fn test_connection(mut self: Pin<&mut Self>, client_id: QString, client_secret: QString) {
        if *self.as_ref().busy() {
            return;
        }
        let client_id = client_id.to_string();
        let client_secret = client_secret.to_string();
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Testing the IGDB connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-test".into())
            .spawn(move || {
                let result = effective_credentials(client_id, client_secret)
                    .and_then(|(client_id, client_secret)| Client::new(client_id, client_secret))
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
                    self.as_mut().set_credentials_saved(true);
                }
                self.as_mut().set_message(qstring(if saved {
                    "IGDB connection succeeded. The credentials are saved securely."
                } else {
                    "IGDB connection succeeded."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("IGDB connection failed: {error}"))),
        }
    }

    pub fn clear_credentials(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-clear-credentials".into())
            .spawn(move || {
                let result = crate::settings::save_igdb_credentials("", "")
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(()) => {
                            model.as_mut().set_credentials_saved(false);
                            model.as_mut().set_message(qstring(
                                "IGDB Twitch credentials removed from the credential store.",
                            ));
                        }
                        Err(error) => model.as_mut().set_message(qstring(format!(
                            "Could not remove IGDB Twitch credentials: {error}"
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
        _platform: QString,
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
        let kind = requested_kind(&artwork_type.to_string());
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
            "Checking reviewed IGDB artwork for {title}…"
        )));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-begin".into())
            .spawn(move || {
                let result = (|| {
                    let (client_id, client_secret) =
                        effective_credentials(String::new(), String::new())?;
                    let client = Client::new(client_id, client_secret)?;
                    let store = SettingsStore::open_default()?;
                    if let Some(link) =
                        store.media_provider_game_link(PROVIDER, i64::from(database_id))?
                    {
                        let game_id = link
                            .provider_game_id
                            .parse::<i64>()
                            .map_err(|_| anyhow::anyhow!("saved IGDB game ID is invalid"))?;
                        let artwork = client.artwork_for_game(game_id, kind)?;
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
                self.as_mut().rust_mut().selected_game_id =
                    link.provider_game_id.parse::<i64>().ok();
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut()
                    .set_selected_game_name(qstring(&link.provider_game_name));
                self.as_mut().set_artwork_count(saturating_i32(count));
                let artwork_type = self.as_ref().artwork_type().to_string();
                self.as_mut().set_message(qstring(if count == 0 {
                    format!(
                        "The reviewed IGDB link is {}, but it has no {} artwork.",
                        link.provider_game_name, artwork_type
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
                    "No IGDB games matched. Change the search text and try again.".to_owned()
                } else {
                    format!(
                        "Found {count} possible games. Choose the exact record before browsing artwork."
                    )
                }));
            }
            Err(error) => {
                eprintln!("LUNCHBOX_IGDB_ERROR phase=begin error={error}");
                self.as_mut()
                    .set_message(qstring(format!("IGDB search failed: {error}")));
            }
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
            .set_message(qstring(format!("Searching IGDB for “{query}”…")));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-search".into())
            .spawn(move || {
                let result = effective_credentials(String::new(), String::new())
                    .and_then(|(client_id, client_secret)| Client::new(client_id, client_secret))
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
                    "No IGDB games matched. Try another exact title.".to_owned()
                } else {
                    format!("Found {count} possible games. Nothing is linked until you choose one.")
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("IGDB search failed: {error}"))),
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
            .name("lunchbox-igdb-link".into())
            .spawn(move || {
                let result = (|| {
                    let (client_id, client_secret) =
                        effective_credentials(String::new(), String::new())?;
                    let client = Client::new(client_id, client_secret)?;
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
                    format!("Linked to {}, but this category has no artwork.", game.name)
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
            .set_message(qstring("Loading IGDB artwork in the selected category…"));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-artwork".into())
            .spawn(move || {
                let result = effective_credentials(String::new(), String::new())
                    .and_then(|(client_id, client_secret)| Client::new(client_id, client_secret))
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
                    "No IGDB artwork exists in this category.".to_owned()
                } else {
                    format!("Choose one of {count} reviewed artwork files.")
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not load IGDB artwork: {error}"))),
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
        self.as_mut().set_message(qstring(
            "Downloading and validating the selected IGDB artwork…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-igdb-publish".into())
            .spawn(move || {
                let result = effective_credentials(String::new(), String::new())
                    .and_then(|(client_id, client_secret)| Client::new(client_id, client_secret))
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
                "Could not start the IGDB artwork download: {error}"
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
                    "LUNCHBOX_IGDB_PUBLISHED database_id={} game={} path={}",
                    self.as_ref().database_id(),
                    self.as_ref().selected_game_name(),
                    path.display()
                );
                self.as_mut().set_message(qstring(format!(
                    "Saved reviewed IGDB artwork to {}.",
                    path.display()
                )));
                let revision = self.as_ref().published_revision().wrapping_add(1);
                self.as_mut().set_published_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save IGDB artwork: {error}"))),
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
                let mut parts = vec![format!("IGDB ID {}", game.id)];
                if let Some(timestamp) = game.first_release_date {
                    parts.push(release_year(timestamp).to_string());
                }
                let platforms: Vec<&str> = game
                    .platforms
                    .iter()
                    .filter_map(|platform| {
                        if !platform.abbreviation.trim().is_empty() {
                            Some(platform.abbreviation.as_str())
                        } else if !platform.name.trim().is_empty() {
                            Some(platform.name.as_str())
                        } else {
                            None
                        }
                    })
                    .take(5)
                    .collect();
                if !platforms.is_empty() {
                    parts.push(platforms.join(", "));
                }
                qstring(parts.join(" · "))
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
                let dimensions = if artwork.width > 0 && artwork.height > 0 {
                    format!(" · {}×{}", artwork.width, artwork.height)
                } else {
                    String::new()
                };
                qstring(format!("{} · {}{}", artwork.id, artwork.source, dimensions))
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

fn release_year(timestamp: i64) -> i64 {
    let days = timestamp.div_euclid(86_400) + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 }.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    year + i64::from(month_prime >= 10)
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_year_handles_dates_before_and_after_unix_epoch() {
        assert_eq!(release_year(0), 1970);
        assert_eq!(release_year(499_132_800), 1985);
        assert_eq!(release_year(-315_619_200), 1960);
    }

    #[test]
    fn igdb_artwork_categories_never_masquerade_as_unsupported_media() {
        assert_eq!(requested_kind("box-front"), ArtworkKind::BoxFront);
        assert_eq!(requested_kind("screenshot"), ArtworkKind::Screenshot);
        assert_eq!(requested_kind("clear-logo"), ArtworkKind::Fanart);
    }
}
