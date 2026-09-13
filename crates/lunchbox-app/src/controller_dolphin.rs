//! Dolphin GameCube input contract, pinned to 6094cfcf7b8fba733b3116fdf3414d51c1c0e4a4.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};

pub(crate) mod standalone;

/// Case-insensitive native INI keys. Raw cheat/patch lines are not settings and
/// are deliberately excluded; their original bytes remain in NativeSnapshot.
pub(crate) type NativeIni = BTreeMap<(String, String), String>;

fn profile_choices(root: &Path, setting: &str) -> Result<Vec<PathBuf>> {
    let root = root
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Dolphin profile root must be UTF-8"))?;
    ensure!(
        !setting.contains('\0'),
        "Dolphin profile choice contains NUL"
    );
    ensure!(
        setting.split(',').count() <= 256,
        "Too many Dolphin profile list entries"
    );
    let mut result = Vec::new();
    for choice in setting.split(',') {
        // InputProfile uses string concatenation, not absolute-path replacement.
        let path = PathBuf::from(format!(
            "{root}/{}",
            choice.trim_matches([' ', '\t', '\r', '\n'])
        ));
        let metadata = match std::fs::metadata(&path) {
            Ok(value) => Some(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if metadata.is_some_and(|value| value.is_dir()) {
            let mut pending = vec![(path, 0usize)];
            let mut found = Vec::new();
            let mut visited = 0usize;
            while let Some((directory, depth)) = pending.pop() {
                ensure!(
                    depth <= 64,
                    "Dolphin profile directory nesting exceeds preparation limit"
                );
                for entry in std::fs::read_dir(directory)? {
                    let entry = entry?;
                    visited += 1;
                    ensure!(
                        visited <= 10000,
                        "Dolphin profile directory exceeds preparation limit"
                    );
                    let path = entry.path();
                    let kind = entry.file_type()?;
                    if kind.is_dir() {
                        pending.push((path, depth + 1));
                    } else if !std::fs::metadata(&path)?.is_dir()
                        && path
                            .extension()
                            .and_then(|v| v.to_str())
                            .is_some_and(|v| v.eq_ignore_ascii_case("ini"))
                    {
                        // recursive_directory_iterator does not follow directory
                        // symlinks by default; file symlinks remain candidates.
                        found.push(path);
                    }
                }
            }
            found.sort();
            found.dedup();
            result.extend(found);
        } else {
            let file = PathBuf::from(format!(
                "{}.ini",
                path.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Dolphin profile path must be UTF-8"))?
            ));
            match std::fs::metadata(&file) {
                Ok(_) => result.push(file),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        ensure!(
            result.len() <= 10000,
            "Too many Dolphin controller profile choices"
        );
    }
    Ok(result)
}

pub(crate) fn merge_native_ini(values: &mut NativeIni, bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    ensure!(
        !text.contains('\0'),
        "Dolphin native INI contains a NUL byte"
    );
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let trim = |value: &str| value.trim_matches([' ', '\t', '\r', '\n']).to_owned();
    let mut section = None;
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        // Do not trim the line before classification: Dolphin recognizes
        // section headers and raw-line markers only in the first column.
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                section = Some(rest[..end].to_ascii_lowercase());
            }
            continue;
        }
        let Some(section) = &section else { continue };
        if line.starts_with(['#', '$', '+', '*']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = trim(key).to_ascii_lowercase();
        let value = trim(value);
        ensure!(
            value != "\"",
            "Ambiguous lone double-quote delimiter in Dolphin native INI"
        );
        let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
            value[1..value.len() - 1].to_owned()
        } else {
            value
        };
        if !key.is_empty() || !value.is_empty() {
            values.insert((section.clone(), key), value);
        }
    }
    Ok(())
}
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq)]
struct ConfigurationStamp {
    canonical: PathBuf,
    digest: [u8; 32],
}

