//! Resolve CrocoDS's own input configuration without rewriting user settings.
//! Source: libretro/libretro-crocods a9c63b29443715ae2add392010fca4eae7f93e67,
//! crocods-core/platform.c loadIni/nds_ReadKey and platform.h CPC_SCANCODE.
use anyhow::{Context, Result, bail, ensure};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const BUTTONS: [&str; 13] = [
    "up", "down", "left", "right", "start", "a", "b", "x", "y", "l", "r", "l2", "r2",
];
// loadIni's default R scan code is CPC_5, but opening the keyboard suppresses
// scan-code delivery while the keyboard is visible. L is explicitly cleared.
const JOYSTICK: [i32; 13] = [72, 73, 74, 75, 18, 76, 77, 65, 57, 80, 49, 48, 41];

pub(crate) struct InputSnapshot(Vec<(PathBuf, Option<String>)>);

impl InputSnapshot {
    pub(crate) fn verify(&self) -> Result<()> {
        for (path, expected) in &self.0 {
            ensure!(
                read_ini(path)? == *expected,
                "CrocoDS configuration changed during launch preparation: {}",
                path.display()
            );
        }
        Ok(())
    }
}

fn read_ini(path: &Path) -> Result<Option<String>> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("Inspecting {}", path.display())),
        Ok(metadata) => ensure!(
            metadata.is_file() && metadata.len() <= 64 * 1024,
            "CrocoDS input configuration is not a bounded regular file: {}",
            path.display()
        ),
    }
    Ok(Some(
        std::fs::read_to_string(path).with_context(|| format!("Reading {}", path.display()))?,
    ))
}

// Accept the unambiguous INI subset shared with iniparser. Unsupported syntax is
// surfaced, never treated as an absent input override. Names are case-insensitive.
fn parse_ini(text: &str) -> Result<BTreeMap<String, String>> {
    let mut section = String::new();
    let mut values = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        ensure!(
            line.len() < 1024 && !line.ends_with('\\'),
            "CrocoDS INI continuation or oversized line needs explicit resolution"
        );
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            ensure!(!name.contains(['[', ']']), "Ambiguous CrocoDS INI section");
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .context("Unresolved CrocoDS INI syntax")?;
        let key = key.trim().to_ascii_lowercase();
        ensure!(
            !key.is_empty() && !key.contains([':', '[', ']']),
            "Ambiguous CrocoDS INI key"
        );
        let value = value.trim();
        let value = if value.starts_with(['\'', '"']) {
            let quote = value.chars().next().unwrap();
            let tail = &value[1..];
            let end = tail.find(quote).context("Unclosed CrocoDS INI quote")?;
            let rest = tail[end + 1..].trim();
            ensure!(
                rest.is_empty() || rest.starts_with(['#', ';']),
                "Trailing CrocoDS INI value"
            );
            tail[..end].to_owned()
        } else {
            value.split(['#', ';']).next().unwrap().trim().to_owned()
        };
        let name = format!("{section}:{key}");
        ensure!(
            values.insert(name, value).is_none(),
            "Duplicate CrocoDS INI key needs explicit resolution"
        );
    }
    Ok(values)
}

fn integer(value: &str) -> Result<i32> {
    let (negative, digits) = if let Some(tail) = value.strip_prefix('-') {
        (true, tail)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let (base, digits) = if let Some(tail) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        (16, tail)
    } else if digits.len() > 1 && digits.starts_with('0') {
        (8, &digits[1..])
    } else {
        (10, digits)
    };
    let number = i32::from_str_radix(digits, base).context("Unresolved CrocoDS scan code")?;
    Ok(if negative { -number } else { number })
}

pub(crate) fn prepare(home: &Path, content: &Path) -> Result<InputSnapshot> {
    ensure!(
        cfg!(target_os = "linux"),
        "CrocoDS input defaults need a host-specific resolver"
    );
    ensure!(
        home.is_absolute(),
        "CrocoDS needs the effective absolute HOME"
    );
    let name = content
        .file_name()
        .and_then(|name| name.to_str())
        .context("CrocoDS needs a UTF-8 content basename")?;
    // The core uses a fixed-size basename and appends .ini (including .dsk).
    ensure!(
        name.len() < 255,
        "CrocoDS content basename exceeds the reviewed lookup bound"
    );
    let root = home.join(".crocods");
    let paths = [
        root.join("cfg/.ini"),
        root.join("crocods.ini"),
        root.join("cfg").join(format!("{name}.ini")),
    ];
    let mut snapshot = InputSnapshot(Vec::new());
    for path in paths {
        snapshot.0.push((path.clone(), read_ini(&path)?));
    }
    ensure!(
        snapshot.0[0].1.is_none(),
        "CrocoDS startup cfg/.ini overrides global input initialization; resolve that file before calibrated launch"
    );
    ensure!(
        snapshot.0[1].1.is_some(),
        "Initialize CrocoDS once with native setup before calibrated launch; its default menu bindings depend on successfully creating and loading crocods.ini"
    );
    let mut keys = JOYSTICK;
    let mut menus: [String; 13] = std::array::from_fn(|index| match index {
        9 => "screennext".to_owned(),
        10 => "virtualkeyboard".to_owned(),
        _ => "empty".to_owned(),
    });
    let mut emulation = 3;
    for (layer, (_, text)) in snapshot.0[1..].iter().enumerate() {
        let Some(text) = text else {
            continue;
        };
        let values = parse_ini(text)?;
        if layer == 0 {
            // Global joy:l defaults to CPC_4 first. Only an absent menu:l
            // clears it later; an explicit screen action does not clear it.
            keys[9] = 56;
        }
        if let Some(value) = values.get("key:emulation") {
            let value = integer(value)?;
            if value == 2 || value == 3 {
                emulation = value;
            }
        }
        for (index, button) in BUTTONS.iter().enumerate() {
            if let Some(value) = values.get(&format!("joy:{button}")) {
                let value = integer(value)?;
                if value != -1 {
                    keys[index] = value;
                }
            }
            if let Some(value) = values.get(&format!("menu:{button}")) {
                menus[index] = value.to_ascii_lowercase();
            } else if layer == 0 && index == 9 {
                // Global loadIni clears L when menu:l is omitted, even if
                // joy:l was explicit. R has no corresponding clear.
                keys[index] = 80;
            }
        }
    }
    ensure!(
        emulation == 3,
        "CrocoDS is configured for physical-keyboard mode; choose joystick mode before calibrated launch"
    );
    for (index, button) in BUTTONS.iter().enumerate() {
        let menu = match index {
            9 => "screennext",
            10 => "virtualkeyboard",
            _ => "empty",
        };
        if keys[index] != JOYSTICK[index] || menus[index] != menu {
            bail!(
                "CrocoDS saved mapping for {button} differs from the joystick/keyboard contract; select matching controls or use native setup (settings were not changed)"
            );
        }
    }
    Ok(snapshot)
}
