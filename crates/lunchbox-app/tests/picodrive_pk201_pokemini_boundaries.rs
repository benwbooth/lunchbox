//! Focused source/artifact and controller-boundary guards for this batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_sources_and_host_boundaries() {
    assert_eq!(
        record("picodrive")["sources"][0]["commit"],
        "26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee"
    );
    assert_eq!(
        record("pk201")["sources"][0]["sha256"],
        "b0350fdb7f5cbee609189cdfdf574b0ab89bb536ea44ddbbde3ce3e14b1e6673"
    );
    assert!(
        record("pokemini")["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| {
                source["kind"] == "artifact"
                    && source["sha256"]
                        == "19de65332abe6d203c7065a70764202f28c974fa851b67cf0fe689a5d5c1f17e"
            })
    );
    assert_eq!(
        record("picodrive")["platforms"]["linux"]["paths"][1]["status"],
        "captured"
    );
    assert_eq!(
        record("pokemini")["platforms"]["windows"]["paths"][1]["status"],
        "captured"
    );
    for host in ["linux", "linux-flatpak", "windows", "macos"] {
        assert_eq!(
            record("pk201")["platform_gaps"][host]["status"],
            "unsupported"
        );
    }
    for host in ["linux-flatpak", "windows", "macos"] {
        assert_eq!(
            record("picodrive")["platform_gaps"][host]["status"],
            "unresolved"
        );
    }
    for host in ["linux-flatpak", "macos"] {
        assert_eq!(
            record("pokemini")["platform_gaps"][host]["status"],
            "unsupported"
        );
    }
}
