//! Source-pin and fail-closed guards for Visual Pinball, WASM-4 and Wataroo.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_the_reviewed_source_or_artifact() {
    let vp = record("visual-pinball");
    assert_eq!(
        vp["sources"][0]["commit"],
        "93d93d9653d7fdc295967cb5f30fff33b734e1fa"
    );
    let wasm4 = record("wasm4-w4");
    assert_eq!(
        wasm4["sources"][0]["commit"],
        "9d6c962785cfe3719d0245fd279ecbe98a4dbb63"
    );
    let wataroo = record("wataroo");
    assert_eq!(
        wataroo["sources"][1]["sha256"],
        "f00e32eaa5af0535190cef6f27c209f25c43dae47a5ab4d317df03ff4ab329e0"
    );
}

#[test]
fn each_record_exposes_the_six_platform_purposes() {
    for slug in ["visual-pinball", "wasm4-w4", "wataroo"] {
        let value = record(slug);
        for host in ["linux", "linux-flatpak", "windows", "macos"] {
            if let Some(paths) = value["platforms"][host]["paths"].as_array() {
                for purpose in ["config", "input", "saves", "states", "bios", "keys"] {
                    assert!(
                        paths.iter().any(|path| path["purpose"] == purpose),
                        "{slug} {host} {purpose}"
                    );
                }
            } else {
                assert!(
                    value["platform_gaps"].get(host).is_some(),
                    "{slug} {host} has no disposition"
                );
            }
        }
        assert_eq!(
            value["controller_boundary"]["native_writer"],
            "not_implemented"
        );
    }
}

#[test]
fn unsupported_hosts_are_not_presented_as_captured() {
    let wataroo = record("wataroo");
    for host in ["linux", "linux-flatpak", "macos"] {
        assert_eq!(wataroo["platform_gaps"][host]["status"], "unsupported");
        assert!(wataroo["platforms"].get(host).is_none());
    }
    for slug in ["visual-pinball", "wasm4-w4"] {
        assert_eq!(
            record(slug)["platform_gaps"]["linux-flatpak"]["status"],
            "unresolved"
        );
    }
}
