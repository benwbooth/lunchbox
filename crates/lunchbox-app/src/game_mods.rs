//! Per-game patch stacks and cheats. No original content or global emulator
//! configuration is modified. Launch failures are explicit, never silently vanilla.
use anyhow::{Context, Result, bail, ensure};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub patches: Vec<Patch>,
    pub cheats_enabled: bool,
    pub cheats: Vec<Cheat>,
    /// Optional SHA256 supplied by the patch author for formats without one.
    pub base_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Patch {
    pub name: String,
    pub file: String,
    pub sha256: String,
    pub format: String,
    pub enabled: bool,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub source_name: String,
    #[serde(default)]
    pub expected_inputs: Vec<BaseRom>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BaseRom {
    pub name: String,
    pub crc32: String,
    pub sha1: String,
    pub md5: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Cheat {
    pub name: String,
    pub code: String,
    pub enabled: bool,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

impl Profile {
    pub fn load(store: &crate::settings::SettingsStore, game: &str) -> Result<Self> {
        let json = store
            .connection()?
            .query_row(
                "SELECT profile_json FROM game_mod_profiles WHERE game_uid=?1",
                [game],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).context("Reading patch/cheat profile"))
            .unwrap_or_else(|| Ok(Self::default()))
    }
    pub fn save(&self, store: &crate::settings::SettingsStore, game: &str) -> Result<()> {
        ensure!(!game.trim().is_empty(), "Select a game first");
        ensure!(
            self.patches.len() <= 64 && self.cheats.len() <= 4096,
            "Too many patches or cheats"
        );
        ensure!(
            self.base_sha256.is_empty()
                || (self.base_sha256.len() == 64
                    && self.base_sha256.bytes().all(|b| b.is_ascii_hexdigit())),
            "Base SHA256 must have 64 hexadecimal digits"
        );
        for cheat in &self.cheats {
            validate_cheat(cheat)?;
        }
        store.connection()?.execute("INSERT INTO game_mod_profiles(game_uid,profile_json) VALUES(?1,?2) ON CONFLICT(game_uid) DO UPDATE SET profile_json=excluded.profile_json", rusqlite::params![game, serde_json::to_string(self)?])?;
        Ok(())
    }
}

fn data_root() -> Result<PathBuf> {
    Ok(
        directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
            .context("Finding Lunchbox data directory")?
            .data_local_dir()
            .join("game-mods"),
    )
}
pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn hash_file(path: &Path, cancel: &AtomicBool) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut hash = Sha256::new();
    loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Patch preparation cancelled"
        );
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}

pub fn import_patch(path: &Path) -> Result<Patch> {
    ensure!(
        fs::metadata(path)?.len() <= lunchbox_patching::MAX_PATCH,
        "Patch exceeds 512 MiB"
    );
    let bytes = fs::read(path)?;
    let format = lunchbox_patching::format(&bytes)?.to_owned();
    let hash = digest(&bytes);
    let name = path
        .file_name()
        .context("Patch has no filename")?
        .to_string_lossy()
        .into_owned();
    let file = format!(
        "{}-{}",
        &hash[..16],
        crate::qbittorrent::safe_path_component(&name)
    );
    let root = data_root()?.join("patches");
    fs::create_dir_all(&root)?;
    let destination = root.join(&file);
    if !destination.exists() {
        let mut tmp = tempfile::NamedTempFile::new_in(&root)?;
        use std::io::Write;
        tmp.write_all(&bytes)?;
        tmp.persist_noclobber(&destination)?;
    }
    Ok(Patch {
        name,
        file,
        sha256: hash,
        format,
        enabled: false,
        source_url: String::new(),
        source_name: String::new(),
        expected_inputs: Vec::new(),
    })
}

fn fingerprint(path: &Path) -> Result<String> {
    let m = fs::metadata(path)?;
    Ok(format!(
        "{}:{}:{}",
        path.display(),
        m.len(),
        m.modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ))
}