fn read_configuration(path: &Path) -> Result<(Option<ConfigurationStamp>, Vec<u8>)> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((None, Vec::new()));
        }
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let canonical = path.canonicalize()?;
    let before = std::fs::metadata(&canonical)?;
    const LIMIT: u64 = 8 * 1024 * 1024;
    ensure!(
        before.is_file() && before.len() <= LIMIT,
        "Dolphin configuration must be a regular file at most 8 MiB: {}",
        path.display()
    );
    let file = std::fs::File::open(&canonical)?;
    ensure!(
        same_file_state(&before, &file.metadata()?),
        "Dolphin configuration replaced before reading"
    );
    let mut bytes = Vec::new();
    (&file).take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 == before.len()
            && same_file_state(&before, &file.metadata()?)
            && same_file_state(&before, &std::fs::metadata(path)?)
            && path.canonicalize()? == canonical,
        "Dolphin configuration changed while reading"
    );
    Ok((
        Some(ConfigurationStamp {
            canonical,
            digest: Sha256::digest(&bytes).into(),
        }),
        bytes,
    ))
}

pub(crate) struct NativeSnapshot {
    files: Vec<(PathBuf, Option<ConfigurationStamp>)>,
    directories: Vec<(PathBuf, DirectoryStamp)>,
    profile_queries: Vec<(PathBuf, String, Vec<PathBuf>)>,
    /// Ordered source bytes for native INI interpretation, never rewritten.
    pub(crate) documents: Vec<(PathBuf, Vec<u8>)>,
}

impl NativeSnapshot {
    pub(crate) fn game_settings(&self, paths: &NativePaths) -> Result<NativeIni> {
        let mut merged = NativeIni::new();
        for path in &paths.game_settings {
            let (_, bytes) = self
                .documents
                .iter()
                .find(|(candidate, _)| candidate == path)
                .ok_or_else(|| anyhow::anyhow!("Dolphin game INI was not snapshotted"))?;
            merge_native_ini(&mut merged, bytes)?;
        }
        Ok(merged)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        for (path, expected) in &self.directories {
            ensure!(
                &directory_stamp(path)? == expected,
                "Dolphin configuration directory changed during preparation: {}",
                path.display()
            );
        }
        for (root, setting, expected) in &self.profile_queries {
            ensure!(
                &profile_choices(root, setting)? == expected,
                "Dolphin controller profile choices changed during preparation"
            );
        }
        for (path, expected) in &self.files {
            ensure!(
                &read_configuration(path)?.0 == expected,
                "Dolphin native configuration changed during preparation: {}",
                path.display()
            );
        }
        Ok(())
    }
}

