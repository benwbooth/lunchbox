//! GitHub is a release transport, not a patch catalog. Inspect actual published
//! assets before presenting a project as installable. Never run build scripts.
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Seek, SeekFrom},
};

const BLOCK: u64 = 64 * 1024;
const INSPECT_LIMIT: u64 = 2 * 1024 * 1024;
const SEARCH_BYTES: u64 = 24 * 1024 * 1024;
const SMALL_7Z: u64 = 4 * 1024 * 1024;

fn cache_path(key: &str) -> Result<PathBuf> {
    let dir = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("Finding patch cache")?
        .cache_dir()
        .join("community-patches/github-v2");
    fs::create_dir_all(&dir)?;
    Ok(dir.join(format!(
        "{}.json",
        hex::encode(Sha256::digest(key.as_bytes()))
    )))
}
fn read_cache<T: serde::de::DeserializeOwned>(key: &str, seconds: u64) -> Option<T> {
    let path = cache_path(key).ok()?;
    let meta = fs::metadata(&path).ok()?;
    if meta.len() > 8 * 1024 * 1024 || meta.modified().ok()?.elapsed().ok()?.as_secs() > seconds {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}
fn write_cache(key: &str, value: &impl Serialize) -> Result<()> {
    let path = cache_path(key)?;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.persist(path)?;
    Ok(())
}
fn metadata(url: &str, cancel: &AtomicBool) -> Result<Value> {
    cancelled(cancel)?;
    if let Some(value) = read_cache(url, 15 * 60) {
        return Ok(value);
    }
    let value = json(url, None, cancel)?;
    let _ = write_cache(url, &value);
    Ok(value)
}

fn repository(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('/');
    let value = value.strip_prefix("https://github.com/").unwrap_or(value);
    let value = value.strip_suffix(".git").unwrap_or(value);
    let mut parts = value.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    if parts.next().is_some()
        || [owner, name].iter().any(|p| {
            p.is_empty()
                || p.contains("..")
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return None;
    }
    Some(format!("{owner}/{name}"))
}
fn normalized(value: &str) -> Vec<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|v| !v.is_empty())
        .map(|v| {
            match v {
                "ii" => "2",
                "iii" => "3",
                "iv" => "4",
                "v" => "5",
                "vi" => "6",
                "vii" => "7",
                "viii" => "8",
                "ix" => "9",
                _ => v,
            }
            .into()
        })
        .collect()
}
fn matches_game(value: &str, query: &str) -> bool {
    let query = normalized(&clean_title(query));
    let value = normalized(value);
    if query.is_empty() {
        return false;
    }
    if value.windows(query.len()).any(|v| v == query) {
        return true;
    }
    // Common title abbreviations such as FF2, DQ3 and SD3; never match a
    // neighboring numbered entry by substring (FF2 is not FF12 or FF3).
    if query.len() >= 3
        && query
            .last()
            .is_some_and(|v| v.chars().all(|c| c.is_ascii_digit()))
    {
        let acronym = query[..query.len() - 1]
            .iter()
            .filter_map(|v| v.chars().next())
            .collect::<String>()
            + query.last().unwrap();
        return value.contains(&acronym);
    }
    false
}
fn known_systems(value: &str) -> Vec<&'static str> {
    let value = normalized(value)
        .join(" ")
        .replace("super nintendo", "snes")
        .replace("super famicom", "snes")
        .replace("nintendo entertainment system", "nes")
        .replace("game boy advance", "gba")
        .replace("game boy color", "gbc")
        .replace("game boy", "gb")
        .replace("nintendo 64", "n64")
        .replace("nintendo ds", "nds")
        .replace("playstation 2", "ps2")
        .replace("playstation", "psx");
    let tokens: Vec<_> = value.split_whitespace().collect();
    [
        ("nes", &["nes", "famicom"][..]),
        ("snes", &["snes", "sfc"]),
        ("gb", &["gb"]),
        ("gbc", &["gbc"]),
        ("gba", &["gba"]),
        ("n64", &["n64"]),
        ("nds", &["nds"]),
        ("psx", &["psx", "ps1"]),
        ("ps2", &["ps2"]),
        ("genesis", &["genesis", "megadrive"]),
    ]
    .into_iter()
    .filter_map(|(system, aliases)| aliases.iter().any(|a| tokens.contains(a)).then_some(system))
    .collect()
}
fn same_system(candidate: &str, platform: &str) -> bool {
    let candidate = known_systems(candidate);
    let selected = known_systems(platform);
    candidate.is_empty() || selected.is_empty() || candidate.iter().any(|p| selected.contains(p))
}

