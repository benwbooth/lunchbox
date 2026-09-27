//! J-R platform-record completeness guard.
//!
//! These records are source-backed inventory, not proof that a controller
//! writer is safe to synthesize.  Keep every captured host explicit about all
//! six persistence/input/key purposes while leaving empty host entries to the
//! owning gap audit.

use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

const PURPOSES: [&str; 6] = ["config", "input", "saves", "states", "bios", "keys"];

#[test]
fn j_through_r_captured_hosts_have_six_purposes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../emulator_details/records");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let slug = path.file_stem().unwrap().to_str().unwrap();
        let Some(&first) = slug.as_bytes().first() else {
            continue;
        };
        if !(b'j'..=b'r').contains(&first) {
            continue;
        }
        let record: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let Some(platforms) = record["platforms"].as_object() else {
            continue;
        };
        for (platform, host) in platforms {
            let Some(paths) = host["paths"].as_array() else {
                continue;
            };
            if paths.is_empty() {
                continue;
            }
            let purposes: BTreeSet<_> = paths
                .iter()
                .filter_map(|path| path["purpose"].as_str())
                .collect();
            for purpose in PURPOSES {
                assert!(
                    purposes.contains(purpose),
                    "{slug}/{platform} lacks {purpose}"
                );
            }
        }
    }
}
