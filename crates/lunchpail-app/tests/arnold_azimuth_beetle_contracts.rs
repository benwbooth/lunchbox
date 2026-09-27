//! Focused source-backfill guards for the Arnold/Azimuth/Beetle VB batch.

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
fn arnold_has_pinned_unix_grammar_and_windows_registry_boundary() {
    let r = record("arnold");
    assert_eq!(
        r["sources"][0]["commit"],
        "e5dce08964f94add100f7db992a6d0e49fe74f01"
    );
    assert!(
        r["platforms"]["linux"]["paths"][0]["path"]
            .as_str()
            .unwrap()
            .contains(".arnold")
    );
    assert!(
        r["platforms"]["windows"]["paths"][0]["path"]
            .as_str()
            .unwrap()
            .contains("HKCU\\Software\\Arnold")
    );
}

#[test]
fn azimuth_is_artifact_only_without_fabricated_desktop_support() {
    let r = record("azimuth");
    assert_eq!(r["android_boundary"]["native_writer"], "not_implemented");
    for platform in ["linux", "linux-flatpak", "windows", "macos"] {
        assert_eq!(r["platform_gaps"][platform]["status"], "unsupported");
    }
}

#[test]
fn beetle_vb_remains_a_retroarch_hosted_core() {
    let r = record("beetle-vb");
    assert_eq!(
        r["sources"][0]["commit"],
        "83ed42608601fb7b01d41e4f8fb2007a37b8c84e"
    );
    assert!(r["notes"].as_str().unwrap().contains("libretro core"));
    let input = r["platforms"]["linux"]["paths"]
        .as_array()
        .unwrap()
        .iter()
        .find(|path| path["purpose"] == "input")
        .unwrap();
    assert!(input["naming"].as_str().unwrap().contains("input_playerN"));
}
