use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::media::ArtworkKind;
use crate::provider_image::{approved_url, download_and_publish};

const DEFAULT_TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const DEFAULT_API_BASE_URL: &str = "https://api.igdb.com/v4";
const DEFAULT_IMAGE_BASE_URL: &str = "https://images.igdb.com/igdb/image/upload";

static TOKEN_CACHE: OnceLock<Mutex<HashMap<String, CachedToken>>> = OnceLock::new();

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct GameCandidate {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub storyline: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub aggregated_rating: Option<f64>,
    #[serde(default)]
    pub first_release_date: Option<i64>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub involved_companies: Vec<InvolvedCompany>,
    #[serde(default)]
    pub platforms: Vec<Platform>,
    #[serde(default)]
    pub cover: Option<Image>,
    #[serde(default)]
    pub screenshots: Vec<Image>,
    #[serde(default)]
    pub artworks: Vec<Image>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Genre {
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InvolvedCompany {
    #[serde(default)]
    pub company: Option<Company>,
    #[serde(default)]
    pub developer: bool,
    #[serde(default)]
    pub publisher: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Company {
    #[serde(default)]
    pub name: String,
}

impl GameCandidate {
    pub fn metadata_value(&self, field: &str) -> String {
        match field.trim() {
            "title" => self.name.trim().to_owned(),
            "description" => self
                .summary
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    self.storyline
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                })
                .map(str::trim)
                .unwrap_or_default()
                .to_owned(),
            "release_date" => self
                .first_release_date
                .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
                .map(|value| value.date_naive().format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            "developer" => self.company_names(|company| company.developer),
            "publisher" => self.company_names(|company| company.publisher),
            "genre" => joined_unique(self.genres.iter().map(|genre| genre.name.as_str())),
            "players" => String::new(),
            "rating" => self
                .aggregated_rating
                .or(self.rating)
                .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
                .map(|value| format!("{:.1}", value / 20.0))
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn company_names(&self, include: impl Fn(&InvolvedCompany) -> bool) -> String {
        joined_unique(self.involved_companies.iter().filter_map(|involved| {
            include(involved).then_some(involved.company.as_ref()?.name.as_str())
        }))
    }
}

fn joined_unique<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    let mut output = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty()
            && !output
                .iter()
                .any(|stored: &&str| stored.eq_ignore_ascii_case(value))
        {
            output.push(value);
        }
    }
    output.join(", ")
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Platform {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub abbreviation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Image {
    pub id: i64,
    pub image_id: String,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtworkCandidate {
    pub id: String,
    pub url: String,
    pub thumb: String,
    pub source: &'static str,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

pub struct Client {
    client_id: String,
    client_secret: String,
    token_url: String,
    api_base_url: String,
    image_base_url: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Result<Self> {
        let client_id = client_id.into().trim().to_owned();
        let client_secret = client_secret.into().trim().to_owned();
        if client_id.is_empty() || client_secret.is_empty() {
            bail!("an IGDB Twitch client ID and client secret are required");
        }
        let token_url = configured_url("LUNCHBOX_IGDB_TOKEN_URL", DEFAULT_TOKEN_URL);
        let api_base_url = configured_url("LUNCHBOX_IGDB_API_URL", DEFAULT_API_BASE_URL);
        let image_base_url = configured_url("LUNCHBOX_IGDB_IMAGE_URL", DEFAULT_IMAGE_BASE_URL);
        for (label, url) in [
            ("IGDB token", &token_url),
            ("IGDB API", &api_base_url),
            ("IGDB image", &image_base_url),
        ] {
            if !approved_url(url) {
                bail!("{label} URL must use HTTPS");
            }
        }
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(25)))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            client_id,
            client_secret,
            token_url,
            api_base_url: api_base_url.trim_end_matches('/').to_owned(),
            image_base_url: image_base_url.trim_end_matches('/').to_owned(),
            agent,
        })
    }

    pub fn test_connection(&self) -> Result<()> {
        let body = format!("fields {}; limit 1;", game_fields());
        let games: Vec<GameCandidate> = self.post_games(&body)?;
        if games.is_empty() {
            bail!("IGDB connection succeeded but its catalog query returned no records");
        }
        Ok(())
    }

    pub fn search_games(&self, query: &str) -> Result<Vec<GameCandidate>> {
        let query = query.trim();
        if query.chars().count() < 2 {
            bail!("enter at least two characters to search IGDB");
        }
        let body = format!(
            "search \"{}\"; fields {}; where version_parent = null; limit 50;",
            escape_apicalypse_string(query),
            game_fields()
        );
        let mut games: Vec<GameCandidate> = self.post_games(&body)?;
        games.retain(|game| game.id > 0 && !game.name.trim().is_empty());
        games.truncate(50);
        Ok(games)
    }

    pub fn game(&self, game_id: i64) -> Result<GameCandidate> {
        if game_id <= 0 {
            bail!("IGDB game ID must be positive");
        }
        let body = format!("fields {}; where id = {game_id}; limit 1;", game_fields());
        self.post_games::<Vec<GameCandidate>>(&body)?
            .into_iter()
            .next()
            .context("the reviewed IGDB game no longer exists")
    }

    pub fn artwork_for_game(
        &self,
        game_id: i64,
        kind: ArtworkKind,
    ) -> Result<Vec<ArtworkCandidate>> {
        let game = self.game(game_id)?;
        self.artwork_for_candidate(&game, kind)
    }

    pub(crate) fn artwork_for_candidate(
        &self,
        game: &GameCandidate,
        kind: ArtworkKind,
    ) -> Result<Vec<ArtworkCandidate>> {
        let (images, source, thumbnail_size, full_size): (
            Vec<Image>,
            &'static str,
            &'static str,
            &'static str,
        ) = match kind {
            ArtworkKind::BoxFront => (
                game.cover.clone().into_iter().collect(),
                "cover",
                "cover_big",
                "cover_big_2x",
            ),
            ArtworkKind::Fanart => {
                let mut images = game.artworks.clone();
                let artwork_count = images.len();
                images.extend(game.screenshots.clone());
                return Ok(images
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, image)| {
                        self.artwork_candidate(
                            image,
                            if index < artwork_count {
                                "artwork"
                            } else {
                                "screenshot"
                            },
                            "screenshot_med",
                            "1080p",
                        )
                    })
                    .collect());
            }
            ArtworkKind::Screenshot | ArtworkKind::GameOverScreen => (
                game.screenshots.clone(),
                "screenshot",
                "screenshot_med",
                "1080p",
            ),
            ArtworkKind::BoxBack
            | ArtworkKind::Box3d
            | ArtworkKind::TitleScreen
            | ArtworkKind::ClearLogo
            | ArtworkKind::CartFront
            | ArtworkKind::CartBack
            | ArtworkKind::Cart3d
            | ArtworkKind::Disc
            | ArtworkKind::Marquee
            | ArtworkKind::Banner
            | ArtworkKind::Flyer => {
                bail!("IGDB does not provide this Lunchbox artwork category")
            }
        };
        Ok(images
            .into_iter()
            .filter_map(|image| self.artwork_candidate(image, source, thumbnail_size, full_size))
            .collect())
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
            "igdb",
            kind,
            &candidate.id,
        )
    }

    fn artwork_candidate(
        &self,
        image: Image,
        source: &'static str,
        thumbnail_size: &str,
        full_size: &str,
    ) -> Option<ArtworkCandidate> {
        if image.id <= 0 || image.image_id.trim().is_empty() {
            return None;
        }
        let thumb = self.image_url(thumbnail_size, &image.image_id);
        let url = self.image_url(full_size, &image.image_id);
        if !approved_url(&thumb) || !approved_url(&url) {
            return None;
        }
        Some(ArtworkCandidate {
            id: image.image_id,
            url,
            thumb,
            source,
            width: image.width,
            height: image.height,
        })
    }

    fn image_url(&self, size: &str, image_id: &str) -> String {
        format!("{}/t_{size}/{image_id}.jpg", self.image_base_url)
    }

    fn post_games<T>(&self, body: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let token = self.access_token()?;
        let url = format!("{}/games", self.api_base_url);
        let mut response = self
            .agent
            .post(&url)
            .header("Client-ID", &self.client_id)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Accept", "application/json")
            .header("Content-Type", "text/plain")
            .header("User-Agent", "Lunchbox/0.1 IGDB artwork client")
            .send(body)
            .context("requesting the IGDB games endpoint")?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .context("reading the IGDB games response")?;
        if !(200..300).contains(&status) {
            bail!(
                "IGDB request failed with HTTP {status}: {}",
                concise_body(&body)
            );
        }
        serde_json::from_str(&body).context("parsing the IGDB games response")
    }

    fn access_token(&self) -> Result<String> {
        let cache_key = self.token_cache_key();
        let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache
            .lock()
            .map_err(|_| anyhow::anyhow!("IGDB access-token cache was poisoned"))?;
        if let Some(cached) = cache.get(&cache_key)
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.access_token.clone());
        }
        let mut token_url =
            url::Url::parse(&self.token_url).context("parsing the configured IGDB token URL")?;
        token_url.query_pairs_mut().clear().extend_pairs([
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "client_credentials"),
        ]);
        let mut response = self
            .agent
            .post(token_url.as_str())
            .header("Accept", "application/json")
            .header("User-Agent", "Lunchbox/0.1 IGDB authentication client")
            .send_empty()
            .context("requesting an IGDB application access token from Twitch")?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .context("reading the Twitch access-token response")?;
        if !(200..300).contains(&status) {
            bail!("{}", token_error_message(status, &body));
        }
        let token: TokenResponse =
            serde_json::from_str(&body).context("parsing the Twitch access-token response")?;
        if token.access_token.trim().is_empty() || token.expires_in == 0 {
            bail!("Twitch returned an invalid IGDB access token");
        }
        let lifetime = token.expires_in.saturating_sub(60).max(1);
        cache.insert(
            cache_key,
            CachedToken {
                access_token: token.access_token.clone(),
                expires_at: Instant::now() + Duration::from_secs(lifetime),
            },
        );
        Ok(token.access_token)
    }

    fn token_cache_key(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.client_secret.hash(&mut hasher);
        format!(
            "{}|{}|{:016x}",
            self.client_id,
            self.token_url,
            hasher.finish()
        )
    }
}

