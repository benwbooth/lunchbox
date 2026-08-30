#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, initialized)]
        #[qproperty(bool, busy)]
        #[qproperty(bool, credentials_saved)]
        #[qproperty(QString, message)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(QString, selected_game_name)]
        #[qproperty(QString, last_media_kind)]
        #[qproperty(i32, database_id)]
        #[qproperty(i32, published_revision)]
        #[qproperty(i32, transfer_progress)]
        #[qproperty(bool, cancel_requested)]
        type EmuMoviesModel = super::EmuMoviesModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut EmuMoviesModel>);

        #[qinvokable]
        fn save_and_test_credentials(
            self: Pin<&mut EmuMoviesModel>,
            username: QString,
            password: QString,
        );

        #[qinvokable]
        fn test_connection(self: Pin<&mut EmuMoviesModel>, username: QString, password: QString);

        #[qinvokable]
        fn clear_credentials(self: Pin<&mut EmuMoviesModel>);

        #[qinvokable]
        fn begin_selection(
            self: Pin<&mut EmuMoviesModel>,
            database_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn choose_artwork_kind(self: Pin<&mut EmuMoviesModel>, artwork_type: QString);

        #[qinvokable]
        fn download_artwork(self: Pin<&mut EmuMoviesModel>);

        #[qinvokable]
        fn download_video(
            self: Pin<&mut EmuMoviesModel>,
            game_id: QString,
            database_id: i32,
            title: QString,
            platform: QString,
        );

        #[qinvokable]
        fn download_manual(
            self: Pin<&mut EmuMoviesModel>,
            game_id: QString,
            database_id: i32,
            title: QString,
            platform: QString,
        );

        #[qinvokable]
        fn cancel(self: Pin<&mut EmuMoviesModel>);
    }

    impl cxx_qt::Threading for EmuMoviesModel {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;

use crate::emumovies::{
    EmuMoviesClient, EmuMoviesConfig, EmuMoviesMediaType, ProgressCallback, transfer_was_cancelled,
};

pub struct EmuMoviesModelRust {
    initialized: bool,
    busy: bool,
    credentials_saved: bool,
    message: QString,
    artwork_type: QString,
    selected_game_name: QString,
    last_media_kind: QString,
    database_id: i32,
    published_revision: i32,
    transfer_progress: i32,
    cancel_requested: bool,
    title: String,
    platform: String,
    generation: u64,
    cancel_token: Option<Arc<AtomicBool>>,
}

impl Default for EmuMoviesModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            credentials_saved: false,
            message: qstring(
                "Add your EmuMovies forum credentials in Settings to use the legacy FTP media library.",
            ),
            artwork_type: qstring("fanart"),
            selected_game_name: QString::default(),
            last_media_kind: QString::default(),
            database_id: 0,
            published_revision: 0,
            transfer_progress: -1,
            cancel_requested: false,
            title: String::new(),
            platform: String::new(),
            generation: 0,
            cancel_token: None,
        }
    }
}

enum DownloadRequest {
    Artwork {
        database_id: i64,
        title: String,
        platform: String,
        media_type: EmuMoviesMediaType,
    },
    Supplemental {
        game_id: String,
        database_id: i64,
        title: String,
        platform: String,
        media_type: EmuMoviesMediaType,
    },
}

