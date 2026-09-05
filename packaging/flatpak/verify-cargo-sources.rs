use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

#[derive(Default)]
struct LockedPackage {
    name: Option<String>,
    version: Option<String>,
    source: Option<String>,
    checksum: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct Archive {
    url: String,
    checksum: String,
}

#[derive(Default)]
struct FlatpakSources {
    archives: BTreeMap<String, Archive>,
    inline_checksums: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SourceKind {
    Archive,
    Inline,
}

fn quoted_value(line: &str, key: &str) -> Option<String> {
    line.strip_prefix(key)?
        .strip_prefix(" = \"")?
        .strip_suffix('"')
        .map(str::to_owned)
}

fn finish_package(
    package: &mut LockedPackage,
    packages: &mut BTreeMap<String, Archive>,
) -> Result<(), String> {
    let Some(source) = package.source.take() else {
        *package = LockedPackage::default();
        return Ok(());
    };
    if source != "registry+https://github.com/rust-lang/crates.io-index" {
        return Err(format!(
            "unsupported Cargo.lock source {source:?}; update this verifier for the generator's source format"
        ));
    }

    let name = package
        .name
        .take()
        .ok_or("registry package without a name")?;
    let version = package
        .version
        .take()
        .ok_or_else(|| format!("registry package {name} has no version"))?;
    let checksum = package
        .checksum
        .take()
        .ok_or_else(|| format!("registry package {name} {version} has no checksum"))?;
    let destination = format!("cargo/vendor/{name}-{version}");
    let archive = Archive {
        url: format!("https://static.crates.io/crates/{name}/{name}-{version}.crate"),
        checksum,
    };
    if packages.insert(destination.clone(), archive).is_some() {
        return Err(format!(
            "duplicate registry package destination {destination}"
        ));
    }
    *package = LockedPackage::default();
    Ok(())
}

fn locked_packages(contents: &str) -> Result<BTreeMap<String, Archive>, String> {
    let mut packages = BTreeMap::new();
    let mut package = LockedPackage::default();

    for line in contents.lines().chain(["[[package]]"]) {
        if line == "[[package]]" {
            finish_package(&mut package, &mut packages)?;
        } else if let Some(value) = quoted_value(line, "name") {
            package.name = Some(value);
        } else if let Some(value) = quoted_value(line, "version") {
            package.version = Some(value);
        } else if let Some(value) = quoted_value(line, "source") {
            package.source = Some(value);
        } else if let Some(value) = quoted_value(line, "checksum") {
            package.checksum = Some(value);
        }
    }

    Ok(packages)
}

fn json_string(line: &str, key: &str) -> Option<String> {
    let value = line.trim().strip_prefix(&format!("\"{key}\": \""))?;
    let value = value.strip_suffix(',').unwrap_or(value).strip_suffix('"')?;
    Some(value.to_owned())
}

fn inline_checksum(contents: &str) -> Option<String> {
    contents
        .strip_prefix("{\\\"package\\\": \\\"")?
        .strip_suffix("\\\", \\\"files\\\": {}}")
        .map(str::to_owned)
}

fn flatpak_sources(contents: &str) -> Result<FlatpakSources, String> {
    let mut sources = FlatpakSources::default();
    let mut kind = None;
    let mut url = None;
    let mut checksum = None;
    let mut destination = None;
    let mut inline_contents = None;
    let mut destination_filename = None;

    for line in contents.lines() {
        let parsed_kind = match line.trim() {
            "\"type\": \"archive\"," => Some(SourceKind::Archive),
            "\"type\": \"inline\"," => Some(SourceKind::Inline),
            _ => None,
        };
        if let Some(parsed_kind) = parsed_kind {
            if kind.replace(parsed_kind).is_some() {
                return Err("nested entries in Flatpak source manifest".to_owned());
            }
        } else if kind.is_some() {
            url = url.or_else(|| json_string(line, "url"));
            checksum = checksum.or_else(|| json_string(line, "sha256"));
            destination = destination.or_else(|| json_string(line, "dest"));
            inline_contents = inline_contents.or_else(|| json_string(line, "contents"));
            destination_filename =
                destination_filename.or_else(|| json_string(line, "dest-filename"));

            if line.trim() == "}," || line.trim() == "}" {
                let destination = destination
                    .take()
                    .ok_or("Flatpak source entry without a destination")?;
                match kind.take().expect("source kind was checked above") {
                    SourceKind::Archive => {
                        let archive = Archive {
                            url: url.take().ok_or("Flatpak archive entry without a URL")?,
                            checksum: checksum
                                .take()
                                .ok_or("Flatpak archive entry without a checksum")?,
                        };
                        if sources
                            .archives
                            .insert(destination.clone(), archive)
                            .is_some()
                        {
                            return Err(format!(
                                "duplicate Flatpak archive destination {destination}"
                            ));
                        }
                    }
                    SourceKind::Inline if destination.starts_with("cargo/vendor/") => {
                        if destination_filename.as_deref() != Some(".cargo-checksum.json") {
                            return Err(format!(
                                "Flatpak checksum entry for {destination} has the wrong filename"
                            ));
                        }
                        let package_checksum = inline_contents
                            .take()
                            .and_then(|contents| inline_checksum(&contents))
                            .ok_or_else(|| {
                                format!("malformed Flatpak checksum entry for {destination}")
                            })?;
                        if sources
                            .inline_checksums
                            .insert(destination.clone(), package_checksum)
                            .is_some()
                        {
                            return Err(format!(
                                "duplicate Flatpak checksum destination {destination}"
                            ));
                        }
                    }
                    SourceKind::Inline => {}
                }
                url = None;
                checksum = None;
                inline_contents = None;
                destination_filename = None;
            }
        }
    }

    if kind.is_some() {
        return Err("unterminated entry in Flatpak source manifest".to_owned());
    }
    Ok(sources)
}

fn find_drift(expected: &BTreeMap<String, Archive>, actual: &FlatpakSources) -> Vec<String> {
    let mut drift = Vec::new();
    for (destination, expected_archive) in expected {
        match actual.archives.get(destination) {
            None => drift.push(format!("missing archive {destination}")),
            Some(actual_archive) if actual_archive != expected_archive => {
                drift.push(format!("stale archive {destination}"));
            }
            Some(_) => {}
        }
        match actual.inline_checksums.get(destination) {
            None => drift.push(format!("missing checksum metadata {destination}")),
            Some(actual_checksum) if actual_checksum != &expected_archive.checksum => {
                drift.push(format!("stale checksum metadata {destination}"));
            }
            Some(_) => {}
        }
    }
    for destination in actual.archives.keys() {
        if !expected.contains_key(destination) {
            drift.push(format!("unexpected archive {destination}"));
        }
    }
    for destination in actual.inline_checksums.keys() {
        if !expected.contains_key(destination) {
            drift.push(format!("unexpected checksum metadata {destination}"));
        }
    }
    drift
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let lock_path = args
        .next()
        .ok_or("usage: verify-cargo-sources Cargo.lock cargo-sources.json")?;
    let sources_path = args
        .next()
        .ok_or("usage: verify-cargo-sources Cargo.lock cargo-sources.json")?;
    if args.next().is_some() {
        return Err("usage: verify-cargo-sources Cargo.lock cargo-sources.json".to_owned());
    }

    let lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("failed to read {lock_path}: {error}"))?;
    let sources = fs::read_to_string(&sources_path)
        .map_err(|error| format!("failed to read {sources_path}: {error}"))?;
    let expected = locked_packages(&lock)?;
    let actual = flatpak_sources(&sources)?;
    let drift = find_drift(&expected, &actual);