pub(crate) fn snapshot_native(paths: &NativePaths) -> Result<NativeSnapshot> {
    let mut files = Vec::new();
    let mut documents = Vec::new();
    for path in [&paths.main_settings, &paths.controller_settings]
        .into_iter()
        .chain(paths.game_settings.iter())
    {
        let (stamp, bytes) = read_configuration(path)?;
        files.push((path.clone(), stamp));
        documents.push((path.clone(), bytes));
    }
    for name in [
        "GBA.ini",
        "GCKeyNew.ini",
        "FreeLookController.ini",
        "WiimoteNew.ini",
    ] {
        let path = paths.user.join("Config").join(name);
        let (stamp, bytes) = read_configuration(&path)?;
        files.push((path.clone(), stamp));
        documents.push((path, bytes));
    }
    let mut snapshot = NativeSnapshot {
        files,
        documents,
        profile_queries: Vec::new(),
        directories: Vec::new(),
    };
    let game_settings = snapshot.game_settings(paths)?;
    for (prefix, directory) in [
        ("pad", "GCPad"),
        ("gba", "GBA"),
        ("gckey", "GCKey"),
        ("freelookcontroller", "FreeLookController"),
        ("wiimote", "Wiimote"),
    ] {
        for port in 1..=4 {
            if let Some(setting) =
                game_settings.get(&("controls".into(), format!("{prefix}profile{port}")))
            {
                let root = paths.user.join("Config/Profiles").join(directory);
                let choices = profile_choices(&root, setting)?;
                let selected = choices.first().ok_or_else(|| {
                    anyhow::anyhow!("Dolphin {prefix}Profile{port} has no existing profile choices")
                })?;
                let (stamp, bytes) = read_configuration(selected)?;
                ensure!(
                    stamp.is_some(),
                    "Dolphin selected controller profile disappeared"
                );
                let mut parsed = NativeIni::new();
                merge_native_ini(&mut parsed, &bytes)?;
                if !snapshot.files.iter().any(|(path, _)| path == selected) {
                    snapshot.files.push((selected.clone(), stamp));
                    snapshot.documents.push((selected.clone(), bytes));
                }
                snapshot
                    .profile_queries
                    .push((root, setting.clone(), choices));
            }
        }
    }
    // Modern references load independently in each game layer. They are
    // literal profile names, unlike the legacy comma/directory lists above.
    for layer_paths in paths.game_settings.chunks(4) {
        let mut layer = NativeIni::new();
        for path in layer_paths {
            let (_, bytes) = snapshot
                .documents
                .iter()
                .find(|(candidate, _)| candidate == path)
                .ok_or_else(|| anyhow::anyhow!("Dolphin layer INI was not snapshotted"))?;
            merge_native_ini(&mut layer, bytes)?;
        }
        for (system, prefix, directory) in [
            ("gcpad", "pad", "GCPad"),
            ("gcpad", "gba", "GBA"),
            ("wiimote", "wiimote", "Wiimote"),
        ] {
            for port in 1..=4 {
                if let Some(name) = layer.get(&(
                    format!("{system}.controls"),
                    format!("{prefix}profile{port}"),
                )) {
                    let root = paths.user.join("Config/Profiles").join(directory);
                    let root = root
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("Dolphin profile root must be UTF-8"))?;
                    let path = PathBuf::from(format!("{root}/{name}.ini"));
                    let (stamp, bytes) = read_configuration(&path)?;
                    ensure!(
                        stamp.is_some(),
                        "Dolphin modern controller profile does not exist: {}",
                        path.display()
                    );
                    let mut parsed = NativeIni::new();
                    merge_native_ini(&mut parsed, &bytes)?;
                    if !snapshot
                        .files
                        .iter()
                        .any(|(candidate, _)| candidate == &path)
                    {
                        snapshot.files.push((path.clone(), stamp));
                        snapshot.documents.push((path, bytes));
                    }
                }
            }
        }
    }
    let directories: std::collections::BTreeSet<_> = [paths.user.clone(), paths.assets.clone()]
        .into_iter()
        .chain(
            snapshot
                .files
                .iter()
                .filter_map(|(path, _)| path.parent().map(Path::to_path_buf)),
        )
        .collect();
    for path in directories {
        let stamp = directory_stamp(&path)?;
        snapshot.directories.push((path, stamp));
    }
    snapshot.verify()?;
    Ok(snapshot)
}

pub(crate) struct NativePaths {
    pub(crate) user: PathBuf,
    pub(crate) assets: PathBuf,
    /// Merge order: all system game INIs, then all user game INIs.
    pub(crate) game_settings: Vec<PathBuf>,
    pub(crate) controller_settings: PathBuf,
    pub(crate) main_settings: PathBuf,
}

#[derive(PartialEq, Eq)]
struct DirectoryStamp {
    existing: PathBuf,
    canonical: PathBuf,
    #[cfg(unix)]
    identity: (u64, u64),
}

/// Missing directories are anchored to their nearest existing ancestor.
/// Appearance, replacement and symlink retargeting change the resulting stamp.
fn directory_stamp(path: &Path) -> Result<DirectoryStamp> {
    let mut existing = path.to_path_buf();
    loop {
        match std::fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ensure!(existing.pop(), "No existing Dolphin directory ancestor");
            }
            Err(error) => return Err(error.into()),
        }
    }
    let canonical = existing.canonicalize()?;
    let metadata = std::fs::metadata(&canonical)?;
    ensure!(
        metadata.is_dir(),
        "Dolphin configuration ancestor is not a directory"
    );
    Ok(DirectoryStamp {
        existing,
        canonical,
        #[cfg(unix)]
        identity: {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        },
    })
}

