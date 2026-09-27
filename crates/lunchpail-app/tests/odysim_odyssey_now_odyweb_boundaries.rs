//! Source/artifact and fail-closed guards for the Magnavox Odyssey batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_primary_artifacts_and_refuse_native_writers() {
    let cases = [
        ("odysim", "https://archive.org/details/OdySim"),
        (
            "odyssey-now-hal",
            "7eb0c1da4f46728dd22eab189840f8f51f03118b",
        ),
        ("odyweb", "f1951bbe832c"),
    ];
    for (slug, pin) in cases {
        let record = record(slug);
        assert_eq!(
            record["controller_boundary"]["native_writer"],
            "not_implemented"
        );
        let sources = record["sources"].as_array().unwrap();
        let source_text = serde_json::to_string(sources).unwrap();
        assert!(source_text.contains(pin), "{slug} lacks primary pin");
        assert!(
            record["controller_boundary"]["refusal"]
                .as_str()
                .unwrap()
                .contains("Do not invent")
        );
    }
}

#[test]
fn records_have_explicit_host_dispositions() {
    for slug in ["odysim", "odyssey-now-hal", "odyweb"] {
        let record = record(slug);
        for host in ["linux", "linux-flatpak", "windows", "macos"] {
            assert!(
                record["platforms"].get(host).is_some()
                    || record["platform_gaps"].get(host).is_some(),
                "{slug} {host}"
            );
        }
    }
}