/// Stable filenames isolate patched saves, while a metadata receipt avoids
/// re-reading multi-gigabyte ISOs on unchanged launches.
pub fn prepare(
    source: &Path,
    profile: &Profile,
    game: &str,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    prepare_in(source, profile, game, cancel, &data_root()?)
}
fn prepare_in(
    source: &Path,
    profile: &Profile,
    game: &str,
    cancel: &AtomicBool,
    root: &Path,
) -> Result<PathBuf> {
    let active: Vec<_> = profile.patches.iter().filter(|p| p.enabled).collect();
    if active.is_empty() {
        return Ok(source.to_path_buf());
    }
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    ensure!(
        ![
            "cue", "m3u", "gdi", "ccd", "mds", "chd", "cso", "rvz", "pbp", "zip", "7z", "rar"
        ]
        .contains(&extension.as_str()),
        "This patch needs the author's raw ROM/ISO/BIN input, not a playlist, cue sheet or compressed disc. Select the correct extracted image before patching"
    );
    let stack = serde_json::to_vec(&active.iter().map(|p| &p.sha256).collect::<Vec<_>>())?;
    let variant = digest(&stack);
    let parent = root
        .join("patched")
        .join(digest(game.as_bytes()))
        .join(&variant[..16]);
    fs::create_dir_all(&parent)?;
    let name = crate::qbittorrent::safe_path_component(
        &source.file_stem().unwrap_or_default().to_string_lossy(),
    );
    let output = parent.join(format!("{name} [mod-{}].{extension}", &variant[..8]));
    let receipt = parent.join("receipt.json");
    let mut identity = vec![fingerprint(source)?, profile.base_sha256.clone()];
    let mut paths = Vec::new();
    for patch in &active {
        ensure!(
            Path::new(&patch.file).components().count() == 1 && !patch.file.contains(['/', '\\']),
            "Invalid managed patch path"
        );
        let path = root.join("patches").join(&patch.file);
        identity.push(fingerprint(&path)?);
        identity.push(patch.sha256.clone());
        identity.push(serde_json::to_string(&patch.expected_inputs)?);
        paths.push(path);
    }
    if let Ok(saved) = fs::read(&receipt)
        && let Ok((old_identity, old_output)) =
            serde_json::from_slice::<(Vec<String>, String)>(&saved)
        && identity == old_identity
        && fingerprint(&output).is_ok_and(|f| f == old_output)
    {
        return Ok(output);
    }
    if !profile.base_sha256.is_empty() {
        ensure!(
            hash_file(source, cancel)?.eq_ignore_ascii_case(&profile.base_sha256),
            "Base SHA256 mismatch: use the patch author's exact ROM/disc revision"
        );
    }
    let work = tempfile::tempdir_in(&parent)?;
    let mut input = source.to_path_buf();
    for (index, path) in paths.iter().enumerate() {
        ensure!(
            hash_file(path, cancel)? == active[index].sha256,
            "Imported patch changed or was damaged. Re-import {}",
            active[index].name
        );
        validate_base(&input, &active[index].expected_inputs, cancel)?;
        let next = work.path().join(format!("step-{index}.{extension}"));
        lunchbox_patching::apply(&input, path, &next, cancel)?;
        if input.starts_with(work.path()) {
            fs::remove_file(&input)?;
        }
        input = next;
    }
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Patch preparation cancelled"
    );
    // Remove only a previous derived cache file, never the source or its saves.
    if output.exists() {
        fs::remove_file(&output)?;
    }
    fs::rename(&input, &output)?;
    fs::write(
        receipt,
        serde_json::to_vec(&(identity, fingerprint(&output)?))?,
    )?;
    Ok(output)
}

