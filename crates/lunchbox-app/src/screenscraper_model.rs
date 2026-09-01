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
        type ScreenScraperModel = super::ScreenScraperModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut ScreenScraperModel>);

        #[qinvokable]
        fn save_and_test_credentials(
            self: Pin<&mut ScreenScraperModel>,
            developer_id: QString,
            developer_password: QString,
            user_id: QString,
            user_password: QString,
        );

        #[qinvokable]
        fn test_connection(
            self: Pin<&mut ScreenScraperModel>,
            developer_id: QString,
            developer_password: QString,
            user_id: QString,
            user_password: QString,
        );

        #[qinvokable]
        fn clear_credentials(self: Pin<&mut ScreenScraperModel>);

        #[qinvokable]
        fn begin_selection(
            self: Pin<&mut ScreenScraperModel>,
            database_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn search_games(self: Pin<&mut ScreenScraperModel>, query: QString);

        #[qinvokable]
        fn choose_game(self: Pin<&mut ScreenScraperModel>, index: i32);

        #[qinvokable]
        fn choose_artwork_kind(self: Pin<&mut ScreenScraperModel>, artwork_type: QString);

        #[qinvokable]
        fn publish_artwork(self: Pin<&mut ScreenScraperModel>, index: i32);

        #[qinvokable]
        fn cancel(self: Pin<&mut ScreenScraperModel>);

        #[qinvokable]
        fn game_name_at(self: &ScreenScraperModel, index: i32) -> QString;

        #[qinvokable]
        fn game_detail_at(self: &ScreenScraperModel, index: i32) -> QString;

        #[qinvokable]
        fn artwork_thumbnail_at(self: &ScreenScraperModel, index: i32) -> QUrl;

        #[qinvokable]
        fn artwork_detail_at(self: &ScreenScraperModel, index: i32) -> QString;

        #[qinvokable]
        fn selected_metadata_value(self: &ScreenScraperModel, field: QString) -> QString;
    }

    impl cxx_qt::Threading for ScreenScraperModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::media::ArtworkKind;
use crate::screenscraper::{
    ArtworkCandidate, Client, GameCandidate, artwork_candidates, configured_credentials,
    effective_credentials,
};
use crate::settings::{MediaProviderGameLink, ScreenScraperCredentials, SettingsStore};

const PROVIDER: &str = "screenscraper";

pub struct ScreenScraperModelRust {
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
    platform: String,
    games: Vec<GameCandidate>,
    artwork: Vec<ArtworkCandidate>,
    selected_game: Option<GameCandidate>,
    generation: u64,
}

impl Default for ScreenScraperModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            credentials_saved: false,
            message: qstring(
                "Add authorized ScreenScraper developer credentials in Settings to browse its optional media catalog.",
            ),
            query: QString::default(),
            artwork_type: qstring("fanart"),
            selected_game_name: QString::default(),
            database_id: 0,
            game_count: 0,
            artwork_count: 0,
            revision: 0,
            published_revision: 0,
            platform: String::new(),
            games: Vec::new(),
            artwork: Vec::new(),
            selected_game: None,
            generation: 0,
        }
    }
}

