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
        #[qproperty(i32, entry_count)]
        #[qproperty(i32, pending_count)]
        #[qproperty(i32, registered_count)]
        #[qproperty(i32, invalid_count)]
        #[qproperty(i32, revision)]
        #[qproperty(QString, directory)]
        #[qproperty(QString, message)]
        type WatchedTorrentModel = super::WatchedTorrentModelRust;

        #[qinvokable]
        fn initialize(self: Pin<&mut WatchedTorrentModel>);

        #[qinvokable]
        fn refresh(self: Pin<&mut WatchedTorrentModel>);

        #[qinvokable]
        fn archive_registered_file(self: Pin<&mut WatchedTorrentModel>, source: QUrl);

        #[qinvokable]
        fn file_name_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn file_path_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn file_url_at(self: &WatchedTorrentModel, index: i32) -> QUrl;

        #[qinvokable]
        fn torrent_name_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn info_hash_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn detail_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn status_at(self: &WatchedTorrentModel, index: i32) -> QString;

        #[qinvokable]
        fn can_review_at(self: &WatchedTorrentModel, index: i32) -> bool;
    }

    impl cxx_qt::Threading for WatchedTorrentModel {}
}

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::settings::SettingsStore;
use crate::watched_torrent::{WatchedTorrentEntry, WatchedTorrentScan};

pub struct WatchedTorrentModelRust {
    initialized: bool,
    busy: bool,
    entry_count: i32,
    pending_count: i32,
    registered_count: i32,
    invalid_count: i32,
    revision: i32,
    directory: QString,
    message: QString,
    entries: Vec<WatchedTorrentEntry>,
    generation: u64,
    refresh_requested: bool,
    pending_archive: Option<std::path::PathBuf>,
    archive_notice: Option<String>,
}

impl Default for WatchedTorrentModelRust {
    fn default() -> Self {
        Self {
            initialized: false,
            busy: false,
            entry_count: 0,
            pending_count: 0,
            registered_count: 0,
            invalid_count: 0,
            revision: 0,
            directory: QString::default(),
            message: QString::from(
                "Choose a watched torrent inbox. Lunchbox will detect .torrent files without queueing their payloads.",
            ),
            entries: Vec::new(),
            generation: 0,
            refresh_requested: false,
            pending_archive: None,
            archive_notice: None,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

impl qobject::WatchedTorrentModel {
    pub fn initialize(mut self: Pin<&mut Self>) {
        if *self.as_ref().initialized() {
            return;
        }
        self.as_mut().set_initialized(true);
        self.as_mut().refresh();
    }

    pub fn refresh(mut self: Pin<&mut Self>) {
        if *self.as_ref().busy() {
            self.as_mut().rust_mut().refresh_requested = true;
            return;
        }
        self.as_mut().rust_mut().refresh_requested = false;
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Scanning the watched torrent inbox…"));
        let previous = self.as_ref().rust().entries.clone();
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-watched-torrents".into())
            .spawn(move || {
                let scanned = SettingsStore::open_default()
                    .and_then(|store| {
                        let settings = store.load()?;
                        crate::watched_torrent::scan_watched_torrents_cached(
                            &settings.watched_torrent_directory,
                            &settings.watched_torrent_archive_directory,
                            &store,
                            &previous,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_refresh(generation, scanned);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start watched torrent scan: {error}"
            )));
        }
    }

    pub fn archive_registered_file(mut self: Pin<&mut Self>, source: QUrl) {
        let Some(source) = source
            .to_local_file()
            .map(|path| std::path::PathBuf::from(path.to_string()))
        else {
            self.as_mut()
                .set_message(qstring("Choose a local watched .torrent file."));
            return;
        };
        if *self.as_ref().busy() {
            self.as_mut().rust_mut().pending_archive = Some(source);
            return;
        }
        self.as_mut().start_archive(source);
    }

    fn start_archive(mut self: Pin<&mut Self>, source: std::path::PathBuf) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Archiving registered torrent metadata…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-archive-torrent".into())
            .spawn(move || {
                let archived = SettingsStore::open_default()
                    .and_then(|store| {
                        let settings = store.load()?;
                        crate::watched_torrent::archive_registered_torrent(
                            &source,
                            &settings.watched_torrent_directory,
                            &settings.watched_torrent_archive_directory,
                            &store,
                        )
                    })
                    .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_archive(generation, archived);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start torrent archival: {error}"
            )));
        }
    }

    fn finish_archive(
        mut self: Pin<&mut Self>,
        generation: u64,
        archived: Result<Option<crate::watched_torrent::ArchivedTorrent>, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match archived {
            Ok(Some(archived)) => {
                let source_name = archived
                    .source
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "torrent metadata".to_owned());
                let action = if archived.reused_existing {
                    format!(
                        "Removed duplicate {source_name}; exact metadata was already archived at {}",
                        archived.destination.display()
                    )
                } else {
                    format!(
                        "Archived {source_name} at {}",
                        archived.destination.display()
                    )
                };
                self.as_mut().rust_mut().archive_notice = Some(action);
                self.as_mut().refresh();
            }
            Ok(None) => self.as_mut().refresh(),
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not archive added torrent: {error}"))),
        }
    }

