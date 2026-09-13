use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn dchector_is_artifact_pinned_and_refuses_unexposed_grammar() {
    let r = record("dchector");
    assert_eq!(
        r["sources"][1]["sha256"],
        "eb4c69ff01e033523d11113b64cef38a78b804c56abe71cb2788129737b28c1d"
    );
    assert_eq!(
        r["platforms"]["windows"]["paths"][0]["status"],
        "unresolved"
    );
}

#[test]
fn demul_keeps_plugin_numeric_input_boundary() {
    let r = record("demul");
    assert_eq!(
        r["sources"][1]["sha256"],
        "ae3f11ed5d36c4f327b3428b8947181284a7f9ae302d811852d4d7a4e9af9148"
    );
    assert_eq!(
        r["platforms"]["windows"]["paths"][1]["status"],
        "unresolved"
    );
}

#[test]
fn dolphin_triforce_is_source_pinned_and_has_real_state_root() {
    let r = record("dolphin-triforce");
    assert_eq!(
        r["sources"][0]["commit"],
        "842a088dcecbc99348b24557396f56456ceaf2ac"
    );
    assert!(
        r["platforms"]["linux"]["paths"][3]["path"]
            .as_str()
            .unwrap()
            .contains("StateSaves")
    );
    assert!(
        r["notes"]
            .as_str()
            .unwrap()
            .contains("legacy Dolphin Triforce")
    );
}
