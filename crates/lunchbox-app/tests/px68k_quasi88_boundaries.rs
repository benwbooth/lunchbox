//! Source and fail-closed guards for PX68k and QUASI88.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_primary_cores_and_refuse_native_writers() {
    let cases = [
        ("px68k", "0ad84d7058a12b7db4f7f7a906e87fad4e2f26f6", "PX68k"),
        (
            "quasi88",
            "459bbc6e90caa3dc392ae8e64a9b0881b1e5ef77",
            "QUASI88",
        ),
    ];
    for (slug, commit, name) in cases {
        let value = record(slug);
        let sources = serde_json::to_string(&value["sources"]).unwrap();
        assert!(sources.contains(commit), "{slug} lacks pinned source");
        assert_eq!(value["emulator"], name);
        assert_eq!(
            value["controller_boundary"]["native_writer"],
            "not_implemented"
        );
        let refusal = value["controller_boundary"]["refusal"].as_str().unwrap();
        assert!(refusal.contains("Do not") || refusal.contains("unavailable"));
    }
}

#[test]
fn each_host_has_the_six_purposes() {
    for slug in ["px68k", "quasi88"] {
        let value = record(slug);
        for host in ["linux", "linux-flatpak", "windows", "macos"] {
            let paths = value["platforms"][host]["paths"].as_array().unwrap();
            let purposes: Vec<_> = paths
                .iter()
                .map(|path| path["purpose"].as_str().unwrap())
                .collect();
            for purpose in ["config", "input", "saves", "states", "bios", "keys"] {
                assert!(purposes.contains(&purpose), "{slug} {host} {purpose}");
            }
        }
    }
}

#[test]
fn puae_retroarch_keeps_the_shared_native_boundary() {
    let value = record("puae-retroarch");
    assert_eq!(
        value["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        value["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("second PUAE writer")
    );
    assert_eq!(
        value["sources"][2]["path"],
        "docs/PUAE_CONTROLLER_CONTRACT.md"
    );
}
