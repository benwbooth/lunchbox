//! Source pins, four-host dispositions, and refusal guards for three
//! compatibility/obsolete Windows records.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_the_reviewed_artifacts_or_source() {
    let cases = [
        (
            "windows-sorcerer",
            "b3171b5cf2020884c66cf5f3dee6481d230737492d914cc3797e6733e2b6c693",
        ),
        ("wine", "788d90c4e1d628fab6672623f0c8094b984ea2fa"),
        (
            "xebra",
            "5f522e51a0cad7395bdcb070e7c6955327f4bd3449a9c04dc7ed35c6693c1bd2",
        ),
    ];
    for (slug, pin) in cases {
        let record = record(slug);
        assert!(
            serde_json::to_string(&record["sources"])
                .unwrap()
                .contains(pin),
            "{slug} lacks pin {pin}"
        );
        assert_eq!(
            record["controller_boundary"]["native_writer"], "not_implemented",
            "{slug} must fail closed"
        );
    }
}

#[test]
fn every_record_has_a_four_host_disposition() {
    for slug in ["windows-sorcerer", "wine", "xebra"] {
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
fn captured_hosts_expose_the_six_catalog_purposes() {
    for slug in ["windows-sorcerer", "wine", "xebra"] {
        let record = record(slug);
        let platforms = record["platforms"].as_object().unwrap();
        for (host, value) in platforms {
            let paths = value["paths"].as_array().unwrap();
            for purpose in ["config", "input", "saves", "states", "bios", "keys"] {
                assert!(
                    paths.iter().any(|path| path["purpose"] == purpose),
                    "{slug} {host} lacks {purpose}"
                );
            }
        }
    }
}

#[test]
fn source_boundaries_name_the_specific_failure_modes() {
    assert!(
        record("windows-sorcerer")["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("DirectX")
    );
    assert!(
        record("wine")["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("DirectInput")
    );
    assert!(
        record("xebra")["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("axis")
    );
}
