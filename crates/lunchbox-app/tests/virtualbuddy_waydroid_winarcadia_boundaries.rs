//! Source pins, four-host dispositions, and fail-closed controller guards.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_reviewed_sources_and_artifacts() {
    let cases = [
        ("virtualbuddy", "683739a7e75921f83c492d15ece1ab89aa66e4cf"),
        ("waydroid", "5a51271131bfca8b7ee75ed067d09b26460f3a7b"),
        (
            "winarcadia",
            "061544e10b182ad06e0e084ecd1460303917e52f4877fee9ca4fb4ab221565f",
        ),
    ];
    for (slug, pin) in cases {
        let record = record(slug);
        assert!(
            serde_json::to_string(&record["sources"])
                .unwrap()
                .contains(pin)
        );
        assert_eq!(
            record["controller_boundary"]["native_writer"],
            "not_implemented"
        );
    }
}

#[test]
fn every_record_has_an_explicit_four_host_disposition() {
    for slug in ["virtualbuddy", "waydroid", "winarcadia"] {
        let record = record(slug);
        for host in ["linux", "linux-flatpak", "windows", "macos"] {
            assert!(
                record["platforms"].get(host).is_some()
                    || record["platform_gaps"].get(host).is_some(),
                "{slug} lacks disposition for {host}"
            );
        }
    }
}

#[test]
fn unsupported_hosts_and_container_or_vm_boundaries_are_explicit() {
    let virtualbuddy = record("virtualbuddy");
    assert_eq!(
        virtualbuddy["platform_gaps"]["linux"]["status"],
        "unsupported"
    );
    assert!(
        virtualbuddy["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("portable host physical-controller")
    );

    let waydroid = record("waydroid");
    assert_eq!(
        waydroid["platform_gaps"]["linux-flatpak"]["status"],
        "no_verified_package"
    );
    assert!(
        waydroid["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("frontend-owned")
    );

    let winarcadia = record("winarcadia");
    assert_eq!(
        winarcadia["platform_gaps"]["macos"]["status"],
        "unsupported"
    );
    assert!(
        winarcadia["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("DirectInput")
    );
}
