#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, message)]
        #[qproperty(QString, game_uid)]
        #[qproperty(QString, game_title)]
        #[qproperty(QString, game_platform)]
        #[qproperty(i32, file_count)]
        #[qproperty(i32, selected_file)]
        #[qproperty(i32, candidate_count)]
        #[qproperty(i32, selected_candidate)]
        #[qproperty(i32, history_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, change_revision)]
        #[qproperty(QString, last_game_uid)]
        #[qproperty(QString, last_game_title)]
        #[qproperty(QString, last_game_platform)]
        #[qproperty(i64, last_launchbox_db_id)]
        type CollectionIdentityModel = super::CollectionIdentityModelRust;

        #[qinvokable]
        fn load_game(
            self: Pin<&mut CollectionIdentityModel>,
            game_uid: QString,
            title: QString,
            platform: QString,
        );

        #[qinvokable]
        fn search_targets(
            self: Pin<&mut CollectionIdentityModel>,
            query: QString,
            platform: QString,
        );

        #[qinvokable]
        fn select_file(self: Pin<&mut CollectionIdentityModel>, index: i32);

        #[qinvokable]
        fn select_candidate(self: Pin<&mut CollectionIdentityModel>, index: i32);

        #[qinvokable]
        fn relink_selected(self: Pin<&mut CollectionIdentityModel>);

        #[qinvokable]
        fn undo_at(self: Pin<&mut CollectionIdentityModel>, index: i32);

        #[qinvokable]
        fn file_label_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn file_path_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn file_detail_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn candidate_title_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn candidate_platform_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn candidate_match_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn candidate_game_uid_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn history_summary_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn history_detail_at(self: &CollectionIdentityModel, index: i32) -> QString;

        #[qinvokable]
        fn history_occurred_at(self: &CollectionIdentityModel, index: i32) -> i64;

        #[qinvokable]
        fn history_undoable_at(self: &CollectionIdentityModel, index: i32) -> bool;
    }
}

use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::collection_identity::{self, IdentityEvent, ImportedGameFile};
use crate::local_import::ManualMatchCandidate;

#[derive(Default)]
pub struct CollectionIdentityModelRust {
    message: QString,
    game_uid: QString,
    game_title: QString,
    game_platform: QString,
    file_count: i32,
    selected_file: i32,
    candidate_count: i32,
    selected_candidate: i32,
    history_count: i32,
    revision: i32,
    change_revision: i32,
    last_game_uid: QString,
    last_game_title: QString,
    last_game_platform: QString,
    last_launchbox_db_id: i64,
    files: Vec<ImportedGameFile>,
    candidates: Vec<ManualMatchCandidate>,
    history: Vec<IdentityEvent>,
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn usize_index(index: i32, len: usize) -> Option<usize> {
    usize::try_from(index).ok().filter(|index| *index < len)
}

fn saturating_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn human_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format!("{:.2} GiB", bytes_f / GIB)
    } else if bytes_f >= MIB {
        format!("{:.1} MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.1} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} bytes")
    }
}

impl qobject::CollectionIdentityModel {
    pub fn load_game(
        mut self: Pin<&mut Self>,
        game_uid: QString,
        title: QString,
        platform: QString,
    ) {
        let game_uid = game_uid.to_string();
        let title = title.to_string();
        let platform = platform.to_string();
        self.as_mut().set_game_uid(qstring(&game_uid));
        self.as_mut().set_game_title(qstring(&title));
        self.as_mut().set_game_platform(qstring(&platform));
        self.as_mut().rust_mut().candidates.clear();
        self.as_mut().set_candidate_count(0);
        self.as_mut().set_selected_candidate(-1);

        let result = crate::settings::state_database_path()
            .and_then(|path| collection_identity::imported_files_for_game(&path, &game_uid));
        match result {
            Ok(files) => {
                self.as_mut().rust_mut().files = files;
                let count = saturating_i32(self.as_ref().rust().files.len());
                self.as_mut().set_file_count(count);
                self.as_mut()
                    .set_selected_file(if count > 0 { 0 } else { -1 });
                self.as_mut().refresh_history();
                self.as_mut().set_message(qstring(if count > 0 {
                    "Choose an exact catalog game below. No ROM data will be moved or changed."
                } else {
                    "No user-imported ROM records are linked to this game."
                }));
            }
            Err(error) => {
                self.as_mut().rust_mut().files.clear();
                self.as_mut().set_file_count(0);
                self.as_mut().set_selected_file(-1);
                self.as_mut().rust_mut().history.clear();
                self.as_mut().set_history_count(0);
                self.as_mut().set_message(qstring(format!(
                    "Could not load imported ROM identities: {error:#}"
                )));
            }
        }
        self.as_mut().bump_revision();
    }

