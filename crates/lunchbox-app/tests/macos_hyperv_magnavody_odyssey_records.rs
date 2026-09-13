//! Focused record guards for the macOS-on-Hyper-V, Magnavody, and Odyssey DS batch.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn magnavody_captures_godot_desktop_user_data_and_declines_flatpak() {
    let r = record("magnavody");
    assert_eq!(
        r["sources"][0]["commit"],
        "b07ecc571f26ba6c934a7b9cc05557e396284984"
    );
    for platform in ["linux", "windows", "macos"] {
        assert!(
            r["platforms"][platform]["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|path| path["purpose"] == "input")
        );
    }
    assert_eq!(
        r["platform_gaps"]["linux-flatpak"]["status"],
        "no_verified_package"
    );
    assert!(r["notes"].as_str().unwrap().contains("ResourceSaver"));
}

#[test]
fn hyperv_and_ds_remain_desktop_boundaries() {
    let hyperv = record("macos-on-hyper-v");
    assert_eq!(
        hyperv["sources"][0]["commit"],
        "cacf043c6b362c621037d44a85ad15feca9ef7aa"
    );
    assert!(
        hyperv["platforms"]["windows"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| path["purpose"] == "bios")
    );
    assert_eq!(hyperv["platform_gaps"]["macos"]["status"], "unsupported");

    let ds = record("magnavox-odyssey-ds");
    for platform in ["linux", "linux-flatpak", "windows", "macos"] {
        assert_eq!(ds["platform_gaps"][platform]["status"], "unsupported");
    }
    assert!(ds["notes"].as_str().unwrap().contains("R/L"));
}