fn configured_url(variable: &str, default: &str) -> String {
    std::env::var(variable)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn game_fields() -> &'static str {
    "id,name,summary,storyline,rating,aggregated_rating,first_release_date,genres.name,involved_companies.company.name,involved_companies.developer,involved_companies.publisher,platforms.name,platforms.abbreviation,cover.image_id,cover.width,cover.height,screenshots.image_id,screenshots.width,screenshots.height,artworks.image_id,artworks.width,artworks.height"
}

fn escape_apicalypse_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' | '\r' | '\0' => escaped.push(' '),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn concise_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        "no response body".to_owned()
    } else {
        body.chars().take(240).collect()
    }
}

fn token_error_message(status: u16, body: &str) -> String {
    let provider_message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("message")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| concise_body(body));
    if provider_message
        .to_ascii_lowercase()
        .contains("invalid client")
    {
        return format!(
            "Twitch rejected the application credentials (HTTP {status}). Use the Client ID and newest Client Secret from a Confidential Twitch Developer application. An IGDB login, Twitch password, or IGDB MCP credential will not work; generating a new secret invalidates the previous one."
        );
    }
    format!(
        "Twitch rejected the IGDB application credentials with HTTP {status}: {provider_message}"
    )
}

#[cfg(test)]
mod tests {
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
        let mut content_length = None;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let header_end = header_end + 4;
                if content_length.is_none() {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers.lines().find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .or_else(|| line.strip_prefix("content-length: "))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    });
                }
                if bytes.len() >= header_end + content_length.unwrap_or(0) {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn test_client(address: SocketAddr) -> Client {
        let root = format!("http://{address}");
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(3)))
            .http_status_as_error(false)
            .build()
            .into();
        Client {
            client_id: "client-id".to_owned(),
            client_secret: "client-secret".to_owned(),
            token_url: format!("{root}/oauth2/token"),
            api_base_url: format!("{root}/v4"),
            image_base_url: format!("{root}/images"),
            agent,
        }
    }

    #[test]
    fn oauth_search_and_artwork_use_reviewable_stable_ids() {
        let game = br#"[{"id":7346,"name":"Super Mario Bros.","summary":"Rescue the Mushroom Kingdom.","rating":86.0,"first_release_date":-141177600,"genres":[{"name":"Platform"}],"involved_companies":[{"company":{"name":"Nintendo R&D4"},"developer":true,"publisher":false},{"company":{"name":"Nintendo"},"developer":false,"publisher":true}],"platforms":[{"name":"Nintendo Entertainment System","abbreviation":"NES"}],"cover":{"id":11,"image_id":"co1abc","width":264,"height":374},"screenshots":[{"id":12,"image_id":"sc1abc","width":1280,"height":720}],"artworks":[{"id":13,"image_id":"ar1abc","width":1920,"height":1080}]}]"#;
        let (address, requests, worker) = mock_server(vec![
            MockResponse {
                content_type: "application/json",
                body:
                    br#"{"access_token":"fixture-token","expires_in":3600,"token_type":"bearer"}"#
                        .to_vec(),
            },
            MockResponse {
                content_type: "application/json",
                body: game.to_vec(),
            },
            MockResponse {
                content_type: "application/json",
                body: game.to_vec(),
            },
        ]);
        let client = test_client(address);
        let games = client.search_games("Super \"Mario\"").unwrap();
        assert_eq!(games[0].id, 7346);
        assert_eq!(
            games[0].metadata_value("description"),
            "Rescue the Mushroom Kingdom."
        );
        assert_eq!(games[0].metadata_value("release_date"), "1965-07-12");
        assert_eq!(games[0].metadata_value("developer"), "Nintendo R&D4");
        assert_eq!(games[0].metadata_value("publisher"), "Nintendo");
        assert_eq!(games[0].metadata_value("genre"), "Platform");
        assert_eq!(games[0].metadata_value("rating"), "4.3");
        let artwork = client.artwork_for_game(7346, ArtworkKind::Fanart).unwrap();
        assert_eq!(artwork.len(), 2);
        assert_eq!(artwork[0].id, "ar1abc");
        assert!(artwork[0].url.ends_with("/t_1080p/ar1abc.jpg"));

        let token_request = requests.recv().unwrap();
        assert!(token_request.starts_with("POST /oauth2/token?"));
        assert!(token_request.contains("client_id=client-id"));
        assert!(token_request.contains("client_secret=client-secret"));
        assert!(token_request.contains("grant_type=client_credentials"));
        let search_request = requests.recv().unwrap();
        assert!(search_request.starts_with("POST /v4/games "));
        assert!(
            search_request
                .to_ascii_lowercase()
                .contains("client-id: client-id")
        );
        assert!(
            search_request
                .to_ascii_lowercase()
                .contains("authorization: bearer fixture-token")
        );
        assert!(search_request.contains("search \"Super \\\"Mario\\\"\";"));
        let linked_request = requests.recv().unwrap();
        assert!(linked_request.contains("where id = 7346;"));
        worker.join().unwrap();
    }

    #[test]
    fn publishing_uses_the_shared_igdb_cache_boundary() {
        let jpeg = b"\xff\xd8\xfffixture".to_vec();
        let (address, _, worker) = mock_server(vec![MockResponse {
            content_type: "image/jpeg",
            body: jpeg.clone(),
        }]);
        let client = test_client(address);
        let directory = tempfile::tempdir().unwrap();
        let other_provider = directory
            .path()
            .join("lb-140/steamgriddb/fanart-background.png");
        std::fs::create_dir_all(other_provider.parent().unwrap()).unwrap();
        std::fs::write(&other_provider, b"other").unwrap();
        let candidate = ArtworkCandidate {
            id: "ar1abc".to_owned(),
            url: format!("http://{address}/images/t_1080p/ar1abc.jpg"),
            thumb: format!("http://{address}/images/t_screenshot_med/ar1abc.jpg"),
            source: "artwork",
            width: 1920,
            height: 1080,
        };
        let output = client
            .download_and_publish(&candidate, directory.path(), 140, ArtworkKind::Fanart)
            .unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), jpeg);
        assert_eq!(std::fs::read(&other_provider).unwrap(), b"other");
        assert!(output.ends_with("lb-140/igdb/fanart-background.jpg"));
        worker.join().unwrap();
    }

    #[test]
    fn apicalypse_search_input_is_escaped_without_changing_identity() {
        assert_eq!(
            escape_apicalypse_string("Line \"one\"\nnext"),
            "Line \\\"one\\\" next"
        );
    }

    #[test]
    fn invalid_client_error_explains_the_required_twitch_application_credentials() {
        let message = token_error_message(400, r#"{"status":400,"message":"invalid client"}"#);
        assert!(message.contains("Confidential Twitch Developer application"));
        assert!(message.contains("newest Client Secret"));
        assert!(!message.contains("client-secret"));
    }
}
