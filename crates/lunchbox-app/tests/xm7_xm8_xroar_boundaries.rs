//! Source pins, six-purpose platform captures, and controller ownership guards.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn purposes<'a>(record: &'a Value, slug: &str, host: &str) -> Vec<&'a str> {
    record["platforms"][host]["paths"]
        .as_array()
        .unwrap_or_else(|| panic!("{slug} {host} is not a captured platform"))
        .iter()
        .map(|path| path["purpose"].as_str().unwrap())
        .collect()
}

#[test]
fn records_pin_reviewed_sources_and_writer_boundaries() {
    let xm7 = record("xm7");
    assert_eq!(
        xm7["sources"][0]["commit"],
        "37f83f77d94d7510162952755be40b7e6f5d0d45"
    );
    assert_eq!(
        xm7["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        xm7["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("Do not")
    );

    let xm8 = record("xm8");
    assert_eq!(
        xm8["sources"][0]["commit"],
        "2c4bf025840711a696d7d9ef55f3be3d0f3c84f9"
    );
    assert_eq!(
        xm8["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        xm8["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("Do not")
    );

    let xroar = record("xroar");
    assert_eq!(
        xroar["sources"][0]["commit"],
        "0229f97a636c3c80d51fd27e7d145d792f0a8932"
    );
    assert_eq!(xroar["controller_boundary"]["native_writer"], "writer_only");
    assert_eq!(
        xroar["controller_boundary"]["writer"],
        "crates/lunchbox-app/src/controller_xroar_native.rs"
    );
}

#[test]
fn captured_hosts_expose_all_six_purposes() {
    for slug in ["xm7", "xm8"] {
        let value = record(slug);
        for host in ["linux", "windows"] {
            let actual = purposes(&value, slug, host);
            for expected in ["config", "input", "saves", "states", "bios", "keys"] {
                assert!(actual.contains(&expected), "{slug} {host} lacks {expected}");
            }
        }
        assert_eq!(
            value["platform_gaps"]["linux-flatpak"]["status"],
            "no_verified_package"
        );
        assert!(value["platform_gaps"].get("macos").is_some());
    }

    let xroar = record("xroar");
    for host in ["linux", "windows", "macos"] {
        let actual = purposes(&xroar, "xroar", host);
        for expected in ["config", "input", "saves", "states", "bios", "keys"] {
            assert!(actual.contains(&expected), "xroar {host} lacks {expected}");
        }
    }
    assert_eq!(
        xroar["platform_gaps"]["linux-flatpak"]["status"],
        "no_verified_package"
    );
}
