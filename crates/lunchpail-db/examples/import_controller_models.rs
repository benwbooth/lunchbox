//! Reproducible, offline import from pinned upstream checkouts. No network at runtime.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const SDL: &str = "28a856f2b92da8891b161acd0abd64fbf4445d97";
const RA: &str = "033151045d378b64e712a92592467800d7924227";

fn check_revision(path: &Path, expected: &str) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "HEAD"])
        .output()?;
    ensure!(
        output.status.success() && String::from_utf8(output.stdout)?.trim() == expected,
        "Unexpected upstream revision: {}",
        path.display()
    );
    Ok(())
}
fn guid_word(guid: &str, byte: usize) -> Option<u16> {
    let lo = u8::from_str_radix(guid.get(byte * 2..byte * 2 + 2)?, 16).ok()?;
    let hi = u8::from_str_radix(guid.get(byte * 2 + 2..byte * 2 + 4)?, 16).ok()?;
    let value = u16::from_le_bytes([lo, hi]);
    (value != 0).then_some(value)
}
fn numeric(value: Option<&String>) -> Option<u16> {
    let text = value?;
    let value = if let Some(hex) = text.strip_prefix("0x") {
        u16::from_str_radix(hex, 16).ok()?
    } else {
        text.parse().ok()?
    };
    (value != 0).then_some(value)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: import_controller_models SDL_CHECKOUT RETROARCH_CHECKOUT OUTPUT_DIRECTORY"
    );
    let sdl = Path::new(&args[0]);
    let ra = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    check_revision(sdl, SDL)?;
    check_revision(ra, RA)?;
    let mut models = BTreeMap::<String, Value>::new();
    for line in fs::read_to_string(sdl.join("gamecontrollerdb.txt"))?
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    {
        let mut fields = line.split(',');
        let guid = fields.next().unwrap_or_default();
        let name = fields.next().unwrap_or_default();
        ensure!(
            guid == "xinput" || (guid.len() == 32 && guid.bytes().all(|c| c.is_ascii_hexdigit())),
            "Invalid SDL GUID: {guid}"
        );
        let bindings: BTreeMap<_, _> = fields.filter_map(|field| field.split_once(':')).collect();
        let os = match bindings.get("platform").copied().unwrap_or("") {
            "Windows" => "windows",
            "Mac OS X" => "macos",
            "Linux" => "linux",
            _ => continue,
        };
        let id = format!("sdl:{os}:{guid}:{name}");
        let row = json!({"id":id,"name":name,"device_name":name,"source":"SDL","os":os,"driver":"sdl","guid":guid,"vendor":guid_word(guid,4),"product":guid_word(guid,8),"bus":guid_word(guid,0),"version":guid_word(guid,12),"bindings":bindings,"raw":line});
        if let Some(old) = models.insert(id, row.clone()) {
            ensure!(old == row, "Conflicting duplicate SDL identity");
        }
    }
    let mut paths = walkdir::WalkDir::new(ra)
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    paths.sort_by_key(|entry| entry.path().to_owned());
    for entry in paths
        .into_iter()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "cfg"))
    {
        let path = entry.path().strip_prefix(ra)?;
        let driver = path
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_string_lossy();
        let os = match driver.as_ref() {
            "udev" | "linuxraw" => "linux",
            "dinput" | "xinput" => "windows",
            "hid" => "macos",
            "sdl2" => "any",
            _ => continue,
        };
        let raw = fs::read_to_string(entry.path())?;
        let bindings: BTreeMap<String, String> = raw
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with('#') {
                    return None;
                }
                let (key, value) = line.split_once('=')?;
                Some((
                    key.trim().to_owned(),
                    value.trim().trim_matches('"').to_owned(),
                ))
            })
            .collect();
        let Some(device_name) = bindings.get("input_device") else {
            continue;
        };
        let name = bindings
            .get("input_device_display_name")
            .unwrap_or(device_name);
        let id = format!("retroarch:{}", path.to_string_lossy());
        models.insert(id.clone(), json!({"id":id,"name":name,"device_name":device_name,"source":"RetroArch","os":os,"driver":driver,"guid":null,"vendor":numeric(bindings.get("input_vendor_id")),"product":numeric(bindings.get("input_product_id")),"bus":null,"version":null,"bindings":bindings,"raw":raw}));
    }
    fs::create_dir_all(output)?;
    fs::copy(sdl.join("LICENSE"), output.join("SDL-LICENSE.txt"))?;
    fs::copy(ra.join("COPYING"), output.join("RetroArch-LICENSE.txt"))?;
    let count = models.len();
    let data = json!({"schema_version":1,"sources":[{"project":"SDL_GameControllerDB","revision":SDL,"license":"Zlib","url":"https://github.com/mdqinc/SDL_GameControllerDB"},{"project":"retroarch-joypad-autoconfig","revision":RA,"license":"MIT (profiles); Zlib (SDL-derived data)","url":"https://github.com/libretro/retroarch-joypad-autoconfig"}],"models":models.into_values().collect::<Vec<_>>()});
    fs::write(
        output.join("models.json"),
        serde_json::to_string_pretty(&data)? + "\n",
    )?;
    println!("Imported {count} platform/driver-specific model profiles");
    Ok(())
}
