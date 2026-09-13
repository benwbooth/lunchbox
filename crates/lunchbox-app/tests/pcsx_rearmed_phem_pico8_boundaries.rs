//! Focused source-pin and fail-closed guards for the PCSX-ReARMed, PHEM, and PICO-8 batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_primary_evidence_and_preserve_boundaries() {
    assert_eq!(
        record("pcsx-rearmed")["sources"][0]["commit"],
        "d7d741db1d974cf8dd05a23149a1f12758fc1894"
    );
    assert_eq!(
        record("phem")["sources"][0]["commit"],
        "7f2268390e1e60bd2e51c7dff617a05ca84c51f5"
    );
    let pico = record("pico-8");
    assert!(
        pico["platforms"]["linux"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["path"] == "~/.lexaloffle/pico-8/config.txt")
    );
    assert!(
        pico["platforms"]["linux"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["path"] == "~/.lexaloffle/pico-8/sdl_controllers.txt")
    );
    assert_eq!(
        pico["platform_gaps"]["linux-flatpak"]["status"],
        "unsupported"
    );
    for host in ["linux", "linux-flatpak", "windows", "macos"] {
        assert_eq!(
            record("pcsx-rearmed")["platform_gaps"][host]["status"],
            "unresolved"
        );
        assert_eq!(
            record("phem")["platform_gaps"][host]["status"],
            "unsupported"
        );
    }
}