pub(crate) struct InputSnapshot {
    content: ContentSnapshot,
    native: NativeSnapshot,
    frontend_directories: Vec<(PathBuf, DirectoryStamp)>,
    append: String,
}

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        for (path, expected) in &self.frontend_directories {
            ensure!(
                &directory_stamp(path)? == expected,
                "Dolphin frontend directory changed"
            );
        }
        self.content.verify()?;
        self.native.verify()
    }

    pub(crate) fn append_config(&self) -> &str {
        &self.append
    }
}

pub(crate) fn prepare(system: &Path, save: &Path, content: &Path) -> Result<InputSnapshot> {
    let content = prepare_raw_content(content)?;
    let paths = native_paths(system, Some(save), &content.game_id, content.revision)?;
    let native = snapshot_native(&paths)?;
    let mut frontend_directories = Vec::new();
    let mut append = String::new();
    for (key, path) in [("system_directory", system), ("savefile_directory", save)] {
        let value = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Dolphin frontend path must be UTF-8"))?;
        ensure!(
            !value
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | '\\')),
            "Dolphin frontend path cannot be represented in RetroArch configuration"
        );
        frontend_directories.push((path.to_path_buf(), directory_stamp(path)?));
        append.push_str(&format!("{key} = \"{value}\"\n"));
    }
    // The caller has already applied content/core sorting to this path. Keep
    // RetroArch from appending those components again for the owned session.
    append.push_str("sort_savefiles_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsavefiles_in_content_dir = \"false\"\n");
    let snapshot = InputSnapshot {
        content,
        native,
        frontend_directories,
        append,
    };
    snapshot.verify()?;
    Ok(snapshot)
}

/// Resolve the explicit frontend paths used by Boot.cpp without redirecting
/// saves or creating a second Dolphin user directory.
pub(crate) fn native_paths(
    system: &Path,
    save: Option<&Path>,
    game_id: &[u8; 6],
    revision: u8,
) -> Result<NativePaths> {
    ensure!(
        system.is_absolute() && system.is_dir(),
        "Dolphin requires an existing absolute system directory"
    );
    if let Some(save) = save {
        ensure!(
            save.is_absolute() && save.is_dir(),
            "Dolphin save directory must be existing and absolute"
        );
    }
    // Byte slicing is safe only after this check. Never allow a disc-supplied
    // ID to escape the configuration directory or introduce path separators.
    ensure!(
        game_id.iter().all(u8::is_ascii_alphanumeric),
        "Unsupported Dolphin configuration game ID"
    );
    let id = std::str::from_utf8(game_id)?;
    let user = save.map_or_else(|| system.join("dolphin-emu/User"), |save| save.join("User"));
    let assets = system.join("dolphin-emu/Sys");
    let names = [
        format!("{}.ini", &id[..1]),
        format!("{}.ini", &id[..3]),
        format!("{id}.ini"),
        format!("{id}r{revision}.ini"),
    ];
    let game_settings = [assets.join("GameSettings"), user.join("GameSettings")]
        .into_iter()
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .collect();
    Ok(NativePaths {
        controller_settings: user.join("Config/GCPadNew.ini"),
        main_settings: user.join("Config/Dolphin.ini"),
        user,
        assets,
        game_settings,
    })
}

pub(crate) struct ContentSnapshot {
    path: PathBuf,
    canonical: PathBuf,
    metadata: std::fs::Metadata,
    game_id: [u8; 6],
    revision: u8,
}

fn same_file_state(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if left.dev() != right.dev()
            || left.ino() != right.ino()
            || left.ctime() != right.ctime()
            || left.ctime_nsec() != right.ctime_nsec()
        {
            return false;
        }
    }
    left.is_file()
        && right.is_file()
        && left.len() == right.len()
        && left.modified().ok().is_some()
        && left.modified().ok() == right.modified().ok()
}

