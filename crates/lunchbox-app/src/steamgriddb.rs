use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::media::ArtworkKind;
use crate::provider_image::{approved_url, download_and_publish};

const DEFAULT_API_BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct GameCandidate {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ArtworkCandidate {
    pub id: i64,
    pub url: String,
    pub thumb: String,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub author: ArtworkAuthor,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ArtworkAuthor {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(default)]
    data: T,
    #[serde(default)]
    errors: Vec<String>,
}

pub struct Client {
    api_key: String,
    base_url: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let api_key = api_key.into().trim().to_owned();
        if api_key.is_empty() {
            bail!("a SteamGridDB API key is required");
        }
        let base_url = std::env::var("LUNCHBOX_STEAMGRIDDB_API_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_owned())
            .trim_end_matches('/')
            .to_owned();
        if !approved_url(&base_url) {
            bail!("SteamGridDB API URL must use HTTPS");
        }
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(20)))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            api_key,
            base_url,
            agent,
        })
    }

    pub fn test_connection(&self) -> Result<()> {
        let _: Vec<GameCandidate> = self.get("/search/autocomplete/Lunchbox")?;
        Ok(())
    }

    pub fn search_games(&self, query: &str) -> Result<Vec<GameCandidate>> {
        let query = query.trim();
        if query.chars().count() < 2 {
            bail!("enter at least two characters to search SteamGridDB");
        }
        let path = format!("/search/autocomplete/{}", percent_encode_segment(query));
        let mut games: Vec<GameCandidate> = self.get(&path)?;
        games.retain(|game| game.id > 0 && !game.name.trim().is_empty());
        games.truncate(50);
        Ok(games)
    }

    pub fn artwork_for_game(
        &self,
        game_id: i64,
        kind: ArtworkKind,
    ) -> Result<Vec<ArtworkCandidate>> {
        if game_id <= 0 {
            bail!("SteamGridDB game ID must be positive");
        }
        let endpoint = endpoint_for_kind(kind)?;
        let path = format!(
            "/{endpoint}/game/{game_id}?types=static&nsfw=false&humor=false&epilepsy=false&limit=50"
        );
        let mut artwork: Vec<ArtworkCandidate> = self.get(&path)?;
        artwork.retain(|candidate| {
            candidate.id > 0 && approved_url(&candidate.url) && approved_url(&candidate.thumb)
        });
        Ok(artwork)
    }

    pub fn download_and_publish(
        &self,
        candidate: &ArtworkCandidate,
        media_root: &Path,
        database_id: i64,
        kind: ArtworkKind,
    ) -> Result<PathBuf> {
        download_and_publish(
            &self.agent,
            &candidate.url,
            media_root,
            database_id,
            "steamgriddb",
            kind,
            &candidate.id.to_string(),
        )
    }

    fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned + Default,
    {
        let url = format!("{}{path}", self.base_url);
        let mut response = self
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .header("User-Agent", "Lunchbox/0.1 SteamGridDB client")
            .call()
            .with_context(|| format!("requesting SteamGridDB API {path}"))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .context("reading SteamGridDB API response")?;
        let parsed = serde_json::from_str::<ApiResponse<T>>(&body);
        if !(200..300).contains(&status) {
            let detail = parsed
                .ok()
                .map(|response| response.errors.join("; "))
                .filter(|detail| !detail.is_empty())
                .unwrap_or_else(|| concise_body(&body));
            bail!("SteamGridDB request failed with HTTP {status}: {detail}");
        }
        let parsed = parsed.context("parsing SteamGridDB API response")?;
        if !parsed.success {
            bail!("SteamGridDB request failed: {}", parsed.errors.join("; "));
        }
        Ok(parsed.data)
    }
}

fn endpoint_for_kind(kind: ArtworkKind) -> Result<&'static str> {
    match kind {
        ArtworkKind::BoxFront => Ok("grids"),
        ArtworkKind::Fanart => Ok("heroes"),
        ArtworkKind::ClearLogo => Ok("logos"),
        ArtworkKind::BoxBack
        | ArtworkKind::Box3d
        | ArtworkKind::Screenshot
        | ArtworkKind::TitleScreen
        | ArtworkKind::CartFront
        | ArtworkKind::CartBack
        | ArtworkKind::Cart3d
        | ArtworkKind::Disc
        | ArtworkKind::Marquee
        | ArtworkKind::Banner
        | ArtworkKind::Flyer
        | ArtworkKind::GameOverScreen => {
            bail!("SteamGridDB does not provide this Lunchbox artwork category")
        }
    }
}

