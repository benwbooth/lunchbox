//! Validate the emulator platform-capture records under
//! emulator_details/records. Each record captures, per host platform, the
//! save/state locations and naming, the controller-configuration location
//! and syntax pointer, and firmware pointers. Facts must cite evidence.
use std::collections::BTreeSet;
use std::path::Path;

const PLATFORMS: [&str; 4] = ["linux", "linux-flatpak", "windows", "macos"];
const PURPOSES: [&str; 6] = ["saves", "states", "config", "input", "bios", "keys"];
const GAP_STATUSES: [&str; 3] = ["unsupported", "no_verified_package", "unresolved"];
const CAPTURE_STATUSES: [&str; 4] = ["captured", "not_supported", "not_required", "unresolved"];

#[test]
fn platform_records_are_complete_and_evidenced() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../emulator_details/records");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("records directory exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "at least one record must exist");
    let mut seen = BTreeSet::new();
    for path in entries {
        let text = std::fs::read_to_string(&path).unwrap();
        let record: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", path.display()));
        let slug = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let emulator = record["emulator"]
            .as_str()
            .unwrap_or_else(|| panic!("{}: missing emulator name", path.display()));
        assert!(
            seen.insert(emulator.to_owned()),
            "{}: duplicate emulator {emulator}",
            path.display()
        );
        assert_eq!(
            record["slug"].as_str(),
            Some(slug.as_str()),
            "{}: slug must match the filename",
            path.display()
        );
        assert_eq!(record["schema_version"], 1, "{}", path.display());
        let sources = record["sources"]
            .as_array()
            .unwrap_or_else(|| panic!("{}: sources must be an array", path.display()));
        assert!(
            !sources.is_empty(),
            "{}: cite at least one source",
            path.display()
        );
        let platforms = record
            .get("platforms")
            .map(|value| {
                value
                    .as_object()
                    .unwrap_or_else(|| panic!("{}: platforms must be an object", path.display()))
            })
            .cloned()
            .unwrap_or_default();
        for (platform, entry) in &platforms {
            assert!(
                PLATFORMS.contains(&platform.as_str()),
                "{}: unknown platform {platform}",
                path.display()
            );
            let paths = entry["paths"]
                .as_array()
                .unwrap_or_else(|| panic!("{}: {platform} paths must be an array", path.display()));
            assert!(
                !paths.is_empty(),
                "{}: {platform} has no paths",
                path.display()
            );
            let mut purposes = BTreeSet::new();
            for captured in paths {
                let purpose = captured["purpose"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{}: {platform} path lacks purpose", path.display()));
                assert!(
                    PURPOSES.contains(&purpose),
                    "{}: unknown purpose {purpose}",
                    path.display()
                );
                assert!(
                    purposes.insert(purpose),
                    "{}: {platform} duplicates {purpose}",
                    path.display()
                );
                let status = captured
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("captured");
                assert!(
                    CAPTURE_STATUSES.contains(&status),
                    "{}: {platform} {purpose} has unknown status {status}",
                    path.display()
                );
                captured["path"].as_str().unwrap_or_else(|| {
                    panic!("{}: {platform} {purpose} lacks path", path.display())
                });
                let evidence = captured["evidence"].as_array().unwrap_or_else(|| {
                    panic!(
                        "{}: {platform} {purpose} evidence must be an array",
                        path.display()
                    )
                });
                assert!(
                    !evidence.is_empty(),
                    "{}: {platform} {purpose} cites no evidence",
                    path.display()
                );
                if purpose == "saves" || purpose == "states" {
                    captured["naming"].as_str().unwrap_or_else(|| {
                        panic!(
                            "{}: {platform} {purpose} lacks naming conventions",
                            path.display()
                        )
                    });
                }
            }
            assert!(
                purposes.contains("saves")
                    || purposes.contains("states")
                    || purposes.contains("config"),
                "{}: {platform} captures neither saves, states nor config",
                path.display()
            );
        }
        let gaps = record
            .get("platform_gaps")
            .map(|value| {
                value.as_object().unwrap_or_else(|| {
                    panic!("{}: platform_gaps must be an object", path.display())
                })
            })
            .cloned()
            .unwrap_or_default();
        for (platform, gap) in &gaps {
            assert!(
                PLATFORMS.contains(&platform.as_str()),
                "{}: unknown platform gap {platform}",
                path.display()
            );
            assert!(
                !platforms.contains_key(platform),
                "{}: {platform} is both captured and a gap",
                path.display()
            );
            let status = gap["status"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: {platform} gap lacks status", path.display()));
            assert!(
                GAP_STATUSES.contains(&status),
                "{}: {platform} has unknown gap status {status}",
                path.display()
            );
            assert!(
                !gap["reason"].as_str().unwrap_or_default().trim().is_empty(),
                "{}: {platform} gap lacks a reason",
                path.display()
            );
            assert!(
                gap["evidence"]
                    .as_array()
                    .is_some_and(|entries| !entries.is_empty()),
                "{}: {platform} gap cites no evidence",
                path.display()
            );
        }
        for platform in PLATFORMS {
            assert!(
                platforms.contains_key(platform) || gaps.contains_key(platform),
                "{}: {platform} has neither a capture nor a gap disposition",
                path.display()
            );
        }
    }
}
