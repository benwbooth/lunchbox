//! Community patch discovery. Only documented APIs and a published, pinned
//! archive are used; pages/readmes are data, never scripts to run.
use crate::game_mods::{self, BaseRom};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use url::Url;

const ARCHIVE: &str = "https://raw.githubusercontent.com/zach-morris/romhack_db/69ac23facd2983d1fd31190aa6a6ea08d72c759c";
const PLAZA: &str = "https://romhackplaza.org/api/v1";
const KEY: &str = "romhack-plaza-api-key";
const MAX_DOWNLOAD: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 256 * 1024 * 1024;
static PLAZA_RATE: Mutex<Option<Instant>> = Mutex::new(None);
static PLAZA_DOWNLOAD_RATE: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Clone, Debug, Default, Serialize)]
pub struct Entry {
    pub id: String,
    pub provider: String,
    pub title: String,
    pub kind: String,
    pub description: String,
    pub platform: String,
    pub language: String,
    pub version: String,
    pub source_url: String,
    pub bases: Vec<BaseRom>,
    pub files: Vec<Download>,
    #[serde(skip)]
    pub archive_set: String,
    #[serde(skip)]
    pub archive_member: String,
    #[serde(skip)]
    pub patch_crc: String,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Download {
    pub name: String,
    pub size: u64,
    #[serde(skip)]
    pub url: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Variant {
    pub name: String,
    pub format: String,
    pub size: u64,
    #[serde(skip)]
    pub path: PathBuf,
}
pub struct Package {
    pub directory: tempfile::TempDir,
    pub entry: Entry,
    pub variants: Vec<Variant>,
    pub readme: String,
}
impl Package {
    pub fn json(&self) -> Value {
        serde_json::json!({"variants": self.variants, "readme": self.readme, "title": self.entry.title})
    }
}
pub fn key() -> Result<Option<String>> {
    crate::settings::load_secret(KEY, "Romhack Plaza API key")
}
pub fn save_key(value: &str) -> Result<()> {
    ensure!(
        !value.trim().chars().any(char::is_control),
        "Invalid API key"
    );
    crate::settings::save_secret(KEY, value.trim(), "Romhack Plaza API key")
}
fn cancelled(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "Patch operation cancelled");
    Ok(())
}
fn rate_limit(download: bool, cancel: &AtomicBool) -> Result<()> {
    let wait_for = |lock: &Mutex<Option<Instant>>, interval: Duration| -> Result<()> {
        loop {
            cancelled(cancel)?;
            {
                let mut next = lock
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Source request lock failed"))?;
                let now = Instant::now();
                if next.is_none_or(|time| time <= now) {
                    *next = Some(now + interval);
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    };
    if download {
        wait_for(&PLAZA_DOWNLOAD_RATE, Duration::from_secs(13))?;
    }
    wait_for(&PLAZA_RATE, Duration::from_millis(2100))
}
fn allowed_url(value: &str) -> Result<Url> {
    let url = Url::parse(value)?;
    let host = url.host_str().unwrap_or("");
    ensure!(
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port_or_known_default() == Some(443)
            && (host == "romhackplaza.org"
                || host.ends_with(".romhackplaza.org")
                || matches!(
                    host,
                    "github.com"
                        | "api.github.com"
                        | "raw.githubusercontent.com"
                        | "objects.githubusercontent.com"
                        | "release-assets.githubusercontent.com"
                )),
        "This source returned an unsupported download host; no file was downloaded"
    );
    Ok(url)
}
fn fetch(
    url: &str,
    secret: Option<&str>,
    maximum: u64,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_recv_response(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(120)))
        .build()
        .into();
    let mut current = allowed_url(url)?;
    for _ in 0..6 {
        cancelled(cancel)?;
        let mut request = agent.get(current.as_str()).header(
            "User-Agent",
            concat!("Lunchbox/", env!("CARGO_PKG_VERSION")),
        );
        // Never forward the API key to GitHub, a file CDN or a redirected host.
        if current.host_str() == Some("romhackplaza.org")
            && let Some(secret) = secret
        {
            request = request.header("Authorization", &format!("Bearer {secret}"));
        }
        let mut response = request.call().map_err(|_| {
            anyhow::anyhow!("The patch source could not be reached; try again later")
        })?;
        if response.status().is_redirection() {
            let target = response
                .headers()
                .get("location")
                .context("Source redirect has no location")?
                .to_str()?;
            current = allowed_url(current.join(target)?.as_str())?;
            continue;
        }
        match response.status().as_u16() {
            401 | 403 => bail!(
                "Source access was denied. Romhack Plaza needs a valid API key with entries:read and entries:download scopes; GitHub may also rate-limit anonymous requests."
            ),
            429 => {
                bail!("The source's request limit was reached. Wait a minute before trying again.")
            }
            200 => {}
            code => bail!("Patch source returned HTTP {code}; no patch was installed"),
        }
        let total = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        ensure!(
            total.is_none_or(|size| size <= maximum),
            "Download exceeds the size limit"
        );
        let mut reader = response.body_mut().as_reader().take(maximum + 1);
        let mut bytes = Vec::new();
        let mut buffer = vec![0; 64 * 1024];
        loop {
            cancelled(cancel)?;
            let n = reader.read(&mut buffer).context("Reading patch download")?;
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
            ensure!(
                bytes.len() as u64 <= maximum,
                "Download exceeds the size limit"
            );
            progress(bytes.len() as u64, total);
        }
        return Ok(bytes);
    }
    bail!("Too many redirects from patch source")
}
fn json(url: &str, secret: Option<&str>, cancel: &AtomicBool) -> Result<Value> {
    let bytes = fetch(url, secret, 8 * 1024 * 1024, cancel, &mut |_, _| {})?;
    serde_json::from_slice(&bytes).context("Source returned invalid JSON")
}
fn plaza_json(path: &str, cancel: &AtomicBool) -> Result<Value> {
    let key = key()?.context(
        "Connect Romhack Plaza below with your API key to search and download current releases",
    )?;
    rate_limit(path.contains("/download"), cancel)?;
    let value = json(&format!("{PLAZA}/{path}"), Some(&key), cancel)?;
    ensure!(
        value["ok"].as_bool() == Some(true),
        "Romhack Plaza did not accept this request. Check API-key scopes."
    );
    Ok(value)
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_owned()
}
fn array(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn data_array(v: &Value) -> &[Value] {
    if v["data"].is_array() {
        array(&v["data"])
    } else {
        array(&v["data"]["data"])
    }
}
fn names(v: &Value) -> String {
    array(v)
        .iter()
        .map(|v| text(v, "name"))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
pub fn clean_title(title: &str) -> String {
    title
        .split(['(', '['])
        .next()
        .unwrap_or(title)
        .trim()
        .to_owned()
}
fn title_matches(title: &str, query: &str) -> bool {
    let tokens = |s: &str| {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let actual = tokens(&clean_title(title));
    let requested = tokens(&clean_title(query));
    !requested.is_empty()
        && actual
            .windows(requested.len())
            .any(|part| part == requested)
}
pub fn search(
    provider: &str,
    query: &str,
    platform: &str,
    kind: &str,
    cancel: &AtomicBool,
) -> Result<Vec<Entry>> {
    ensure!(
        query.trim().len() >= 2 && query.len() <= 160,
        "Enter a game title (2–160 characters)"
    );
    ensure!(
        matches!(kind, "translations" | "romhacks"),
        "Choose translations or mods"
    );
    match provider {
        "plaza" => {
            let mut url = Url::parse(&format!("{PLAZA}/entries/search"))?;
            url.query_pairs_mut()
                .append_pair("search", query)
                .append_pair("types[]", kind)
                .append_pair("per_page", "30")
                .append_pair("sort", "title")
                .append_pair("dir", "asc");
            let value = plaza_json(
                &format!("entries/search?{}", url.query().unwrap_or("")),
                cancel,
            )?;
            Ok(data_array(&value)
                .iter()
                .filter_map(|v| {
                    let id = v["id"].as_u64()?;
                    Some(Entry {
                        id: id.to_string(),
                        provider: provider.into(),
                        title: text(v, "title"),
                        kind: text(v, "type"),
                        ..Default::default()
                    })
                })
                .collect())
        }
        "github" => {
            let term = if kind == "translations" {
                "translation"
            } else {
                "romhack"
            };
            let mut url = Url::parse("https://api.github.com/search/repositories")?;
            url.query_pairs_mut()
                .append_pair("q", &format!("\"{}\" {term}", clean_title(query)))
                .append_pair("per_page", "20");
            let value = json(url.as_str(), None, cancel)?;
            Ok(array(&value["items"])
                .iter()
                .filter_map(|v| {
                    let id = text(v, "full_name");
                    if id.split('/').count() != 2 {
                        return None;
                    }
                    Some(Entry {
                        id,
                        provider: provider.into(),
                        title: text(v, "name"),
                        description: text(v, "description"),
                        source_url: text(v, "html_url"),
                        kind: kind.into(),
                        ..Default::default()
                    })
                })
                .collect())
        }
        "archive" => {
            let set = archive_set(platform, kind).context("The 2019 community archive has no collection for this system/category. Try Romhack Plaza or GitHub.")?;
            let bytes = cached_archive(
                &format!("{set}.xml"),
                8 * 1024 * 1024,
                cancel,
                &mut |_, _| {},
            )?;
            parse_archive(&String::from_utf8(bytes)?, set, platform, query)
        }
        _ => bail!("Unknown community patch source"),
    }
}
fn archive_set(platform: &str, kind: &str) -> Option<&'static str> {
    match (platform, kind) {
        ("Nintendo Entertainment System", "translations") => Some("NES_translations"),
        ("Nintendo Entertainment System", "romhacks") => Some("NES_romhacks"),
        ("Super Nintendo Entertainment System", "translations") => Some("SNES_translations"),
        ("Super Nintendo Entertainment System", "romhacks") => Some("SNES_romhacks"),
        ("Nintendo Game Boy", "translations") => Some("GB_translations"),
        ("Nintendo Game Boy Advance", "romhacks") => Some("GBA_romhacks"),
        ("Nintendo 64", "romhacks") => Some("N64_romhacks"),
        ("Sega Genesis", "romhacks") => Some("GEN_romhacks"),
        _ => None,
    }
}
fn cache_root() -> Result<PathBuf> {
    let root = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("Finding patch cache")?
        .cache_dir()
        .join("community-patches")
        .join("rhdn-2019-69ac23fa");
    fs::create_dir_all(&root)?;
    Ok(root)
}
fn cached_archive(
    name: &str,
    maximum: u64,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Vec<u8>> {
    ensure!(
        !name.contains(['/', '\\']) && name.len() < 64,
        "Invalid archive collection"
    );
    let root = cache_root()?;
    let path = root.join(name);
    if path.is_file() && fs::metadata(&path)?.len() <= maximum {
        return Ok(fs::read(path)?);
    }
    let bytes = fetch(
        &format!("{ARCHIVE}/{name}"),
        None,
        maximum,
        cancel,
        progress,
    )?;
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    temp.write_all(&bytes)?;
    temp.persist(&path)?;
    Ok(bytes)
}
fn attr(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> String {
    element
        .attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
        .unwrap_or_default()
}
pub fn plain_text(value: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    let decoded = quick_xml::escape::unescape(value).unwrap_or_else(|_| value.into());
    for c in decoded.chars() {
        match c {
            '<' => {
                tag = true;
                out.push(' ');
            }
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn parse_archive(xml: &str, set: &str, platform: &str, query: &str) -> Result<Vec<Entry>> {
    // The published snapshot contains a few mismatched closing tags. Parse
    // each <game> independently so one malformed record cannot lose all games.
    let mut result = Vec::new();
    for chunk in xml.split("<game ").skip(1) {
        let Some(end) = chunk.find("</game>") else {
            continue;
        };
        let record = format!("<game {}", &chunk[..end + 7]);
        let mut reader = quick_xml::Reader::from_str(&record);
        reader.config_mut().trim_text(true);
        reader.config_mut().check_end_names = false;
        let mut entry = Entry {
            provider: "archive".into(),
            platform: platform.into(),
            kind: if set.ends_with("translations") {
                "translations"
            } else {
                "romhacks"
            }
            .into(),
            version: "Community snapshot (2019, partial)".into(),
            language: "Not recorded — check author notes".into(),
            archive_set: set.into(),
            ..Default::default()
        };
        let mut field = Vec::new();
        loop {
            use quick_xml::events::Event;
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    field = e.name().as_ref().to_vec();
                    match e.name().as_ref() {
                        b"game" => entry.title = attr(&e, b"name"),
                        b"patch" => {
                            entry.archive_member = attr(&e, b"name");
                            entry.patch_crc = attr(&e, b"crc");
                        }
                        b"rom" => entry.bases.push(BaseRom {
                            name: attr(&e, b"name"),
                            crc32: attr(&e, b"crc"),
                            md5: attr(&e, b"md5"),
                            sha1: attr(&e, b"sha1"),
                        }),
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    let decoded = e.unescape().unwrap_or_default().into_owned();
                    match field.as_slice() {
                        b"description" => entry.title = decoded,
                        b"info" => entry.description.push_str(&plain_text(&decoded)),
                        b"patch_id" => entry.id = decoded,
                        b"url" => entry.source_url = decoded,
                        _ => {}
                    }
                }
                Ok(Event::End(_)) => field.clear(),
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
        }
        if !entry.id.is_empty()
            && !entry.archive_member.is_empty()
            && title_matches(&entry.title, query)
        {
            entry.title = format!(
                "{} — {} #{}",
                entry.title,
                if entry.kind == "translations" {
                    "translation"
                } else {
                    "mod"
                },
                entry.id
            );
            result.push(entry);
        }
    }
    Ok(result.into_iter().take(100).collect())
}

pub fn details(mut entry: Entry, cancel: &AtomicBool) -> Result<Entry> {
    match entry.provider.as_str() {
        "plaza" => {
            let response = plaza_json(&format!("entries/{}", entry.id), cancel)?;
            let v = &response["data"];
            entry.title = text(v, "complete_title");
            if entry.title.is_empty() {
                entry.title = text(v, "title");
            }
            entry.description = plain_text(&text(v, "description"));
            entry.platform = text(&v["platform"], "name");
            entry.language = names(&v["languages"]);
            entry.version = text(v, "version");
            entry.source_url = format!(
                "https://romhackplaza.org/{}/{}",
                entry.kind,
                text(v, "slug")
            );
            entry.bases = array(&v["hashes"])
                .iter()
                .map(|h| BaseRom {
                    name: text(h, "filename"),
                    crc32: text(h, "hash_crc32"),
                    sha1: text(h, "hash_sha1"),
                    md5: String::new(),
                })
                .collect();
            let downloads = plaza_json(&format!("entries/{}/download", entry.id), cancel)?;
            entry.files = data_array(&downloads)
                .iter()
                .map(|f| Download {
                    name: text(f, "filename"),
                    size: f["filesize"].as_u64().unwrap_or(0),
                    url: text(f, "download"),
                })
                .collect();
        }
        "github" => {
            ensure!(
                entry.id.split('/').count() == 2
                    && entry
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b))
                    && !entry.id.contains(".."),
                "Invalid GitHub repository"
            );
            let releases = json(
                &format!(
                    "https://api.github.com/repos/{}/releases?per_page=5",
                    entry.id
                ),
                None,
                cancel,
            )?;
            let release = array(&releases).iter().find(|r| r["draft"].as_bool() != Some(true) && r["prerelease"].as_bool() != Some(true)).context("This project has no stable GitHub release assets. Try another result or import the author's patch file.")?;
            entry.description = text(release, "body");
            entry.version = text(release, "tag_name");
            entry.source_url = text(release, "html_url");
            entry.files = array(&release["assets"])
                .iter()
                .filter(|a| package_extension(&text(a, "name")))
                .map(|a| Download {
                    name: text(a, "name"),
                    size: a["size"].as_u64().unwrap_or(0),
                    url: text(a, "browser_download_url"),
                })
                .collect();
        }
        "archive" => {
            let ext = if entry.archive_set == "SNES_romhacks" {
                "7z"
            } else {
                "zip"
            };
            entry.files = vec![Download {
                name: format!("{}.{}", entry.archive_set, ext),
                size: 0,
                url: format!("{ARCHIVE}/{}.{}", entry.archive_set, ext),
            }];
        }
        _ => bail!("Unknown patch source"),
    }
    ensure!(
        !entry.files.is_empty(),
        "This entry has no supported downloadable patch packages"
    );
    Ok(entry)
}
fn patch_extension(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "ips" | "ips32" | "bps" | "ups" | "ppf" | "xdelta" | "xdelta3" | "vcdiff"
    )
}
fn package_extension(name: &str) -> bool {
    patch_extension(name)
        || matches!(
            name.rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "zip" | "7z"
        )
}
fn readme_extension(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "txt" | "md" | "nfo"
    )
}
fn safe_member(name: &str) -> bool {
    !name.starts_with(['/', '\\'])
        && !name.contains('\0')
        && !name.contains(':')
        && !name.split(['/', '\\']).any(|p| p == "..")
}
pub fn download(
    mut entry: Entry,
    index: usize,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Package> {
    let file = entry
        .files
        .get(index)
        .cloned()
        .context("Choose a patch package")?;
    ensure!(file.size <= MAX_DOWNLOAD, "Patch package exceeds 128 MiB");
    let bytes = if entry.provider == "archive" {
        cached_archive(&file.name, MAX_DOWNLOAD, cancel, progress)?
    } else {
        let secret = if entry.provider == "plaza" {
            key()?
        } else {
            None
        };
        fetch(&file.url, secret.as_deref(), MAX_DOWNLOAD, cancel, progress)?
    };
    entry.files = vec![file];
    unpack(entry, &bytes, cancel)
}
fn unpack(entry: Entry, bytes: &[u8], cancel: &AtomicBool) -> Result<Package> {
    let mut package = Package {
        directory: tempfile::tempdir()?,
        entry,
        variants: Vec::new(),
        readme: String::new(),
    };
    let only_member = if package.entry.provider == "archive" {
        Some(format!(
            "{}/{}/{}",
            package.entry.archive_set, package.entry.id, package.entry.archive_member
        ))
    } else {
        None
    };
    let mut expanded = 0u64;
    if bytes.starts_with(b"PK") {
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("Reading patch ZIP")?;
        ensure!(
            archive.len() <= 20_000,
            "Patch archive has too many members"
        );
        for i in 0..archive.len() {
            cancelled(cancel)?;
            let mut file = archive.by_index(i)?;
            ensure!(
                !file.encrypted()
                    && !file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
                    && safe_member(file.name()),
                "Patch archive contains unsafe or encrypted members"
            );
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_owned();
            if only_member.as_ref().is_some_and(|wanted| wanted != &name) {
                continue;
            }
            let size = file.size();
            collect_member(&mut package, &name, size, &mut file, &mut expanded, cancel)?;
        }
    } else if bytes.starts_with(b"7z\xbc\xaf\x27\x1c") {
        let source = package.directory.path().join("download.7z");
        fs::write(&source, bytes)?;
        let mut archive =
            sevenz_rust2::ArchiveReader::open(&source, sevenz_rust2::Password::empty())?;
        ensure!(
            archive.archive().files.len() <= 20_000,
            "Patch archive has too many members"
        );
        // Bound even unselected solid members: decoding them still costs memory/CPU.
        let total = archive.archive().files.iter().try_fold(0u64, |n, e| {
            n.checked_add(e.size()).context("Archive size overflow")
        })?;
        ensure!(
            total <= MAX_EXPANDED,
            "Expanded patch archive exceeds 256 MiB"
        );
        archive.for_each_entries(|entry, reader| {
            let result = (|| -> Result<()> {
                cancelled(cancel)?;
                ensure!(safe_member(entry.name()), "Unsafe patch archive path");
                if entry.is_directory() {
                    return Ok(());
                }
                if only_member
                    .as_ref()
                    .is_none_or(|wanted| wanted == entry.name())
                {
                    collect_member(
                        &mut package,
                        entry.name(),
                        entry.size(),
                        reader,
                        &mut expanded,
                        cancel,
                    )?;
                }
                // Solid 7z members share a decoder. Unselected/readme-skipped
                // streams must still be consumed before reading the next one.
                let mut buffer = [0; 64 * 1024];
                let mut drained = 0u64;
                loop {
                    cancelled(cancel)?;
                    let n = reader.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    drained += n as u64;
                    ensure!(
                        drained <= MAX_EXPANDED,
                        "Expanded archive member exceeds limit"
                    );
                }
                Ok(())
            })();
            result
                .map(|()| true)
                .map_err(|e| std::io::Error::other(e.to_string()).into())
        })?;
    } else {
        let name = package
            .entry
            .files
            .first()
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "patch".into());
        collect_member(
            &mut package,
            &name,
            bytes.len() as u64,
            &mut std::io::Cursor::new(bytes),
            &mut expanded,
            cancel,
        )?;
    }
    ensure!(
        !package.variants.is_empty(),
        "No supported IPS/BPS/UPS/PPF/xdelta patch was found. ROMs and executables are never imported from a package."
    );
    if !package.entry.patch_crc.is_empty() {
        let expected = u32::from_str_radix(&package.entry.patch_crc, 16)
            .context("Invalid archive patch checksum")?;
        for variant in &package.variants {
            ensure!(
                crc32fast::hash(&fs::read(&variant.path)?) == expected,
                "The archive's patch checksum does not match its catalog"
            );
        }
    }
    Ok(package)
}
fn collect_member(
    package: &mut Package,
    name: &str,
    size: u64,
    reader: &mut dyn Read,
    expanded: &mut u64,
    cancel: &AtomicBool,
) -> Result<()> {
    cancelled(cancel)?;
    let is_patch = patch_extension(name);
    let readme = readme_extension(name);
    if !is_patch && !readme {
        return Ok(());
    }
    *expanded = expanded
        .checked_add(size)
        .context("Archive size overflow")?;
    ensure!(
        *expanded <= MAX_EXPANDED && package.variants.len() < 128,
        "Expanded patch package is too large"
    );
    let maximum = if is_patch { MAX_EXPANDED } else { 64 * 1024 };
    if !is_patch && size > maximum {
        return Ok(());
    }
    let mut bytes = Vec::new();
    reader.take(maximum + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum && bytes.len() as u64 == size,
        "Patch package member size mismatch"
    );
    cancelled(cancel)?;
    if is_patch {
        let format = lunchbox_patching::format(&bytes)?.to_owned();
        let extension = name
            .rsplit('.')
            .next()
            .unwrap_or("patch")
            .to_ascii_lowercase();
        let path = package.directory.path().join(format!(
            "variant-{}.{}",
            package.variants.len(),
            extension
        ));
        fs::write(&path, &bytes)?;
        package.variants.push(Variant {
            name: name.into(),
            format,
            size,
            path,
        });
    } else if package.readme.len() < 128 * 1024 {
        package
            .readme
            .push_str(&format!("\n{name}\n{}\n", String::from_utf8_lossy(&bytes)));
    }
    Ok(())
}
pub fn import_variant(package: &Package, index: usize) -> Result<game_mods::Patch> {
    let variant = package
        .variants
        .get(index)
        .context("Choose a patch variant")?;
    let mut patch = game_mods::import_patch(&variant.path)?;
    patch.name = format!("{} — {}", package.entry.title, variant.name);
    patch.source_url = package.entry.source_url.clone();
    patch.source_name = match package.entry.provider.as_str() {
        "plaza" => "Romhack Plaza",
        "archive" => "RHDN community archive (2019)",
        _ => "GitHub author release",
    }
    .into();
    patch.expected_inputs = package.entry.bases.clone();
    Ok(patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    const IPS: &[u8] = b"PATCH\0\0\x01\0\x01XEOF";
    fn zip(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in members {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn archive_matches_exact_title_tokens_and_preserves_requirements() {
        let record = r#"<game name="139 - Final Fantasy II"><description>Final Fantasy II (J) [!]</description><info>English &amp; notes &lt;br/&gt; here</info><patch name="ff2.ips" crc="12345678"><patch_id>139</patch_id><url>https://www.romhacking.net/translations/139/</url></patch><rom name="ff2.nes" crc="b7327510" md5="74669f264df775499830f9e6fb60a1e7"/></game>"#;
        let xml = format!(
            "<game name=\"bad\"><info>bad</wrong></game>{record}{}",
            record
                .replace("Final Fantasy II", "Final Fantasy III")
                .replace("139", "140")
        );
        let entries = parse_archive(
            &xml,
            "NES_translations",
            "Nintendo Entertainment System",
            "Final Fantasy II (Japan)",
        )
        .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "139");
        assert_eq!(entries[0].bases[0].crc32, "b7327510");
        assert_eq!(entries[0].archive_member, "ff2.ips");
        assert_eq!(entries[0].description, "English & notes here");
        assert!(!title_matches("Final Fantasy III", "Final Fantasy II"));
    }
    #[test]
    fn variants_are_separate_and_executables_and_roms_are_not_extracted() {
        let bytes = zip(&[
            ("a.ips", IPS),
            ("variant/b.ips", IPS),
            ("readme.txt", b"Use the Japanese original"),
            ("patcher.exe", b"MZ"),
            ("game.nes", b"NES"),
        ]);
        let package = unpack(Entry::default(), &bytes, &AtomicBool::new(false)).unwrap();
        assert_eq!(package.variants.len(), 2);
        assert!(package.readme.contains("Japanese original"));
        assert_eq!(fs::read_dir(package.directory.path()).unwrap().count(), 2);
        assert!(
            package
                .variants
                .iter()
                .all(|v| fs::read(&v.path).unwrap() == IPS)
        );
        assert!(!package.json().to_string().contains("variant-0.ips"));
    }
    #[test]
    fn unsafe_members_and_false_patch_extensions_are_rejected() {
        for name in ["../a.ips", "/a.ips", "C:\\a.ips", "sub\\..\\a.ips"] {
            assert!(
                unpack(
                    Entry::default(),
                    &zip(&[(name, IPS)]),
                    &AtomicBool::new(false)
                )
                .is_err()
            );
        }
        assert!(
            unpack(
                Entry::default(),
                &zip(&[("fake.ips", b"MZ")]),
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert!(
            unpack(
                Entry::default(),
                &zip(&[("a.ips", IPS)]),
                &AtomicBool::new(true)
            )
            .is_err()
        );
    }
    #[test]
    fn collection_extracts_only_selected_entry_and_checks_patch_crc() {
        let bytes = zip(&[
            ("NES_translations/139/ff2.ips", IPS),
            ("NES_translations/140/ff2.ips", IPS),
        ]);
        let mut entry = Entry {
            provider: "archive".into(),
            id: "139".into(),
            archive_set: "NES_translations".into(),
            archive_member: "ff2.ips".into(),
            patch_crc: format!("{:08x}", crc32fast::hash(IPS)),
            ..Default::default()
        };
        let package = unpack(entry.clone(), &bytes, &AtomicBool::new(false)).unwrap();
        assert_eq!(package.variants.len(), 1);
        assert!(package.variants[0].name.contains("/139/"));
        entry.patch_crc = "00000000".into();
        assert!(unpack(entry, &bytes, &AtomicBool::new(false)).is_err());
    }
    #[test]
    fn solid_seven_zip_skipped_members_do_not_corrupt_selected_patch() {
        let directory = tempfile::tempdir().unwrap();
        let contents = directory.path().join("contents");
        fs::create_dir(&contents).unwrap();
        fs::write(contents.join("a.exe"), b"MZ ignored executable").unwrap();
        fs::write(contents.join("b.ips"), IPS).unwrap();
        fs::write(contents.join("c.txt"), b"Patch notes").unwrap();
        let path = directory.path().join("patches.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&path).unwrap();
        writer.push_source_path(&contents, |_| true).unwrap();
        writer.finish().unwrap();
        let package = unpack(
            Entry::default(),
            &fs::read(path).unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(package.variants.len(), 1);
        assert_eq!(fs::read(&package.variants[0].path).unwrap(), IPS);
        assert!(package.readme.contains("Patch notes"));
    }
    #[test]
    fn download_hosts_are_https_and_metadata_never_exposes_signed_urls() {
        for url in [
            "http://github.com/a",
            "https://github.com.evil.test/a",
            "https://user:secret@github.com/a",
            "https://127.0.0.1/a",
            "file:///tmp/a",
        ] {
            assert!(allowed_url(url).is_err());
        }
        assert!(allowed_url("https://release-assets.githubusercontent.com/a").is_ok());
        let file = Download {
            name: "a.ips".into(),
            url: "signed-secret".into(),
            size: 10,
        };
        assert!(
            !serde_json::to_string(&file)
                .unwrap()
                .contains("signed-secret")
        );
        let entry = Entry {
            files: vec![file],
            ..Default::default()
        };
        assert_eq!(
            unpack(entry, IPS, &AtomicBool::new(false))
                .unwrap()
                .variants[0]
                .name,
            "a.ips"
        );
        assert_eq!(
            data_array(&serde_json::json!({"data":{"data":[{"id":1}]}})).len(),
            1
        );
    }
    #[test]
    #[ignore = "explicit network smoke test; downloads the public pinned patch-only archive"]
    fn live_archive_final_fantasy_ii_patch_download() {
        let cancel = AtomicBool::new(false);
        let entries = search(
            "archive",
            "Final Fantasy II",
            "Nintendo Entertainment System",
            "translations",
            &cancel,
        )
        .unwrap();
        let entry = entries
            .into_iter()
            .find(|e| e.id == "139")
            .expect("FFII translation in published archive");
        assert_eq!(entry.bases[0].crc32.to_lowercase(), "b7327510");
        let entry = details(entry, &cancel).unwrap();
        let package = download(entry, 0, &cancel, &mut |_, _| {}).unwrap();
        assert_eq!(package.variants.len(), 1);
        assert_eq!(package.variants[0].format, "IPS");
        assert!(package.variants[0].size > 1000);
        eprintln!(
            "Verified published FFII patch: {} ({} bytes), catalog CRC {}",
            package.variants[0].name, package.variants[0].size, package.entry.patch_crc
        );
    }
}
