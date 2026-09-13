//! Source pins and fail-closed boundaries for the RetroVM/Retro8/VICE xpet batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_the_reviewed_upstreams() {
    let cases = [
        (
            "retro-virtual-machine",
            "e2b3cbeda8f96d92947a1a1d004e2548da95db33",
        ),
        ("retro8", "ddc06a142398ee9755894b3f0bb17c8dc428151d"),
        ("vice-xpet", "d322f7a8d6c269b97162c74e73214c58eaad9a71"),
    ];
    for (slug, pin) in cases {
        let sources = record(slug)["sources"].clone();
        assert!(
            serde_json::to_string(&sources).unwrap().contains(pin),
            "{slug} lacks source pin {pin}"
        );
        assert_eq!(
            record(slug)["controller_boundary"]["native_writer"],
            "not_implemented",
            "{slug} must fail closed"
        );
    }
}

#[test]
fn every_record_has_an_explicit_four_host_disposition() {
    for slug in ["retro-virtual-machine", "retro8", "vice-xpet"] {
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
fn xpet_does_not_reuse_generic_vice_or_xvic_identity() {
    let xpet = record("vice-xpet");
    assert_eq!(xpet["slug"], "vice-xpet");
    assert_eq!(xpet["emulator"], "VICE (xpet)");
    let refusal = xpet["controller_boundary"]["refusal"].as_str().unwrap();
    assert!(refusal.contains("generic VICE joystick"));
    assert!(refusal.contains("xpet"));
}
