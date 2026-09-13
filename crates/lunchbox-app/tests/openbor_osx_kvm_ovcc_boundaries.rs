//! Source and contract guards for OpenBOR, OSX-KVM and OVCC.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_the_three_source_oracles_and_writer_boundary() {
    for (slug, commit, writer) in [
        (
            "openbor",
            "9d81480f8481fbb9e76b0b5f2a5dfa408376761a",
            "not_implemented",
        ),
        (
            "osx-kvm",
            "4c378a4b5e0b219783683012bec680325eb40719",
            "not_implemented",
        ),
        (
            "ovcc",
            "cc936b25be3da2c03b21a9bf1cc2ff4a494a5f09",
            "implemented",
        ),
    ] {
        let r = record(slug);
        assert_eq!(r["sources"][0]["commit"], commit, "{slug}");
        assert_eq!(r["controller_boundary"]["native_writer"], writer, "{slug}");
        assert!(r["controller_boundary"]["refusal"].as_str().unwrap().len() > 40);
        for host in ["linux", "windows", "macos"] {
            let paths = r["platforms"][host]["paths"].as_array().unwrap();
            for purpose in ["config", "input", "saves", "states", "bios", "keys"] {
                assert!(
                    paths.iter().any(|p| p["purpose"] == purpose),
                    "{slug} {host} {purpose}"
                );
            }
        }
    }
}

#[test]
fn flatpak_and_unsupported_host_boundaries_remain_explicit() {
    for slug in ["openbor", "osx-kvm", "ovcc"] {
        assert_eq!(
            record(slug)["platform_gaps"]["linux-flatpak"]["status"],
            "no_verified_package"
        );
    }
    let osx_kvm = record("osx-kvm");
    let windows = osx_kvm["platforms"]["windows"]["paths"].as_array().unwrap();
    assert!(windows.iter().all(|path| path["status"] == "not_supported"));
    for host in ["linux", "macos"] {
        let states = osx_kvm["platforms"][host]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .find(|path| path["purpose"] == "states")
            .unwrap();
        assert_eq!(
            states["status"], "captured",
            "QEMU monitor savevm is documented for {host}"
        );
    }
}