impl ContentSnapshot {
    /// Identity read from the classified raw disc, not a filename or library
    /// label. Native launch will classify and verify it again before handoff.
    pub(crate) fn game_identity(&self) -> ([u8; 6], u8) {
        (self.game_id, self.revision)
    }

    /// Re-run classification as well as checking path identity. This is a
    /// preparation race check, not a whole-disc content hash or a runtime lock.
    pub(crate) fn verify(&self) -> Result<()> {
        let current = prepare_raw_content(&self.path)?;
        ensure!(
            current.canonical == self.canonical
                && current.game_id == self.game_id
                && current.revision == self.revision
                && same_file_state(&current.metadata, &self.metadata),
            "Dolphin content changed during controller preparation"
        );
        Ok(())
    }
}

pub(crate) fn prepare_raw_content(path: &Path) -> Result<ContentSnapshot> {
    ensure!(
        path.is_absolute(),
        "Dolphin content requires an absolute path"
    );
    ensure!(
        path.extension()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("iso") || v.eq_ignore_ascii_case("gcm")),
        "This Dolphin reader requires a raw ISO/GCM disc; container decoding is not connected"
    );
    let canonical = path.canonicalize()?;
    let mut file = std::fs::File::open(&canonical)?;
    let metadata = file.metadata()?;
    ensure!(metadata.is_file(), "Dolphin disc must be a regular file");
    let game_id = validate_raw_gamecube(&mut file)?;
    file.seek(SeekFrom::Start(7))?;
    let mut revision = [0];
    file.read_exact(&mut revision)?;
    ensure!(
        same_file_state(&metadata, &file.metadata()?)
            && canonical == path.canonicalize()?
            && same_file_state(&metadata, &std::fs::metadata(path)?),
        "Dolphin disc changed during classification"
    );
    Ok(ContentSnapshot {
        path: path.to_path_buf(),
        canonical,
        metadata,
        game_id,
        revision: revision[0],
    })
}

/// Inspect an uncompressed disc stream. Container decoding must happen before
/// this boundary; extension or title matching is not content classification.
pub(crate) fn validate_raw_gamecube<R: Read + Seek>(reader: &mut R) -> Result<[u8; 6]> {
    let length = reader.seek(SeekFrom::End(0))?;
    ensure!(length >= 0x42c, "GameCube disc header is truncated");
    reader.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; 0x42c];
    reader.read_exact(&mut header)?;
    let word = |bytes: &[u8], offset: usize| -> u32 {
        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
    };
    ensure!(
        word(&header, 0x18) != 0x5d1c9ea3,
        "Wii content requires a Wii controller contract"
    );
    ensure!(
        word(&header, 0x1c) == 0xc2339f3d,
        "Not an uncompressed GameCube disc"
    );
    let offset = u64::from(word(&header, 0x424));
    let size = u64::from(word(&header, 0x428));
    ensure!(
        (12..=128 * 1024 * 1024).contains(&size)
            && offset.checked_add(size).is_some_and(|end| end <= length),
        "GameCube filesystem is out of bounds"
    );
    reader.seek(SeekFrom::Start(offset))?;
    let mut fst = vec![0; size as usize];
    reader.read_exact(&mut fst)?;
    let count = word(&fst, 8) as usize;
    let names = count
        .checked_mul(12)
        .ok_or_else(|| anyhow::anyhow!("GameCube entry count overflow"))?;
    ensure!(
        count > 0 && names <= fst.len() && fst[0] != 0 && fst.last() == Some(&0),
        "Invalid GameCube filesystem root"
    );
    // Directory end indices establish ancestry. Inspect all names and bounds,
    // but only a root boot.id has the meaning used by VolumeGC.
    let mut parents = vec![(0usize, count)];
    let mut boot_id = None;
    for index in 1..count {
        while parents.last().is_some_and(|(_, end)| index >= *end) {
            parents.pop();
        }
        let &(parent, parent_end) = parents
            .last()
            .ok_or_else(|| anyhow::anyhow!("Invalid GameCube directory nesting"))?;
        let entry = index * 12;
        let descriptor = word(&fst, entry);
        let name_offset = names + (descriptor & 0x00ff_ffff) as usize;
        ensure!(
            name_offset < fst.len(),
            "GameCube filename is out of bounds"
        );
        let tail = &fst[name_offset..];
        let end = tail
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| anyhow::anyhow!("Unterminated GameCube filename"))?;
        let name = &tail[..end];
        let value = word(&fst, entry + 4);
        let extent = word(&fst, entry + 8);
        if descriptor >> 24 != 0 {
            ensure!(
                value as usize == parent
                    && extent as usize > index
                    && extent as usize <= parent_end,
                "Invalid GameCube directory extent"
            );
            parents.push((index, extent as usize));
        } else {
            ensure!(
                u64::from(value) + u64::from(extent) <= length,
                "GameCube file is out of bounds"
            );
            if parent == 0 && name.eq_ignore_ascii_case(b"boot.id") {
                ensure!(boot_id.is_none(), "Ambiguous GameCube boot.id entries");
                boot_id = Some((u64::from(value), extent));
            }
        }
    }
    if let Some((offset, size)) = boot_id {
        if size >= 4 {
            reader.seek(SeekFrom::Start(offset))?;
            let mut magic = [0; 4];
            reader.read_exact(&mut magic)?;
            ensure!(
                &magic != b"BTID",
                "Triforce content requires its service/baseboard controller contract"
            );
        }
    }
    Ok(header[..6].try_into().unwrap())
}

