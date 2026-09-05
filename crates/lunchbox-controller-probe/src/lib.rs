//! Isolated SDL3 inventory for emulator adapters. Optionally opens explicitly
//! selected gamepads to query resolved bindings; never sends rumble or writes settings.
//! The caller must supply a trusted SDL library from the target runtime.
use anyhow::{Context, Result, bail, ensure};
use libloading::Library;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};

const SUBSYSTEMS: u32 = 0x00000200 | 0x00002000; // SDL_INIT_JOYSTICK | SDL_INIT_GAMEPAD

pub mod bindings;
pub mod duckstation;
pub mod linux_classic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Process-local SDL instance ID, not an emulator player number.
    pub instance_id: u32,
    pub name: Option<String>,
    pub path: Option<String>,
    pub guid: String,
    pub vendor: u16,
    pub product: u16,
    pub product_version: u16,
    pub is_gamepad: bool,
    /// SDL's hint before opening; DuckStation can choose a different fallback.
    pub reported_player_index: i32,
    pub mapping: Option<String>,
    /// Absent unless explicitly requested and queried from an opened gamepad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<bindings::ResolvedGamepad>,
    /// Present only for a verified Linux classic backend with matching counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_classic: Option<linux_classic::ClassicMap>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub host_os: String,
    pub library: PathBuf,
    pub library_sha256: String,
    pub sdl_version: i32,
    pub sdl_revision: Option<String>,
    pub requested_hints: BTreeMap<String, String>,
    pub effective_hints: BTreeMap<String, Option<String>>,
    pub mapping_database_sha256: Option<String>,
    pub devices: Vec<Device>,
    pub warnings: Vec<String>,
}

impl Snapshot {
    /// No fallback to names, GUIDs or VID/PID: identical models can have different
    /// layouts, and device-less HID backends need another verified identity path.
    pub fn device_at_path(&self, path: &str) -> Result<&Device> {
        ensure!(!path.is_empty(), "A nonempty device path is required");
        let mut matches = self
            .devices
            .iter()
            .filter(|d| d.path.as_deref() == Some(path));
        let device = matches
            .next()
            .context("No SDL device at the exact requested path")?;
        ensure!(matches.next().is_none(), "Ambiguous SDL device path");
        Ok(device)
    }
}

#[repr(C)]
struct Guid {
    data: [u8; 16],
}

type Free = unsafe extern "C" fn(*mut c_void);
type GetString = unsafe extern "C" fn() -> *const c_char;

struct Allocation {
    pointer: *mut c_void,
    free: Free,
}
impl Drop for Allocation {
    fn drop(&mut self) {
        // SDL owns these allocations; never use Rust's allocator to release them.
        unsafe { (self.free)(self.pointer) }
    }
}
struct Subsystems(unsafe extern "C" fn(u32));
impl Drop for Subsystems {
    fn drop(&mut self) {
        unsafe { (self.0)(SUBSYSTEMS) }
    }
}

/// SDL promises NUL-terminated strings or null from these getter APIs. Copy
/// immediately; reject invalid UTF-8 rather than silently changing identity paths.
unsafe fn string(pointer: *const c_char) -> Result<Option<String>> {
    if pointer.is_null() {
        return Ok(None);
    }
    Ok(Some(
        unsafe { CStr::from_ptr(pointer) }.to_str()?.to_owned(),
    ))
}

