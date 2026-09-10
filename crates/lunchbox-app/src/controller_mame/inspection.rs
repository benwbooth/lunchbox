//! Rust-owned request/response boundary for MAME's native Lua field API.
//! The generated program is an autoboot API adapter, not a substitute emulator.
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActiveFieldSnapshot {
    pub schema_version: u32,
    pub machine: String,
    pub joystick_enabled: bool,
    pub joysticks: Vec<NativeJoystick>,
    /// Optional native-class evidence, not a host/frontend mouse route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mouse_state: Option<NativeMouseState>,
    pub fields: Vec<ActiveField>,
    /// Display-only names keyed by exact field identity, never name matching.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_labels: Vec<FieldLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub analog_states: Vec<AnalogFieldState>,
    /// Missing in older snapshots: never infer keyboard readiness from absence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_state: Option<KeyboardState>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldLabel {
    pub field: ActiveField,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AnalogFieldState {
    pub field: ActiveField,
    pub keydelta: i32,
    pub centerdelta: Option<i32>,
    pub sensitivity: i32,
    pub reverse: bool,
    pub reset: bool,
    pub wraps: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyboardState {
    pub natural_in_use: bool,
    pub devices: Vec<KeyboardDevice>,
    #[serde(default)]
    pub field_owners: Vec<KeyboardFieldOwner>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyboardFieldOwner {
    pub field: ActiveField,
    pub device_tag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeyboardDevice {
    pub tag: String,
    pub enabled: bool,
    pub is_keypad: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeJoystick {
    pub id: String,
    pub devindex: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeMouseState {
    pub enabled: bool,
    pub devices: Vec<NativeJoystick>,
    /// Core-declared names keyed by native ID, not physical device identity.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub device_names: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActiveField {
    pub class: FieldClass,
    pub tag: String,
    pub input_type: String,
    pub mask: u32,
    pub defvalue: u32,
    pub analog: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldClass {
    Controller,
    Keyboard,
    Config,
    Dipswitch,
    Misc,
    /// Native non-user-mappable signal, never inferred from a getter error alone.
    Internal,
    Unknown,
}

impl ActiveFieldSnapshot {
    pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 4;

    /// A retained legacy snapshot is editable data, never launch authority.
    pub(crate) fn needs_reinspection(&self) -> bool {
        self.schema_version != Self::CURRENT_SCHEMA_VERSION
    }

    /// Validate shape/identity, not provenance or complete controller support.
    /// The launch owner must bind the result to its exact runtime/content run.
    pub(crate) fn parse(bytes: &[u8], expected_machine: &str) -> Result<Self> {
        ensure!(
            bytes.len() <= 16 * 1024 * 1024,
            "MAME field snapshot exceeds 16 MiB"
        );
        let snapshot: Self = serde_json::from_slice(bytes)?;
        snapshot.validate(expected_machine)?;
        Ok(snapshot)
    }

    pub(crate) fn validate(&self, expected_machine: &str) -> Result<()> {
        ensure!(
            !self.needs_reinspection(),
            "MAME field snapshot schema {} needs reinspection (current schema {}); inspect the machine again, review and save its setup",
            self.schema_version,
            Self::CURRENT_SCHEMA_VERSION
        );
        self.validate_stored(expected_machine)
    }

    /// Preserve the known schema-3 shape during settings load/save without
    /// upgrading its version or permitting it through current inspection APIs.
    pub(crate) fn validate_stored(&self, expected_machine: &str) -> Result<()> {
        let snapshot = self;
        ensure!(
            matches!(snapshot.schema_version, 3 | Self::CURRENT_SCHEMA_VERSION),
            "Unsupported stored MAME snapshot schema {}",
            snapshot.schema_version
        );
        ensure!(
            !snapshot.machine.is_empty()
                && snapshot.machine.len() <= 64
                && snapshot.machine != "default"
                && snapshot
                    .machine
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "Invalid MAME snapshot machine identity"
        );
        ensure!(
            snapshot.machine == expected_machine,
            "MAME field snapshot does not match the requested machine"
        );
        ensure!(snapshot.fields.len() <= 32768, "Too many MAME fields");
        ensure!(
            snapshot.field_labels.len() <= 32768,
            "Too many native field labels"
        );
        let mut labelled = BTreeSet::new();
        for entry in &snapshot.field_labels {
            let field = &entry.field;
            ensure!(
                snapshot.fields.contains(field)
                    && entry.label.len() <= 2048
                    && !entry.label.contains('\0')
                    && labelled.insert((&field.tag, field.mask, &field.input_type, field.defvalue)),
                "Invalid or duplicate native field label"
            );
        }
        ensure!(
            snapshot.analog_states.len() <= 32768,
            "Too many native analog states"
        );
        let mut analog_identities = BTreeSet::new();
        for state in &snapshot.analog_states {
            let field = &state.field;
            ensure!(
                field.analog
                    && snapshot.fields.contains(field)
                    && analog_identities.insert((
                        &field.tag,
                        field.mask,
                        &field.input_type,
                        field.defvalue
                    )),
                "Invalid or duplicate native analog state"
            );
        }
        if let Some(keyboard) = &snapshot.keyboard_state {
            ensure!(keyboard.devices.len() <= 4096, "Too many native keyboards");
            let mut tags = BTreeSet::new();
            for device in &keyboard.devices {
                ensure!(
                    device.tag.starts_with(':')
                        && device.tag.len() <= 512
                        && !device.tag.chars().any(char::is_control)
                        && tags.insert(&device.tag),
                    "Invalid or duplicate native keyboard identity"
                );
            }
            ensure!(
                keyboard.field_owners.len() <= 32768,
                "Too many keyboard field owners"
            );
            let mut owners = BTreeSet::new();
            for owner in &keyboard.field_owners {
                let field = &owner.field;
                ensure!(
                    field.class == FieldClass::Keyboard
                        && !field.analog
                        && snapshot.fields.contains(field)
                        && tags.contains(&owner.device_tag)
                        && owners.insert((
                            &field.tag,
                            field.mask,
                            &field.input_type,
                            field.defvalue
                        )),
                    "Invalid or duplicate keyboard field ownership"
                );
            }
        }
        self.joystick_routes()?;
        if let Some(mouse) = &self.mouse_state {
            ensure!(mouse.devices.len() <= 255, "Too many native mouse devices");
            ensure!(
                mouse.device_names.len() <= 255
                    && mouse.device_names.iter().all(|(id, name)| mouse
                        .devices
                        .iter()
                        .any(|device| &device.id == id)
                        && !name.is_empty()
                        && name.len() <= 256
                        && !name.chars().any(char::is_control)),
                "Invalid native mouse name evidence"
            );
            let mut ids = BTreeSet::new();
            let mut indices = BTreeSet::new();
            for device in &mouse.devices {
                ensure!(
                    !device.id.is_empty()
                        && device.id.len() <= 128
                        && !device.id.chars().any(char::is_control)
                        && device.devindex < 255
                        && ids.insert(&device.id)
                        && indices.insert(device.devindex),
                    "Invalid or ambiguous native mouse identity"
                );
            }
        }
        let mut identities = BTreeSet::new();
        for field in &snapshot.fields {
            ensure!(
                snapshot.schema_version == Self::CURRENT_SCHEMA_VERSION
                    || field.class != FieldClass::Internal,
                "Legacy MAME snapshots cannot classify internal signals"
            );
            ensure!(
                field.tag.starts_with(':')
                    && field.tag.len() <= 512
                    && !field.tag.chars().any(char::is_control)
                    && !field.input_type.is_empty()
                    && field.input_type.len() <= 128
                    && valid_type_token(&field.input_type)
                    && field.mask != 0
                    && field.defvalue & !field.mask == 0,
                "Invalid MAME field identity"
            );
            ensure!(
                identities.insert((&field.tag, field.mask, &field.input_type, field.defvalue)),
                "Duplicate MAME field identity"
            );
        }
        Ok(())
    }

    pub(crate) fn joystick_routes(&self) -> Result<[usize; 8]> {
        ensure!(
            self.joysticks.len() == 8,
            "MAME snapshot requires all eight native RetroPads"
        );
        let mut routes = [0usize; 8];
        let mut occupied = BTreeSet::new();
        for device in &self.joysticks {
            let port = device
                .id
                .strip_prefix("RetroPad")
                .and_then(|suffix| suffix.parse::<usize>().ok())
                .filter(|port| *port < 8)
                .ok_or_else(|| anyhow::anyhow!("Unexpected native MAME joystick identity"))?;
            ensure!(
                device.id == format!("RetroPad{port}")
                    && routes[port] == 0
                    && device.devindex < 0xff
                    && occupied.insert(device.devindex),
                "Ambiguous native MAME joystick routing"
            );
            routes[port] = device.devindex + 1;
        }
        Ok(routes)
    }

    /// Pinned input_retro.cpp creates RetroMouse0..7 for frontend ports 0..7.
    /// Return one-based native sequence indices, NOT frontend device indices.
    pub(crate) fn require_game_mouse_mode(&self) -> Result<()> {
        self.mouse_routes()?;
        let mouse = self.mouse_state.as_ref().expect("mouse routes validated");
        ensure!(
            mouse.device_names.len() == 8
                && (0..8).all(|port| {
                    let id = format!("RetroMouse{port}");
                    mouse.device_names.get(&id) == Some(&format!("{id} [LunchboxGameMouseV1]"))
                }),
            "Native core does not declare the required game-only mouse mode; inspect the opt-in core before button launch"
        );
        Ok(())
    }

    pub(crate) fn mouse_routes(&self) -> Result<[usize; 8]> {
        self.validate(&self.machine)?;
        let mouse = self.mouse_state.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Native mouse evidence is absent; reinspect before relative routing")
        })?;
        ensure!(mouse.enabled, "Native MAME mouse input is disabled");
        ensure!(
            mouse.devices.len() == 8,
            "Unexpected native MAME mouse device count"
        );
        let mut routes = [0; 8];
        for device in &mouse.devices {
            let port = (0..8)
                .find(|port| device.id == format!("RetroMouse{port}"))
                .ok_or_else(|| anyhow::anyhow!("Unexpected native MAME mouse identity"))?;
            ensure!(routes[port] == 0, "Duplicate native MAME mouse port");
            routes[port] = device.devindex + 1;
        }
        ensure!(
            routes.iter().all(|index| *index > 0),
            "Incomplete native mouse routes"
        );
        Ok(routes)
    }
}

fn valid_type_token(token: &str) -> bool {
    if token
        .bytes()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return !token.is_empty();
    }
    token
        .strip_prefix("TYPE_OTHER(")
        .and_then(|value| value.strip_suffix(')'))
        .and_then(|value| value.split_once(','))
        .is_some_and(|(kind, player)| {
            !kind.is_empty()
                && !player.is_empty()
                && kind
                    .bytes()
                    .chain(player.bytes())
                    .all(|byte| byte.is_ascii_digit())
                && kind.parse::<u32>().is_ok()
                && player.parse::<u32>().is_ok()
        })
}

/// Build the native .cmd payload for the MAME-specific inspection process.
/// The caller must materialize this private tree and supply matching RetroArch
/// system/save directories before invoking the trusted runtime. No arbitrary
/// native arguments or user-selected script are accepted by this constructor.
pub(crate) fn inspection_command(
    root: &std::path::Path,
    machine: &str,
    has_controller_profile: bool,
) -> Result<String> {
    inspection_command_with_mouse(root, machine, has_controller_profile, false)
}

pub(super) fn inspection_command_with_mouse(
    root: &std::path::Path,
    machine: &str,
    has_controller_profile: bool,
    inspect_mouse: bool,
) -> Result<String> {
    native_command(root, machine, has_controller_profile, None, inspect_mouse)
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistentPaths {
    pub nvram: std::path::PathBuf,
    pub diff: std::path::PathBuf,
    pub states: std::path::PathBuf,
    pub snapshots: std::path::PathBuf,
    pub recordings: std::path::PathBuf,
}

impl PersistentPaths {
    pub(crate) fn directories(&self) -> [&std::path::Path; 5] {
        [
            &self.nvram,
            &self.diff,
            &self.states,
            &self.snapshots,
            &self.recordings,
        ]
    }
}

/// Game-session command uses the same path routing without the inspection
/// autoboot script (which exits the machine after collecting fields).
pub(crate) fn session_command(
    root: &std::path::Path,
    machine: &str,
    persistent: &PersistentPaths,
) -> Result<String> {
    session_command_with_mouse(root, machine, persistent, false)
}

/// Preserve an explicitly inspected native mouse class in the game session.
/// This does not establish frontend routes or authorize physical forwarding.
pub(crate) fn session_command_with_mouse(
    root: &std::path::Path,
    machine: &str,
    persistent: &PersistentPaths,
    enable_mouse: bool,
) -> Result<String> {
    for path in persistent.directories() {
        ensure!(
            path.is_absolute()
                && path.is_dir()
                && path.canonicalize()? == path
                && !path.starts_with(root),
            "MAME persistence requires resolved existing directories outside the private session"
        );
    }
    native_command(root, machine, true, Some(persistent), enable_mouse)
}

fn native_command(
    root: &std::path::Path,
    machine: &str,
    has_controller_profile: bool,
    persistent: Option<&PersistentPaths>,
    inspect_mouse: bool,
) -> Result<String> {
    ensure!(
        !machine.is_empty()
            && machine.len() <= 64
            && machine != "default"
            && machine
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        "Expected an exact MAME machine short name"
    );
    ensure!(
        root.is_absolute()
            && root.components().all(|part| matches!(
                part,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )),
        "MAME inspection requires a normalized absolute private root"
    );
    let quote = |path: &std::path::Path| -> Result<String> {
        let value = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("MAME command paths must be UTF-8"))?;
        ensure!(
            value.len() <= 511 && !value.chars().any(|c| c.is_control() || "\"\\;".contains(c)),
            "MAME command path exceeds limits or contains unsupported delimiters"
        );
        Ok(format!("\"{value}\""))
    };
    // The wrapper recognizes the leading core name and treats the final token
    // as the machine. Absolute option paths avoid its relative-path rewriting.
    let mut args = vec!["mame".to_owned()];
    for (option, directory) in [
        ("-cfg_directory", "cfg"),
        ("-ctrlrpath", "ctrlr"),
        ("-rompath", "roms"),
        ("-nvram_directory", "nvram"),
        ("-diff_directory", "diff"),
        ("-state_directory", "states"),
        ("-snapshot_directory", "snaps"),
        ("-input_directory", "input"),
    ] {
        args.push(option.to_owned());
        let private = root.join(directory);
        let path = match (persistent, directory) {
            (Some(paths), "nvram") => &paths.nvram,
            (Some(paths), "diff") => &paths.diff,
            (Some(paths), "states") => &paths.states,
            (Some(paths), "snaps") => &paths.snapshots,
            (Some(paths), "input") => &paths.recordings,
            _ => &private,
        };
        args.push(quote(path)?);
    }
    if persistent.is_none() {
        args.extend(["-autoboot_script".into(), quote(&root.join("inspect.lua"))?]);
    }
    args.extend(
        [
            "-autoboot_delay",
            "0",
            "-noreadconfig",
            "-nowriteconfig",
            "-noautosave",
            "-nocheat",
            "-joystick",
        ]
        .map(str::to_owned),
    );
    if has_controller_profile {
        args.extend(["-ctrlr".to_owned(), "lunchbox-original".to_owned()]);
    }
    if inspect_mouse {
        args.push("-mouse".to_owned());
    }
    args.push(machine.to_owned());
    // Leave capacity for the wrapper's own standard/path arguments (128 total).
    ensure!(args.len() <= 40, "MAME inspection argument budget exceeded");
    let command = args.join(" ");
    ensure!(
        command.len() <= 4095,
        "MAME inspection command exceeds its native first-line capacity"
    );
    Ok(format!("{command}\n"))
}

/// Emit a read-only active-field snapshot through the core's native Lua API.
/// No process is started here. The caller owns an isolated inspection run,
/// private output file, deadline and successful completion check. The native
/// adapter writes only that caller-owned output file. Only currently
/// enabled fields are visible; later DIP/slot/condition changes require refresh.
pub(crate) fn active_field_script(machine: &str, output: &std::path::Path) -> Result<String> {
    ensure!(
        !machine.is_empty()
            && machine.len() <= 64
            && machine
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
        "Expected an exact MAME machine short name"
    );
    let output = output
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("MAME inspection path must be UTF-8"))?;
    ensure!(
        std::path::Path::new(output).is_absolute()
            && output.len() <= 4096
            && !output.chars().any(char::is_control),
        "Invalid MAME inspection output path"
    );
    // Decimal byte escapes preserve UTF-8 and avoid Lua/JSON quoting differences.
    let literal = |value: &str| {
        let mut encoded = String::from("\"");
        for byte in value.bytes() {
            encoded.push_str(&format!("\\{byte:03}"));
        }
        encoded.push('"');
        encoded
    };
    let mut script = format!(
        "local expected = {}\nlocal output = {}\nlocal snapshot_schema = {}\n",
        literal(machine),
        literal(output),
        ActiveFieldSnapshot::CURRENT_SCHEMA_VERSION
    );
    script.push_str(r#"
local ok, failure = pcall(function()
    local machine = manager.machine
    assert(machine.system.name == expected, 'Wrong MAME machine')
    local function quoted(value)
        assert(type(value) == 'string', 'Expected native string')
        return '"' .. value:gsub('[%z\1-\31\\"]', function(c)
            return string.format('\\u%04x', string.byte(c))
        end) .. '"'
    end
    local tags = {}
    for tag, _ in pairs(machine.ioport.ports) do
        tags[#tags + 1] = tag
        assert(#tags <= 4096, 'Too many native input ports')
    end
    table.sort(tags)
    local fields = {}
    local field_labels = {}
    local analog_states = {}
    local keyboard_owners = {}
    local joystick_class = assert(machine.input.device_classes.joystick, 'No native joystick class')
    local joysticks = {}
    for _, device in ipairs(joystick_class.devices) do
        assert(#device.id <= 128, 'Native joystick identity too long')
        joysticks[#joysticks + 1] = '{"id":' .. quoted(device.id)
            .. ',"devindex":' .. tostring(device.devindex) .. '}'
        assert(#joysticks <= 8, 'Unexpected native joystick count')
    end
    local mouse_state = 'null'
    local mouse_class = machine.input.device_classes.mouse
    if mouse_class then
        local mice = {}
        local mouse_names = {}
        for _, device in ipairs(mouse_class.devices) do
            assert(#device.id <= 128, 'Native mouse identity too long')
            mice[#mice + 1] = '{"id":' .. quoted(device.id)
                .. ',"devindex":' .. tostring(device.devindex) .. '}'
            mouse_names[#mouse_names + 1] = quoted(device.id) .. ':' .. quoted(device.name)
            assert(#mice <= 255, 'Unexpected native mouse count')
        end
        mouse_state = '{"enabled":' .. tostring(mouse_class.enabled)
            .. ',"devices":[' .. table.concat(mice, ',') .. ']'
            .. ',"device_names":{' .. table.concat(mouse_names, ',') .. '}}'
    end
    for _, tag in ipairs(tags) do
        local port = machine.ioport.ports[tag]
        -- At 4fc9a931, luaengine_input.cpp 244-264 inserts EVERY non-internal
        -- field under field.name before adding aliases. Collisions overwrite
        -- values but cannot remove a key. Absence of that exact name therefore
        -- proves native exclusion; presence never proves a field's identity.
        -- Use this table only as an omission witness, never for enumeration.
        local named_fields = port.fields
        assert(type(named_fields) == 'table', 'Missing native mappable-field table')
        local seen = {}
        -- ioport_value is 32 bits. Unlike the name-keyed fields table,
        -- field(mask) finds an enabled field without display-name collisions.
        for bit = 0, 31 do
            local field = port:field(1 << bit)
            if field and field.enabled then
                local token = machine.ioport:input_type_to_token(field.type, field.player)
                local mask = field.mask
                local defvalue = field.defvalue & mask
                -- The pinned class getter throws for internal signals. Require
                -- independent native-table evidence before classifying them.
                -- Name collisions or other getter failures remain unknown.
                local class_ok, class = pcall(function() return field.type_class end)
                if not class_ok then
                    class = 'unknown'
                    local name_ok, name = pcall(function() return field.name end)
                    if name_ok and type(name) == 'string' and rawget(named_fields, name) == nil then
                        class = 'internal'
                    end
                end
                local identity = token .. ':' .. tostring(mask) .. ':' .. tostring(defvalue)
                if not seen[identity] then
                    seen[identity] = true
                    assert(#tag <= 512 and #token <= 128, 'Native field identity too long')
                    fields[#fields + 1] = '{"tag":' .. quoted(tag)
                        .. ',"class":' .. quoted(class)
                        .. ',"input_type":' .. quoted(token)
                        .. ',"mask":' .. tostring(mask)
                        .. ',"defvalue":' .. tostring(defvalue)
                        .. ',"analog":' .. tostring(field.is_analog) .. '}'
                    if class ~= 'internal' then
                        local name_ok, name = pcall(function() return field.name end)
                        if name_ok and type(name) == 'string' then
                            assert(#name <= 2048, 'Native field label too long')
                            field_labels[#field_labels + 1] = '{"field":' .. fields[#fields]
                                .. ',"label":' .. quoted(name) .. '}'
                        end
                    end
                    if class == 'keyboard' then
                        -- natkeyboard.cpp groups keyfields by port.device, not
                        -- a guessed tag prefix or the field's declaration device.
                        keyboard_owners[#keyboard_owners + 1] = '{"field":' .. fields[#fields]
                            .. ',"device_tag":' .. quoted(port.device.tag) .. '}'
                    end
                    if field.is_analog then
                        local center = field.centerdelta
                        analog_states[#analog_states + 1] = '{"field":' .. fields[#fields]
                            .. ',"keydelta":' .. tostring(field.keydelta)
                            .. ',"centerdelta":' .. (center == nil and 'null' or tostring(center))
                            .. ',"sensitivity":' .. tostring(field.sensitivity)
                            .. ',"reverse":' .. tostring(field.analog_reverse)
                            .. ',"reset":' .. tostring(field.analog_reset)
                            .. ',"wraps":' .. tostring(field.analog_wraps) .. '}'
                    end
                    assert(#fields <= 32768, 'Too many native fields')
                end
            end
        end
    end
    -- Observe only. Natural keyboard mode can lock out ordinary field
    -- sequences; never change it or enable devices as part of inspection.
    local natkbd = machine.natkeyboard
    local keyboard_devices = {}
    for _, device in pairs(natkbd.keyboards) do
        assert(#device.tag <= 512, 'Native keyboard tag too long')
        keyboard_devices[#keyboard_devices + 1] = '{"tag":' .. quoted(device.tag)
            .. ',"enabled":' .. tostring(device.enabled)
            .. ',"is_keypad":' .. tostring(device.is_keypad) .. '}'
        assert(#keyboard_devices <= 4096, 'Too many native keyboards')
    end
    table.sort(keyboard_devices)
    local keyboard_state = '{"natural_in_use":' .. tostring(natkbd.in_use)
        .. ',"devices":[' .. table.concat(keyboard_devices, ',') .. ']'
        .. ',"field_owners":[' .. table.concat(keyboard_owners, ',') .. ']}'
    local document = '{"schema_version":' .. tostring(snapshot_schema) .. ',"machine":' .. quoted(expected)
        .. ',"joystick_enabled":' .. tostring(joystick_class.enabled)
        .. ',"joysticks":[' .. table.concat(joysticks, ',') .. ']'
        .. ',"mouse_state":' .. mouse_state
        .. ',"keyboard_state":' .. keyboard_state
        .. ',"field_labels":[' .. table.concat(field_labels, ',') .. ']'
        .. ',"analog_states":[' .. table.concat(analog_states, ',') .. ']'
        .. ',"fields":[' .. table.concat(fields, ',') .. ']}'
    assert(#document <= 16 * 1024 * 1024, 'Native snapshot too large')
    local file = assert(io.open(output, 'wb'))
    local written, write_error = file:write(document)
    local closed, close_error = file:close()
    assert(written, write_error)
    assert(closed, close_error)
end)
if not ok then io.stderr:write('Lunchbox MAME field inspection failed: ' .. tostring(failure) .. '\n') end
manager.machine:exit()
"#);
    Ok(script)
}
