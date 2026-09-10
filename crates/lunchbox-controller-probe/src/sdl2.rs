//! SDL2-only inventory for native BizHawk. Run in the target runtime and process
//! environment. It is not interchangeable with the SDL3/DuckStation inventory.
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Default)]
struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCounts {
    pub buttons: u32,
    pub axes: u32,
    pub hats: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub device_index: u32,
    pub instance_id: i32,
    pub path: Option<String>,
    pub name: Option<String>,
    pub is_game_controller: bool,
    /// Diagnostic model/backend GUID, never used as a unique device identity.
    #[serde(default)]
    pub guid: String,
    /// Effective SDL mapping, copied and freed using the same runtime's allocator.
    #[serde(default)]
    pub mapping: Option<String>,
    #[serde(default)]
    pub controls: Option<ControlCounts>,
    /// Populated only for an explicitly opened Linux /dev/input/jsN device
    /// whose kernel numbering agrees with the target SDL2 control counts.
    #[serde(default)]
    pub linux_classic: Option<crate::linux_classic::ClassicMap>,
    #[serde(default)]
    pub linux_evdev: Option<crate::sdl2_evdev::EvdevMap>,
    /// Instantaneous sampled state, not a neutral calibration. Deliberately
    /// excluded from routing equality because normal input changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampled_state: Option<crate::sdl2_mapping::InputState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub library: PathBuf,
    pub library_sha256: String,
    #[serde(default)]
    pub input_environment_sha256: String,
    pub version: [u8; 3],
    pub devices: Vec<Device>,
}

impl Snapshot {
    /// Translate a measured gesture using identical runtime routing. The caller
    /// must additionally keep its kernel topology guard across both probes;
    /// path equality alone cannot detect unplug/replug of an identical pad.
    pub fn game_controller_changes(
        &self,
        pressed: &Self,
        path: &str,
    ) -> Result<Vec<crate::sdl2_mapping::OutputChange>> {
        self.ensure_same_routing(pressed)?;
        let released_device = self.device_at_path(path)?;
        let pressed_device = pressed.device_at_path(path)?;
        ensure!(
            released_device.is_game_controller,
            "Logical SDL2 calibration requires a recognized GameController"
        );
        let mapping = crate::sdl2_mapping::parse(
            released_device
                .mapping
                .as_deref()
                .context("SDL2 GameController has no effective mapping")?,
        )?;
        crate::sdl2_mapping::changed_outputs(
            &mapping,
            released_device
                .controls
                .as_ref()
                .context("Missing SDL2 control counts")?,
            released_device
                .sampled_state
                .as_ref()
                .context("Missing released SDL2 sample")?,
            pressed_device
                .sampled_state
                .as_ref()
                .context("Missing pressed SDL2 sample")?,
        )
    }

    /// Recheck immediately before launch. SDL instance IDs belong to each probe
    /// process and are deliberately excluded; ordered paths and mappings do not.
    /// This detects observed changes, not hotplug occurring after the recheck.
    pub fn ensure_same_routing(&self, fresh: &Self) -> Result<()> {
        ensure!(
            self.schema_version == fresh.schema_version
                && self.library == fresh.library
                && self.library_sha256 == fresh.library_sha256
                && self.input_environment_sha256.len() == 64
                && self.input_environment_sha256 == fresh.input_environment_sha256
                && self.version == fresh.version,
            "SDL2 runtime changed during launch preparation"
        );
        ensure!(
            self.devices.len() == fresh.devices.len(),
            "SDL2 device inventory changed during launch preparation"
        );
        for (old, new) in self.devices.iter().zip(&fresh.devices) {
            ensure!(
                old.device_index == new.device_index
                    && old.path == new.path
                    && old.guid == new.guid
                    && old.mapping == new.mapping
                    && old.is_game_controller == new.is_game_controller
                    && old.controls == new.controls
                    && old.linux_classic == new.linux_classic
                    && old.linux_evdev == new.linux_evdev,
                "SDL2 device numbering or mapping changed during launch preparation"
            );
            ensure!(
                old.path.as_deref().is_some_and(|path| !path.is_empty()),
                "Cannot establish stable SDL2 ordering for a device without a runtime path"
            );
        }
        Ok(())
    }