/// Random-access reader for the ZIP central directory and patch signatures.
/// Search never downloads a complete large ZIP just to discover it is a tool.
struct RangeReader<F> {
    len: u64,
    pos: u64,
    blocks: BTreeMap<u64, Vec<u8>>,
    read_range: F,
}
impl<F: FnMut(u64, u64) -> Result<Vec<u8>>> Read for RangeReader<F> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pos == self.len || out.is_empty() {
            return Ok(0);
        }
        let start = self.pos / BLOCK * BLOCK;
        if !self.blocks.contains_key(&start) {
            let end = (start + BLOCK).min(self.len) - 1;
            let bytes = (self.read_range)(start, end).map_err(std::io::Error::other)?;
            if bytes.len() as u64 != end - start + 1 {
                return Err(std::io::Error::other("Incomplete package byte range"));
            }
            self.blocks.insert(start, bytes);
        }
        let block = &self.blocks[&start];
        let offset = (self.pos - start) as usize;
        let n = out.len().min(block.len() - offset);
        out[..n].copy_from_slice(&block[offset..offset + n]);
        self.pos += n as u64;
        Ok(n)
    }
}
impl<F> Seek for RangeReader<F> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let pos = match from {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::End(n) => self.len as i128 + n as i128,
            SeekFrom::Current(n) => self.pos as i128 + n as i128,
        };
        if pos < 0 || pos > self.len as i128 {
            return Err(std::io::Error::other("Invalid package seek"));
        }
        self.pos = pos as u64;
        Ok(self.pos)
    }
}
fn zip_patches(reader: impl Read + Seek, cancel: &AtomicBool) -> Result<Vec<String>> {
    let mut archive = zip::ZipArchive::new(reader)?;
    ensure!(archive.len() <= 20_000, "Too many archive members");
    let mut patches = Vec::new();
    let mut size = 0u64;
    for i in 0..archive.len() {
        cancelled(cancel)?;
        let mut file = archive.by_index(i)?;
        ensure!(
            !file.encrypted()
                && safe_member(file.name())
                && !file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000),
            "Unsafe or encrypted patch package"
        );
        if file.is_dir() || !patch_extension(file.name()) {
            continue;
        }
        size = size
            .checked_add(file.size())
            .context("Patch size overflow")?;
        ensure!(
            size <= MAX_EXPANDED && patches.len() < 128,
            "Expanded patches exceed package limit"
        );
        let mut signature = Vec::new();
        (&mut file).take(8).read_to_end(&mut signature)?;
        lunchbox_patching::format(&signature)?;
        patches.push(file.name().to_owned());
    }
    ensure!(!patches.is_empty(), "Archive contains no supported patches");
    Ok(patches)
}

#[derive(Serialize, Deserialize)]
struct Inspection {
    patches: Vec<String>,
}
fn inspect(asset: &Value, cancel: &AtomicBool, budget: &mut u64) -> Result<Vec<String>> {
    let name = text(asset, "name");
    let url = text(asset, "browser_download_url");
    let size = asset["size"]
        .as_u64()
        .context("Release has no asset size")?;
    ensure!(size > 0 && size <= MAX_DOWNLOAD, "Unsupported package size");
    let parsed = allowed_url(&url)?;
    ensure!(
        parsed.host_str() == Some("github.com") && parsed.path().contains("/releases/download/"),
        "Not a GitHub release asset"
    );
    let key = format!(
        "inspection:{}:{}:{}:{}",
        url,
        size,
        text(asset, "updated_at"),
        text(asset, "digest")
    );
    if let Some(inspection) = read_cache::<Inspection>(&key, 6 * 60 * 60) {
        return Ok(inspection.patches);
    }
    let patches = if name.to_lowercase().ends_with(".zip") {
        let mut inspected = 0;
        let reader = RangeReader {
            len: size,
            pos: 0,
            blocks: BTreeMap::new(),
            read_range: |start, end| {
                let length = end - start + 1;
                ensure!(
                    length <= *budget && inspected + length <= INSPECT_LIMIT,
                    "Package inspection limit reached"
                );
                *budget -= length;
                inspected += length;
                fetch_part(
                    &url,
                    None,
                    BLOCK,
                    cancel,
                    &mut |_, _| {},
                    Some((start, end, size)),
                )
            },
        };
        zip_patches(reader, cancel)?
    } else if name.to_lowercase().ends_with(".7z") {
        ensure!(
            size <= SMALL_7Z && size <= *budget,
            "7z package is too large for search-time inspection"
        );
        *budget -= size;
        let bytes = fetch(&url, None, SMALL_7Z, cancel, &mut |_, _| {})?;
        let package = unpack(Entry::default(), &bytes, cancel)?;
        package.variants.into_iter().map(|v| v.name).collect()
    } else {
        ensure!(patch_extension(&name), "Not a supported patch");
        let length = size.min(8);
        let charge = size.min(BLOCK); // bound servers that ignore Range for small files
        ensure!(charge <= *budget, "Search inspection limit reached");
        *budget -= charge;
        let bytes = fetch_part(
            &url,
            None,
            BLOCK,
            cancel,
            &mut |_, _| {},
            Some((0, length - 1, size)),
        )?;
        lunchbox_patching::format(&bytes)?;
        vec![name]
    };
    let _ = write_cache(
        &key,
        &Inspection {
            patches: patches.clone(),
        },
    );
    Ok(patches)
}