fn percent_encode_segment(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            output.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

fn concise_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        "no response body".to_owned()
    } else {
        body.chars().take(240).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc;

    use super::*;

    struct MockResponse {
        content_type: &'static str,
        body: Vec<u8>,
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
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let _ = sender.send(read_request(&mut stream));
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
                    response.body.len(),
                    response.content_type
                );
                stream.write_all(reply.as_bytes()).unwrap();
                stream.write_all(&response.body).unwrap();
                stream.flush().unwrap();
            }
        });
        (address, receiver, worker)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[test]
    fn search_and_reviewed_artwork_use_bearer_auth_and_stable_ids() {
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                content_type: "application/json",
                body: br#"{"success":true,"data":[{"id":101,"name":"Exact Game","types":["game"],"verified":true}]}"#.to_vec(),
            },
            MockResponse {
                content_type: "application/json",
                body: br#"{"success":true,"data":[{"id":501,"url":"https://cdn.example/image.png","thumb":"https://cdn.example/thumb.png","score":12,"author":{"name":"Artist"}}]}"#.to_vec(),
            },
        ]);
        let client = Client::new_with_base_url_for_test("secret", format!("http://{address}"));
        let games = client.search_games("Exact Game").unwrap();
        assert_eq!(games[0].id, 101);
        let artwork = client.artwork_for_game(101, ArtworkKind::Fanart).unwrap();
        assert_eq!(artwork[0].id, 501);
        let search_request = requests.recv().unwrap();
        let artwork_request = requests.recv().unwrap();
        assert!(search_request.starts_with("GET /search/autocomplete/Exact%20Game "));
        assert!(
            search_request
                .to_ascii_lowercase()
                .contains("authorization: bearer secret")
        );
        assert!(artwork_request.starts_with("GET /heroes/game/101?"));
        worker.join().unwrap();
    }

    #[test]
    fn publishing_validates_image_bytes_and_replaces_only_same_provider_kind() {
        let png = b"\x89PNG\r\n\x1a\nfixture".to_vec();
        let (address, _, worker) = mock_server(vec![MockResponse {
            content_type: "image/png",
            body: png.clone(),
        }]);
        let client = Client::new_with_base_url_for_test("secret", format!("http://{address}"));
        let directory = tempfile::tempdir().unwrap();
        let old = directory
            .path()
            .join("lb-27/steamgriddb/fanart-background.jpg");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, b"old").unwrap();
        let candidate = ArtworkCandidate {
            id: 77,
            url: format!("http://{address}/image.png"),
            thumb: format!("http://{address}/thumb.png"),
            score: 0,
            style: String::new(),
            author: ArtworkAuthor::default(),
        };
        let output = client
            .download_and_publish(&candidate, directory.path(), 27, ArtworkKind::Fanart)
            .unwrap();
        assert_eq!(fs::read(&output).unwrap(), png);
        assert!(!old.exists());
        assert!(output.ends_with("lb-27/steamgriddb/fanart-background.png"));
        worker.join().unwrap();
    }

    #[test]
    fn invalid_image_content_is_rejected_before_publication() {
        let (address, _, worker) = mock_server(vec![MockResponse {
            content_type: "text/html",
            body: b"not an image".to_vec(),
        }]);
        let client = Client::new_with_base_url_for_test("secret", format!("http://{address}"));
        let directory = tempfile::tempdir().unwrap();
        let candidate = ArtworkCandidate {
            id: 77,
            url: format!("http://{address}/image.png"),
            thumb: format!("http://{address}/thumb.png"),
            score: 0,
            style: String::new(),
            author: ArtworkAuthor::default(),
        };
        assert!(
            client
                .download_and_publish(&candidate, directory.path(), 27, ArtworkKind::BoxFront)
                .is_err()
        );
        assert!(!directory.path().join("lb-27").exists());
        worker.join().unwrap();
    }

    #[test]
    fn supported_artwork_kinds_use_the_official_routes() {
        assert_eq!(endpoint_for_kind(ArtworkKind::BoxFront).unwrap(), "grids");
        assert_eq!(endpoint_for_kind(ArtworkKind::Fanart).unwrap(), "heroes");
        assert_eq!(endpoint_for_kind(ArtworkKind::ClearLogo).unwrap(), "logos");
        assert!(endpoint_for_kind(ArtworkKind::Screenshot).is_err());
    }

    impl Client {
        fn new_with_base_url_for_test(api_key: &str, base_url: String) -> Self {
            let agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(3)))
                .http_status_as_error(false)
                .build()
                .into();
            Self {
                api_key: api_key.to_owned(),
                base_url,
                agent,
            }
        }
    }
}