/// Author-supplied hashes are alternatives of complete ROM identities, not
/// interchangeable hash fields. Do not silently strip headers or pad a mismatch.
pub fn validate_base(path: &Path, bases: &[BaseRom], cancel: &AtomicBool) -> Result<()> {
    let bases: Vec<_> = bases
        .iter()
        .filter(|b| !b.crc32.is_empty() || !b.sha1.is_empty() || !b.md5.is_empty())
        .collect();
    if bases.is_empty() {
        return Ok(());
    }
    let mut crc = crc32fast::Hasher::new();
    let mut sha = sha1::Sha1::new();
    let mut md5 = md5::Md5::new();
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Patch preparation cancelled"
        );
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        crc.update(&buffer[..n]);
        sha.update(&buffer[..n]);
        md5.update(&buffer[..n]);
    }
    let crc = format!("{:08x}", crc.finalize());
    let sha = hex::encode(sha.finalize());
    let md5 = format!("{:x}", md5.finalize());
    let matches =
        |expected: &str, actual: &str| expected.is_empty() || expected.eq_ignore_ascii_case(actual);
    ensure!(
        bases
            .iter()
            .any(|b| matches(&b.crc32, &crc) && matches(&b.sha1, &sha) && matches(&b.md5, &md5)),
        "The selected ROM does not match the patch's required input. Check its region, revision and header. Your original file was not changed (CRC32 {crc}, SHA1 {sha})."
    );
    Ok(())
}

const CHEAT_FIELDS: &[&str] = &[
    "handler",
    "memory_search_size",
    "cheat_type",
    "value",
    "address",
    "address_bit_position",
    "big_endian",
    "rumble_type",
    "rumble_value",
    "rumble_port",
    "rumble_primary_strength",
    "rumble_primary_duration",
    "rumble_secondary_strength",
    "rumble_secondary_duration",
    "repeat_count",
    "repeat_add_to_value",
    "repeat_add_to_address",
];
fn validate_cheat(cheat: &Cheat) -> Result<()> {
    ensure!(
        !cheat.name.trim().is_empty()
            && cheat.name.len() <= 1024
            && !cheat.name.contains(['\n', '\r', '\0', '"', '\\']),
        "Cheat name must be a single line without quotes or backslashes"
    );
    ensure!(
        cheat.code.len() <= 8192
            && cheat
                .code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b" +-:._".contains(&b)),
        "Invalid cheat code characters"
    );
    ensure!(
        !cheat.code.trim().is_empty() || cheat.fields.get("handler").is_some_and(|h| h == "1"),
        "Enter a code or import a RetroArch memory cheat"
    );
    for (key, value) in &cheat.fields {
        ensure!(
            CHEAT_FIELDS.contains(&key.as_str())
                && (value == "true" || value == "false" || value.parse::<u32>().is_ok()),
            "Invalid cheat field {key}"
        );
    }
    Ok(())
}

pub fn import_cheats(text: &str) -> Result<Vec<Cheat>> {
    ensure!(text.len() <= 4 * 1024 * 1024, "Cheat file exceeds 4 MiB");
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .context("Expected a RetroArch .cht file")?;
        let value = value.trim();
        let value = if value.starts_with('"') {
            value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .context("Unclosed quote in .cht file")?
        } else {
            value
        };
        ensure!(
            values
                .insert(key.trim().to_owned(), value.to_owned())
                .is_none(),
            "Duplicate cheat-file key"
        );
    }
    let count: usize = values
        .get("cheats")
        .context("Missing cheats count in .cht file")?
        .parse()?;
    ensure!(count <= 4096, "Cheat file exceeds 4096 entries");
    let mut result = Vec::new();
    for index in 0..count {
        let prefix = format!("cheat{index}_");
        let code = values
            .get(&format!("{prefix}code"))
            .cloned()
            .unwrap_or_default();
        let name = values
            .get(&format!("{prefix}desc"))
            .cloned()
            .unwrap_or_else(|| format!("Cheat {}", index + 1));
        let fields = CHEAT_FIELDS
            .iter()
            .filter_map(|key| {
                values
                    .get(&format!("{prefix}{key}"))
                    .map(|value| (key.to_string(), value.clone()))
            })
            .collect();
        let cheat = Cheat {
            name,
            code,
            enabled: false,
            fields,
        };
        validate_cheat(&cheat)?;
        result.push(cheat);
    }
    Ok(result)
}
pub fn export_cheats(profile: &Profile) -> Result<String> {
    let mut text = format!("cheats = {}\n", profile.cheats.len());
    for (index, cheat) in profile.cheats.iter().enumerate() {
        validate_cheat(cheat)?;
        text.push_str(&format!(
            "cheat{index}_desc = \"{}\"\ncheat{index}_code = \"{}\"\ncheat{index}_enable = {}\n",
            cheat.name,
            cheat.code,
            cheat.enabled && profile.cheats_enabled
        ));
        for (key, value) in &cheat.fields {
            text.push_str(&format!("cheat{index}_{key} = {value}\n"));
        }
    }
    Ok(text)
}