impl DownloadRequest {
    fn kind(&self) -> &'static str {
        match self {
            Self::Artwork { .. } => "artwork",
            Self::Supplemental { media_type, .. } if *media_type == EmuMoviesMediaType::Video => {
                "video"
            }
            Self::Supplemental { .. } => "manual",
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn effective_credentials(
    entered_username: String,
    entered_password: String,
) -> Result<(String, String)> {
    let entered_username = entered_username.trim().to_owned();
    let entered_password = entered_password.trim().to_owned();
    if !entered_username.is_empty() || !entered_password.is_empty() {
        if entered_username.is_empty() || entered_password.is_empty() {
            bail!("enter both the EmuMovies forum username and password");
        }
        return Ok((entered_username, entered_password));
    }

    let environment_username = std::env::var("LUNCHBOX_EMUMOVIES_USERNAME")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let environment_password = std::env::var("LUNCHBOX_EMUMOVIES_PASSWORD")
        .ok()
        .filter(|value| !value.trim().is_empty());
    if environment_username.is_some() || environment_password.is_some() {
        return match (environment_username, environment_password) {
            (Some(username), Some(password)) => Ok((username, password)),
            _ => bail!(
                "both LUNCHBOX_EMUMOVIES_USERNAME and LUNCHBOX_EMUMOVIES_PASSWORD must be set"
            ),
        };
    }

    crate::settings::load_emumovies_credentials()?
        .ok_or_else(|| anyhow::anyhow!("no EmuMovies credentials are saved; add them in Settings"))
}

fn client(username: String, password: String) -> EmuMoviesClient {
    EmuMoviesClient::new(
        EmuMoviesConfig { username, password },
        crate::media::requested_media_directory(),
    )
}

fn artwork_media_type(value: &str) -> Option<EmuMoviesMediaType> {
    match value {
        "box-front" => Some(EmuMoviesMediaType::BoxFront),
        "box-back" => Some(EmuMoviesMediaType::BoxBack),
        "box-3d" => Some(EmuMoviesMediaType::Box3D),
        "screenshot" => Some(EmuMoviesMediaType::Screenshot),
        "title-screen" => Some(EmuMoviesMediaType::TitleScreen),
        "fanart" => Some(EmuMoviesMediaType::Fanart),
        "clear-logo" => Some(EmuMoviesMediaType::ClearLogo),
        _ => None,
    }
}

fn execute_download(
    request: DownloadRequest,
    progress: Option<&ProgressCallback>,
) -> Result<(PathBuf, &'static str)> {
    let (username, password) = effective_credentials(String::new(), String::new())?;
    let client = client(username, password);
    match request {
        DownloadRequest::Artwork {
            database_id,
            title,
            platform,
            media_type,
        } => {
            if database_id <= 0 {
                bail!("EmuMovies artwork requires a positive catalog database ID");
            }
            let game_directory =
                crate::media::requested_media_directory().join(format!("lb-{database_id}"));
            let path = client.get_media_from_archive(
                &platform,
                media_type,
                &title,
                &game_directory,
                progress,
            )?;
            Ok((path, "artwork"))
        }
        DownloadRequest::Supplemental {
            game_id,
            database_id,
            title,
            platform,
            media_type,
        } => {
            let game_directory = crate::media::game_media_directory(&game_id, database_id)?;
            let path = if media_type == EmuMoviesMediaType::Video {
                client.get_video(&platform, &title, &game_directory, progress)?
            } else {
                client.get_manual(&platform, &title, &game_directory, progress)?
            };
            let kind = if media_type == EmuMoviesMediaType::Video {
                "video"
            } else {
                "manual"
            };
            Ok((path, kind))
        }
    }
}

pub(crate) fn download_saved_video(
    game_id: &str,
    database_id: i64,
    title: &str,
    platform: &str,
    progress: Option<crate::emumovies::ProgressCallback>,
) -> Result<PathBuf> {
    let (username, password) = effective_credentials(String::new(), String::new())?;
    let client = client(username, password);
    let game_directory = crate::media::game_media_directory(game_id, database_id)?;
    client.get_video(platform, title, &game_directory, progress.as_ref())
}

impl qobject::EmuMoviesModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() || *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emumovies-credentials".into())
            .spawn(move || {
                let result = crate::settings::load_emumovies_credentials()
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
                    "EmuMovies forum credentials are stored in the operating system credential store."
                } else {
                    "No EmuMovies credentials are saved. Existing members can use their forum login for FTP access."
                }));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not inspect the operating system credential store: {error}"
            ))),
        }
    }

    pub fn save_and_test_credentials(
        mut self: Pin<&mut Self>,
        username: QString,
        password: QString,
    ) {
        self.as_mut()
            .start_credential_action(username.to_string(), password.to_string(), true);
    }

    pub fn test_connection(mut self: Pin<&mut Self>, username: QString, password: QString) {
        self.as_mut()
            .start_credential_action(username.to_string(), password.to_string(), false);
    }

    fn start_credential_action(
        mut self: Pin<&mut Self>,
        username: String,
        password: String,
        save: bool,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_transfer_progress(-1);
        self.as_mut().set_cancel_requested(false);
        self.as_mut()
            .set_message(qstring("Connecting to the EmuMovies FTP library…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emumovies-connection".into())
            .spawn(move || {
                let result = effective_credentials(username, password)
                    .and_then(|(username, password)| {
                        client(username.clone(), password.clone()).test_connection()?;
                        if save {
                            crate::settings::save_emumovies_credentials(&username, &password)?;
                        }
                        Ok(())
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_credential_action(result, save);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the EmuMovies connection test: {error}"
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
                    "EmuMovies FTP connection succeeded. Automatic gameplay-video downloads are enabled."
                } else {
                    "EmuMovies FTP connection succeeded, but these credentials were not saved. Choose Save & enable automatic media to use automatic gameplay videos."
                }));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("EmuMovies FTP connection failed: {error}"))),
        }
    }

    pub fn clear_credentials(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        match crate::settings::save_emumovies_credentials("", "") {
            Ok(()) => {
                self.as_mut().set_credentials_saved(false);
                self.as_mut().set_message(qstring(
                    "EmuMovies credentials removed from the operating system credential store.",
                ));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not remove EmuMovies credentials: {error}"
            ))),
        }
    }

    pub fn begin_selection(
        mut self: Pin<&mut Self>,
        database_id: i32,
        title: QString,
        platform: QString,
        artwork_type: QString,
    ) {
        if let Some(cancel_token) = self.as_ref().rust().cancel_token.clone() {
            cancel_token.store(true, Ordering::Release);
            self.as_mut().set_cancel_requested(true);
            self.as_mut().set_message(qstring(
                "Cancelling the active EmuMovies transfer before changing games…",
            ));
            return;
        }
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().set_database_id(database_id);
        self.as_mut().rust_mut().title = title.to_string();
        self.as_mut().rust_mut().platform = platform.to_string();
        let selected_title = self.as_ref().rust().title.clone();
        self.as_mut()
            .set_selected_game_name(qstring(selected_title));
        self.as_mut().choose_artwork_kind(artwork_type);
        self.as_mut().set_message(qstring(
            "EmuMovies uses the legacy platform and title matcher. Choose a media category, then download the exact cache candidate.",
        ));
    }

    pub fn choose_artwork_kind(mut self: Pin<&mut Self>, artwork_type: QString) {
        let artwork_type = artwork_type.to_string();
        if artwork_media_type(&artwork_type).is_none() {
            self.as_mut().set_message(qstring(
                "That artwork category is not supported by EmuMovies.",
            ));
            return;
        }
        self.as_mut().set_artwork_type(qstring(artwork_type));
    }

    pub fn download_artwork(mut self: Pin<&mut Self>) {
        let Some(media_type) = artwork_media_type(&self.as_ref().artwork_type().to_string()) else {
            self.as_mut()
                .set_message(qstring("Choose a supported EmuMovies artwork category."));
            return;
        };
        let request = DownloadRequest::Artwork {
            database_id: i64::from(*self.as_ref().database_id()),
            title: self.as_ref().rust().title.clone(),
            platform: self.as_ref().rust().platform.clone(),
            media_type,
        };
        self.as_mut().start_download(request);
    }

    pub fn download_video(
        mut self: Pin<&mut Self>,
        game_id: QString,
        database_id: i32,
        title: QString,
        platform: QString,
    ) {
        self.as_mut().start_download(DownloadRequest::Supplemental {
            game_id: game_id.to_string(),
            database_id: i64::from(database_id),
            title: title.to_string(),
            platform: platform.to_string(),
            media_type: EmuMoviesMediaType::Video,
        });
    }

    pub fn download_manual(
        mut self: Pin<&mut Self>,
        game_id: QString,
        database_id: i32,
        title: QString,
        platform: QString,
    ) {
        self.as_mut().start_download(DownloadRequest::Supplemental {
            game_id: game_id.to_string(),
            database_id: i64::from(database_id),
            title: title.to_string(),
            platform: platform.to_string(),
            media_type: EmuMoviesMediaType::Manual,
        });
    }

    fn start_download(mut self: Pin<&mut Self>, request: DownloadRequest) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().set_last_media_kind(qstring(request.kind()));
        let generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().generation = generation;
        let cancel_token = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel_token = Some(Arc::clone(&cancel_token));
        self.as_mut().set_busy(true);
        self.as_mut().set_cancel_requested(false);
        self.as_mut().set_transfer_progress(0);
        self.as_mut().set_message(qstring(
            "Searching and downloading from the EmuMovies FTP library…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let progress_thread = qt_thread.clone();
        let progress_state = Arc::new(Mutex::new((
            Instant::now() - Duration::from_secs(1),
            -1_i32,
        )));
        let progress_state_for_worker = Arc::clone(&progress_state);
        let cancel_token_for_worker = Arc::clone(&cancel_token);
        let progress: ProgressCallback = Box::new(move |value| {
            if cancel_token_for_worker.load(Ordering::Acquire) {
                return false;
            }

            let percent = (value * 100.0).round().clamp(0.0, 100.0) as i32;
            let Ok(mut state) = progress_state_for_worker.lock() else {
                return !cancel_token_for_worker.load(Ordering::Acquire);
            };
            let elapsed = state.0.elapsed();
            if percent != 100 && (percent == state.1 || elapsed < Duration::from_millis(120)) {
                return !cancel_token_for_worker.load(Ordering::Acquire);
            }
            state.0 = Instant::now();
            state.1 = percent;
            let _ = progress_thread.queue(move |mut model| {
                model.as_mut().update_transfer_progress(generation, percent);
            });
            !cancel_token_for_worker.load(Ordering::Acquire)
        });
        let spawn = std::thread::Builder::new()
            .name("lunchbox-emumovies-download".into())
            .spawn(move || {
                let result = execute_download(request, Some(&progress))
                    .map(|(path, kind)| (path.to_string_lossy().into_owned(), kind))
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_download(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().rust_mut().cancel_token = None;
            self.as_mut().set_busy(false);
            self.as_mut().set_cancel_requested(false);
            self.as_mut().set_transfer_progress(-1);
            self.as_mut().set_message(qstring(format!(
                "Could not start the EmuMovies download: {error}"
            )));
        }
    }

    fn update_transfer_progress(mut self: Pin<&mut Self>, generation: u64, percent: i32) {
        if generation == self.as_ref().rust().generation
            && *self.as_ref().busy()
            && !*self.as_ref().cancel_requested()
        {
            self.as_mut().set_transfer_progress(percent.clamp(0, 100));
        }
    }

    fn finish_download(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<(String, &'static str), String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().rust_mut().cancel_token = None;
        self.as_mut().set_busy(false);
        self.as_mut().set_cancel_requested(false);
        match result {
            Ok((path, kind)) => {
                self.as_mut().set_transfer_progress(100);
                self.as_mut().set_last_media_kind(qstring(kind));
                let revision = self.as_ref().published_revision().wrapping_add(1);
                self.as_mut().set_published_revision(revision);
                self.as_mut()
                    .set_message(qstring(format!("Downloaded EmuMovies {kind} to {path}.")));
            }
            Err(error) if transfer_was_cancelled(&error) => {
                self.as_mut().set_transfer_progress(-1);
                self.as_mut().set_message(qstring(
                    "EmuMovies transfer cancelled. No partial media was kept.",
                ));
            }
            Err(error) => {
                self.as_mut().set_transfer_progress(-1);
                self.as_mut()
                    .set_message(qstring(format!("EmuMovies download failed: {error}")));
            }
        }
    }

    pub fn cancel(mut self: Pin<&mut Self>) {
        let cancel_token = self.as_ref().rust().cancel_token.clone();
        if *self.as_ref().busy()
            && let Some(cancel_token) = cancel_token
        {
            cancel_token.store(true, Ordering::Release);
            self.as_mut().set_cancel_requested(true);
            self.as_mut()
                .set_message(qstring("Cancelling the EmuMovies FTP transfer…"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_native_artwork_choice_maps_to_legacy_emumovies_media() {
        for kind in [
            "box-front",
            "box-back",
            "box-3d",
            "screenshot",
            "title-screen",
            "fanart",
            "clear-logo",
        ] {
            assert!(artwork_media_type(kind).is_some(), "missing {kind}");
        }
        assert!(artwork_media_type("video").is_none());
    }
}