/// These settings describe the mixed-trigger mode, not all Dolphin modes.
/// Native paths, content classification and lifecycle checks remain separate.
pub(crate) fn validate_gamecube_options(options: &BTreeMap<String, String>) -> Result<()> {
    for (key, expected) in [
        ("dolphin_save_load_settings", "enabled"),
        ("dolphin_enable_gamecube_mic", "disabled"),
        ("dolphin_enable_rumble", "disabled"),
        ("dolphin_alt_gc_ports_on_wii", "disabled"),
        ("dolphin_disc_based_games_boot_to_wii_menu", "disabled"),
        ("dolphin_hotkey_activate_microphone", "Disabled"),
    ] {
        ensure!(
            options.get(key).map(String::as_str) == Some(expected),
            "Dolphin GameCube mixed-trigger mode requires {key} = {expected}"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_ini_merge_preserves_sections_and_ignores_raw_patch_lines() {
        let mut values = NativeIni::new();
        merge_native_ini(
            &mut values,
            b"\xEF\xBB\xBF[GCPad1]\nDevice = \"evdev/0/pad\"\n$raw-cheat\nButtons/A = Button 0\n",
        )
        .unwrap();
        assert_eq!(
            values.get(&("gcpad1".to_owned(), "device".to_owned())),
            Some(&"evdev/0/pad".to_owned())
        );
        assert!(!values.values().any(|value| value.contains("raw-cheat")));
    }

    #[test]
    fn mixed_trigger_options_require_explicit_safe_defaults() {
        let options = [
            ("dolphin_save_load_settings", "enabled"),
            ("dolphin_enable_gamecube_mic", "disabled"),
            ("dolphin_enable_rumble", "disabled"),
            ("dolphin_alt_gc_ports_on_wii", "disabled"),
            ("dolphin_disc_based_games_boot_to_wii_menu", "disabled"),
            ("dolphin_hotkey_activate_microphone", "Disabled"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
        assert!(validate_gamecube_options(&options).is_ok());
        let mut changed = options;
        changed.insert("dolphin_enable_rumble".to_owned(), "enabled".to_owned());
        assert!(validate_gamecube_options(&changed).is_err());
    }
}
