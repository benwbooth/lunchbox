//! Focused source/refusal guards for the J2ME Loader, JConfig, and JL-Mod batch.

use serde_json::Value;
use std::{fs, path::Path};

fn record(slug: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../emulator_details/records")
        .join(format!("{slug}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn android_j2me_loaders_keep_four_host_refusal_and_android_boundary() {
    for (slug, commit) in [
        ("j2me-loader", "9b0fa48a0a0d1e61376c0b9af28b3d2caec0a4cc"),
        ("jl-mod", "f723a190c0bdb44b31c3bc0ead6f8665c7ea517d"),
    ] {
        let r = record(slug);
        assert_eq!(r["sources"][0]["commit"], commit);
        assert_eq!(r["android_boundary"]["native_writer"], "not_implemented");
        assert!(
            r["android_boundary"]["saves"]
                .as_str()
                .unwrap()
                .contains(".rsh")
        );
        for platform in ["linux", "linux-flatpak", "macos", "windows"] {
            assert_eq!(r["platform_gaps"][platform]["status"], "unsupported");
        }
    }
}

#[test]
fn jconfig_keeps_windows_only_unresolved_input_and_save_patch_boundary() {
    let r = record("jconfig");
    assert_eq!(
        r["sources"][1]["sha256"],
        "3c475df3211bde59acd3bcfec30845e17829b50d16d6ced564bab83f95d75805"
    );
    assert_eq!(
        r["platforms"]["windows"]["paths"][1]["status"],
        "unresolved"
    );
    assert!(
        r["platforms"]["windows"]["paths"][2]["path"]
            .as_str()
            .unwrap()
            .contains("SV")
    );
    for platform in ["linux", "linux-flatpak", "macos"] {
        assert_eq!(r["platform_gaps"][platform]["status"], "unsupported");
    }
}