    fn finish_refresh(
        mut self: Pin<&mut Self>,
        generation: u64,
        scanned: Result<WatchedTorrentScan, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match scanned {
            Ok(scan) => {
                self.as_mut()
                    .set_directory(qstring(scan.root.to_string_lossy()));
                self.as_mut()
                    .set_entry_count(i32::try_from(scan.entries.len()).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_pending_count(i32::try_from(scan.pending_count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_registered_count(i32::try_from(scan.registered_count).unwrap_or(i32::MAX));
                self.as_mut()
                    .set_invalid_count(i32::try_from(scan.invalid_count).unwrap_or(i32::MAX));
                let message = self
                    .as_mut()
                    .rust_mut()
                    .archive_notice
                    .take()
                    .map(|notice| format!("{notice} · {}", scan.message()))
                    .unwrap_or_else(|| scan.message());
                self.as_mut().set_message(qstring(message));
                self.as_mut().rust_mut().entries = scan.entries;
                let next = self.as_ref().revision().wrapping_add(1);
                self.as_mut().set_revision(next);
            }
            Err(error) => {
                self.as_mut().rust_mut().entries.clear();
                self.as_mut().set_entry_count(0);
                self.as_mut().set_pending_count(0);
                self.as_mut().set_registered_count(0);
                self.as_mut().set_invalid_count(0);
                self.as_mut()
                    .set_message(qstring(format!("Could not scan watched torrents: {error}")));
            }
        }
        if let Some(source) = self.as_mut().rust_mut().pending_archive.take() {
            self.as_mut().start_archive(source);
        } else if self.as_ref().rust().refresh_requested {
            self.as_mut().rust_mut().refresh_requested = false;
            self.as_mut().refresh();
        }
    }

    pub fn file_name_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.file_name))
            .unwrap_or_default()
    }

    pub fn file_path_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(entry.path.to_string_lossy()))
            .unwrap_or_default()
    }

    pub fn file_url_at(&self, index: i32) -> QUrl {
        self.entry(index)
            .map(|entry| QUrl::from_local_file(&qstring(entry.path.to_string_lossy())))
            .unwrap_or_default()
    }

    pub fn torrent_name_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.torrent_name))
            .unwrap_or_default()
    }

    pub fn info_hash_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.info_hash))
            .unwrap_or_default()
    }

    pub fn detail_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.detail))
            .unwrap_or_default()
    }

    pub fn status_at(&self, index: i32) -> QString {
        self.entry(index)
            .map(|entry| qstring(&entry.status))
            .unwrap_or_default()
    }

    pub fn can_review_at(&self, index: i32) -> bool {
        self.entry(index)
            .is_some_and(|entry| entry.status == "pending")
    }

    fn entry(&self, index: i32) -> Option<&WatchedTorrentEntry> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().entries.get(index))
    }
}