pub fn attach_cheats(
    profile: &Profile,
    option: &crate::emulator::RomEmulatorOption,
    plan: &mut crate::emulator::LaunchPlan,
) -> Result<()> {
    if !profile.cheats_enabled || !profile.cheats.iter().any(|c| c.enabled) {
        return Ok(());
    }
    ensure!(
        option.runtime_kind == crate::emulator::EmulatorRuntimeKind::RetroArch,
        "Automatic cheats currently require RetroArch. Disable cheats for this game or select RetroArch; export .cht for manual use elsewhere"
    );
    let content = plan
        .retroarch_content
        .as_ref()
        .context("Missing exact RetroArch content for cheats")?;
    let core_name = query_core_name(&content.core)?;
    ensure!(
        !core_name.is_empty()
            && !core_name.contains(['/', '\\', '"', '\n', '\r'])
            && core_name != "."
            && core_name != "..",
        "Invalid core library name"
    );
    let root = data_root()?.join("sessions");
    fs::create_dir_all(&root)?;
    let session = tempfile::tempdir_in(&root)?;
    let core_dir = session.path().join(&core_name);
    fs::create_dir(&core_dir)?;
    let file = content
        .content
        .file_stem()
        .context("Content has no filename")?
        .to_string_lossy();
    fs::write(
        core_dir.join(format!("{file}.cht")),
        export_cheats(profile)?,
    )?;
    let directory = session.path().to_string_lossy().replace('\\', "/");
    ensure!(
        !directory.contains(['"', '\n', '\r']),
        "Unsupported cheat directory characters"
    );
    let config = session.path().join("cheats.cfg");
    fs::write(
        &config,
        format!(
            "cheat_database_path = \"{directory}\"\napply_cheats_after_load = \"true\"\napply_cheats_after_toggle = \"true\"\n"
        ),
    )?;
    crate::controller_launch::attach_config(plan, &option.executable, &config)?;
    plan.cleanup_paths.push(session.keep());
    Ok(())
}