    pub fn search_targets(mut self: Pin<&mut Self>, query: QString, platform: QString) {
        let query = query.to_string();
        let platform = platform.to_string();
        let Some(discovery_path) = crate::catalog::requested_discovery_database_path() else {
            self.as_mut().set_message(qstring(
                "The discovery games database is required to search exact catalog identities.",
            ));
            return;
        };
        match collection_identity::search_targets(&discovery_path, &query, &platform) {
            Ok(mut candidates) => {
                let current = self.as_ref().game_uid().to_string();
                candidates.retain(|candidate| candidate.identity.game_uid != current);
                self.as_mut().rust_mut().candidates = candidates;
                let count = saturating_i32(self.as_ref().rust().candidates.len());
                self.as_mut().set_candidate_count(count);
                self.as_mut().set_selected_candidate(-1);
                self.as_mut()
                    .set_message(qstring(if query.trim().chars().count() < 2 {
                        "Type at least two characters to search the catalog."
                    } else if count == 0 {
                        "No exact catalog candidates matched this search. Try all platforms."
                    } else {
                        "Select the exact game identity, review it, then confirm the relink."
                    }));
                self.as_mut().bump_revision();
            }
            Err(error) => self.as_mut().set_message(qstring(format!(
                "Could not search catalog identities: {error:#}"
            ))),
        }
    }

    pub fn select_file(mut self: Pin<&mut Self>, index: i32) {
        if usize_index(index, self.as_ref().rust().files.len()).is_none() {
            return;
        }
        self.as_mut().set_selected_file(index);
        self.as_mut().set_selected_candidate(-1);
        self.as_mut().refresh_history();
        self.as_mut().bump_revision();
    }

    pub fn select_candidate(mut self: Pin<&mut Self>, index: i32) {
        if usize_index(index, self.as_ref().rust().candidates.len()).is_none() {
            return;
        }
        self.as_mut().set_selected_candidate(index);
        self.as_mut().set_message(qstring(
            "Review the title and platform below. Relink changes only Lunchbox's identity record.",
        ));
        self.as_mut().bump_revision();
    }