    pub fn device_at_path(&self, path: &str) -> Result<&Device> {
        ensure!(!path.is_empty(), "SDL2 device path must not be empty");
        let mut found = self
            .devices
            .iter()
            .filter(|device| device.path.as_deref() == Some(path));
        let device = found
            .next()
            .context("No SDL2 device at the exact requested path")?;
        ensure!(found.next().is_none(), "Ambiguous SDL2 device path");
        Ok(device)
    }
}

struct JoystickLock(unsafe extern "C" fn());
/// Hash only controller/loader-relevant environment, never return its values.
/// Length-prefixing makes separators inside values unambiguous.
fn input_environment_sha256() -> Result<String> {
    let selected: std::collections::BTreeMap<_, _> = std::env::vars_os()
        .filter(|(key, _)| {
            let key = key.to_string_lossy();
            key.starts_with("SDL_")
                || matches!(
                    key.as_ref(),
                    "LD_LIBRARY_PATH"
                        | "LD_PRELOAD"
                        | "LD_AUDIT"
                        | "MONO_PATH"
                        | "MONO_CFG_DIR"
                        | "BIZHAWK_HOME"
                        | "BIZHAWK_INT_SYSLIB_PATH"
                        | "BIZHAWK_DATA_HOME"
                        | "DISPLAY"
                        | "WAYLAND_DISPLAY"
                        | "XDG_RUNTIME_DIR"
                )
        })
        .collect();
    ensure!(selected.len() <= 1024, "Oversized SDL2 input environment");
    let mut digest = Sha256::new();
    digest.update(b"lunchbox-sdl2-input-environment-v1");
    let mut total = 0usize;
    for (key, value) in selected {
        for part in [key, value] {
            let bytes = part.as_os_str().as_encoded_bytes();
            total = total
                .checked_add(bytes.len())
                .context("SDL2 input environment size overflow")?;
            ensure!(total <= 1024 * 1024, "Oversized SDL2 input environment");
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}
impl Drop for JoystickLock {
    fn drop(&mut self) {
        unsafe { (self.0)() }
    }
}

/// Loads only an explicitly trusted runtime library. No device is opened and no
/// input or rumble is emitted. The caller must use the emulator's environment;
/// host inventory cannot establish controller numbering inside another runtime.
pub fn inspect(library_path: &Path) -> Result<Snapshot> {
    inspect_with_controls(library_path, &[])
}

struct OpenJoystick {
    pointer: *mut c_void,
    close: unsafe extern "C" fn(*mut c_void),
}
impl Drop for OpenJoystick {
    fn drop(&mut self) {
        unsafe { (self.close)(self.pointer) }
    }
}

/// Opening is explicit by unique exact runtime path. SDL drivers may initialize
/// hardware while opening; this function sends no application input or rumble.
pub fn inspect_with_controls(library_path: &Path, control_paths: &[String]) -> Result<Snapshot> {
    inspect_with_mapping_database(library_path, control_paths, None)
}

fn read_mapping_database(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    ensure!(
        path.is_absolute(),
        "SDL2 mapping database must be an absolute target-runtime path"
    );
    let file = std::fs::File::open(path).context("Opening target SDL2 mapping database")?;
    ensure!(
        file.metadata()?.is_file(),
        "SDL2 mapping database is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4 * 1024 * 1024,
        "SDL2 mapping database exceeds size limit"
    );
    Ok(bytes)
}

/// Match SDL frontends such as PPSSPP which add their VFS mapping database
/// after subsystem initialization, before enumerating/opening controllers.
/// None preserves the existing native BizHawk inventory contract unchanged.
pub fn inspect_with_mapping_database(
    library_path: &Path,
    control_paths: &[String],
    mapping_database: Option<&Path>,
) -> Result<Snapshot> {
    let database_bytes = mapping_database.map(read_mapping_database).transpose()?;
    let mut environment_sha256 = input_environment_sha256()?;
    if let Some(bytes) = &database_bytes {
        let mut fingerprint = Sha256::new();
        fingerprint.update(b"SDL2-post-init-mapping-database-v1\0");
        fingerprint.update(environment_sha256.as_bytes());
        fingerprint.update(bytes);
        environment_sha256 = format!("{:x}", fingerprint.finalize());
    }
    let requested: std::collections::BTreeSet<_> = control_paths.iter().collect();
    ensure!(
        requested.len() == control_paths.len() && requested.iter().all(|path| !path.is_empty()),
        "SDL2 control paths must be unique and nonempty"
    );
    ensure!(
        library_path.is_absolute(),
        "SDL2 runtime library must be absolute"
    );
    let library_path =
        std::fs::canonicalize(library_path).context("Resolving SDL2 runtime library")?;
    let before = std::fs::read(&library_path).context("Reading SDL2 runtime library")?;
    let library_sha256 = format!("{:x}", Sha256::digest(&before));
    // Public SDL2 C ABI. Resolve SDL2-only symbols before SDL_GetVersion, whose
    // signature differs in SDL3. Keep the library alive past all RAII guards.
    unsafe {
        let library = Library::new(&library_path).context("Loading trusted SDL2 runtime")?;
        let is_controller =
            *library.get::<unsafe extern "C" fn(c_int) -> c_int>(b"SDL_IsGameController\0")?;
        let get_path = *library
            .get::<unsafe extern "C" fn(c_int) -> *const c_char>(b"SDL_JoystickPathForIndex\0")?;
        let get_guid = *library
            .get::<unsafe extern "C" fn(c_int) -> super::Guid>(b"SDL_JoystickGetDeviceGUID\0")?;
        let get_mapping = *library.get::<unsafe extern "C" fn(c_int) -> *mut c_char>(
            b"SDL_GameControllerMappingForDeviceIndex\0",
        )?;
        let free = *library.get::<super::Free>(b"SDL_free\0")?;
        let get_version =
            *library.get::<unsafe extern "C" fn(*mut Version)>(b"SDL_GetVersion\0")?;
        let mut version = Version::default();
        get_version(&mut version);
        ensure!(
            version.major == 2 && version.minor >= 24,
            "SDL 2.24 or later in major 2 is required"
        );
        let ready = *library.get::<unsafe extern "C" fn()>(b"SDL_SetMainReady\0")?;
        let init = *library.get::<unsafe extern "C" fn(u32) -> c_int>(b"SDL_InitSubSystem\0")?;
        let quit = *library.get::<unsafe extern "C" fn(u32)>(b"SDL_QuitSubSystem\0")?;
        let get_error =
            *library.get::<unsafe extern "C" fn() -> *const c_char>(b"SDL_GetError\0")?;
        let count = *library.get::<unsafe extern "C" fn() -> c_int>(b"SDL_NumJoysticks\0")?;
        let name = *library
            .get::<unsafe extern "C" fn(c_int) -> *const c_char>(b"SDL_JoystickNameForIndex\0")?;
        let instance = *library
            .get::<unsafe extern "C" fn(c_int) -> i32>(b"SDL_JoystickGetDeviceInstanceID\0")?;
        let update = *library.get::<unsafe extern "C" fn()>(b"SDL_JoystickUpdate\0")?;
        let lock = *library.get::<unsafe extern "C" fn()>(b"SDL_LockJoysticks\0")?;
        let unlock = *library.get::<unsafe extern "C" fn()>(b"SDL_UnlockJoysticks\0")?;
        #[cfg(target_os = "linux")]
        let hint_boolean = *library
            .get::<unsafe extern "C" fn(*const c_char, c_int) -> c_int>(b"SDL_GetHintBoolean\0")?;
        let open =
            *library.get::<unsafe extern "C" fn(c_int) -> *mut c_void>(b"SDL_JoystickOpen\0")?;
        let close = *library.get::<unsafe extern "C" fn(*mut c_void)>(b"SDL_JoystickClose\0")?;
        let opened_id = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_JoystickInstanceID\0")?;
        let buttons = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"SDL_JoystickNumButtons\0")?;
        let axes =
            *library.get::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"SDL_JoystickNumAxes\0")?;
        let hats =
            *library.get::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"SDL_JoystickNumHats\0")?;
        let attached = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"SDL_JoystickGetAttached\0")?;
        let axis_value = *library
            .get::<unsafe extern "C" fn(*mut c_void, c_int) -> i16>(b"SDL_JoystickGetAxis\0")?;
        let button_value = *library
            .get::<unsafe extern "C" fn(*mut c_void, c_int) -> u8>(b"SDL_JoystickGetButton\0")?;
        let hat_value = *library
            .get::<unsafe extern "C" fn(*mut c_void, c_int) -> u8>(b"SDL_JoystickGetHat\0")?;
        ready();
        ensure!(
            init(super::SUBSYSTEMS) == 0,
            "SDL2 initialization failed: {}",
            super::string(get_error())?.unwrap_or_default()
        );
        let _subsystems = super::Subsystems(quit);
        if let Some(bytes) = &database_bytes {
            let from_memory = *library
                .get::<unsafe extern "C" fn(*const c_void, c_int) -> *mut c_void>(
                    b"SDL_RWFromConstMem\0",
                )?;
            let add_mappings = *library.get::<unsafe extern "C" fn(*mut c_void, c_int) -> c_int>(
                b"SDL_GameControllerAddMappingsFromRW\0",
            )?;
            let stream = from_memory(bytes.as_ptr().cast(), bytes.len() as c_int);
            ensure!(
                !stream.is_null(),
                "SDL2 could not open mapping database memory"
            );
            // SDL consumes/frees the RWops; the backing bytes remain alive for
            // the entire call. No allocator is mixed across runtime libraries.
            ensure!(
                add_mappings(stream, 1) >= 0,
                "SDL2 rejected the target mapping database: {}",
                super::string(get_error())?.unwrap_or_default()
            );
        }
        update();
        lock();
        let _lock = JoystickLock(unlock);
        let total = count();
        ensure!((0..=1024).contains(&total), "Invalid SDL2 joystick count");
        let mut devices = Vec::with_capacity(total as usize);
        for index in 0..total {
            let instance_id = instance(index);
            ensure!(instance_id >= 0, "SDL2 device disappeared during inventory");
            let recognized = is_controller(index) == 1;
            let guid = get_guid(index)
                .data
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let mapping = if recognized {
                let pointer = get_mapping(index);
                let allocation = super::Allocation {
                    pointer: pointer.cast(),
                    free,
                };
                let copied = super::string(pointer)?;
                drop(allocation);
                copied
            } else {
                None
            };
            devices.push(Device {
                device_index: index as u32,
                instance_id,
                path: super::string(get_path(index))?,
                name: super::string(name(index))?,
                is_game_controller: recognized,
                guid,
                mapping,
                controls: None,
                linux_classic: None,
                linux_evdev: None,
                sampled_state: None,
            });
        }
        // Validate all requested identities before opening the first device.
        for path in &requested {
            ensure!(
                devices
                    .iter()
                    .filter(|device| device.path.as_ref() == Some(*path))
                    .count()
                    == 1,
                "SDL2 control path is missing or ambiguous: {path}"
            );
        }
        for device in &mut devices {
            if !device
                .path
                .as_ref()
                .is_some_and(|path| requested.contains(path))
            {
                continue;
            }
            #[cfg(target_os = "linux")]
            let physical_before = {
                let path = Path::new(device.path.as_ref().expect("requested path was matched"));
                let classic_path = path.parent() == Some(Path::new("/dev/input"))
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.strip_prefix("js"))
                        .is_some_and(|index| {
                            !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
                        });
                if classic_path {
                    Some(crate::linux_classic::read(path)?)
                } else {
                    None
                }
            };
            #[cfg(target_os = "linux")]
            let evdev_before = {
                let path = Path::new(device.path.as_ref().expect("requested path was matched"));
                let event_path = path.parent() == Some(Path::new("/dev/input"))
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.strip_prefix("event"))
                        .is_some_and(|index| {
                            !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
                        });
                if event_path {
                    Some(crate::sdl2_evdev::read(
                        path,
                        hint_boolean(c"SDL_LINUX_JOYSTICK_DEADZONES".as_ptr(), 0) != 0,
                        hint_boolean(c"SDL_LINUX_HAT_DEADZONES".as_ptr(), 1) != 0,
                        hint_boolean(c"SDL_LINUX_DIGITAL_HATS".as_ptr(), 0) != 0,
                    )?)
                } else {
                    None
                }
            };
            let pointer = open(device.device_index as c_int);
            ensure!(
                !pointer.is_null(),
                "Opening SDL2 controller for control counts failed"
            );
            let joystick = OpenJoystick { pointer, close };
            ensure!(
                opened_id(pointer) == device.instance_id,
                "SDL2 device changed while opening it"
            );
            let counts = [buttons(pointer), axes(pointer), hats(pointer)];
            ensure!(
                counts.iter().all(|count| (0..=1024).contains(count)),
                "Invalid SDL2 control counts"
            );
            device.controls = Some(ControlCounts {
                buttons: counts[0] as u32,
                axes: counts[1] as u32,
                hats: counts[2] as u32,
            });
            #[cfg(target_os = "linux")]
            if let Some(physical) = physical_before {
                ensure!(
                    physical.buttons.len() == counts[0] as usize
                        && physical.axes.len() == counts[1] as usize
                        && physical.hats.len() == counts[2] as usize,
                    "Linux classic numbering disagrees with SDL2 control counts"
                );
                let path = Path::new(device.path.as_ref().expect("requested path was matched"));
                ensure!(
                    crate::linux_classic::read(path)? == physical,
                    "Linux joystick numbering or correction changed while opening SDL2 device"
                );
                device.linux_classic = Some(physical);
            }
            #[cfg(target_os = "linux")]
            if let Some(physical) = evdev_before {
                ensure!(
                    physical.buttons.len() == counts[0] as usize
                        && physical.axes.len() == counts[1] as usize
                        && physical.hats.len() == counts[2] as usize,
                    "Linux evdev numbering disagrees with SDL2 control counts"
                );
                let path = Path::new(device.path.as_ref().expect("requested path was matched"));
                ensure!(
                    crate::sdl2_evdev::read(
                        path,
                        hint_boolean(c"SDL_LINUX_JOYSTICK_DEADZONES".as_ptr(), 0) != 0,
                        hint_boolean(c"SDL_LINUX_HAT_DEADZONES".as_ptr(), 1) != 0,
                        hint_boolean(c"SDL_LINUX_DIGITAL_HATS".as_ptr(), 0) != 0
                    )? == physical,
                    "Linux evdev capabilities or SDL2 hint settings changed during device opening"
                );
                device.linux_evdev = Some(physical);
            }
            // Read through the same SDL runtime that supplied the numbering.
            // This is a sample only: the caller must arrange and confirm the
            // released/pressed gesture before treating it as calibration.
            update();
            ensure!(
                attached(pointer) == 1 && opened_id(pointer) == device.instance_id,
                "SDL2 controller disconnected before sampling inputs"
            );
            let mut state = crate::sdl2_mapping::InputState::default();
            for index in 0..counts[0] {
                let value = button_value(pointer, index);
                ensure!(value <= 1, "Invalid SDL2 sampled button state");
                state.buttons.insert(index as u32, value != 0);
            }
            for index in 0..counts[1] {
                state.axes.insert(index as u32, axis_value(pointer, index));
            }
            for index in 0..counts[2] {
                state.hats.insert(index as u32, hat_value(pointer, index));
            }
            state.validate(device.controls.as_ref().expect("counts recorded above"))?;
            ensure!(
                attached(pointer) == 1 && opened_id(pointer) == device.instance_id,
                "SDL2 controller disconnected while sampling inputs"
            );
            device.sampled_state = Some(state);
            drop(joystick);
        }
        ensure!(
            std::fs::read(&library_path)? == before,
            "SDL2 library changed during inventory"
        );
        if let (Some(path), Some(before)) = (mapping_database, database_bytes.as_ref()) {
            ensure!(
                read_mapping_database(path)? == *before,
                "SDL2 mapping database changed during capture"
            );
        }
        Ok(Snapshot {
            // The effective environment fingerprint includes an explicit
            // post-init database, without changing old None-based snapshots.
            schema_version: 1,
            library: library_path,
            library_sha256,
            input_environment_sha256: environment_sha256,
            version: [version.major, version.minor, version.patch],
            devices,
        })
    }
}