fn query_core_name(core: &Path) -> Result<String> {
    use std::process::{Command, Stdio};
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--cheat-core-identity")
        .arg(core)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn()?;
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            ensure!(
                status.success(),
                "Could not inspect this core's cheat identity; it may require its own runtime libraries"
            );
            let mut name = String::new();
            child
                .stdout
                .take()
                .context("Missing core identity output")?
                .take(4096)
                .read_to_string(&mut name)?;
            return Ok(name.trim().to_owned());
        }
        if started.elapsed().as_secs() >= 5 {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Core identity query timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

pub fn core_identity_helper() -> i32 {
    let result = (|| -> Result<()> {
        let path = PathBuf::from(std::env::args_os().nth(2).context("Core path required")?);
        let hash = lunchbox_controller_probe::file_hash(&path)?;
        let identity = lunchbox_controller_probe::libretro_input::core_identity(&path, &hash)?;
        println!("{}", identity.core_name);
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("{error:#}");
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn community_base_hashes_match_complete_alternatives_and_cancel() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), b"abc").unwrap();
        let mut base = BaseRom {
            crc32: "352441C2".into(),
            sha1: "a9993e364706816aba3e25717850c26c9cd0d89d".into(),
            md5: "900150983cd24fb0d6963f7d28e17f72".into(),
            ..Default::default()
        };
        let cancel = AtomicBool::new(false);
        validate_base(file.path(), &[base.clone()], &cancel).unwrap();
        base.md5 = "bad".into();
        assert!(validate_base(file.path(), &[base.clone()], &cancel).is_err());
        validate_base(
            file.path(),
            &[
                base,
                BaseRom {
                    crc32: "352441c2".into(),
                    ..Default::default()
                },
            ],
            &cancel,
        )
        .unwrap();
        cancel.store(true, Ordering::Relaxed);
        assert!(
            validate_base(
                file.path(),
                &[BaseRom {
                    crc32: "352441c2".into(),
                    ..Default::default()
                }],
                &cancel
            )
            .is_err()
        );
        let old: Patch = serde_json::from_str(
            r#"{"name":"p","file":"p.ips","sha256":"a","format":"IPS","enabled":false}"#,
        )
        .unwrap();
        assert!(old.expected_inputs.is_empty());
    }
    #[test]
    fn cheat_import_is_opt_in_and_retains_memory_fields() {
        let cheats = import_cheats("cheats = 1\ncheat0_desc = \"Lives\"\ncheat0_code = \"\"\ncheat0_enable = true\ncheat0_handler = 1\ncheat0_address = 123\ncheat0_value = 9\n").unwrap();
        assert!(!cheats[0].enabled);
        assert_eq!(cheats[0].fields["address"], "123");
        let mut p = Profile {
            cheats,
            cheats_enabled: true,
            ..Profile::default()
        };
        p.cheats[0].enabled = true;
        let text = export_cheats(&p).unwrap();
        assert!(text.contains("cheat0_enable = true"));
        assert!(text.contains("cheat0_address = 123"));
        assert_eq!(import_cheats(&text).unwrap()[0].fields, p.cheats[0].fields);
    }
    #[test]
    fn cheat_config_injection_is_rejected() {
        let p = Profile {
            cheats: vec![Cheat {
                name: "bad\"\nconfig = 1".into(),
                code: "ABC".into(),
                ..Cheat::default()
            }],
            ..Profile::default()
        };
        assert!(export_cheats(&p).is_err());
        assert!(import_cheats("cheats = 9000").is_err());
    }
    #[test]
    fn profiles_persist_separately_and_patch_cache_invalidates() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::settings::SettingsStore::at(dir.path().join("state.db")).unwrap();
        let p = Profile {
            base_sha256: "a".repeat(64),
            ..Profile::default()
        };
        p.save(&store, "one").unwrap();
        assert_eq!(
            Profile::load(&store, "one").unwrap().base_sha256,
            p.base_sha256
        );
        assert!(Profile::load(&store, "two").unwrap().base_sha256.is_empty());
        let source = dir.path().join("game.nes");
        fs::write(&source, b"abc").unwrap();
        fs::create_dir(dir.path().join("patches")).unwrap();
        let bytes = b"PATCH\0\0\x01\0\x01XEOF";
        fs::write(dir.path().join("patches/p.ips"), bytes).unwrap();
        let mut profile = Profile {
            patches: vec![Patch {
                name: "p".into(),
                file: "p.ips".into(),
                sha256: digest(bytes),
                format: "IPS".into(),
                enabled: true,
                source_url: String::new(),
                source_name: String::new(),
                expected_inputs: Vec::new(),
            }],
            ..Profile::default()
        };
        let cancel = AtomicBool::new(false);
        let output = prepare_in(&source, &profile, "one", &cancel, dir.path()).unwrap();
        assert_eq!(fs::read(&source).unwrap(), b"abc");
        assert_eq!(fs::read(&output).unwrap(), b"aXc");
        let old = fingerprint(&output).unwrap();
        assert_eq!(
            prepare_in(&source, &profile, "one", &cancel, dir.path()).unwrap(),
            output
        );
        assert_eq!(fingerprint(&output).unwrap(), old);
        // Adding metadata to an already cached patch must revalidate the ROM.
        profile.patches[0].expected_inputs = vec![BaseRom {
            crc32: "00000000".into(),
            ..Default::default()
        }];
        assert!(prepare_in(&source, &profile, "one", &cancel, dir.path()).is_err());
        profile.patches[0].expected_inputs.clear();
        fs::write(&source, b"1234").unwrap();
        prepare_in(&source, &profile, "one", &cancel, dir.path()).unwrap();
        assert_eq!(fs::read(&output).unwrap(), b"1X34");
        fs::write(dir.path().join("patches/p.ips"), b"damaged").unwrap();
        assert!(prepare_in(&source, &profile, "one", &cancel, dir.path()).is_err());
    }
}
