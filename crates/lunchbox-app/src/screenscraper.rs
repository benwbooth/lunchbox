use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::media::ArtworkKind;
use crate::provider_image::{approved_url, download_and_publish};
use crate::settings::ScreenScraperCredentials;

const DEFAULT_API_BASE_URL: &str = "https://api.screenscraper.fr/api2";
const MAX_API_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameCandidate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub developer: String,
    pub publisher: String,
    pub release_date: String,
    pub genre: String,
    pub players: String,
    pub rating: String,
    media: Vec<MediaEntry>,
}

impl GameCandidate {
    pub fn metadata_value(&self, field: &str) -> &str {
        match field.trim() {
            "title" => &self.name,
            "description" => &self.description,
            "release_date" => &self.release_date,
            "developer" => &self.developer,
            "publisher" => &self.publisher,
            "genre" => &self.genre,
            "players" => &self.players,
            "rating" => &self.rating,
            _ => "",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtworkCandidate {
    pub id: String,
    pub url: String,
    pub thumb: String,
    pub media_type: String,
    pub region: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct LocalizedText {
    #[serde(default)]
    region: String,
    #[serde(default)]
    text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct TextValue {
    #[serde(default)]
    text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct SynopsisText {
    #[serde(default)]
    langue: String,
    #[serde(default)]
    text: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct GenrePayload {
    #[serde(default)]
    noms: Vec<LocalizedText>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct MediaEntry {
    #[serde(rename = "type", default)]
    media_type: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    region: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct GamePayload {
    #[serde(default)]
    id: String,
    #[serde(default)]
    noms: Vec<LocalizedText>,
    #[serde(default)]
    synopsis: Vec<SynopsisText>,
    #[serde(default)]
    developpeur: Option<TextValue>,
    #[serde(default)]
    editeur: Option<TextValue>,
    #[serde(default)]
    dates: Vec<LocalizedText>,
    #[serde(default)]
    genres: Vec<GenrePayload>,
    #[serde(default)]
    joueurs: Option<TextValue>,
    #[serde(default)]
    note: Option<TextValue>,
    #[serde(default)]
    medias: Vec<MediaEntry>,
}

pub struct Client {
    credentials: ScreenScraperCredentials,
    base_url: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(credentials: ScreenScraperCredentials) -> Result<Self> {
        let base_url = std::env::var("LUNCHBOX_SCREENSCRAPER_API_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_owned())
            .trim_end_matches('/')
            .to_owned();
        Self::with_base_url(credentials, base_url)
    }

    fn with_base_url(
        credentials: ScreenScraperCredentials,
        base_url: impl Into<String>,
    ) -> Result<Self> {
        let credentials = ScreenScraperCredentials::validated(
            &credentials.developer_id,
            &credentials.developer_password,
            credentials.user_id.as_deref().unwrap_or_default(),
            credentials.user_password.as_deref().unwrap_or_default(),
        )?;
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        if !approved_url(&base_url) {
            bail!("ScreenScraper API URL must use HTTPS");
        }
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(35)))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            credentials,
            base_url,
            agent,
        })
    }

    pub fn test_connection(&self) -> Result<()> {
        let body = self.get("ssinfraInfos.php", &[])?;
        let lowered = body.to_ascii_lowercase();
        if lowered.contains("erreur") || lowered.contains("error") {
            bail!(
                "ScreenScraper rejected the developer credentials: {}",
                concise_body(&body)
            );
        }
        Ok(())
    }

    pub fn search_games(&self, query: &str, platform: &str) -> Result<Vec<GameCandidate>> {
        let query = query.trim();
        if query.chars().count() < 2 {
            bail!("enter at least two characters to search ScreenScraper");
        }
        let platform_id = platform_id(platform);
        let mut pairs = vec![("recherche", query.to_owned())];
        if let Some(platform_id) = platform_id {
            pairs.push(("systemeid", platform_id.to_string()));
        }
        let body = self.get("jeuRecherche.php", &pairs)?;
        let mut games = parse_games(&body)?;
        games.sort_by(|left, right| {
            title_rank(&left.name, query)
                .cmp(&title_rank(&right.name, query))
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.id.cmp(&right.id))
        });
        games.dedup_by_key(|game| game.id);
        games.truncate(30);
        Ok(games)
    }

    pub fn game(&self, game_id: i64) -> Result<GameCandidate> {
        if game_id <= 0 {
            bail!("ScreenScraper game ID must be positive");
        }
        let body = self.get("jeuInfos.php", &[("gameid", game_id.to_string())])?;
        parse_games(&body)?
            .into_iter()
            .find(|game| game.id == game_id)
            .with_context(|| format!("ScreenScraper did not return game ID {game_id}"))
    }

    pub fn artwork_for_game(
        &self,
        game: &GameCandidate,
        kind: ArtworkKind,
    ) -> Result<Vec<ArtworkCandidate>> {
        artwork_candidates(game, kind)
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
            "screenscraper",
            kind,
            &candidate.id,
        )
    }

    pub fn download_linked_artwork(
        &self,
        game_id: i64,
        media_root: &Path,
        database_id: i64,
        requested_kind: ArtworkKind,
        exact_only: bool,
    ) -> Result<Option<(ArtworkKind, PathBuf)>> {
        let game = self.game(game_id)?;
        for kind in crate::media::retrieval_kinds(requested_kind, exact_only) {
            let Some(candidate) = self.artwork_for_game(&game, kind)?.into_iter().next() else {
                continue;
            };
            let path = self.download_and_publish(&candidate, media_root, database_id, kind)?;
            return Ok(Some((kind, path)));
        }
        Ok(None)
    }

    fn get(&self, endpoint: &str, pairs: &[(&str, String)]) -> Result<String> {
        let mut url = url::Url::parse(&format!("{}/{}", self.base_url, endpoint))
            .context("parsing the ScreenScraper API URL")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("devid", &self.credentials.developer_id);
            query.append_pair("devpassword", &self.credentials.developer_password);
            query.append_pair("softname", "lunchbox");
            query.append_pair("output", "json");
            if let (Some(user_id), Some(user_password)) = (
                self.credentials.user_id.as_deref(),
                self.credentials.user_password.as_deref(),
            ) {
                query.append_pair("ssid", user_id);
                query.append_pair("sspassword", user_password);
            }
            for (name, value) in pairs {
                query.append_pair(name, value);
            }
        }
        let mut response = self
            .agent
            .get(url.as_str())
            .header("Accept", "application/json")
            .header("User-Agent", "Lunchbox/0.1 ScreenScraper client")
            .call()
            .with_context(|| format!("requesting ScreenScraper {endpoint}"))?;
        let status = response.status().as_u16();
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_API_RESPONSE_BYTES + 1)
            .read_to_string(&mut body)
            .with_context(|| format!("reading ScreenScraper {endpoint} response"))?;
        if body.len() as u64 > MAX_API_RESPONSE_BYTES {
            bail!("ScreenScraper response exceeded the 4 MiB safety limit");
        }
        if !(200..300).contains(&status) {
            bail!(
                "ScreenScraper request failed with HTTP {status}: {}",
                concise_body(&body)
            );
        }
        Ok(body)
    }
}

pub(crate) fn configured_credentials() -> Result<Option<ScreenScraperCredentials>> {
    let developer_id = environment("LUNCHBOX_SCREENSCRAPER_DEVELOPER_ID");
    let developer_password = environment("LUNCHBOX_SCREENSCRAPER_DEVELOPER_PASSWORD");
    let user_id = environment("LUNCHBOX_SCREENSCRAPER_USER_ID");
    let user_password = environment("LUNCHBOX_SCREENSCRAPER_USER_PASSWORD");
    if developer_id.is_some()
        || developer_password.is_some()
        || user_id.is_some()
        || user_password.is_some()
    {
        return ScreenScraperCredentials::validated(
            developer_id.as_deref().unwrap_or_default(),
            developer_password.as_deref().unwrap_or_default(),
            user_id.as_deref().unwrap_or_default(),
            user_password.as_deref().unwrap_or_default(),
        )
        .map(Some);
    }
    crate::settings::load_screenscraper_credentials()
}

pub(crate) fn effective_credentials(
    developer_id: &str,
    developer_password: &str,
    user_id: &str,
    user_password: &str,
) -> Result<ScreenScraperCredentials> {
    if [developer_id, developer_password, user_id, user_password]
        .iter()
        .any(|value| !value.trim().is_empty())
    {
        return ScreenScraperCredentials::validated(
            developer_id,
            developer_password,
            user_id,
            user_password,
        );
    }
    configured_credentials()?.ok_or_else(|| {
        anyhow::anyhow!(
            "no ScreenScraper credentials are saved; add authorized developer credentials in Settings"
        )
    })
}

fn environment(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub fn artwork_candidates(
    game: &GameCandidate,
    kind: ArtworkKind,
) -> Result<Vec<ArtworkCandidate>> {
    let mut artwork = game
        .media
        .iter()
        .enumerate()
        .filter(|(_, media)| media_kind(&media.media_type) == Some(kind))
        .filter(|(_, media)| approved_url(&media.url))
        .map(|(index, media)| ArtworkCandidate {
            id: format!("{}-{}-{index}", game.id, kind.key()),
            url: media.url.clone(),
            thumb: media.url.clone(),
            media_type: media.media_type.clone(),
            region: media.region.clone(),
        })
        .collect::<Vec<_>>();
    artwork.sort_by(|left, right| {
        region_rank(&left.region)
            .cmp(&region_rank(&right.region))
            .then_with(|| left.media_type.cmp(&right.media_type))
            .then_with(|| left.url.cmp(&right.url))
    });
    artwork.dedup_by(|left, right| left.url == right.url);
    Ok(artwork)
}

pub fn platform_id(platform: &str) -> Option<i32> {
    let normalized = platform
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match normalized.as_str() {
        "nintendoentertainmentsystem" | "nintendones" | "nes" => Some(3),
        "supernintendoentertainmentsystem" | "supernintendo" | "snes" => Some(4),
        "nintendo64" | "n64" => Some(14),
        "nintendogameboyadvance" | "gameboyadvance" | "gba" => Some(12),
        "nintendogameboycolor" | "gameboycolor" | "gbc" => Some(11),
        "nintendogameboy" | "gameboy" | "gb" => Some(10),
        "nintendods" | "nds" => Some(15),
        "nintendo3ds" | "3ds" => Some(17),
        "nintendogamecube" | "gamecube" | "ngc" => Some(13),
        "nintendowiiu" | "wiiu" => Some(18),
        "nintendowii" | "wii" => Some(16),
        "nintendoswitch" | "switch" => Some(225),
        "segagenesis" | "genesis" | "segamegadrive" | "megadrive" => Some(1),
        "segamastersystem" | "mastersystem" => Some(2),
        "segagamegear" | "gamegear" => Some(21),
        "segasaturn" | "saturn" => Some(22),
        "segadreamcast" | "dreamcast" => Some(23),
        "segacd" | "megacd" => Some(20),
        "sega32x" | "32x" => Some(19),
        "sonyplaystation2" | "playstation2" | "ps2" => Some(58),
        "sonyplaystation3" | "playstation3" | "ps3" => Some(59),
        "sonypsp" | "playstationportable" | "psp" => Some(61),
        "sonyplaystationvita" | "playstationvita" | "psvita" | "vita" => Some(62),
        "sonyplaystation" | "playstation" | "ps1" | "psx" => Some(57),
        "necturbografx16" | "turbografx16" | "pcengine" => Some(31),
        "snkneogeopocket" | "neogeopocket" => Some(82),
        "snkneogeo" | "neogeo" => Some(142),
        "atari2600" => Some(26),
        "atari5200" => Some(40),
        "atari7800" => Some(41),
        "atarilynx" => Some(28),
        "atarijaguar" => Some(27),
        "colecovision" => Some(48),
        "mattelintellivision" | "intellivision" => Some(115),
        "arcade" | "mame" => Some(75),
        "msdos" | "dos" => Some(135),
        "windows" | "microsoftwindows" => Some(138),
        _ => None,
    }
}

fn parse_games(body: &str) -> Result<Vec<GameCandidate>> {
    let value: serde_json::Value =
        serde_json::from_str(body).context("parsing ScreenScraper JSON response")?;
    let mut payloads = Vec::new();
    collect_game_payloads(&value, &mut payloads);
    let mut seen = HashSet::new();
    Ok(payloads
        .into_iter()
        .filter_map(game_candidate)
        .filter(|game| seen.insert(game.id))
        .collect())
}

fn collect_game_payloads(value: &serde_json::Value, output: &mut Vec<GamePayload>) {
    match value {
        serde_json::Value::Object(object) => {
            if object.contains_key("id")
                && object.contains_key("noms")
                && let Ok(payload) = serde_json::from_value::<GamePayload>(value.clone())
            {
                output.push(payload);
                return;
            }
            for child in object.values() {
                collect_game_payloads(child, output);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                collect_game_payloads(child, output);
            }
        }
        _ => {}
    }
}

fn game_candidate(payload: GamePayload) -> Option<GameCandidate> {
    let id = payload.id.parse::<i64>().ok().filter(|id| *id > 0)?;
    let name = preferred_text(&payload.noms)?;
    Some(GameCandidate {
        id,
        name,
        description: preferred_synopsis(&payload.synopsis).unwrap_or_default(),
        developer: payload
            .developpeur
            .map(|value| value.text.trim().to_owned())
            .unwrap_or_default(),
        publisher: payload
            .editeur
            .map(|value| value.text.trim().to_owned())
            .unwrap_or_default(),
        release_date: preferred_text(&payload.dates).unwrap_or_default(),
        genre: joined_unique(
            payload
                .genres
                .iter()
                .filter_map(|genre| preferred_text(&genre.noms)),
        ),
        players: payload
            .joueurs
            .map(|value| value.text.trim().to_owned())
            .unwrap_or_default(),
        rating: payload
            .note
            .and_then(|value| value.text.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite() && (0.0..=20.0).contains(value))
            .map(|value| format!("{:.1}", value / 4.0))
            .unwrap_or_default(),
        media: payload.medias,
    })
}

fn preferred_synopsis(values: &[SynopsisText]) -> Option<String> {
    values
        .iter()
        .filter(|value| !value.text.trim().is_empty())
        .min_by_key(|value| match value.langue.to_ascii_lowercase().as_str() {
            "en" => 0,
            "" => 1,
            _ => 2,
        })
        .map(|value| value.text.trim().to_owned())
}

fn joined_unique(values: impl IntoIterator<Item = String>) -> String {
    let mut output = Vec::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty()
            && !output
                .iter()
                .any(|stored: &String| stored.eq_ignore_ascii_case(value))
        {
            output.push(value.to_owned());
        }
    }
    output.join(", ")
}

fn preferred_text(values: &[LocalizedText]) -> Option<String> {
    values
        .iter()
        .filter(|value| !value.text.trim().is_empty())
        .min_by_key(|value| region_rank(&value.region))
        .map(|value| value.text.trim().to_owned())
}

fn media_kind(media_type: &str) -> Option<ArtworkKind> {
    match media_type.to_ascii_lowercase().as_str() {
        "box-2d" | "box-2d-front" => Some(ArtworkKind::BoxFront),
        "box-2d-back" => Some(ArtworkKind::BoxBack),
        "ss" | "screenshot" => Some(ArtworkKind::Screenshot),
        "sstitle" | "sstitle-hd" | "title" => Some(ArtworkKind::TitleScreen),
        "fanart" => Some(ArtworkKind::Fanart),
        "wheel" | "wheel-hd" | "wheel-carbon" | "wheel-steel" => Some(ArtworkKind::ClearLogo),
        _ => None,
    }
}

fn title_rank(candidate: &str, query: &str) -> u8 {
    let candidate = normalize_title(candidate);
    let query = normalize_title(query);
    if candidate == query {
        0
    } else if candidate.starts_with(&query) {
        1
    } else if candidate.contains(&query) {
        2
    } else {
        3
    }
}

fn normalize_title(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn region_rank(region: &str) -> u8 {
    match region.to_ascii_lowercase().as_str() {
        "us" | "usa" => 0,
        "wor" | "world" => 1,
        "eu" | "europe" => 2,
        "uk" => 3,
        "jp" | "japan" => 4,
        "" => 5,
        _ => 6,
    }
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

    fn credentials() -> ScreenScraperCredentials {
        ScreenScraperCredentials::validated("developer", "secret", "member", "password").unwrap()
    }

    #[test]
    fn platform_ids_use_exact_normalized_aliases() {
        assert_eq!(platform_id("Nintendo Entertainment System"), Some(3));
        assert_eq!(platform_id("SNES"), Some(4));
        assert_eq!(platform_id("Sony PlayStation"), Some(57));
        assert_eq!(platform_id("Nintendo Switch"), Some(225));
        assert_eq!(platform_id("Switchblade Arcade"), None);
    }

    #[test]
    fn parses_search_results_and_preserves_explicit_media_candidates() {
        let body = r#"{
          "response": {"jeux": [
            {"id":"3","noms":[{"region":"jp","text":"スーパーマリオ"},{"region":"us","text":"Super Mario Bros."}],
             "synopsis":[{"langue":"fr","text":"Description française"},{"langue":"en","text":"The Mushroom Kingdom needs a hero."}],
             "developpeur":{"text":"Nintendo"},"editeur":{"text":"Nintendo"},
             "dates":[{"region":"us","text":"1985-10-18"}],
             "genres":[{"noms":[{"region":"en","text":"Platform"}]},{"noms":[{"region":"en","text":"Action"}]}],
             "joueurs":{"text":"1-2"},"note":{"text":"18"},
             "medias":[
               {"type":"box-2D","url":"https://cdn.example/box-us.png","region":"us"},
               {"type":"box-2D","url":"https://cdn.example/box-jp.png","region":"jp"},
               {"type":"wheel-hd","url":"https://cdn.example/logo.png","region":"wor"}
             ]}
          ]}}
        "#;
        let games = parse_games(body).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, 3);
        assert_eq!(games[0].name, "Super Mario Bros.");
        assert_eq!(games[0].description, "The Mushroom Kingdom needs a hero.");
        assert_eq!(games[0].developer, "Nintendo");
        assert_eq!(games[0].genre, "Platform, Action");
        assert_eq!(games[0].players, "1-2");
        assert_eq!(games[0].rating, "4.5");
        let client = Client::new(credentials()).unwrap();
        let boxes = client
            .artwork_for_game(&games[0], ArtworkKind::BoxFront)
            .unwrap();
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].region, "us");
        assert_eq!(boxes[1].region, "jp");
    }

    #[test]
    fn official_query_shape_keeps_credentials_and_review_identity_bounded() {
        let body = br#"{"response":{"jeux":[{"id":"3","noms":[{"region":"us","text":"Super Mario Bros."}],"medias":[]}]}}"#.to_vec();
        let (address, requests, worker) = mock_server(vec![MockResponse {
            content_type: "application/json",
            body,
        }]);
        let games = Client::with_base_url(credentials(), format!("http://{address}"))
            .unwrap()
            .search_games("Super Mario", "Nintendo Entertainment System")
            .unwrap();
        assert_eq!(games[0].id, 3);
        let request = requests.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(request.starts_with("GET /jeuRecherche.php?"));
        assert!(request.contains("devid=developer"));
        assert!(request.contains("devpassword=secret"));
        assert!(request.contains("ssid=member"));
        assert!(request.contains("systemeid=3"));
        assert!(request.contains("recherche=Super+Mario"));
        worker.join().unwrap();
    }

    #[test]
    fn linked_identity_downloads_the_best_bounded_candidate_without_title_search() {
        let image = b"\x89PNG\r\n\x1a\nlinked-screenscraper-fixture".to_vec();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let body = format!(
            r#"{{"response":{{"jeu":{{"id":"3","noms":[{{"region":"us","text":"Super Mario Bros."}}],"medias":[{{"type":"box-2D","url":"http://{address}/box.png","region":"us"}}]}}}}}}"#
        )
        .into_bytes();
        let worker = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [
                MockResponse {
                    content_type: "application/json",
                    body,
                },
                MockResponse {
                    content_type: "image/png",
                    body: image,
                },
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
                    response.body.len(),
                    response.content_type
                );
                stream.write_all(reply.as_bytes()).unwrap();
                stream.write_all(&response.body).unwrap();
                stream.flush().unwrap();
            }
            requests
        });

        let media = tempfile::tempdir().unwrap();
        let result = Client::with_base_url(credentials(), format!("http://{address}"))
            .unwrap()
            .download_linked_artwork(3, media.path(), 42, ArtworkKind::BoxFront, true)
            .unwrap()
            .unwrap();
        assert_eq!(result.0, ArtworkKind::BoxFront);
        assert_eq!(
            result.1,
            media.path().join("lb-42/screenscraper/box-front.png")
        );
        assert_eq!(
            fs::read(&result.1).unwrap(),
            b"\x89PNG\r\n\x1a\nlinked-screenscraper-fixture"
        );
        let requests = worker.join().unwrap();
        assert!(requests[0].starts_with("GET /jeuInfos.php?"));
        assert!(requests[0].contains("gameid=3"));
        assert!(requests[1].starts_with("GET /box.png "));
    }
}
