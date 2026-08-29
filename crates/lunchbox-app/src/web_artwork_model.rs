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
        #[qproperty(bool, candidate_ready)]
        #[qproperty(QString, message)]
        #[qproperty(QString, query)]
        #[qproperty(QString, artwork_type)]
        #[qproperty(QString, selected_game_name)]
        #[qproperty(QUrl, candidate_url)]
        #[qproperty(i32, database_id)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, artwork_count)]
        #[qproperty(i32, revision)]
        #[qproperty(i32, published_revision)]
        type WebArtworkModel = super::WebArtworkModelRust;

        #[qinvokable]
        fn begin_selection(
            self: Pin<&mut WebArtworkModel>,
            database_id: i32,
            title: QString,
            platform: QString,
            artwork_type: QString,
        );

        #[qinvokable]
        fn search_games(self: Pin<&mut WebArtworkModel>, query: QString);

        #[qinvokable]
        fn choose_game(self: Pin<&mut WebArtworkModel>, index: i32);

        #[qinvokable]
        fn choose_artwork_kind(self: Pin<&mut WebArtworkModel>, artwork_type: QString);

        #[qinvokable]
        fn review_url(self: Pin<&mut WebArtworkModel>, url: QString);

        #[qinvokable]
        fn import_local_file(self: Pin<&mut WebArtworkModel>, url: QUrl);

        #[qinvokable]
        fn publish_artwork(self: Pin<&mut WebArtworkModel>, index: i32);

        #[qinvokable]
        fn cancel(self: Pin<&mut WebArtworkModel>);

        #[qinvokable]
        fn game_name_at(self: &WebArtworkModel, index: i32) -> QString;

        #[qinvokable]
        fn game_detail_at(self: &WebArtworkModel, index: i32) -> QString;

        #[qinvokable]
        fn artwork_thumbnail_at(self: &WebArtworkModel, index: i32) -> QUrl;

        #[qinvokable]
        fn artwork_detail_at(self: &WebArtworkModel, index: i32) -> QString;
    }

    impl cxx_qt::Threading for WebArtworkModel {}
}

use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QUrl};

use crate::media::ArtworkKind;

const PROVIDER: &str = "websearch";

#[derive(Clone, Debug)]
enum CandidateSource {
    Remote(String),
    Local(PathBuf),
}

#[derive(Clone, Debug)]
struct ReviewedCandidate {
    source_id: String,
    review_path: PathBuf,
}

pub struct WebArtworkModelRust {
    busy: bool,
    candidate_ready: bool,
    message: QString,
    query: QString,
    artwork_type: QString,
    selected_game_name: QString,
    candidate_url: QUrl,
    database_id: i32,
    game_count: i32,
    artwork_count: i32,
    revision: i32,
    published_revision: i32,
    title: String,
    platform: String,
    candidate: Option<ReviewedCandidate>,
    generation: u64,
}

impl Default for WebArtworkModelRust {
    fn default() -> Self {
        Self {
            busy: false,
            candidate_ready: false,
            message: qstring(
                "Open a focused image search, then review one exact HTTPS image or local file.",
            ),
            query: QString::default(),
            artwork_type: qstring("fanart"),
            selected_game_name: QString::default(),
            candidate_url: QUrl::default(),
            database_id: 0,
            game_count: 0,
            artwork_count: 0,
            revision: 0,
            published_revision: 0,
            title: String::new(),
            platform: String::new(),
            candidate: None,
            generation: 0,
        }
    }
}

fn qstring(value: impl AsRef<str>) -> QString {
    QString::from(value.as_ref())
}

fn requested_kind(value: &str) -> ArtworkKind {
    ArtworkKind::parse(value).unwrap_or(ArtworkKind::Fanart)
}

fn candidate_id(source: &CandidateSource) -> String {
    use sha2::{Digest, Sha256};
    let source = match source {
        CandidateSource::Remote(url) => url.as_bytes(),
        CandidateSource::Local(path) => path.as_os_str().as_encoded_bytes(),
    };
    hex::encode(Sha256::digest(source))
}

