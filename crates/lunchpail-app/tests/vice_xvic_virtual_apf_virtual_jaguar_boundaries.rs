//! Source pins and identity/controller boundaries for xvic, Virtual APF and
//! Virtual Jaguar.

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
        ("vice-xvic", "d322f7a8d6c269b97162c74e73214c58eaad9a71"),
        (
            "virtual-apf",
            "a2a4b5a2781e3858251deb0aea6d6d07b48099b61870a405539d8a590bd933ee",
        ),
        ("virtual-jaguar", "f9a3c89f58836cb2c45a42ad4edfac047050f30a"),
    ];
    for (slug, pin) in cases {
        assert!(
            serde_json::to_string(&record(slug)["sources"])
                .unwrap()
                .contains(pin),
            "{slug} lacks source pin {pin}"
        );
    }
}

#[test]
fn every_record_has_an_explicit_four_host_disposition() {
    for slug in ["vice-xvic", "virtual-apf", "virtual-jaguar"] {
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
fn identities_and_controller_boundaries_do_not_cross_wire() {
    let xvic = record("vice-xvic");
    assert_eq!(xvic["emulator"], "VICE (xvic)");
    assert_eq!(
        xvic["controller_boundary"]["native_writer"],
        "shared_vice_native_vjm"
    );
    assert!(
        xvic["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("one joystick port")
    );

    let apf = record("virtual-apf");
    assert_eq!(
        apf["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        apf["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("opaque")
    );
    assert_eq!(
        apf["platform_gaps"]["linux"]["status"],
        "no_verified_package"
    );

    let jaguar = record("virtual-jaguar");
    assert_eq!(
        jaguar["controller_boundary"]["native_writer"],
        "not_implemented"
    );
    assert!(
        jaguar["controller_boundary"]["refusal"]
            .as_str()
            .unwrap()
            .contains("libretro core")
    );
    assert_eq!(
        jaguar["controller_boundary"]["shared_integration"],
        "emulator_details/retroarch-cores/virtual_jaguar.json"
    );
}
