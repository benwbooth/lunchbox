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
        #[qproperty(bool, busy)]
        #[qproperty(bool, ready)]
        #[qproperty(bool, collection_mode)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, selected_index)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, queued_revision)]
        #[qproperty(i32, registered_revision)]
        #[qproperty(QString, game_title)]
        #[qproperty(QString, game_platform)]
        #[qproperty(QString, source_file_name)]
        #[qproperty(QString, source_kind)]
        #[qproperty(QString, torrent_name)]
        #[qproperty(QString, info_hash)]
        #[qproperty(QString, total_size)]
        #[qproperty(QString, download_scope)]
        #[qproperty(QString, message)]
        #[qproperty(QString, queued_job_id)]
        type ExternalTorrentModel = super::ExternalTorrentModelRust;

        #[qinvokable]
        fn begin_review(
            self: Pin<&mut ExternalTorrentModel>,
            game_uid: QString,
            launchbox_db_id: i32,
            title: QString,
            platform: QString,
        );

        #[qinvokable]
        fn begin_collection_review(self: Pin<&mut ExternalTorrentModel>, platform: QString);

        #[qinvokable]
        fn set_collection_platform(self: Pin<&mut ExternalTorrentModel>, platform: QString);

        #[qinvokable]
        fn inspect_file(self: Pin<&mut ExternalTorrentModel>, url: QUrl);

        #[qinvokable]
        fn inspect_magnet(self: Pin<&mut ExternalTorrentModel>, magnet_uri: QString);

        #[qinvokable]
        fn inspect_probe_fixture(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn select_file(self: Pin<&mut ExternalTorrentModel>, index: i32);

        #[qinvokable]
        fn queue_selected(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn register_source(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn clear(self: Pin<&mut ExternalTorrentModel>);

        #[qinvokable]
        fn file_path_at(self: &ExternalTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn file_size_at(self: &ExternalTorrentModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for ExternalTorrentModel {}
}

use std::pin::Pin;

use anyhow::Context;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::external_torrent::{
    self, CanonicalGameAssociation, CollectionTorrentAssociation, ManualTorrentOffer,
    ReviewedTorrentFile,
};

pub struct ExternalTorrentModelRust {
    busy: bool,
    ready: bool,
    collection_mode: bool,
    file_count: i32,
    selected_index: i32,
    revision: i32,
    queued_revision: i32,
    registered_revision: i32,
    game_title: QString,
    game_platform: QString,
    source_file_name: QString,
    source_kind: QString,
    torrent_name: QString,
    info_hash: QString,
    total_size: QString,
    download_scope: QString,
    message: QString,
    queued_job_id: QString,
    association: Option<CanonicalGameAssociation>,
    collection_association: Option<CollectionTorrentAssociation>,
    offer: Option<ManualTorrentOffer>,
    generation: u64,
    probe_fixture_path: Option<std::path::PathBuf>,
}

impl Default for ExternalTorrentModelRust {
    fn default() -> Self {
        Self {
            busy: false,
            ready: false,
            collection_mode: false,
            file_count: 0,
            selected_index: -1,
            revision: 0,
            queued_revision: 0,
            registered_revision: 0,
            game_title: QString::default(),
            game_platform: QString::default(),
            source_file_name: QString::default(),
            source_kind: QString::from("file"),
            torrent_name: QString::default(),
            info_hash: QString::default(),
            total_size: QString::default(),
            download_scope: QString::from("Selected file only"),
            message: QString::from(
                "Choose a lawful .torrent file, then review its exact contents before anything is downloaded.",
            ),
            queued_job_id: QString::default(),
            association: None,
            collection_association: None,
            offer: None,
            generation: 0,
            probe_fixture_path: None,
        }
    }
}

struct InspectionResult {
    offer: ManualTorrentOffer,
    download_entire_torrent: bool,
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::ExternalTorrentModel {
    pub fn begin_review(
        mut self: Pin<&mut Self>,
        game_uid: QString,
        launchbox_db_id: i32,
        title: QString,
        platform: QString,
    ) {
        if *self.as_ref().busy() {
            return;
        }
        let association = CanonicalGameAssociation {
            game_uid: game_uid.to_string(),
            launchbox_db_id: i64::from(launchbox_db_id),
            title: title.to_string(),
            platform: platform.to_string(),
        };
        if association.game_uid.trim().is_empty()
            || association.title.trim().is_empty()
            || association.platform.trim().is_empty()
        {
            self.as_mut().set_message(qstring(
                "Select an exact catalog game before adding an external torrent source.",
            ));
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        self.as_mut().rust_mut().association = Some(association.clone());
        self.as_mut().rust_mut().collection_association = None;
        self.as_mut().set_collection_mode(false);
        self.as_mut().set_game_title(qstring(association.title));
        self.as_mut()
            .set_game_platform(qstring(association.platform));
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "This source will be associated with the exact game shown above. Torrent labels cannot change catalog identity.",
        ));
    }

    pub fn begin_collection_review(mut self: Pin<&mut Self>, platform: QString) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        let platform = normalized_collection_platform(&platform.to_string());
        self.as_mut().rust_mut().association = None;
        self.as_mut().rust_mut().collection_association = Some(CollectionTorrentAssociation {
            platform: platform.clone(),
        });
        self.as_mut().set_collection_mode(true);
        self.as_mut().set_game_title(qstring("Collection torrent"));
        self.as_mut().set_game_platform(qstring(platform));
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "Review and register this torrent as an on-demand source. No payload files download when it is added.",
        ));
    }

    pub fn set_collection_platform(mut self: Pin<&mut Self>, platform: QString) {
        if *self.as_ref().busy() || *self.as_ref().ready() || !*self.as_ref().collection_mode() {
            return;
        }
        let platform = normalized_collection_platform(&platform.to_string());
        self.as_mut().rust_mut().collection_association = Some(CollectionTorrentAssociation {
            platform: platform.clone(),
        });
        self.as_mut().set_game_platform(qstring(platform));
    }

    pub fn inspect_file(mut self: Pin<&mut Self>, url: QUrl) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before selecting a torrent.",
            ));
            return;
        }
        let Some(path) = url.to_local_file() else {
            self.as_mut()
                .set_message(qstring("Choose a local .torrent file."));
            return;
        };
        let path = std::path::PathBuf::from(path.to_string());
        self.as_mut().start_inspection(path);
    }

    pub fn inspect_magnet(mut self: Pin<&mut Self>, magnet_uri: QString) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before reviewing a magnet link.",
            ));
            return;
        }
        let magnet_uri = magnet_uri.to_string();
        let platform = self.as_ref().game_platform().to_string();
        let collection_mode = *self.as_ref().collection_mode();
        let previous_offer = self.as_mut().rust_mut().offer.take();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Asking qBittorrent for bounded magnet metadata; payload files remain paused…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-magnet-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<InspectionResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )
                        .context("removing the previous unqueued magnet review")?;
                    }
                    let offer = external_torrent::inspect_magnet_source(
                        &settings,
                        &password,
                        &magnet_uri,
                        &platform,
                        collection_mode,
                    )?;
                    Ok(InspectionResult {
                        offer,
                        download_entire_torrent: !collection_mode
                            && settings.download_entire_torrent,
                    })
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start magnet review: {error}")));
        }
    }

    pub fn inspect_probe_fixture(mut self: Pin<&mut Self>) {
        if !std::env::args().any(|argument| argument == "--manual-torrent-ui-probe") {
            self.as_mut().set_message(qstring(
                "The deterministic fixture is available only to the UI probe.",
            ));
            return;
        }
        if *self.as_ref().busy() {
            return;
        }
        let path = std::env::temp_dir()
            .join(format!(
                "lunchbox-manual-torrent-probe-{}",
                std::process::id()
            ))
            .join("reviewed-sample-source.torrent");
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            self.as_mut().set_message(qstring(format!(
                "Could not create the deterministic torrent fixture directory: {error}"
            )));
            return;
        }
        if let Err(error) = std::fs::write(&path, external_torrent::probe_fixture_torrent_bytes()) {
            self.as_mut().set_message(qstring(format!(
                "Could not create the deterministic torrent fixture: {error}"
            )));
            return;
        }
        self.as_mut().rust_mut().probe_fixture_path = Some(path.clone());
        self.as_mut().start_inspection(path);
    }

    fn start_inspection(mut self: Pin<&mut Self>, path: std::path::PathBuf) {
        if *self.as_ref().busy() {
            return;
        }
        if !self.as_ref().rust().has_review_target() {
            self.as_mut().set_message(qstring(
                "Choose an exact game or collection platform before selecting a torrent.",
            ));
            return;
        }
        let previous_offer = self.as_mut().rust_mut().offer.take();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().reset_offer();
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Inspecting bounded torrent metadata…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-review".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<InspectionResult> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    if let Some(previous_offer) = previous_offer.as_ref() {
                        let password = crate::settings::load_password()?.unwrap_or_default();
                        external_torrent::discard_unqueued_magnet_review(
                            &settings,
                            &password,
                            previous_offer,
                        )
                        .context("removing the previous unqueued magnet review")?;
                    }
                    let offer = external_torrent::inspect_torrent_file(&path)?;
                    Ok(InspectionResult {
                        offer,
                        download_entire_torrent: settings.download_entire_torrent,
                    })
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_inspection(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut()
                .set_message(qstring(format!("Could not start torrent review: {error}")));
        }
    }

    fn finish_inspection(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<InspectionResult, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(result) => {
                let offer = result.offer;
                let file_count = offer.files.len();
                let collection_mode = *self.as_ref().collection_mode();
                self.as_mut()
                    .set_file_count(i32::try_from(file_count).unwrap_or(i32::MAX));
                self.as_mut().set_selected_index(0);
                self.as_mut()
                    .set_source_file_name(qstring(&offer.source_file_name));
                self.as_mut().set_source_kind(qstring(&offer.source_kind));
                self.as_mut().set_torrent_name(qstring(&offer.torrent_name));
                self.as_mut().set_info_hash(qstring(&offer.info_hash));
                self.as_mut()
                    .set_total_size(qstring(crate::game_details::format_bytes(
                        offer.total_bytes,
                    )));
                self.as_mut()
                    .set_download_scope(qstring(if collection_mode {
                        "Indexed source · downloads on demand"
                    } else if result.download_entire_torrent {
                        "Entire torrent · required by Settings"
                    } else {
                        "Selected file only"
                    }));
                self.as_mut().set_message(qstring(if collection_mode {
                    format!(
                        "Reviewed {file_count} safe {}. Add the source now; individual games will appear in their matching Get lists.",
                        if file_count == 1 { "file" } else { "files" }
                    )
                } else {
                    format!(
                        "Reviewed {file_count} safe {}. Choose the exact payload for this game.",
                        if file_count == 1 { "file" } else { "files" }
                    )
                }));
                self.as_mut().rust_mut().offer = Some(offer);
                self.as_mut().bump_revision();
                self.as_mut().set_ready(true);
            }
            Err(error) => {
                self.as_mut()
                    .set_message(qstring(format!("Could not review this torrent: {error}")));
            }
        }
    }

    pub fn select_file(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || !*self.as_ref().ready() {
            return;
        }
        let valid = usize::try_from(index)
            .ok()
            .is_some_and(|index| self.as_ref().rust().file(index).is_some());
        if !valid {
            self.as_mut()
                .set_message(qstring("The selected torrent file is no longer available."));
            return;
        }
        self.as_mut().set_selected_index(index);
        self.as_mut().bump_revision();
    }

    pub fn queue_selected(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().ready() {
            return;
        }
        if *self.as_ref().collection_mode() {
            self.as_mut().set_message(qstring(
                "Add this torrent as a source; individual games are downloaded later from their Get lists.",
            ));
            return;
        }
        let Some(offer) = self.as_ref().rust().offer.clone() else {
            self.as_mut()
                .set_message(qstring("Choose and review a torrent first."));
            return;
        };
        let Ok(selected_index) = usize::try_from(*self.as_ref().selected_index()) else {
            self.as_mut()
                .set_message(qstring("Choose an exact file before queueing."));
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Adding the reviewed selection to Lunchbox downloads…",
        ));
        let association = self.as_ref().rust().association.clone();
        let generation = self.as_ref().rust().generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-enqueue".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<String> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    let association = association
                        .context("the exact catalog association is no longer available")?;
                    external_torrent::queue_manual_torrent(
                        &settings,
                        &password,
                        &store,
                        association,
                        offer,
                        selected_index,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_queue(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start the download request: {error}"
            )));
        }
    }

    fn finish_queue(mut self: Pin<&mut Self>, generation: u64, result: Result<String, String>) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(job_id) => {
                if let Some(offer) = self.as_mut().rust_mut().offer.as_mut() {
                    offer.magnet_review_created = false;
                }
                self.as_mut().set_queued_job_id(qstring(job_id));
                let revision = self.as_ref().queued_revision().saturating_add(1);
                self.as_mut().set_queued_revision(revision);
                self.as_mut().set_message(qstring(
                    "Queued with exact game, source, info-hash, and file provenance.",
                ));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not queue this torrent: {error}"))),
        }
    }

    pub fn register_source(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() || !*self.as_ref().ready() || !*self.as_ref().collection_mode() {
            return;
        }
        let Some(offer) = self.as_ref().rust().offer.clone() else {
            self.as_mut()
                .set_message(qstring("Choose and review a torrent first."));
            return;
        };
        let Some(association) = self.as_ref().rust().collection_association.clone() else {
            self.as_mut()
                .set_message(qstring("Choose the platform represented by this torrent."));
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Indexing exact torrent members for on-demand game downloads…",
        ));
        let generation = self.as_ref().rust().generation;
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-torrent-source-register".to_owned())
            .spawn(move || {
                let result = (|| -> anyhow::Result<i64> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    external_torrent::register_collection_torrent(
                        &settings,
                        &password,
                        &store,
                        association,
                        offer,
                    )
                })()
                .map_err(|error| format!("{error:#}"));
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_registration(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start torrent source registration: {error}"
            )));
        }
    }

    fn finish_registration(mut self: Pin<&mut Self>, generation: u64, result: Result<i64, String>) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(_) => {
                if let Some(offer) = self.as_mut().rust_mut().offer.as_mut() {
                    offer.magnet_review_created = false;
                }
                let revision = self.as_ref().registered_revision().saturating_add(1);
                self.as_mut().set_registered_revision(revision);
                self.as_mut().set_message(qstring(
                    "Torrent source added. Matching games can now download individual files from their Get lists.",
                ));
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not add this torrent source: {error}"
            ))),
        }
    }

    pub fn clear(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            return;
        }
        self.as_mut().schedule_current_offer_cleanup();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().association = None;
        self.as_mut().rust_mut().collection_association = None;
        self.as_mut().set_collection_mode(false);
        self.as_mut().set_game_title(QString::default());
        self.as_mut().set_game_platform(QString::default());
        self.as_mut().reset_offer();
        self.as_mut().set_message(qstring(
            "Choose a lawful .torrent file, then review its exact contents before anything is downloaded.",
        ));
    }

    pub fn file_path_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().file(index))
            .map(|file| qstring(&file.path))
            .unwrap_or_default()
    }

    pub fn file_size_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().file(index))
            .map(|file| qstring(crate::game_details::format_bytes(file.byte_size)))
            .unwrap_or_default()
    }

    fn reset_offer(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().offer = None;
        self.as_mut().set_ready(false);
        self.as_mut().set_file_count(0);
        self.as_mut().set_selected_index(-1);
        self.as_mut().set_source_file_name(QString::default());
        self.as_mut().set_source_kind(qstring("file"));
        self.as_mut().set_torrent_name(QString::default());
        self.as_mut().set_info_hash(QString::default());
        self.as_mut().set_total_size(QString::default());
        self.as_mut()
            .set_download_scope(qstring("Selected file only"));
        self.as_mut().set_queued_job_id(QString::default());
        self.as_mut().bump_revision();
    }

    fn schedule_current_offer_cleanup(mut self: Pin<&mut Self>) {
        let Some(offer) = self.as_mut().rust_mut().offer.take() else {
            return;
        };
        if offer.source_kind != "magnet" || !offer.magnet_review_created {
            return;
        }
        let _ = std::thread::Builder::new()
            .name("lunchbox-magnet-review-cleanup".to_owned())
            .spawn(move || {
                let _ = (|| -> anyhow::Result<()> {
                    let store = crate::settings::SettingsStore::open_default()?;
                    let settings = store.load()?;
                    let password = crate::settings::load_password()?.unwrap_or_default();
                    external_torrent::discard_unqueued_magnet_review(&settings, &password, &offer)
                })();
            });
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().saturating_add(1);
        self.as_mut().set_revision(revision);
    }
}

impl ExternalTorrentModelRust {
    fn file(&self, index: usize) -> Option<&ReviewedTorrentFile> {
        self.offer.as_ref()?.files.get(index)
    }

    fn has_review_target(&self) -> bool {
        self.association.is_some() || self.collection_association.is_some()
    }
}

fn normalized_collection_platform(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "Unassigned platform".to_owned()
    } else {
        value.chars().take(512).collect()
    }
}

impl Drop for ExternalTorrentModelRust {
    fn drop(&mut self) {
        if let Some(path) = self.probe_fixture_path.take() {
            let _ = std::fs::remove_file(&path);
            if let Some(parent) = path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }
}
