//! Source pins and non-duplicating controller boundaries for Proton, Provenance,
//! and the distinct native/documentation PUAE records.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn source_records_pin_the_reviewed_projects() {
    let cases = [
        ("proton", "5b89db940e0ebe3a137a6009a3589232fe084c09"),
        ("provenance", "93f49b4079df204fcd77f29883d00bb5bde629d4"),
        ("puae", "6536174a80d74e6c325aaa5390ff091fac8761d0"),
        ("puae-retroarch", "6536174a80d74e6c325aaa5390ff091fac8761d0"),
    ];
    for (slug, pin) in cases {
        let sources = record(slug)["sources"].clone();
        assert!(
            serde_json::to_string(&sources).unwrap().contains(pin),
            "{slug} lacks source pin {pin}"
        );
    }
}

#[test]
fn compatibility_and_documentation_records_fail_closed() {
    for slug in ["proton", "provenance"] {
        let boundary = record(slug)["controller_boundary"].clone();
        assert_eq!(boundary["native_writer"], "not_implemented", "{slug}");
        assert!(
            boundary["refusal"]
                .as_str()
                .unwrap()
                .contains("Do not invent"),
            "{slug} must refuse an unbacked writer"
        );
    }

    assert_eq!(
        record("puae")["controller_boundary"]["native_writer"],
        "implemented"
    );
    assert_eq!(
        record("puae-retroarch")["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        record("puae-retroarch")["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("second PUAE writer")
    );
}

#[test]
fn every_record_has_an_explicit_four_host_disposition() {
    for slug in ["proton", "provenance", "puae", "puae-retroarch"] {
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