enum BeginResult {
    Linked {
        link: MediaProviderGameLink,
        game: GameCandidate,
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

fn requested_kind(value: &str) -> ArtworkKind {
    ArtworkKind::parse(value).unwrap_or(ArtworkKind::Fanart)
}

fn configured_credentials_available() -> anyhow::Result<bool> {
    Ok(configured_credentials()?.is_some())
}

impl qobject::ScreenScraperModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-credentials".into())
            .spawn(move || {
                let result = configured_credentials_available().map_err(|error| error.to_string());
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
                    "ScreenScraper credentials are available for this session."
                } else {
                    "ScreenScraper is optional. Its API requires developer approval before credentials are issued."
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not inspect the operating system credential store: {error}"
            ))),
        }
    }

    pub fn save_and_test_credentials(
        mut self: Pin<&mut Self>,
        developer_id: QString,
        developer_password: QString,
        user_id: QString,
        user_password: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let developer_id = developer_id.to_string();
        let developer_password = developer_password.to_string();
        let user_id = user_id.to_string();
        let user_password = user_password.to_string();
        let credentials = match ScreenScraperCredentials::validated(
            &developer_id,
            &developer_password,
            &user_id,
            &user_password,
        ) {
            Ok(credentials) => credentials,
            Err(error) => {
                self.as_mut().set_message(qstring(error.to_string()));
                return;
            }
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Testing ScreenScraper before saving the credentials…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-save-credentials".into())
            .spawn(move || {
                let result = Client::new(credentials)
                    .and_then(|client| client.test_connection())
                    .and_then(|()| {
                        crate::settings::save_screenscraper_credentials(
                            &developer_id,
                            &developer_password,
                            &user_id,
                            &user_password,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, true);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the ScreenScraper credential test: {error}"
            )));
        }
    }

    pub fn test_connection(
        mut self: Pin<&mut Self>,
        developer_id: QString,
        developer_password: QString,
        user_id: QString,
        user_password: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Testing the ScreenScraper connection…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-test".into())
            .spawn(move || {
                let developer_id = developer_id.to_string();
                let developer_password = developer_password.to_string();
                let user_id = user_id.to_string();
                let user_password = user_password.to_string();
                let result = effective_credentials(
                    &developer_id,
                    &developer_password,
                    &user_id,
                    &user_password,
                )
                .and_then(Client::new)
                .and_then(|client| client.test_connection())
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, false);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the ScreenScraper connection test: {error}"
            )));
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
                    "ScreenScraper connection succeeded. The credentials are saved securely."
                } else {
                    "ScreenScraper connection succeeded."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("ScreenScraper connection failed: {error}"))),
        }
    }

    pub fn clear_credentials(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-clear-credentials".into())
            .spawn(move || {
                let result = crate::settings::save_screenscraper_credentials("", "", "", "")
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(()) => {
                            model.as_mut().set_credentials_saved(false);
                            model.as_mut().set_message(qstring(
                                "ScreenScraper credentials removed from the credential store.",
                            ));
                        }
                        Err(error) => model.as_mut().set_message(qstring(format!(
                            "Could not remove ScreenScraper credentials: {error}"
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
        self.as_mut().rust_mut().games.clear();
        self.as_mut().rust_mut().artwork.clear();
        self.as_mut().rust_mut().selected_game = None;
        self.as_mut().rust_mut().platform = platform.clone();
        self.as_mut().set_database_id(database_id);
        self.as_mut().set_query(qstring(&title));
        self.as_mut().set_artwork_type(qstring(kind.key()));
        self.as_mut().set_selected_game_name(QString::default());
        self.as_mut().set_game_count(0);
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Loading the reviewed ScreenScraper link or searching for candidates…",
        ));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-begin".into())
            .spawn(move || {
                let result = (|| {
                    let client = Client::new(effective_credentials("", "", "", "")?)?;
                    let store = SettingsStore::open_default()?;
                    if let Some(link) =
                        store.media_provider_game_link(PROVIDER, i64::from(database_id))?
                    {
                        let provider_id = link.provider_game_id.parse::<i64>().map_err(|_| {
                            anyhow::anyhow!("saved ScreenScraper game ID is invalid")
                        })?;
                        let game = client.game(provider_id)?;
                        let artwork = client.artwork_for_game(&game, kind)?;
                        Ok(BeginResult::Linked {
                            link,
                            game,
                            artwork,
                        })
                    } else {
                        Ok(BeginResult::Search(client.search_games(&title, &platform)?))
                    }
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_begin_selection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start ScreenScraper selection: {error}"
            )));
        }
    }

    fn finish_begin_selection(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<BeginResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(BeginResult::Linked {
                link,
                game,
                artwork,
            }) => {
                let count = artwork.len();
                self.as_mut().rust_mut().selected_game = Some(game);
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut()
                    .set_selected_game_name(qstring(&link.provider_game_name));
                self.as_mut().set_artwork_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    format!(
                        "The reviewed link {} has no safe artwork in this category.",
                        link.provider_game_name
                    )
                } else {
                    format!(
                        "Showing {count} artwork choices for the reviewed link {}.",
                        link.provider_game_name
                    )
                }));
            }
            Ok(BeginResult::Search(games)) => self.as_mut().publish_search_results(games),
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("ScreenScraper search failed: {error}"))),
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
        let platform = self.as_ref().rust().platform.clone();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().rust_mut().games.clear();
        self.as_mut().rust_mut().artwork.clear();
        self.as_mut().rust_mut().selected_game = None;
        self.as_mut().set_query(qstring(&query));
        self.as_mut().set_selected_game_name(QString::default());
        self.as_mut().set_game_count(0);
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring(format!("Searching ScreenScraper for “{query}”…")));
        self.as_mut().bump_revision();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-search".into())
            .spawn(move || {
                let result = effective_credentials("", "", "", "")
                    .and_then(Client::new)
                    .and_then(|client| client.search_games(&query, &platform))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(games) => model.as_mut().publish_search_results(games),
                        Err(error) => model
                            .as_mut()
                            .set_message(qstring(format!("ScreenScraper search failed: {error}"))),
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start game search: {error}")));
        }
    }

    fn publish_search_results(mut self: Pin<&mut Self>, games: Vec<GameCandidate>) {
        let count = games.len();
        self.as_mut().rust_mut().games = games;
        self.as_mut().set_game_count(saturating_i32(count));
        self.as_mut().set_message(qstring(if count == 0 {
            "No ScreenScraper games matched. Try another exact title.".to_owned()
        } else {
            format!("Found {count} possible games. Nothing is linked until you choose one.")
        }));
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
            .name("lunchbox-screenscraper-link".into())
            .spawn(move || {
                let result = (|| {
                    let client = Client::new(effective_credentials("", "", "", "")?)?;
                    let full_game = client.game(game.id)?;
                    let artwork = client.artwork_for_game(&full_game, kind)?;
                    SettingsStore::open_default()?.save_media_provider_game_link(
                        &MediaProviderGameLink {
                            provider: PROVIDER.to_owned(),
                            launchbox_db_id: database_id,
                            provider_game_id: full_game.id.to_string(),
                            provider_game_name: full_game.name.clone(),
                            reviewed_at: unix_timestamp(),
                        },
                    )?;
                    Ok((full_game, artwork))
                })()
                .map_err(|error: anyhow::Error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok((game, artwork)) => {
                            let count = artwork.len();
                            model.as_mut().rust_mut().selected_game = Some(game.clone());
                            model.as_mut().rust_mut().artwork = artwork;
                            model.as_mut().rust_mut().games.clear();
                            model.as_mut().set_selected_game_name(qstring(&game.name));
                            model.as_mut().set_game_count(0);
                            model.as_mut().set_artwork_count(saturating_i32(count));
                            model.as_mut().set_message(qstring(if count == 0 {
                                format!(
                                    "Linked to {}, but no safe artwork exists in this category.",
                                    game.name
                                )
                            } else {
                                format!(
                                    "Linked to {} by explicit review. Choose one of {count} artwork files.",
                                    game.name
                                )
                            }));
                        }
                        Err(error) => model.as_mut().set_message(qstring(format!(
                            "Could not save reviewed link: {error}"
                        ))),
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start provider link: {error}")));
        }
    }

    pub fn choose_artwork_kind(mut self: Pin<&mut Self>, artwork_type: QString) {
        if *self.as_ref().busy() {
            return;
        }
        let kind = requested_kind(&artwork_type.to_string());
        self.as_mut().set_artwork_type(qstring(kind.key()));
        let Some(game) = self.as_ref().rust().selected_game.clone() else {
            self.as_mut().bump_revision();
            return;
        };
        match artwork_candidates(&game, kind) {
            Ok(artwork) => {
                let count = artwork.len();
                self.as_mut().rust_mut().artwork = artwork;
                self.as_mut().set_artwork_count(saturating_i32(count));
                self.as_mut().set_message(qstring(if count == 0 {
                    "No safe artwork exists in this category.".to_owned()
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
        self.as_mut().set_message(qstring(
            "Downloading and validating the selected ScreenScraper artwork…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-screenscraper-publish".into())
            .spawn(move || {
                let result = effective_credentials("", "", "", "")
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
                    if generation != model.as_ref().rust().generation {
                        return;
                    }
                    model.as_mut().set_busy(false);
                    match result {
                        Ok(path) => {
                            eprintln!(
                                "LUNCHBOX_SCREENSCRAPER_PUBLISHED database_id={} game={} path={}",
                                model.as_ref().database_id(),
                                model.as_ref().selected_game_name(),
                                path.display()
                            );
                            model.as_mut().set_message(qstring(format!(
                                "Saved reviewed ScreenScraper artwork to {}.",
                                path.display()
                            )));
                            let revision = model.as_ref().published_revision().wrapping_add(1);
                            model.as_mut().set_published_revision(revision);
                        }
                        Err(error) => model
                            .as_mut()
                            .set_message(qstring(format!("Could not save artwork: {error}"))),
                    }
                    model.as_mut().bump_revision();
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start artwork download: {error}"
            )));
        }
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
                let mut detail = format!("ScreenScraper ID {}", game.id);
                if !game.release_date.is_empty() {
                    detail.push_str(" · ");
                    detail.push_str(&game.release_date);
                }
                if !game.developer.is_empty() {
                    detail.push_str(" · ");
                    detail.push_str(&game.developer);
                } else if !game.publisher.is_empty() {
                    detail.push_str(" · ");
                    detail.push_str(&game.publisher);
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
                qstring(format!(
                    "{} · {}",
                    artwork.media_type,
                    if artwork.region.trim().is_empty() {
                        "region-neutral"
                    } else {
                        artwork.region.as_str()
                    }
                ))
            })
            .unwrap_or_default()
    }

    pub fn selected_metadata_value(&self, field: QString) -> QString {
        self.rust()
            .selected_game
            .as_ref()
            .map(|game| qstring(game.metadata_value(&field.to_string())))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_artwork_categories_preserve_screenscraper_capabilities() {
        assert_eq!(requested_kind("box-front"), ArtworkKind::BoxFront);
        assert_eq!(requested_kind("box-back"), ArtworkKind::BoxBack);
        assert_eq!(requested_kind("screenshot"), ArtworkKind::Screenshot);
        assert_eq!(requested_kind("title-screen"), ArtworkKind::TitleScreen);
        assert_eq!(requested_kind("clear-logo"), ArtworkKind::ClearLogo);
        assert_eq!(requested_kind("unsupported"), ArtworkKind::Fanart);
    }
}
