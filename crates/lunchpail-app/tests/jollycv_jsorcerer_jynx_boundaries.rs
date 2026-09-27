//! Source/refusal guards for the JollyCV, JSorcerer, and Jynx records.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn records_pin_sources_and_refuse_unbacked_writers() {
    let cases = [
        (
            "jollycv",
            "commit",
            "7141b3438c9f6d21e619df56cea76a8d998f38c5",
        ),
        (
            "jsorcerer",
            "sha256",
            "2d8e625ff94b1d5c9f39aa00e2763fe2a120149ef14943b4dd283a2ef29d199e",
        ),
        ("jynx", "commit", "a02560be8a9b6a13b3a5a33de7e032d8f86dc658"),
    ];
    for (slug, field, pin) in cases {
        let record = record(slug);
        assert_eq!(record["sources"][0][field].as_str(), Some(pin), "{slug}");
        assert_eq!(
            record["controller_boundary"]["native_writer"],
            "not_implemented"
        );
        let refusal = record["controller_boundary"]["refusal"].as_str().unwrap();
        assert!(refusal.contains("Do not") || refusal.contains("unavailable"));
    }
}

#[test]
fn every_verified_host_record_has_the_six_purposes() {
    for slug in ["jollycv", "jsorcerer", "jynx"] {
        let record = record(slug);
        for host in ["linux", "windows"] {
            let purposes: Vec<_> = record["platforms"][host]["paths"]
                .as_array()
                .unwrap()
                .iter()
                .map(|path| path["purpose"].as_str().unwrap())
                .collect();
            for purpose in ["config", "input", "saves", "states", "bios", "keys"] {
                assert!(purposes.contains(&purpose), "{slug} {host} {purpose}");
            }
        }
    }
}