    if drift.is_empty() {
        println!(
            "{sources_path} matches {lock_path} ({} registry packages)",
            expected.len()
        );
        Ok(())
    } else {
        Err(format!(
            "{sources_path} is stale relative to {lock_path}:\n{}\nregenerate it with the official flatpak-cargo-generator",
            drift.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
version = 4

[[package]]
name = "demo"
version = "1.2.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abcd"

[[package]]
name = "workspace-package"
version = "0.1.0"
"#;

    const SOURCES: &str = r#"[
    {
        "type": "archive",
        "archive-type": "tar-gzip",
        "url": "https://static.crates.io/crates/demo/demo-1.2.3.crate",
        "sha256": "abcd",
        "dest": "cargo/vendor/demo-1.2.3"
    },
    {
        "type": "inline",
        "contents": "{\"package\": \"abcd\", \"files\": {}}",
        "dest": "cargo/vendor/demo-1.2.3",
        "dest-filename": ".cargo-checksum.json"
    },
    {
        "type": "inline",
        "contents": "[source.vendored-sources]",
        "dest": "cargo",
        "dest-filename": "config"
    }
]"#;

    #[test]
    fn parses_lock_and_flatpak_source_schema() {
        let expected = locked_packages(LOCK).unwrap();
        assert_eq!(expected.len(), 1);
        assert_eq!(
            expected["cargo/vendor/demo-1.2.3"],
            Archive {
                url: "https://static.crates.io/crates/demo/demo-1.2.3.crate".to_owned(),
                checksum: "abcd".to_owned(),
            }
        );

        let actual = flatpak_sources(SOURCES).unwrap();
        assert_eq!(actual.archives, expected);
        assert_eq!(actual.inline_checksums["cargo/vendor/demo-1.2.3"], "abcd");
    }

    #[test]
    fn reports_missing_and_extra_entries() {
        let expected = locked_packages(LOCK).unwrap();
        let mut actual = FlatpakSources::default();
        actual.archives.insert(
            "cargo/vendor/extra-9.9.9".to_owned(),
            Archive {
                url: "https://example.invalid/extra.crate".to_owned(),
                checksum: "extra".to_owned(),
            },
        );
        actual
            .inline_checksums
            .insert("cargo/vendor/extra-9.9.9".to_owned(), "extra".to_owned());

        assert_eq!(
            find_drift(&expected, &actual),
            [
                "missing archive cargo/vendor/demo-1.2.3",
                "missing checksum metadata cargo/vendor/demo-1.2.3",
                "unexpected archive cargo/vendor/extra-9.9.9",
                "unexpected checksum metadata cargo/vendor/extra-9.9.9",
            ]
        );
    }

    #[test]
    fn reports_archive_and_inline_checksum_mismatches() {
        let expected = locked_packages(LOCK).unwrap();
        let mut actual = flatpak_sources(SOURCES).unwrap();
        actual
            .archives
            .get_mut("cargo/vendor/demo-1.2.3")
            .unwrap()
            .checksum = "wrong".to_owned();
        actual
            .inline_checksums
            .insert("cargo/vendor/demo-1.2.3".to_owned(), "wrong".to_owned());

        assert_eq!(
            find_drift(&expected, &actual),
            [
                "stale archive cargo/vendor/demo-1.2.3",
                "stale checksum metadata cargo/vendor/demo-1.2.3",
            ]
        );
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
