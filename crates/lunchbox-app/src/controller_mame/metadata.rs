//! Same-core dependency discovery, not an import of an unrelated MAME database.
//! Contract: libretro/mame 4fc9a9312baaf34963847f884961ad9793fbbc1d,
//! clifront.cpp::listxml/start_execution and infoxml.cpp::output_device_refs.
//! The owning field-inspection worker fingerprints the runtime/core before and
//! after this operation. Never call this as independent provenance validation.
use super::dependencies::MachineDependencies;
use super::runtime::{Worker, check, config_path};
use anyhow::{Context, Result, bail, ensure};
use quick_xml::{Reader, events::Event};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const MAX_XML_BYTES: usize = 16 * 1024 * 1024;
const MAX_MACHINES: usize = 4096;
const MAX_QUERIES: usize = 32;

fn short_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

struct Document {
    build: String,
    machines: BTreeMap<String, MachineDependencies>,
}

/// Parse only native -nodtd -listxml output. No external entities, file
/// references, display-title matching or metadata-supplied source paths.
fn parse(bytes: &[u8], expected: &str) -> Result<Document> {
    ensure!(bytes.len() <= MAX_XML_BYTES, "MAME metadata exceeds 16 MiB");
    let text = std::str::from_utf8(bytes).context("MAME metadata is not UTF-8")?;
    let mut reader = Reader::from_str(text);
    let mut stack = Vec::<String>::new();
    let mut machines = BTreeMap::new();
    let mut current: Option<MachineDependencies> = None;
    let mut build = None;
    let mut nodes = 0usize;
    loop {
        let event = reader
            .read_event()
            .context("Reading native MAME metadata XML")?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                nodes += 1;
                ensure!(
                    nodes <= 200_000 && stack.len() < 32,
                    "MAME metadata exceeds structural limits"
                );
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                let mut attributes = BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let key = std::str::from_utf8(attribute.key.as_ref())?.to_owned();
                    let value = attribute.unescape_value()?.into_owned();
                    ensure!(
                        attributes.insert(key, value).is_none(),
                        "Duplicate MAME metadata attribute"
                    );
                }
                match stack.len() {
                    0 => {
                        ensure!(
                            name == "mame" && !empty && build.is_none(),
                            "Expected one nonempty MAME metadata root"
                        );
                        ensure!(
                            attributes.get("mameconfig").map(String::as_str) == Some("10"),
                            "Unsupported MAME metadata configuration version"
                        );
                        let value = attributes
                            .get("build")
                            .context("Missing MAME metadata build identity")?;
                        ensure!(
                            !value.is_empty()
                                && value.len() <= 512
                                && !value.chars().any(char::is_control),
                            "Invalid MAME metadata build identity"
                        );
                        build = Some(value.clone());
                    }
                    1 => {
                        ensure!(
                            name == "machine" && !empty && current.is_none(),
                            "Expected a complete MAME machine record"
                        );
                        let machine = attributes
                            .get("name")
                            .context("Missing MAME machine short name")?;
                        let parent = attributes.get("romof").cloned();
                        ensure!(
                            short_name(machine) && parent.as_deref().is_none_or(short_name),
                            "Invalid MAME dependency short name"
                        );
                        current = Some(MachineDependencies {
                            name: machine.clone(),
                            rom_parent: parent,
                            devices: Vec::new(),
                            has_roms: false,
                            disks: Vec::new(),
                            disk_sha1: BTreeMap::new(),
                        });
                    }
                    2 if matches!(name.as_str(), "rom" | "disk" | "device_ref") => {
                        ensure!(empty, "MAME dependency XML entries must be empty elements");
                        let machine = current
                            .as_mut()
                            .context("MAME dependency outside a machine")?;
                        if name == "device_ref" {
                            let device = attributes
                                .get("name")
                                .context("Missing MAME device short name")?;
                            ensure!(short_name(device), "Invalid MAME device short name");
                            ensure!(
                                machine.devices.len() < MAX_MACHINES,
                                "Too many MAME device references"
                            );
                            machine.devices.push(device.clone());
                        } else {
                            let optional = attributes
                                .get("optional")
                                .map(String::as_str)
                                .unwrap_or("no");
                            let status = attributes
                                .get("status")
                                .map(String::as_str)
                                .unwrap_or("good");
                            ensure!(
                                matches!(optional, "yes" | "no")
                                    && matches!(status, "good" | "baddump" | "nodump"),
                                "Unknown MAME ROM availability metadata"
                            );
                            // Undumped and optional images are not mandatory local files.
                            if optional != "yes" && status != "nodump" {
                                if name == "rom" {
                                    machine.has_roms = true;
                                } else {
                                    let disk =
                                        attributes.get("name").context("Missing MAME disk name")?;
                                    ensure!(
                                        super::dependencies::component(disk),
                                        "Invalid MAME disk filename"
                                    );
                                    ensure!(
                                        machine.disks.len() < MAX_MACHINES,
                                        "Too many MAME disks"
                                    );
                                    let identity = attributes
                                        .get("sha1")
                                        .filter(|_| status == "good")
                                        .map(|hash| hash.to_ascii_lowercase());
                                    if machine.disks.contains(disk) {
                                        ensure!(
                                            machine.disk_sha1.get(disk) == identity.as_ref(),
                                            "Conflicting repeated MAME disk identity for {disk}"
                                        );
                                    }
                                    machine.disks.push(disk.clone());
                                    if status == "good" {
                                        if let Some(hash) = attributes.get("sha1") {
                                            ensure!(
                                                hash.len() == 40
                                                    && hash
                                                        .bytes()
                                                        .all(|byte| byte.is_ascii_hexdigit()),
                                                "Invalid MAME disk SHA-1"
                                            );
                                            let hash = hash.to_ascii_lowercase();
                                            if let Some(previous) =
                                                machine.disk_sha1.insert(disk.clone(), hash.clone())
                                            {
                                                ensure!(
                                                    previous == hash,
                                                    "Conflicting MAME disk identity for {disk}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(element) => {
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                ensure!(
                    stack.pop().as_deref() == Some(name.as_str()),
                    "Mismatched MAME metadata element"
                );
                if stack.len() == 1 && name == "machine" {
                    let mut machine = current.take().context("Missing MAME machine record")?;
                    machine.devices.sort();
                    machine.devices.dedup();
                    machine.disks.sort();
                    machine.disks.dedup();
                    ensure!(
                        machines.len() < MAX_MACHINES,
                        "Too many MAME metadata machines"
                    );
                    ensure!(
                        machines.insert(machine.name.clone(), machine).is_none(),
                        "Duplicate MAME metadata machine"
                    );
                }
            }
            Event::Text(value) => {
                let value = value.unescape()?;
                ensure!(
                    stack.len() >= 3 || value.trim().is_empty(),
                    "Unexpected text outside a MAME metadata field"
                );
            }
            Event::CData(_) => ensure!(stack.len() >= 3, "Unexpected MAME metadata CDATA"),
            Event::Decl(_) => ensure!(
                build.is_none() && stack.is_empty(),
                "Misplaced MAME XML declaration"
            ),
            Event::Comment(_) => {}
            Event::Eof => break,
            _ => bail!("Unsupported MAME XML directive; discovery requires native -nodtd output"),
        }
    }
    ensure!(
        stack.is_empty() && current.is_none(),
        "Incomplete MAME metadata document"
    );
    ensure!(
        machines.contains_key(expected),
        "Native metadata omitted the exact requested machine {expected}"
    );
    Ok(Document {
        build: build.context("Missing MAME metadata root")?,
        machines,
    })
}

/// Resolve the reachable graph, asking the selected core for missing parents or
/// devices only. A name in a different release's database is never substituted.
pub(super) fn discover(
    request: &super::InspectionRequest,
    roots: &[PathBuf],
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<Vec<super::InspectionInput>> {
    ensure!(
        short_name(&request.machine),
        "MAME discovery requires an exact machine short name"
    );
    super::dependencies::validate_roots(roots)?;
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-mame-metadata-")
        .tempdir()?;
    let root = directory.path().canonicalize()?;
    for child in ["home", "temporary", "system", "saves", "states", "snaps"] {
        std::fs::create_dir(root.join(child))?;
    }
    let mut config = String::from(
        "config_save_on_exit = \"false\"\nvideo_driver = \"null\"\naudio_driver = \"null\"\nmenu_driver = \"null\"\ninput_driver = \"null\"\ninput_joypad_driver = \"null\"\nauto_overrides_enable = \"false\"\nauto_remaps_enable = \"false\"\ngame_specific_options = \"false\"\nsavestate_auto_load = \"false\"\nsavestate_auto_save = \"false\"\n",
    );
    for (key, child) in [
        ("system_directory", "system"),
        ("savefile_directory", "saves"),
        ("savestate_directory", "states"),
        ("screenshot_directory", "snaps"),
        ("core_options_path", "core-options.cfg"),
    ] {
        config.push_str(&format!("{key} = {}\n", config_path(&root.join(child))?));
    }
    std::fs::write(root.join("retroarch.cfg"), config)?;
    std::fs::write(root.join("core-options.cfg"), &request.core_options)?;
    let mut index = BTreeMap::<String, MachineDependencies>::new();
    let mut build = None;
    for query in 0..=MAX_QUERIES {
        check(cancel, deadline)?;
        let mut pending = vec![request.machine.clone()];
        let mut visited = BTreeSet::new();
        let mut missing = None;
        while let Some(name) = pending.pop() {
            check(cancel, deadline)?;
            if !visited.insert(name.clone()) {
                continue;
            }
            ensure!(
                visited.len() <= MAX_MACHINES,
                "MAME dependency closure exceeds limit"
            );
            let Some(entry) = index.get(&name) else {
                missing = Some(name);
                break;
            };
            pending.extend(entry.rom_parent.iter().cloned());
            pending.extend(entry.devices.iter().cloned());
        }
        let Some(name) = missing else {
            return super::dependencies::resolve(
                &request.machine,
                &index.into_values().collect::<Vec<_>>(),
                roots,
                &request.inputs,
                cancel,
            );
        };
        ensure!(
            query < MAX_QUERIES,
            "MAME dependency discovery exceeds 32 native metadata queries; use an explicit manifest"
        );
        let document = parse(
            &query_native(request, &root, &name, deadline, cancel)?,
            &name,
        )?;
        if let Some(expected) = &build {
            ensure!(
                *expected == document.build,
                "MAME metadata build changed during discovery"
            );
        } else {
            build = Some(document.build);
        }
        for (name, entry) in document.machines {
            if let Some(previous) = index.get(&name) {
                ensure!(*previous == entry, "MAME metadata changed for {name}");
            } else {
                ensure!(index.len() < MAX_MACHINES, "Too many MAME metadata records");
                index.insert(name, entry);
            }
        }
    }
    unreachable!("bounded discovery either resolves or reports its query limit")
}

#[cfg(not(unix))]
fn query_native(
    _request: &super::InspectionRequest,
    _root: &Path,
    _machine: &str,
    _deadline: Instant,
    _cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    bail!("MAME dependency discovery requires the native Unix runtime adapter")
}

#[cfg(unix)]
fn query_native(
    request: &super::InspectionRequest,
    root: &Path,
    machine: &str,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    use std::process::{Command, Stdio};
    ensure!(short_name(machine), "Invalid native MAME metadata query");
    // The wrapper's .cmd parser has fixed-size argument storage. Exact short
    // names and this short command do not admit wildcard or extra option input.
    std::fs::write(
        root.join("metadata.cmd"),
        format!("mame -noreadconfig -nowriteconfig -nodtd -listxml {machine}\n"),
    )?;
    {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        // Read in the supervising thread so cancellation never waits for a
        // scoped reader to join while another process retains the output handle.
        let (mut output, child_output) = UnixStream::pair()?;
        output.set_nonblocking(true)?;
        let child_output: OwnedFd = child_output.into();
        let mut worker = Worker(
            Command::new(&request.retroarch)
                .arg("--config")
                .arg(root.join("retroarch.cfg"))
                .arg("--libretro")
                .arg(&request.core)
                .arg(root.join("metadata.cmd"))
                .envs(request.environment.iter().cloned())
                .env("HOME", root.join("home"))
                .env("XDG_CONFIG_HOME", root.join("home"))
                .env("TMPDIR", root.join("temporary"))
                .env("TMP", root.join("temporary"))
                .env("TEMP", root.join("temporary"))
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::from(child_output))
                .stderr(Stdio::null())
                .spawn()
                .context("Starting selected core's MAME dependency metadata query")?,
        );
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 65536];
        let mut eof = false;
        let mut exited = false;
        loop {
            check(cancel, deadline)?;
            if !eof {
                match output.read(&mut buffer) {
                    Ok(0) => eof = true,
                    Ok(count) => {
                        ensure!(
                            bytes.len() + count <= MAX_XML_BYTES,
                            "MAME metadata exceeds 16 MiB"
                        );
                        bytes.extend_from_slice(&buffer[..count]);
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if !exited && let Some(status) = worker.0.try_wait()? {
                // Successful CLI verbs deliberately return 1 from MAME and
                // false from retro_load_game. RetroArch may therefore exit 1.
                // Complete XML containing the exact name is required as well;
                // neither a generic load failure nor exit 0 establishes success.
                ensure!(
                    matches!(status.code(), Some(0 | 1)),
                    "MAME metadata process failed: {status}"
                );
                exited = true;
            }
            if exited && eof {
                break;
            }
            if eof || bytes.is_empty() {
                std::thread::sleep(Duration::from_millis(20));
            } else {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        check(cancel, deadline)?;
        Ok(bytes)
    }
}