fn release_entry(
    repo: &Value,
    releases: &Value,
    kind: &str,
    mut inspect: impl FnMut(&Value) -> Result<Vec<String>>,
) -> Result<Option<Entry>> {
    let id = repository(&text(repo, "full_name")).context("Invalid repository")?;
    for release in array(releases)
        .iter()
        .filter(|r| r["draft"].as_bool() != Some(true) && r["prerelease"].as_bool() != Some(true))
        .take(5)
    {
        let mut files = Vec::new();
        for asset in array(&release["assets"])
            .iter()
            .filter(|a| {
                a["state"].as_str() == Some("uploaded") && package_extension(&text(a, "name"))
            })
            .take(8)
        {
            if let Ok(patches) = inspect(asset) {
                if patches.is_empty() {
                    continue;
                }
                files.push(Download {
                    name: text(asset, "name"),
                    url: text(asset, "browser_download_url"),
                    size: asset["size"].as_u64().unwrap_or(0),
                    patches,
                });
            }
        }
        if files.is_empty() {
            continue;
        }
        return Ok(Some(Entry {
            id: id.clone(), provider: "github".into(), kind: kind.into(),
            title: format!("{} — {}", id, text(release, "tag_name")),
            description: format!("{}\n\n{}", text(repo, "description"), text(release, "body")),
            version: text(release, "tag_name"), source_url: text(release, "html_url"),
            verification: "Published patch files inspected. Input checksums are checked when available; read the author's ROM revision requirements before applying.".into(),
            files, ..Default::default()
        }));
    }
    Ok(None)
}