fn file_hash(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).with_context(|| format!("Reading {}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn parse_hint(text: &str) -> Result<(String, String)> {
    let (name, value) = text
        .split_once('=')
        .context("Hint must be SDL_NAME=value")?;
    ensure!(
        name.starts_with("SDL_")
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
        "Invalid SDL hint name"
    );
    ensure!(
        !value.contains(['\0', '\r', '\n']),
        "Invalid SDL hint value"
    );
    Ok((name.into(), value.into()))
}

/// Run in a fresh helper process on its main thread, not in the Qt GUI process.
/// `library_path` must be a trusted runtime library: loading native code executes it.
pub fn inspect(
    library_path: &Path,
    mapping_db: Option<&Path>,
    hints: BTreeMap<String, String>,
) -> Result<Snapshot> {
    inspect_with_bindings(library_path, mapping_db, hints, &[])
}

/// Opening is opt-in by exact unique runtime path. SDL drivers may perform their
/// normal device initialization, but this helper never requests LED/rumble changes.
pub fn inspect_with_bindings(
    library_path: &Path,
    mapping_db: Option<&Path>,
    hints: BTreeMap<String, String>,
    binding_paths: &[String],
) -> Result<Snapshot> {
    let unique: std::collections::BTreeSet<_> = binding_paths.iter().collect();
    ensure!(
        unique.len() == binding_paths.len(),
        "Duplicate binding path"
    );
    ensure!(
        binding_paths.iter().all(|p| !p.is_empty()),
        "Empty binding path"
    );
    ensure!(
        library_path.is_absolute(),
        "Pass the absolute SDL3 library path from the target runtime"
    );
    let library_path = library_path
        .canonicalize()
        .context("Resolving SDL3 library")?;
    let library_sha256 = file_hash(&library_path)?;
    let mut hints = hints;
    for (name, value) in &hints {
        parse_hint(&format!("{name}={value}"))?;
    }
    let mapping_database_sha256 = if let Some(path) = mapping_db {
        ensure!(path.is_absolute(), "Mapping database path must be absolute");
        ensure!(
            !hints.contains_key("SDL_GAMECONTROLLERCONFIG_FILE"),
            "Specify the mapping database once"
        );
        let path = path.canonicalize().context("Resolving mapping database")?;
        hints.insert(
            "SDL_GAMECONTROLLERCONFIG_FILE".into(),
            path.to_str()
                .context("Mapping database path is not UTF-8")?
                .into(),
        );
        Some(file_hash(&path)?)
    } else {
        None
    };

    // All function signatures below follow SDL3's public C ABI, available since
    // 3.2.0. Library outlives every symbol, allocation and subsystem guard.
    unsafe {
        let library = Library::new(&library_path).context("Loading target SDL3 library")?;
        // Resolve an SDL3-only symbol before calling SDL_GetVersion: SDL2 has an
        // incompatible function with that name, and must never be called here.
        let get_path = *library
            .get::<unsafe extern "C" fn(u32) -> *const c_char>(b"SDL_GetJoystickPathForID\0")?;
        let get_version = *library.get::<unsafe extern "C" fn() -> c_int>(b"SDL_GetVersion\0")?;
        let version = get_version();
        ensure!(
            (3_002_000..4_000_000).contains(&version),
            "SDL 3.2 or newer (major 3) is required; got {version}"
        );
        let get_revision = *library.get::<GetString>(b"SDL_GetRevision\0")?;
        let get_error = *library.get::<GetString>(b"SDL_GetError\0")?;
        let set_hint = *library
            .get::<unsafe extern "C" fn(*const c_char, *const c_char, c_int) -> bool>(
                b"SDL_SetHintWithPriority\0",
            )?;
        let get_hint = *library
            .get::<unsafe extern "C" fn(*const c_char) -> *const c_char>(b"SDL_GetHint\0")?;
        let set_main_ready = *library.get::<unsafe extern "C" fn()>(b"SDL_SetMainReady\0")?;
        let init = *library.get::<unsafe extern "C" fn(u32) -> bool>(b"SDL_InitSubSystem\0")?;
        let quit = *library.get::<unsafe extern "C" fn(u32)>(b"SDL_QuitSubSystem\0")?;
        let free = *library.get::<Free>(b"SDL_free\0")?;
        let joysticks =
            *library.get::<unsafe extern "C" fn(*mut c_int) -> *mut u32>(b"SDL_GetJoysticks\0")?;
        let get_name = *library
            .get::<unsafe extern "C" fn(u32) -> *const c_char>(b"SDL_GetJoystickNameForID\0")?;
        let get_guid =
            *library.get::<unsafe extern "C" fn(u32) -> Guid>(b"SDL_GetJoystickGUIDForID\0")?;
        let vendor =
            *library.get::<unsafe extern "C" fn(u32) -> u16>(b"SDL_GetJoystickVendorForID\0")?;
        let product =
            *library.get::<unsafe extern "C" fn(u32) -> u16>(b"SDL_GetJoystickProductForID\0")?;
        let product_version = *library
            .get::<unsafe extern "C" fn(u32) -> u16>(b"SDL_GetJoystickProductVersionForID\0")?;
        let player = *library
            .get::<unsafe extern "C" fn(u32) -> c_int>(b"SDL_GetJoystickPlayerIndexForID\0")?;
        let is_gamepad = *library.get::<unsafe extern "C" fn(u32) -> bool>(b"SDL_IsGamepad\0")?;
        let mapping = *library
            .get::<unsafe extern "C" fn(u32) -> *mut c_char>(b"SDL_GetGamepadMappingForID\0")?;

        for (key, value) in &hints {
            let key_c = CString::new(key.as_str())?;
            let value_c = CString::new(value.as_str())?;
            // SDL_HINT_NORMAL mirrors an application's SDL_SetHint; inherited
            // environment overrides are retained and reported as effective values.
            set_hint(key_c.as_ptr(), value_c.as_ptr(), 1);
        }
        set_main_ready();
        if !init(SUBSYSTEMS) {
            let error = string(get_error())?.unwrap_or_default();
            quit(SUBSYSTEMS);
            bail!("SDL joystick initialization failed: {error}");
        }
        let _subsystems = Subsystems(quit);
        let mut effective_hints = BTreeMap::new();
        for name in hints.keys().map(String::as_str).chain([
            "SDL_JOYSTICK_LINUX_CLASSIC",
            "SDL_JOYSTICK_HIDAPI",
            "SDL_GAMECONTROLLERCONFIG_FILE",
            "SDL_GAMECONTROLLERCONFIG",
            "SDL_GAMECONTROLLER_IGNORE_DEVICES",
            "SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT",
        ]) {
            let name_c = CString::new(name)?;
            effective_hints.insert(name.to_owned(), string(get_hint(name_c.as_ptr()))?);
        }
        if mapping_db.is_some() {
            ensure!(
                effective_hints
                    .get("SDL_GAMECONTROLLERCONFIG_FILE")
                    .and_then(Option::as_ref)
                    == hints.get("SDL_GAMECONTROLLERCONFIG_FILE"),
                "The mapping database was overridden by the environment; specify its effective path"
            );
        }
        let mut count = 0;
        let ids = joysticks(&mut count);
        ensure!(
            !ids.is_null(),
            "SDL could not enumerate joysticks: {:?}",
            string(get_error())?
        );
        let _ids = Allocation {
            pointer: ids.cast(),
            free,
        };
        ensure!((0..=1024).contains(&count), "Invalid SDL device count");
        let mut devices = Vec::new();
        for id in std::slice::from_raw_parts(ids, count as usize) {
            let raw_mapping = mapping(*id);
            let _mapping = Allocation {
                pointer: raw_mapping.cast(),
                free,
            };
            devices.push(Device {
                instance_id: *id,
                name: string(get_name(*id))?,
                path: string(get_path(*id))?,
                guid: get_guid(*id)
                    .data
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect(),
                vendor: vendor(*id),
                product: product(*id),
                product_version: product_version(*id),
                reported_player_index: player(*id),
                is_gamepad: is_gamepad(*id),
                mapping: string(raw_mapping)?,
                resolved: None,
                linux_classic: None,
            });
        }
        // Validate the complete selection before opening any device. Never open
        // another same-model pad when a selected pad disappears.
        for path in binding_paths {
            let matches: Vec<_> = devices
                .iter()
                .filter(|d| d.path.as_ref() == Some(path))
                .collect();
            ensure!(
                matches.len() == 1,
                "Binding path must identify exactly one SDL device: {path}"
            );
            ensure!(
                matches[0].is_gamepad,
                "Selected device is not an SDL gamepad: {path}"
            );
        }
        for device in &mut devices {
            if device
                .path
                .as_ref()
                .is_some_and(|p| binding_paths.contains(p))
            {
                device.resolved = Some(query_bindings(
                    &library,
                    device.instance_id,
                    free,
                    get_error,
                )?);
                #[cfg(target_os = "linux")]
                if version == 3_002_020
                    && effective_hints
                        .get("SDL_JOYSTICK_LINUX_CLASSIC")
                        .and_then(Option::as_deref)
                        == Some("1")
                {
                    let path = Path::new(device.path.as_ref().context("Missing SDL device path")?);
                    // HIDAPI devices need their own physical adapter. Never apply
                    // the classic numbering algorithm merely because a hint is set.
                    if path.parent() == Some(Path::new("/dev/input"))
                        && path
                            .file_name()
                            .and_then(|p| p.to_str())
                            .is_some_and(|p| p.starts_with("js"))
                    {
                        let physical = linux_classic::read(path)?;
                        physical.validate_counts(
                            device
                                .resolved
                                .as_ref()
                                .context("Missing resolved bindings")?,
                        )?;
                        device.linux_classic = Some(physical);
                    }
                }
                // Verify that the selected instance still names the same path.
                ensure!(
                    string(get_path(device.instance_id))? == device.path,
                    "SDL device identity changed while querying bindings"
                );
            }
        }
        let mut warnings = vec![
            "Snapshot only: emulator player fallback IDs are not established.".into(),
            "Device hotplug, backend settings and mapping database changes require a new snapshot."
                .into(),
        ];
        for (name, value) in &hints {
            if effective_hints.get(name).and_then(Option::as_ref) != Some(value) {
                warnings.push(format!(
                    "Requested hint {name} was overridden; see effective_hints."
                ));
            }
        }
        Ok(Snapshot {
            schema_version: 2,
            host_os: std::env::consts::OS.into(),
            library: library_path,
            library_sha256,
            sdl_version: version,
            sdl_revision: string(get_revision())?,
            requested_hints: hints,
            effective_hints,
            mapping_database_sha256,
            devices,
            warnings,
        })
    }
}

struct OpenGamepad {
    pointer: *mut c_void,
    close: unsafe extern "C" fn(*mut c_void),
}
impl Drop for OpenGamepad {
    fn drop(&mut self) {
        unsafe { (self.close)(self.pointer) }
    }
}

unsafe fn query_bindings(
    library: &Library,
    id: u32,
    free: Free,
    get_error: GetString,
) -> Result<bindings::ResolvedGamepad> {
    unsafe {
        let open =
            *library.get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"SDL_OpenGamepad\0")?;
        let close = *library.get::<unsafe extern "C" fn(*mut c_void)>(b"SDL_CloseGamepad\0")?;
        let get_joystick = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(b"SDL_GetGamepadJoystick\0")?;
        let get_bindings = *library.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut i32,
        )
            -> *mut *mut bindings::SdlBinding>(
            b"SDL_GetGamepadBindings\0"
        )?;
        let count_axes = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_GetNumJoystickAxes\0")?;
        let count_buttons = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_GetNumJoystickButtons\0")?;
        let count_hats = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_GetNumJoystickHats\0")?;
        let pointer = open(id);
        ensure!(
            !pointer.is_null(),
            "Opening selected SDL gamepad {id}: {:?}",
            string(get_error())?
        );
        let _gamepad = OpenGamepad { pointer, close };
        let joystick = get_joystick(pointer);
        ensure!(!joystick.is_null(), "SDL gamepad has no joystick");
        let mut count = 0;
        let list = get_bindings(pointer, &mut count);
        ensure!(
            !list.is_null(),
            "Reading SDL bindings: {:?}",
            string(get_error())?
        );
        let _list = Allocation {
            pointer: list.cast(),
            free,
        };
        ensure!((0..=4096).contains(&count), "Invalid SDL binding count");
        let mut bindings = Vec::with_capacity(count as usize);
        for binding in std::slice::from_raw_parts(list, count as usize) {
            ensure!(!binding.is_null(), "Null SDL binding entry");
            bindings.push((**binding).copy()?);
        }
        let result = bindings::ResolvedGamepad {
            joystick_axes: count_axes(joystick).try_into()?,
            joystick_buttons: count_buttons(joystick).try_into()?,
            joystick_hats: count_hats(joystick).try_into()?,
            bindings,
        };
        result.validate()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(paths: &[Option<&str>]) -> Snapshot {
        Snapshot {
            schema_version: 1,
            host_os: "linux".into(),
            library: "/test/SDL3".into(),
            library_sha256: "".into(),
            sdl_version: 3_002_000,
            sdl_revision: None,
            requested_hints: BTreeMap::new(),
            effective_hints: BTreeMap::new(),
            mapping_database_sha256: None,
            warnings: vec![],
            devices: paths
                .iter()
                .enumerate()
                .map(|(i, path)| Device {
                    instance_id: i as u32 + 1,
                    name: Some("Xbox 360 Pad".into()),
                    path: path.map(str::to_owned),
                    guid: "same-reported-guid".into(),
                    vendor: 0x045e,
                    product: 0x028e,
                    product_version: 1,
                    is_gamepad: true,
                    reported_player_index: -1,
                    mapping: None,
                    resolved: None,
                    linux_classic: None,
                })
                .collect(),
        }
    }
    #[test]
    fn same_model_requires_exact_unique_path() {
        let s = snapshot(&[Some("/dev/input/js4"), Some("/dev/input/js5"), None]);
        assert_eq!(s.device_at_path("/dev/input/js5").unwrap().instance_id, 2);
        assert!(s.device_at_path("Xbox 360 Pad").is_err());
        assert!(s.device_at_path("").is_err());
        assert!(s.device_at_path("/dev/input/js6").is_err());
        assert!(
            snapshot(&[Some("/dev/input/js4"), Some("/dev/input/js4")])
                .device_at_path("/dev/input/js4")
                .is_err()
        );
    }
    #[test]
    fn snapshot_roundtrip_and_reordering_preserve_identity() {
        let mut s = snapshot(&[Some("/dev/input/js4"), Some("/dev/input/js5")]);
        s.devices.reverse();
        let copy: Snapshot = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(
            copy.device_at_path("/dev/input/js5").unwrap().instance_id,
            2
        );
        assert_eq!(
            copy.device_at_path("/dev/input/js5")
                .unwrap()
                .reported_player_index,
            -1
        );
    }
    #[test]
    fn hint_syntax_is_explicit() {
        assert_eq!(
            parse_hint("SDL_JOYSTICK_LINUX_CLASSIC=1").unwrap(),
            ("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into())
        );
        for bad in [
            "HOME=/tmp",
            "SDL_BAD",
            "SDL_BAD=x\nother",
            "SDL bad=x",
            "SDL_BAD=x\0",
        ] {
            assert!(parse_hint(bad).is_err());
        }
    }
}