fn review_directory() -> PathBuf {
    std::env::temp_dir().join(format!(
        "lunchbox-web-artwork-review-{}",
        std::process::id()
    ))
}

impl qobject::WebArtworkModel {
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
        let title = title.to_string().trim().to_owned();
        let platform = platform.to_string().trim().to_owned();
        let kind = requested_kind(&artwork_type.to_string());
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().rust_mut().title = title.clone();
        self.as_mut().rust_mut().platform = platform.clone();
        self.as_mut().remove_candidate_file();
        self.as_mut().set_busy(false);
        self.as_mut().set_candidate_ready(false);
        self.as_mut().set_database_id(database_id);
        self.as_mut()
            .set_query(qstring(search_query(&title, &platform, kind)));
        self.as_mut().set_artwork_type(qstring(kind.key()));
        self.as_mut().set_selected_game_name(qstring(&title));
        self.as_mut().set_candidate_url(QUrl::default());
        self.as_mut().set_game_count(0);
        self.as_mut().set_artwork_count(0);
        self.as_mut().set_message(qstring(format!(
            "Search the web for {title}, then review one exact {} file before saving it.",
            kind.label().to_lowercase()
        )));
        self.as_mut().bump_revision();

        if std::env::args().any(|argument| argument == "--web-artwork-ui-probe") {
            if let Some(path) = probe_fixture_path() {
                self.as_mut().start_review(CandidateSource::Local(path));
            }
        }
    }

    pub fn search_games(mut self: Pin<&mut Self>, query: QString) {
        let query = query.to_string().trim().to_owned();
        if query.chars().count() < 2 {
            self.as_mut()
                .set_message(qstring("Enter at least two characters for the web search."));
            return;
        }
        self.as_mut().set_query(qstring(query));
        self.as_mut().set_message(qstring(
            "Open the browser search, then paste the exact image URL you chose.",
        ));
    }

    pub fn choose_game(mut self: Pin<&mut Self>, _index: i32) {
        self.as_mut().set_message(qstring(
            "Web artwork has no identity provider. Review the exact image itself.",
        ));
    }

    pub fn choose_artwork_kind(mut self: Pin<&mut Self>, artwork_type: QString) {
        let kind = requested_kind(&artwork_type.to_string());
        self.as_mut().set_artwork_type(qstring(kind.key()));
        let title = self.as_ref().rust().title.clone();
        let platform = self.as_ref().rust().platform.clone();
        self.as_mut()
            .set_query(qstring(search_query(&title, &platform, kind)));
        self.as_mut().set_message(qstring(format!(
            "Now reviewing {}. Existing artwork is not replaced until you confirm a candidate.",
            kind.label().to_lowercase()
        )));
        self.as_mut().bump_revision();
    }

    pub fn review_url(mut self: Pin<&mut Self>, url: QString) {
        let url = url.to_string().trim().to_owned();
        if !crate::provider_image::approved_url(&url) {
            self.as_mut().clear_candidate();
            self.as_mut().set_message(qstring(
                "Use a public HTTPS image URL. Plain HTTP and local-file URLs are rejected.",
            ));
            return;
        }
        self.as_mut().start_review(CandidateSource::Remote(url));
    }

    pub fn import_local_file(mut self: Pin<&mut Self>, url: QUrl) {
        let Some(path) = url.to_local_file() else {
            self.as_mut().clear_candidate();
            self.as_mut()
                .set_message(qstring("Choose a local PNG, JPEG, or WebP file."));
            return;
        };
        self.as_mut()
            .start_review(CandidateSource::Local(PathBuf::from(path.to_string())));
    }

    fn start_review(mut self: Pin<&mut Self>, source: CandidateSource) {
        if *self.as_ref().busy() {
            return;
        }
        let database_id = i64::from(*self.as_ref().database_id());
        let kind = requested_kind(&self.as_ref().artwork_type().to_string());
        let id = candidate_id(&source);
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().clear_candidate();
        self.as_mut().set_busy(true);
        self.as_mut().set_message(qstring(
            "Downloading into quarantine and validating the candidate…",
        ));
        let qt_thread = self.as_ref().qt_thread();
        let source_id = id.clone();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-web-artwork-review".into())
            .spawn(move || {
                let quarantine_root = review_directory().join(&id);
                let result = match source {
                    CandidateSource::Remote(url) => {
                        let agent: ureq::Agent = ureq::Agent::config_builder()
                            .timeout_connect(Some(std::time::Duration::from_secs(10)))
                            .timeout_global(Some(std::time::Duration::from_secs(30)))
                            .http_status_as_error(false)
                            .user_agent("Lunchbox/0.1 reviewed web artwork client")
                            .build()
                            .into();
                        crate::provider_image::download_and_publish(
                            &agent,
                            &url,
                            &quarantine_root,
                            database_id,
                            PROVIDER,
                            kind,
                            &id,
                        )
                    }
                    CandidateSource::Local(path) => crate::provider_image::copy_and_publish(
                        &path,
                        &quarantine_root,
                        database_id,
                        PROVIDER,
                        kind,
                        &id,
                    ),
                }
                .map(|review_path| ReviewedCandidate {
                    source_id,
                    review_path,
                })
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_review(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start candidate review: {error}"
            )));
        }
    }

    fn finish_review(
        mut self: Pin<&mut Self>,
        generation: u64,
        result: Result<ReviewedCandidate, String>,
    ) {
        if generation != self.as_ref().rust().generation {
            if let Ok(candidate) = result {
                std::fs::remove_file(candidate.review_path).ok();
            }
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(candidate) => {
                let preview = candidate.review_path.clone();
                self.as_mut().rust_mut().candidate = Some(candidate);
                self.as_mut()
                    .set_candidate_url(QUrl::from_local_file(&qstring(preview.to_string_lossy())));
                self.as_mut().set_candidate_ready(true);
                self.as_mut().set_artwork_count(1);
                self.as_mut().set_message(qstring(
                    "Candidate passed the 16 MiB and PNG/JPEG/WebP signature checks. Review the quarantined preview, then confirm it.",
                ));
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Candidate was rejected: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn publish_artwork(mut self: Pin<&mut Self>, index: i32) {
        if *self.as_ref().busy() || index != 0 {
            return;
        }
        let Some(candidate) = self.as_ref().rust().candidate.clone() else {
            self.as_mut()
                .set_message(qstring("Review an HTTPS image URL or local file first."));
            return;
        };
        let database_id = i64::from(*self.as_ref().database_id());
        let kind = requested_kind(&self.as_ref().artwork_type().to_string());
        let id = candidate.source_id.clone();
        let review_path = candidate.review_path.clone();
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        let generation = self.as_ref().rust().generation;
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_message(qstring("Validating and publishing the reviewed artwork…"));
        let qt_thread = self.as_ref().qt_thread();
        let spawn = std::thread::Builder::new()
            .name("lunchbox-web-artwork-publish".into())
            .spawn(move || {
                let result = crate::provider_image::copy_and_publish(
                    &review_path,
                    &crate::media::requested_media_directory(),
                    database_id,
                    PROVIDER,
                    kind,
                    &id,
                )
                .map_err(|error| error.to_string());
                let _ = qt_thread.queue(move |mut model| {
                    model.as_mut().finish_publish(generation, result);
                });
            });
        if let Err(error) = spawn {
            self.as_mut().set_busy(false);
            self.as_mut().set_message(qstring(format!(
                "Could not start artwork publication: {error}"
            )));
        }
    }

    fn finish_publish(mut self: Pin<&mut Self>, generation: u64, result: Result<PathBuf, String>) {
        if generation != self.as_ref().rust().generation {
            return;
        }
        self.as_mut().set_busy(false);
        match result {
            Ok(path) => {
                eprintln!(
                    "LUNCHBOX_WEB_ARTWORK_PUBLISHED database_id={} path={}",
                    self.as_ref().database_id(),
                    path.display()
                );
                self.as_mut().set_message(qstring(format!(
                    "Saved explicitly reviewed web artwork to {}.",
                    path.display()
                )));
                self.as_mut().remove_candidate_file();
                self.as_mut().set_candidate_url(QUrl::default());
                self.as_mut().set_candidate_ready(false);
                self.as_mut().set_artwork_count(0);
                let revision = self.as_ref().published_revision().wrapping_add(1);
                self.as_mut().set_published_revision(revision);
            }
            Err(error) => self
                .as_mut()
                .set_message(qstring(format!("Could not save web artwork: {error}"))),
        }
        self.as_mut().bump_revision();
    }

    pub fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().generation = self.as_ref().rust().generation.wrapping_add(1);
        self.as_mut().set_busy(false);
        self.as_mut().clear_candidate();
    }

    pub fn game_name_at(&self, _index: i32) -> QString {
        QString::default()
    }

    pub fn game_detail_at(&self, _index: i32) -> QString {
        QString::default()
    }

    pub fn artwork_thumbnail_at(&self, index: i32) -> QUrl {
        if index == 0 && *self.candidate_ready() {
            self.candidate_url().clone()
        } else {
            QUrl::default()
        }
    }

    pub fn artwork_detail_at(&self, index: i32) -> QString {
        if index == 0 && *self.candidate_ready() {
            qstring("Explicitly reviewed web candidate")
        } else {
            QString::default()
        }
    }

    fn clear_candidate(mut self: Pin<&mut Self>) {
        self.as_mut().remove_candidate_file();
        self.as_mut().set_candidate_url(QUrl::default());
        self.as_mut().set_candidate_ready(false);
        self.as_mut().set_artwork_count(0);
        self.as_mut().bump_revision();
    }

    fn remove_candidate_file(mut self: Pin<&mut Self>) {
        if let Some(candidate) = self.as_mut().rust_mut().candidate.take() {
            std::fs::remove_file(candidate.review_path).ok();
        }
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let revision = self.as_ref().revision().wrapping_add(1);
        self.as_mut().set_revision(revision);
    }
}

fn search_query(title: &str, platform: &str, kind: ArtworkKind) -> String {
    let media_term = match kind {
        ArtworkKind::BoxFront => "box front cover",
        ArtworkKind::BoxBack => "box back cover",
        ArtworkKind::Box3d => "3D box art",
        ArtworkKind::Screenshot => "gameplay screenshot",
        ArtworkKind::TitleScreen => "title screen",
        ArtworkKind::Fanart => "fan art wallpaper",
        ArtworkKind::ClearLogo => "transparent clear logo",
    };
    format!("{title} {platform} {media_term}")
}

fn probe_fixture_path() -> Option<PathBuf> {
    crate::web_artwork_probe_fixture().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_queries_disclose_the_requested_media_category() {
        assert_eq!(
            search_query(
                "Super Mario Bros.",
                "Nintendo Entertainment System",
                ArtworkKind::BoxFront
            ),
            "Super Mario Bros. Nintendo Entertainment System box front cover"
        );
        assert!(
            search_query("Game", "Platform", ArtworkKind::ClearLogo)
                .contains("transparent clear logo")
        );
    }

    #[test]
    fn candidate_ids_are_stable_without_exposing_the_source() {
        let first = candidate_id(&CandidateSource::Remote(
            "https://images.example/game.png".into(),
        ));
        let second = candidate_id(&CandidateSource::Remote(
            "https://images.example/game.png".into(),
        ));
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(!first.contains("images.example"));
    }
}