pub(super) fn search(
    query: &str,
    platform: &str,
    kind: &str,
    cancel: &AtomicBool,
) -> Result<SearchReport> {
    ensure!(
        query.trim().len() >= 2 && query.len() <= 160,
        "Enter a game title or owner/repository (2–160 characters)"
    );
    ensure!(
        matches!(kind, "translations" | "romhacks"),
        "Choose translations or mods"
    );
    let explicit = repository(query);
    let projects = if let Some(repo) = &explicit {
        vec![metadata(
            &format!("https://api.github.com/repos/{repo}"),
            cancel,
        )?]
    } else {
        let term = if kind == "translations" {
            "translation"
        } else {
            "romhack"
        };
        let mut url = Url::parse("https://api.github.com/search/repositories")?;
        url.query_pairs_mut()
            .append_pair(
                "q",
                &format!(
                    "\"{}\" {term} in:name,description fork:false",
                    clean_title(query).replace('"', "")
                ),
            )
            .append_pair("per_page", "12");
        array(&metadata(url.as_str(), cancel)?["items"]).to_vec()
    };
    let mut entries = Vec::new();
    let mut checked = 0;
    let mut skipped = 0;
    let mut unavailable = 0;
    let mut budget = SEARCH_BYTES;
    let mut warning = String::new();
    for repo in projects {
        cancelled(cancel)?;
        let id = text(&repo, "full_name");
        let topics = array(&repo["topics"])
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let candidate = format!("{} {} {}", id, text(&repo, "description"), topics);
        if repository(&id).is_none()
            || !same_system(&candidate, platform)
            || (explicit.is_none() && !matches_game(&candidate, query))
        {
            skipped += 1;
            continue;
        }
        checked += 1;
        let releases = match metadata(
            &format!("https://api.github.com/repos/{id}/releases?per_page=10"),
            cancel,
        ) {
            Ok(value) => value,
            Err(error) => {
                unavailable += 1;
                if error
                    .downcast_ref::<SourceStatus>()
                    .is_some_and(|s| matches!(s.0, 401 | 403 | 429))
                {
                    warning = " GitHub access/request limit reached; results below include only releases already inspected. Try again after the limit resets.".into();
                    break;
                }
                continue;
            }
        };
        let entry = release_entry(&repo, &releases, kind, |asset| {
            inspect(asset, cancel, &mut budget)
        })?;
        cancelled(cancel)?;
        if let Some(mut entry) = entry {
            entry.platform = if known_systems(&candidate).is_empty() {
                "System not declared — check author notes".into()
            } else {
                platform.into()
            };
            entries.push(entry);
        } else {
            skipped += 1;
        }
        if budget == 0 {
            warning.push_str(
                " Search inspection budget reached; enter owner/repository to check one project.",
            );
            break;
        }
    }
    let note = format!(
        "{} patch releases found; {checked} projects checked. {skipped} unrelated or unusable projects omitted; {unavailable} unavailable. Source-only projects, patcher applications, and uninspectable packages are not listed.{}{}",
        entries.len(),
        warning,
        if entries.is_empty() {
            " Try an alternate title or paste the author's GitHub owner/repository."
        } else {
            ""
        }
    );
    Ok(SearchReport { entries, note })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_and_system_matching_rejects_numbered_sequels_and_other_ports() {
        assert!(matches_game(
            "Final-Fantasy-II-Translation",
            "Final Fantasy II (Japan)"
        ));
        assert!(matches_game("FF2 English", "Final Fantasy II"));
        assert!(!matches_game(
            "Final Fantasy III translation",
            "Final Fantasy II"
        ));
        assert!(!matches_game("FF12 English", "Final Fantasy II"));
        assert!(!same_system(
            "Final Fantasy II SNES",
            "Nintendo Entertainment System"
        ));
        assert!(same_system(
            "Final Fantasy II Famicom",
            "Nintendo Entertainment System"
        ));
        assert!(!same_system(
            "Super Famicom translation",
            "Nintendo Entertainment System"
        ));
        assert_eq!(
            repository("https://github.com/author/patches.git").as_deref(),
            Some("author/patches")
        );
        assert!(repository("https://github.com.evil/a/b").is_none());
        assert!(repository("owner/../repo").is_none());
    }
    fn release(tag: &str, files: &[&str]) -> Value {
        serde_json::json!({"tag_name":tag,"draft":false,"prerelease":false,"assets":files.iter().map(|name| serde_json::json!({"name":name,"state":"uploaded","size":100,"browser_download_url":format!("https://github.com/author/translation/releases/download/{tag}/{name}")})).collect::<Vec<_>>()})
    }
    #[test]
    fn never_lists_source_only_or_tool_only_releases_and_pins_working_version() {
        let repo = serde_json::json!({"full_name":"author/translation"});
        let mut preview = release("preview", &["test.ips"]);
        preview["prerelease"] = true.into();
        let versions = serde_json::json!([
            preview,
            release("v3", &[]),
            release("v2", &["tool.zip", "setup.exe"]),
            release("v1", &["english.zip"])
        ]);
        let result = release_entry(&repo, &versions, "translations", |a| {
            if text(a, "name") == "english.zip" {
                Ok(vec!["english.ips".into()])
            } else {
                bail!("No patch in package")
            }
        })
        .unwrap()
        .unwrap();
        assert_eq!(result.version, "v1");
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].patches, ["english.ips"]);
        assert!(
            release_entry(
                &repo,
                &serde_json::json!([release("v2", &["tool.zip"])]),
                "translations",
                |_| Ok(vec![])
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    fn range_reader_checks_zip_contents_without_reading_large_unrelated_payload() {
        let ips = b"PATCH\0\0\x01\0\x01XEOF";
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file(
                "unrelated.dat",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&vec![0; 5 * 1024 * 1024]).unwrap();
        writer
            .start_file("translation.ips", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(ips).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let mut fetched = 0;
        let reader = RangeReader {
            len: bytes.len() as u64,
            pos: 0,
            blocks: BTreeMap::new(),
            read_range: |a, b| {
                fetched += b - a + 1;
                Ok(bytes[a as usize..=b as usize].to_vec())
            },
        };
        assert_eq!(
            zip_patches(reader, &AtomicBool::new(false)).unwrap(),
            ["translation.ips"]
        );
        assert!(fetched < 4 * BLOCK);
        let junk = super::super::tests::zip(&[("translation.ips", b"not a patch")]);
        assert!(zip_patches(std::io::Cursor::new(junk), &AtomicBool::new(false)).is_err());
    }
    #[test]
    #[ignore = "explicit public GitHub release network smoke test"]
    fn live_author_patch_release() {
        let report = search(
            "Dimedime-d/kptranslation",
            "Nintendo Game Boy Advance",
            "translations",
            &AtomicBool::new(false),
        )
        .unwrap();
        eprintln!("{}", report.note);
        for entry in &report.entries {
            eprintln!(
                "{}: {:?}",
                entry.title,
                entry
                    .files
                    .iter()
                    .map(|f| (&f.name, &f.patches))
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].files.len(), 1);
        assert!(report.entries[0].files[0].name.ends_with(".bps"));
        assert!(!report.entries[0].files[0].patches.is_empty());
    }
    #[test]
    #[ignore = "explicit public GitHub metadata network smoke test"]
    fn live_full_rom_release_is_not_a_patch() {
        let report = search(
            "Superptfan/Final-Fantasy-III-SNES-Revamp-Translation",
            "Super Nintendo Entertainment System",
            "translations",
            &AtomicBool::new(false),
        )
        .unwrap();
        eprintln!("{}", report.note);
        assert!(report.entries.is_empty());
    }
}