    pub fn relink_selected(mut self: Pin<&mut Self>) {
        let Some(file_index) = usize_index(
            *self.as_ref().selected_file(),
            self.as_ref().rust().files.len(),
        ) else {
            self.as_mut()
                .set_message(qstring("Select an imported ROM first."));
            return;
        };
        let Some(candidate_index) = usize_index(
            *self.as_ref().selected_candidate(),
            self.as_ref().rust().candidates.len(),
        ) else {
            self.as_mut()
                .set_message(qstring("Select the exact target game first."));
            return;
        };
        let file = self.as_ref().rust().files[file_index].clone();
        let candidate = self.as_ref().rust().candidates[candidate_index].clone();
        let current_game_uid = self.as_ref().game_uid().to_string();
        let result = crate::settings::state_database_path().and_then(|state_path| {
            let discovery_path = crate::catalog::requested_discovery_database_path()
                .ok_or_else(|| anyhow::anyhow!("the discovery games database is unavailable"))?;
            collection_identity::relink_imported_file(
                &state_path,
                &discovery_path,
                &file.id,
                &current_game_uid,
                &candidate.identity.game_uid,
            )
        });
        match result {
            Ok(_) => {
                self.as_mut()
                    .set_last_game_uid(qstring(&candidate.identity.game_uid));
                self.as_mut()
                    .set_last_game_title(qstring(&candidate.identity.title));
                self.as_mut()
                    .set_last_game_platform(qstring(&candidate.identity.platform));
                self.as_mut()
                    .set_last_launchbox_db_id(candidate.identity.launchbox_db_id);
                let revision = self.as_ref().change_revision().saturating_add(1);
                self.as_mut().set_change_revision(revision);
                self.as_mut().load_game(
                    qstring(&candidate.identity.game_uid),
                    qstring(&candidate.identity.title),
                    qstring(&candidate.identity.platform),
                );
                self.as_mut().set_message(qstring(format!(
                    "Relinked {} to {}. The ROM file was not modified.",
                    file.file_name, candidate.identity.title
                )));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not relink this ROM: {error:#}"))),
        }
    }

    pub fn undo_at(mut self: Pin<&mut Self>, index: i32) {
        let Some(index) = usize_index(index, self.as_ref().rust().history.len()) else {
            return;
        };
        let event = self.as_ref().rust().history[index].clone();
        if event.kind != "relink" || event.reverted {
            self.as_mut()
                .set_message(qstring("This history entry cannot be undone."));
            return;
        }
        let result = crate::settings::state_database_path()
            .and_then(|state_path| collection_identity::undo_relink(&state_path, &event.id));
        match result {
            Ok(undo) => {
                let game_uid = undo.to_game_uid.unwrap_or_default();
                let title = undo.to_title.unwrap_or_default();
                let platform = undo.to_platform;
                self.as_mut().set_last_game_uid(qstring(&game_uid));
                self.as_mut().set_last_game_title(qstring(&title));
                self.as_mut().set_last_game_platform(qstring(&platform));
                self.as_mut().set_last_launchbox_db_id(0);
                let revision = self.as_ref().change_revision().saturating_add(1);
                self.as_mut().set_change_revision(revision);
                self.as_mut()
                    .load_game(qstring(&game_uid), qstring(&title), qstring(&platform));
                self.as_mut().set_message(qstring(
                    "Restored the prior exact identity. The ROM file was not modified.",
                ));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not undo this relink: {error:#}"))),
        }
    }

    pub fn file_label_at(&self, index: i32) -> QString {
        usize_index(index, self.rust().files.len())
            .map(|index| {
                let file = &self.rust().files[index];
                if file.archive_member.is_empty() {
                    file.file_name.clone()
                } else {
                    format!("{} · {}", file.file_name, file.archive_member)
                }
            })
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn file_path_at(&self, index: i32) -> QString {
        usize_index(index, self.rust().files.len())
            .map(|index| qstring(&self.rust().files[index].path_display))
            .unwrap_or_default()
    }

    pub fn file_detail_at(&self, index: i32) -> QString {
        usize_index(index, self.rust().files.len())
            .map(|index| {
                let file = &self.rust().files[index];
                format!(
                    "{} · {} · {} · {}",
                    human_size(file.file_size),
                    file.match_state,
                    file.match_method,
                    file.availability
                )
            })
            .map(qstring)
            .unwrap_or_default()
    }

    pub fn candidate_title_at(&self, index: i32) -> QString {
        self.candidate(index)
            .map(|candidate| qstring(&candidate.identity.title))
            .unwrap_or_default()
    }

    pub fn candidate_platform_at(&self, index: i32) -> QString {
        self.candidate(index)
            .map(|candidate| qstring(&candidate.identity.platform))
            .unwrap_or_default()
    }

    pub fn candidate_match_at(&self, index: i32) -> QString {
        self.candidate(index)
            .map(|candidate| {
                if candidate.matched_name.is_empty() {
                    qstring("Canonical title")
                } else {
                    qstring(format!(
                        "Matched alternate title: {}",
                        candidate.matched_name
                    ))
                }
            })
            .unwrap_or_default()
    }

    pub fn candidate_game_uid_at(&self, index: i32) -> QString {
        self.candidate(index)
            .map(|candidate| qstring(&candidate.identity.game_uid))
            .unwrap_or_default()
    }

    pub fn history_summary_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| {
                let from = event.from_title.as_deref().unwrap_or("Unlinked");
                let to = event.to_title.as_deref().unwrap_or("Unlinked");
                if event.kind == "undo" {
                    qstring(format!("Restored {to}"))
                } else if event.reverted {
                    qstring(format!("{from} → {to} · undone"))
                } else {
                    qstring(format!("{from} → {to}"))
                }
            })
            .unwrap_or_default()
    }

    pub fn history_detail_at(&self, index: i32) -> QString {
        self.history_event(index)
            .map(|event| {
                qstring(format!(
                    "{} → {} · exact file identity preserved",
                    event.from_platform, event.to_platform
                ))
            })
            .unwrap_or_default()
    }

    pub fn history_occurred_at(&self, index: i32) -> i64 {
        self.history_event(index)
            .map(|event| event.occurred_at)
            .unwrap_or_default()
    }

    pub fn history_undoable_at(&self, index: i32) -> bool {
        self.history_event(index)
            .is_some_and(|event| event.kind == "relink" && !event.reverted)
    }

    fn candidate(&self, index: i32) -> Option<&ManualMatchCandidate> {
        usize_index(index, self.rust().candidates.len()).map(|index| &self.rust().candidates[index])
    }

    fn history_event(&self, index: i32) -> Option<&IdentityEvent> {
        usize_index(index, self.rust().history.len()).map(|index| &self.rust().history[index])
    }

    fn refresh_history(mut self: Pin<&mut Self>) {
        let history = usize_index(
            *self.as_ref().selected_file(),
            self.as_ref().rust().files.len(),
        )
        .and_then(|index| {
            let file_id = self.as_ref().rust().files[index].id.clone();
            crate::settings::state_database_path()
                .and_then(|path| collection_identity::identity_history_for_file(&path, &file_id))
                .ok()
        })
        .unwrap_or_default();
        self.as_mut().rust_mut().history = history;
        let count = saturating_i32(self.as_ref().rust().history.len());
        self.as_mut().set_history_count(count);
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().saturating_add(1);
        self.as_mut().set_revision(revision);
    }
}
