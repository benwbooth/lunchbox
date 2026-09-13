//! CUzeBox SDL2 game-controller database writer.
//!
//! Pinned source: Jubatian/cuzebox `adcea412e18cca8a4bb9e94af94097a420c2f5cc`.
//! `ginput.c` loads `gamecontrollerdb.txt` from the executable's base path
//! with `SDL_GameControllerAddMappingsFromFile`, then opens the first two
//! SDL game controllers.  The source has no emulator-owned controller
//! profile or physical-slot setting: device order and identity must be
//! measured by the launch layer immediately before starting CUzeBox.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "adcea412e18cca8a4bb9e94af94097a420c2f5cc";
pub(crate) const DATABASE_FILENAME: &str = "gamecontrollerdb.txt";

/// One SDL2 GameController database entry. `fields` are the source-supported
/// SDL logical names (`a`, `b`, `x`, `y`, `dpup`, `dpdown`, `dpleft`,
/// `dpright`, etc.) and raw SDL joystick specs (`bN`, `aN`, `hN.M`, with an
/// optional `~` reversal suffix).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Mapping {
    pub guid: String,
    pub name: String,
    pub platform: Option<String>,
    pub fields: Vec<(String, String)>,
}

fn valid_name(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 128,
        "CUzeBox controller name is empty or too long"
    );
    ensure!(
        value
            .bytes()
            .all(|byte| (byte.is_ascii_graphic() || byte == b' ') && byte != b','),
        "CUzeBox controller name contains a descriptor delimiter or control character"
    );
    Ok(())
}

const REQUIRED_FIELDS: [&str; 12] = [
    "a",
    "b",
    "x",
    "y",
    "back",
    "start",
    "leftshoulder",
    "rightshoulder",
    "dpup",
    "dpdown",
    "dpleft",
    "dpright",
];

fn valid_field(value: &str) -> bool {
    REQUIRED_FIELDS.contains(&value) || matches!(value, "guide" | "leftx" | "lefty")
}

fn decimal(value: &str, limit: u16) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u16>().is_ok_and(|number| number < limit)
}

fn valid_raw(value: &str) -> bool {
    if let Some(button) = value.strip_prefix('b') {
        return decimal(button, 256);
    }
    if let Some(hat) = value.strip_prefix('h') {
        return hat.split_once('.').is_some_and(|(index, mask)| {
            decimal(index, 64) && matches!(mask, "1" | "2" | "4" | "8")
        });
    }
    let axis = value.strip_suffix('~').unwrap_or(value);
    let axis = axis
        .strip_prefix('+')
        .or_else(|| axis.strip_prefix('-'))
        .unwrap_or(axis);
    axis.strip_prefix('a')
        .is_some_and(|index| decimal(index, 256))
}

fn descriptor(mapping: &Mapping) -> Result<String> {
    ensure!(
        mapping.guid.len() == 32 && mapping.guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "CUzeBox SDL GUID must contain 32 hexadecimal digits"
    );
    valid_name(&mapping.name)?;
    if let Some(platform) = &mapping.platform {
        ensure!(
            matches!(
                platform.as_str(),
                "Linux" | "Windows" | "Mac OS X" | "Android" | "iOS"
            ),
            "CUzeBox SDL platform is unsupported"
        );
    }
    ensure!(!mapping.fields.is_empty(), "CUzeBox mapping has no fields");
    let mut seen = BTreeSet::new();
    let mut out = format!("{},{}", mapping.guid.to_ascii_lowercase(), mapping.name);
    if let Some(platform) = &mapping.platform {
        out.push_str(&format!(",platform:{platform}"));
    }
    for (logical, raw) in &mapping.fields {
        ensure!(valid_field(logical), "CUzeBox logical input is unsupported");
        ensure!(valid_raw(raw), "CUzeBox raw input specification is invalid");
        ensure!(
            seen.insert(logical.clone()),
            "CUzeBox mapping has duplicate field {logical}"
        );
        out.push_str(&format!(",{logical}:{raw}"));
    }
    for required in REQUIRED_FIELDS {
        ensure!(
            seen.contains(required),
            "CUzeBox mapping is missing {required}"
        );
    }
    Ok(out)
}

/// Replace entries for the supplied SDL GUIDs in a copied database while
/// preserving comments, blank lines, and mappings for unrelated devices.
/// CUzeBox consumes this file from its executable directory; the caller must
/// copy the install root and verify the selected SDL devices before launch.
pub(crate) fn patch_database(baseline: &[u8], mappings: &[Mapping]) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "CUzeBox database is too large"
    );
    ensure!(
        !mappings.is_empty() && mappings.len() <= 2,
        "CUzeBox mapping count is invalid"
    );
    let original = std::str::from_utf8(baseline).context("CUzeBox database is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "CUzeBox database contains a NUL byte"
    );
    let rendered: Vec<_> = mappings.iter().map(descriptor).collect::<Result<_>>()?;
    let guids: BTreeSet<_> = mappings
        .iter()
        .map(|mapping| mapping.guid.to_ascii_lowercase())
        .collect();
    ensure!(
        guids.len() == mappings.len(),
        "CUzeBox SDL GUID is duplicated"
    );
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    for line in original.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let guid = body
            .split(',')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !guids.contains(&guid) {
            output.push_str(line);
        }
    }
    if !output.is_empty() && !output.ends_with(['\n', '\r']) {
        output.push_str(newline);
    }
    for line in rendered {
        output.push_str(&line);
        output.push_str(newline);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(guid: &str, name: &str) -> Mapping {
        Mapping {
            guid: guid.into(),
            name: name.into(),
            platform: Some("Linux".into()),
            fields: vec![
                ("a".into(), "b0".into()),
                ("b".into(), "b1".into()),
                ("x".into(), "b2".into()),
                ("y".into(), "b3".into()),
                ("back".into(), "b4".into()),
                ("start".into(), "b5".into()),
                ("leftshoulder".into(), "b6".into()),
                ("rightshoulder".into(), "b7".into()),
                ("dpup".into(), "h0.1".into()),
                ("dpdown".into(), "h0.4".into()),
                ("dpleft".into(), "h0.8".into()),
                ("dpright".into(), "h0.2".into()),
            ],
        }
    }

    #[test]
    fn replaces_selected_guid_and_preserves_other_entries() {
        let old = mapping("03000000000000000000000000000000", "Old Pad");
        let replacement = mapping("03000000000000000000000000000000", "New Pad");
        let output = patch_database(
            format!(
                "# keep\n{},a:b1\n03000000000000000000000000000001,Other,a:b2\n",
                descriptor(&old).unwrap()
            )
            .as_bytes(),
            &[replacement],
        )
        .unwrap();
        assert!(output.contains("# keep\n"));
        assert!(output.contains("03000000000000000000000000000001,Other,a:b2\n"));
        assert!(output.contains(",New Pad,platform:Linux,a:b0,b:b1,x:b2,y:b3,back:b4,start:b5,leftshoulder:b6,rightshoulder:b7,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2\n"));
        assert!(!output.contains(",Old Pad,"));
    }

    #[test]
    fn validates_guid_and_fields() {
        let mut bad = mapping("short", "Pad");
        assert!(descriptor(&bad).is_err());
        bad.guid = "03000000000000000000000000000000".into();
        bad.fields.push(("a".into(), "b1".into()));
        assert!(descriptor(&bad).is_err());
        let mut bad = mapping("03000000000000000000000000000000", "Pad");
        bad.fields[0].1 = "button zero".into();
        assert!(descriptor(&bad).is_err());
        assert!(patch_database(b"# only\n", &[]).is_err());
    }
}
