//! Source-pin and fail-closed guards for the MUGEN, NES.emu, NetherSX2 batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn exact_records_have_pins_or_documented_no_pin_and_refuse_desktop_hosts() {
    let mugen = record("mugen");
    assert!(
        mugen["sources"][0]["url"]
            .as_str()
            .unwrap()
            .contains("elecbyte.com")
    );
    assert!(mugen["sources"][0].get("commit").is_none());
    let cases = [
        ("nes-emu", "1c12fac5ce49badaadff2e2f210dcc30b89f4943"),
        ("nethersx2", "98ddad36d6c367a045b65fd06fdb26ba4c611387"),
    ];
    for (slug, commit) in cases {
        assert_eq!(record(slug)["sources"][0]["commit"], commit);
    }
    for slug in ["mugen", "nes-emu", "nethersx2"] {
        let r = record(slug);
        for host in ["linux-flatpak", "windows", "macos"] {
            assert_eq!(r["platform_gaps"][host]["status"], "unsupported");
        }
    }
    assert_eq!(
        record("nes-emu")["platform_gaps"]["linux"]["status"],
        "unresolved"
    );
    assert_eq!(
        record("mugen")["platform_gaps"]["linux"]["status"],
        "unsupported"
    );
    assert_eq!(
        record("nethersx2")["platform_gaps"]["linux"]["status"],
        "unsupported"
    );
}
