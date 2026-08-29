use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use lava_torrent::torrent::v1::Torrent;
use sha2::{Digest, Sha256};

use crate::qbittorrent::{self, EnqueueRequest};
use crate::settings::{AppSettings, ManualTorrentSourceReceipt, SettingsStore, unix_timestamp};

pub(crate) const MAX_TORRENT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TORRENT_FILES: usize = 100_000;
const MAX_TORRENT_NAME_CHARS: usize = 1024;
const MAX_TORRENT_PATH_CHARS: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReviewedTorrentFile {
    pub index: u32,
    pub path: String,
    pub byte_size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManualTorrentOffer {
    pub source_file_name: String,
    pub torrent_name: String,
    pub torrent_sha256: String,
    pub info_hash: String,
    pub total_bytes: u64,
    pub files: Vec<ReviewedTorrentFile>,
    pub torrent_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalGameAssociation {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
}

pub(crate) fn inspect_torrent_file(path: &Path) -> Result<ManualTorrentOffer> {
    let source_file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .context("selected torrent path has no filename")?;
    let mut file =
        File::open(path).with_context(|| format!("opening selected torrent {}", path.display()))?;
    let declared_size = file
        .metadata()
        .with_context(|| format!("reading selected torrent metadata {}", path.display()))?
        .len();
    if declared_size == 0 {
        bail!("the selected torrent file is empty");
    }
    if declared_size > MAX_TORRENT_BYTES {
        bail!(
            "the selected torrent is larger than the {} MiB review limit",
            MAX_TORRENT_BYTES / (1024 * 1024)
        );
    }
    let mut bytes = Vec::with_capacity(usize::try_from(declared_size).unwrap_or_default());
    file.by_ref()
        .take(MAX_TORRENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading selected torrent {}", path.display()))?;
    if bytes.len() as u64 > MAX_TORRENT_BYTES {
        bail!(
            "the selected torrent changed while it was read and now exceeds the {} MiB review limit",
            MAX_TORRENT_BYTES / (1024 * 1024)
        );
    }
    inspect_torrent_bytes(&bytes, &source_file_name)
}

pub(crate) fn inspect_torrent_bytes(
    bytes: &[u8],
    source_file_name: &str,
) -> Result<ManualTorrentOffer> {
    if bytes.is_empty() {
        bail!("the selected torrent file is empty");
    }
    if bytes.len() as u64 > MAX_TORRENT_BYTES {
        bail!(
            "the selected torrent is larger than the {} MiB review limit",
            MAX_TORRENT_BYTES / (1024 * 1024)
        );
    }
    validate_label(source_file_name, "source filename", MAX_TORRENT_NAME_CHARS)?;
    let torrent = Torrent::read_from_bytes(bytes)
        .map_err(|error| anyhow::anyhow!("could not parse selected torrent metadata: {error}"))?;
    validate_label(&torrent.name, "torrent name", MAX_TORRENT_NAME_CHARS)?;
    validate_portable_component(&torrent.name, "torrent name")?;
    let info_hash = torrent.info_hash();
    if info_hash.len() != 40 || !info_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("the selected torrent has an invalid v1 info hash");
    }

    let files = if let Some(files) = torrent.files {
        if files.is_empty() {
            bail!("the selected torrent contains no files");
        }
        if files.len() > MAX_TORRENT_FILES {
            bail!(
                "the selected torrent contains more than {MAX_TORRENT_FILES} files and cannot be reviewed safely"
            );
        }
        files
            .into_iter()
            .enumerate()
            .map(|(index, file)| {
                let path = file
                    .path
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                reviewed_file(index, path, file.length)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![reviewed_file(0, torrent.name.clone(), torrent.length)?]
    };

    let mut seen_paths = HashSet::with_capacity(files.len());
    let mut total_bytes = 0_u64;
    for file in &files {
        let normalized = file.path.replace('\\', "/").to_lowercase();
        if !seen_paths.insert(normalized) {
            bail!("the selected torrent contains duplicate or case-ambiguous file paths");
        }
        total_bytes = total_bytes
            .checked_add(file.byte_size)
            .context("the selected torrent's total byte size overflows")?;
    }
    if total_bytes == 0 {
        bail!("the selected torrent contains no payload bytes");
    }

    Ok(ManualTorrentOffer {
        source_file_name: source_file_name.to_owned(),
        torrent_name: torrent.name,
        torrent_sha256: hex::encode(Sha256::digest(bytes)),
        info_hash,
        total_bytes,
        files,
        torrent_bytes: bytes.to_vec(),
    })
}

pub(crate) fn queue_manual_torrent(
    settings: &AppSettings,
    password: &str,
    store: &SettingsStore,
    association: CanonicalGameAssociation,
    offer: ManualTorrentOffer,
    selected_index: usize,
) -> Result<String> {
    validate_association(&association)?;
    let selected = offer
        .files
        .get(selected_index)
        .context("choose an exact file from the reviewed torrent")?
        .clone();
    let source_locator = format!("manual-torrent:sha256:{}", offer.torrent_sha256);
    let job = qbittorrent::enqueue(
        settings,
        password,
        store,
        EnqueueRequest {
            game_id: association.game_uid.clone(),
            launchbox_db_id: association.launchbox_db_id,
            title: association.title.clone(),
            platform: association.platform.clone(),
            source_kind: "manual_torrent".to_owned(),
            torrent_url: source_locator,
            torrent_bytes: offer.torrent_bytes,
            selected_file_index: selected.index,
            selected_file_path: selected.path.clone(),
            download_plan: None,
        },
    )?;
    let receipt = ManualTorrentSourceReceipt {
        game_uid: association.game_uid,
        launchbox_db_id: association.launchbox_db_id,
        canonical_title: association.title,
        platform: association.platform,
        source_file_name: offer.source_file_name,
        torrent_name: offer.torrent_name,
        torrent_sha256: offer.torrent_sha256,
        info_hash: job.info_hash.clone(),
        selected_file_index: selected.index,
        selected_file_path: job.torrent_file_path.clone(),
        queued_job_id: job.id.clone(),
        reviewed_at: unix_timestamp(),
    };
    store.record_manual_torrent_enqueue(&receipt, &job)?;
    Ok(job.id)
}

fn reviewed_file(index: usize, path: String, signed_size: i64) -> Result<ReviewedTorrentFile> {
    if path.chars().count() > MAX_TORRENT_PATH_CHARS {
        bail!(
            "the selected torrent contains a file path longer than {MAX_TORRENT_PATH_CHARS} characters"
        );
    }
    let safe_path = qbittorrent::safe_torrent_relative_path(&path)?;
    validate_portable_path(&safe_path)?;
    Ok(ReviewedTorrentFile {
        index: u32::try_from(index).context("torrent file index is too large")?,
        path,
        byte_size: u64::try_from(signed_size).context("torrent file has a negative byte length")?,
    })
}

fn validate_association(association: &CanonicalGameAssociation) -> Result<()> {
    validate_label(&association.game_uid, "stable game identity", 512)?;
    validate_label(&association.title, "canonical game title", 1024)?;
    validate_label(&association.platform, "canonical platform", 512)?;
    if association.launchbox_db_id < 0 {
        bail!("canonical database identifier cannot be negative");
    }
    Ok(())
}

fn validate_label(value: &str, label: &str, limit: usize) -> Result<()> {
    if value.trim().is_empty() || value.chars().count() > limit {
        bail!("{label} must contain between 1 and {limit} characters");
    }
    if value.chars().any(char::is_control) {
        bail!("{label} contains control characters");
    }
    Ok(())
}

fn validate_portable_path(path: &Path) -> Result<()> {
    for component in path.components() {
        validate_portable_component(
            &component.as_os_str().to_string_lossy(),
            "torrent file path",
        )?;
    }
    Ok(())
}

fn validate_portable_component(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || matches!(value, "." | "..")
        || value.ends_with([' ', '.'])
        || value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
                )
        })
    {
        bail!("{label} is not portable across Linux, macOS, and Windows");
    }
    let stem = value.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
    {
        bail!("{label} uses a Windows-reserved filename");
    }
    Ok(())
}

pub(crate) fn probe_fixture_torrent_bytes() -> Vec<u8> {
    use lava_torrent::torrent::v1::File as TorrentFile;

    let torrent = Torrent {
        announce: Some("https://tracker.example.invalid/announce".to_owned()),
        announce_list: None,
        length: 10,
        files: Some(vec![
            TorrentFile {
                length: 4,
                path: std::path::PathBuf::from("Game/Sample Game.rom"),
                extra_fields: None,
            },
            TorrentFile {
                length: 6,
                path: std::path::PathBuf::from("Game/Sample Game (Europe).rom"),
                extra_fields: None,
            },
        ]),
        name: "Sample Pack".to_owned(),
        piece_length: 16_384,
        // lava_torrent preserves byte strings as bytes only when they are not
        // valid UTF-8; real SHA-1 piece hashes commonly satisfy that condition.
        pieces: vec![vec![0xff; 20]],
        extra_fields: None,
        extra_info_fields: None,
    };
    let mut bytes = Vec::new();
    torrent
        .write_into(&mut bytes)
        .expect("fixture torrent encodes");
    bytes
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    struct MockResponse {
        body: String,
        cookie: bool,
    }

    fn mock_server(
        responses: Vec<MockResponse>,
    ) -> (
        SocketAddr,
        mpsc::Receiver<String>,
        std::thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_http_request(&mut stream);
                request_sender.send(request).unwrap();
                let cookie = if response.cookie {
                    "Set-Cookie: SID=lunchbox-test; Path=/\r\n"
                } else {
                    ""
                };
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n{cookie}Connection: close\r\n\r\n{}",
                    response.body.len(),
                    response.body
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        (address, request_receiver, worker)
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_length = None;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if expected_length.is_none()
                && let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let body_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                expected_length = Some(header_end + 4 + body_length);
            }
            if expected_length.is_some_and(|length| bytes.len() >= length) {
                break;
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn inspection_is_exact_bounded_and_provider_neutral() {
        let bytes = probe_fixture_torrent_bytes();
        let offer = inspect_torrent_bytes(&bytes, "review.torrent").unwrap();
        assert_eq!(offer.source_file_name, "review.torrent");
        assert_eq!(offer.torrent_name, "Sample Pack");
        assert_eq!(offer.info_hash.len(), 40);
        assert_eq!(offer.torrent_sha256.len(), 64);
        assert_eq!(offer.total_bytes, 10);
        assert_eq!(
            offer.files,
            vec![
                ReviewedTorrentFile {
                    index: 0,
                    path: "Game/Sample Game.rom".to_owned(),
                    byte_size: 4,
                },
                ReviewedTorrentFile {
                    index: 1,
                    path: "Game/Sample Game (Europe).rom".to_owned(),
                    byte_size: 6,
                },
            ]
        );
    }

    #[test]
    fn inspection_rejects_unsafe_and_nonportable_paths() {
        for path in ["../escape.rom", "Game/CON.rom", "Game/bad:name.rom"] {
            let torrent = Torrent {
                announce: None,
                announce_list: None,
                length: 1,
                files: Some(vec![lava_torrent::torrent::v1::File {
                    length: 1,
                    path: std::path::PathBuf::from(path),
                    extra_fields: None,
                }]),
                name: "Safe Pack".to_owned(),
                piece_length: 16_384,
                pieces: vec![vec![0xff; 20]],
                extra_fields: None,
                extra_info_fields: None,
            };
            let mut bytes = Vec::new();
            torrent.write_into(&mut bytes).unwrap();
            assert!(inspect_torrent_bytes(&bytes, "unsafe.torrent").is_err());
        }
    }

    #[test]
    fn inspection_rejects_oversized_input_before_parsing() {
        let bytes = vec![b'x'; usize::try_from(MAX_TORRENT_BYTES + 1).unwrap()];
        let error = inspect_torrent_bytes(&bytes, "huge.torrent").unwrap_err();
        assert!(error.to_string().contains("review limit"));
    }

    #[test]
    fn reviewed_whole_torrent_enqueue_preserves_exact_source_and_game_provenance() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let offer = inspect_torrent_bytes(
            &probe_fixture_torrent_bytes(),
            "user-reviewed-source.torrent",
        )
        .unwrap();
        let info_hash = offer.info_hash.clone();
        let torrent_sha256 = offer.torrent_sha256.clone();
        let responses = vec![
            MockResponse {
                body: "Ok.".into(),
                cookie: true,
            },
            MockResponse {
                body: "[]".into(),
                cookie: false,
            },
            MockResponse {
                body: r#"{"lunchbox":{"name":"lunchbox","savePath":""}}"#.into(),
                cookie: false,
            },
            MockResponse {
                body: String::new(),
                cookie: false,
            },
            MockResponse {
                body: format!(
                    r#"[{{"hash":"{info_hash}","category":"lunchbox","state":"stoppedDL"}}]"#
                ),
                cookie: false,
            },
            MockResponse {
                body: r#"[{"index":0,"name":"Sample Pack/Game/Sample Game.rom","size":4,"progress":0.0},{"index":1,"name":"Sample Pack/Game/Sample Game (Europe).rom","size":6,"progress":0.0}]"#.into(),
                cookie: false,
            },
            MockResponse {
                body: String::new(),
                cookie: false,
            },
            MockResponse {
                body: String::new(),
                cookie: false,
            },
        ];
        let (address, requests, worker) = mock_server(responses);
        let settings = AppSettings {
            qbittorrent_host: address.ip().to_string(),
            qbittorrent_port: address.port(),
            qbittorrent_username: "lunchbox".into(),
            torrent_library_directory: directory.path().join("downloads"),
            qbittorrent_container_torrent_library_directory: "/downloads".into(),
            rom_directory: directory.path().join("roms"),
            download_entire_torrent: true,
            ..AppSettings::default()
        };
        let association = CanonicalGameAssociation {
            game_uid: "canonical-game-uuid".into(),
            launchbox_db_id: 4242,
            title: "Sample Game".into(),
            platform: "Sample System".into(),
        };

        let job_id =
            queue_manual_torrent(&settings, "secret", &store, association.clone(), offer, 0)
                .unwrap();

        let jobs = store.jobs().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, job_id);
        assert_eq!(jobs[0].game_id, association.game_uid);
        assert_eq!(jobs[0].launchbox_db_id, association.launchbox_db_id);
        assert_eq!(jobs[0].source_kind, "manual_torrent");
        assert_eq!(
            jobs[0].torrent_url,
            format!("manual-torrent:sha256:{torrent_sha256}")
        );
        assert_eq!(jobs[0].info_hash, info_hash);
        assert_eq!(jobs[0].torrent_file_index, None);
        assert_eq!(
            jobs[0].torrent_file_path,
            "Sample Pack/Game/Sample Game.rom"
        );
        let receipts = store.manual_torrent_sources().unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].game_uid, association.game_uid);
        assert_eq!(receipts[0].selected_file_index, 0);
        assert_eq!(receipts[0].selected_file_path, jobs[0].torrent_file_path);
        assert_eq!(receipts[0].torrent_sha256, torrent_sha256);

        let captured = (0..8).map(|_| requests.recv().unwrap()).collect::<Vec<_>>();
        assert!(captured[0].starts_with("POST /api/v2/auth/login HTTP/1.1"));
        assert!(captured[3].starts_with("POST /api/v2/torrents/add HTTP/1.1"));
        assert!(captured[3].contains("filename=\"lunchbox-source.torrent\""));
        assert!(captured[5].starts_with(&format!(
            "GET /api/v2/torrents/files?hash={info_hash} HTTP/1.1"
        )));
        assert!(captured[6].contains("id=0%7C1&priority=1"));
        assert!(captured[7].starts_with("POST /api/v2/torrents/start HTTP/1.1"));
        worker.join().unwrap();
    }
}
