//! Pre-database cartridge bytes for BizHawk 8c6b8958bbbe623eaaa36bc82af858b812893628.
//! Mirrors RomGame.cs, not core-specific loading or a resolved GameInfo identity.
use anyhow::{Context, Result, ensure};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;

/// Independent pre-patch GameInfo identity retained through a capture child.
#[cfg(target_os = "linux")]
pub(crate) struct CaptureIdentity {
    artifact: super::RuntimeArtifact,
    database: super::database_snapshot::DatabaseSnapshot,
}

#[cfg(target_os = "linux")]
impl CaptureIdentity {
    pub(crate) fn prepare(
        path: &std::path::Path,
        exe_directory: &std::path::Path,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        configuration: &str,
        expected_system: &str,
        expected_hash: &str,
    ) -> Result<Self> {
        use std::io::Read;
        ensure!(
            configuration.len() <= 16 * 1024 * 1024,
            "Capture configuration exceeds 16 MiB"
        );
        let config: serde_json::Value = serde_json::from_str(configuration)?;
        ensure!(
            config.is_object(),
            "Capture configuration must be an object"
        );
        let artifact = super::RuntimeArtifact::capture(path)?;
        let mut bytes = Vec::new();
        const LIMIT: u64 = 512 * 1024 * 1024;
        std::fs::File::open(path)?
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= LIMIT,
            "Cartridge grew beyond the identity input bound"
        );
        let digest: [u8; 32] = sha2::Sha256::digest(&bytes).into();
        ensure!(
            digest == artifact.sha256,
            "Cartridge changed while deriving capture identity"
        );
        artifact.verify()?;
        let extension = format!(
            ".{}",
            path.extension()
                .and_then(|ext| ext.to_str())
                .context("Cartridge identity needs a UTF-8 file extension")?
                .to_ascii_lowercase()
        );
        let input = DatabaseInput::from_cartridge(&bytes, &extension)?;
        let database = super::database_snapshot::DatabaseSnapshot::prepare_for_environment(
            exe_directory,
            environment,
        )?;
        let (system, hash) = match database.lookup(&input)? {
            Some(found) => (found.record.system.as_str(), found.record.digest.as_str()),
            None => {
                // Database.GetGameInfo's extension branches precede the
                // saved platform preference in RomLoader.LoadRom.
                let system = match extension.as_str() {
                    ".nes" | ".unf" | ".fds" => "NES",
                    ".sms" => "SMS",
                    ".gg" => "GG",
                    ".sg" => "SG",
                    ".gen" | ".md" | ".smd" => "GEN",
                    ".pce" => "PCE",
                    ".sgx" => "SGX",
                    ".sfc" | ".smc" | ".bs" => {
                        if is_satellaview(input.bytes())? {
                            "BSX"
                        } else {
                            "SNES"
                        }
                    }
                    ".bin" | ".rom" => "",
                    _ => anyhow::bail!("Unsupported cartridge loader extension"),
                };
                (system, input.normalized_sha1())
            }
        };
        let system = if system.is_empty() {
            configured_system(&config, &extension)?
        } else {
            system
        };
        ensure!(
            system == expected_system && hash == expected_hash,
            "Requested capture system/hash differs from independently derived cartridge identity"
        );
        let identity = Self { artifact, database };
        identity.verify()?;
        Ok(identity)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.artifact.verify()?;
        self.database.verify()
    }
}

/// RomLoader consults this only when GameInfo.System is null/empty. Exact
/// lowercase keys, nonempty values; no fuzzy platform names or deck fallback.
fn configured_system<'a>(config: &'a serde_json::Value, extension: &str) -> Result<&'a str> {
    let preferences = config
        .get("PreferredPlatformsForExtensions")
        .and_then(serde_json::Value::as_object)
        .context("Cartridge needs a saved PreferredPlatformsForExtensions selection")?;
    let system = preferences.get(extension).and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("No explicit platform selected for {extension}; interactive platform guessing is not supported during capture"))?;
    ensure!(
        system.len() <= 64 && !system.chars().any(char::is_control),
        "Invalid saved platform identity"
    );
    Ok(system)
}

/// Pinned SatellaviewFileTypeDetector scoring, applied only on a database miss.
/// This identifies a loader system; it does not add a Satellaview controller deck.
fn is_satellaview(rom: &[u8]) -> Result<bool> {
    if rom.len() != 0x100000 {
        return Ok(false);
    }
    fn header_matches(header: &[u8], hirom: bool) -> Result<bool> {
        let title = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&header[0x10..0x20])
            .context("Satellaview title has malformed Shift-JIS; .NET replacement behavior is not implemented")?;
        let has_title_character = title.trim_end().chars().any(|character| {
            matches!(character, ' '..='~' | '\u{3041}'..='\u{30ff}' | '\u{ff01}'..='\u{ffee}')
        });
        let mut corruption = if has_title_character { 0 } else { 1 };
        let self_destruct = header[0x25];
        if self_destruct & 0x80 != 0 {
            if !matches!(self_destruct, 0x80 | 0x84 | 0x8c | 0x9c | 0xbc | 0xfc) {
                corruption += 2;
            }
        } else if self_destruct != 0 {
            corruption += 2;
        }
        let speed = header[0x28];
        if speed & 0x0e != 0 || (speed & 1 != 0) != hirom {
            corruption += 2;
        }
        let content_type = header[0x29];
        if content_type & 0x0f != 0 {
            corruption += 2;
        } else if !matches!(content_type >> 4, 0 | 1 | 2 | 3 | 10) {
            corruption += 1;
        }
        if header[0x2a] != 0x33 {
            corruption += 3;
        }
        // Upstream VerifyChecksum is unconditional at this revision. Do not
        // invent a checksum requirement or call this a validated ROM checksum.
        Ok(corruption <= 3)
    }
    if header_matches(&rom[0x7fb0..0x8000], false)? {
        return Ok(true);
    }
    header_matches(&rom[0xffb0..0x10000], true)
}

/// A parsed database row. Field five is the ignored comment/genre field;
/// patch metadata is field six, not the fifth field written by some tools.
pub(crate) struct DatabaseRecord {
    pub digest: String,
    pub name: String,
    pub system: String,
    pub metadata: Option<String>,
    pub region: String,
    pub forced_core: String,
}

impl DatabaseRecord {
    /// Parse a data record only. The include loader must handle comments,
    /// blank lines and directives separately and preserve expansion order.
    pub(crate) fn parse(line: &str) -> Result<Self> {
        ensure!(
            line.len() <= 1024 * 1024,
            "BizHawk database row exceeds 1 MiB"
        );
        ensure!(
            !line.starts_with([';', '#']) && !line.contains(['\r', '\n', '\0']),
            "Expected one BizHawk database data record"
        );
        let mut fields = line.split('\t');
        let hash = fields.next().context("Missing database digest")?;
        // FormatHash removes everything through the first colon, regardless
        // of the prefix text, then uppercases the remaining ASCII characters.
        let digest = hash
            .split_once(':')
            .map_or(hash, |(_, suffix)| suffix)
            .to_ascii_uppercase();
        ensure!(
            matches!(digest.len(), 8 | 32 | 40)
                && digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Database digest is not a CRC32, MD5 or SHA-1 lookup key"
        );
        let _status = fields.next().context("Missing database status field")?;
        let name = fields
            .next()
            .context("Missing database name field")?
            .to_owned();
        let system = fields
            .next()
            .context("Missing database system field")?
            .to_owned();
        let _comment = fields.next();
        let metadata = fields.next().map(str::to_owned);
        let region = fields.next().unwrap_or("").to_owned();
        let forced_core = fields.next().unwrap_or("").to_owned();
        Ok(Self {
            digest,
            name,
            system,
            metadata,
            region,
            forced_core,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum LookupAlgorithm {
    Sha1,
    Md5,
    Crc32,
}

pub(crate) struct DatabaseMatch<'a> {
    pub algorithm: LookupAlgorithm,
    pub record: &'a DatabaseRecord,
}

/// Index primitives, NOT an authenticated effective database snapshot. Only a
/// complete ordered include traversal can establish a meaningful lookup miss.
#[derive(Default)]
pub(crate) struct RecordIndex {
    records: BTreeMap<String, DatabaseRecord>,
}

impl RecordIndex {
    /// Call in source/include expansion order. Later entries win exactly as
    /// in InitializeWork's assignment to _builder[game.Hash].
    pub(crate) fn insert(&mut self, record: DatabaseRecord) -> Result<()> {
        ensure!(
            self.records.contains_key(&record.digest) || self.records.len() < 2_000_000,
            "BizHawk database index exceeds two million unique digests"
        );
        self.records.insert(record.digest.clone(), record);
        Ok(())
    }

    /// Mirrors GetGameInfo lookup priority. None means no match in THIS index;
    /// it is not permission to infer a system or accept a runtime capture.
    pub(crate) fn lookup(&self, input: &DatabaseInput) -> Option<DatabaseMatch<'_>> {
        if let Some(record) = self.records.get(input.normalized_sha1()) {
            return Some(DatabaseMatch {
                algorithm: LookupAlgorithm::Sha1,
                record,
            });
        }
        let md5 = format!("{:X}", md5::Md5::digest(input.bytes()));
        if let Some(record) = self.records.get(&md5) {
            return Some(DatabaseMatch {
                algorithm: LookupAlgorithm::Md5,
                record,
            });
        }
        let crc32 = format!("{:08X}", crc32fast::hash(input.bytes()));
        self.records.get(&crc32).map(|record| DatabaseMatch {
            algorithm: LookupAlgorithm::Crc32,
            record,
        })
    }
}

/// Owns the exact input to Database.GetGameInfo for the supported cartridge
/// file formats. Database resolution and subsequent patching are separate steps.
pub(crate) struct DatabaseInput {
    bytes: Vec<u8>,
    original_sha1: String,
    normalized_sha1: String,
    removed_header_bytes: usize,
    smd_deinterleaved: bool,
}

impl DatabaseInput {
    /// `extension` is HawkFile's normalized extension, including its leading
    /// dot. Never infer the selected core or runtime system from this value.
    /// The caller must retain and recheck the file artifact supplying `file`.
    pub(crate) fn from_cartridge(file: &[u8], extension: &str) -> Result<Self> {
        ensure!(
            !file.is_empty() && file.len() <= 512 * 1024 * 1024,
            "BizHawk cartridge identity input must be nonempty and at most 512 MiB"
        );
        ensure!(
            matches!(
                extension,
                ".nes"
                    | ".unf"
                    | ".fds"
                    | ".sfc"
                    | ".smc"
                    | ".bs"
                    | ".sms"
                    | ".gg"
                    | ".sg"
                    | ".gen"
                    | ".md"
                    | ".smd"
                    | ".pce"
                    | ".sgx"
                    | ".bin"
                    | ".rom"
            ),
            "Unsupported or non-normalized BizHawk cartridge extension: {extension}"
        );
        let original_sha1 = format!("{:X}", Sha1::digest(file));
        // RomGame applies these exceptions by hash regardless of extension.
        let header_exception = matches!(
            original_sha1.as_str(),
            "C4ABF77C2CFC0E7B590E2260C56360F9738C45D6" | "F91D4507BAF41626D839308659E68DE048C767C8"
        );
        let remainder = file.len() % 1024;
        let removed_header_bytes = if !header_exception && matches!(remainder, 128 | 512) {
            remainder
        } else {
            0
        };
        let rom = &file[removed_header_bytes..];
        let smd_deinterleaved = extension == ".smd";
        let bytes = if smd_deinterleaved {
            // Preserve upstream's 4 MiB cap and zero-filled incomplete tail.
            // Hashing only complete pages would produce a different identity.
            let size = rom.len().min(0x400000);
            let mut output = vec![0; size];
            for page in 0..size / 0x4000 {
                let start = page * 0x4000;
                for offset in 0..0x2000 {
                    output[start + offset * 2] = rom[start + 0x2000 + offset];
                    output[start + offset * 2 + 1] = rom[start + offset];
                }
            }
            output
        } else {
            rom.to_vec()
        };
        let normalized_sha1 = format!("{:X}", Sha1::digest(&bytes));
        Ok(Self {
            bytes,
            original_sha1,
            normalized_sha1,
            removed_header_bytes,
            smd_deinterleaved,
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn original_sha1(&self) -> &str {
        &self.original_sha1
    }

    /// SHA-1 lookup key, NOT necessarily GameInfo.Hash. The effective database
    /// can select an MD5/CRC32 entry; patches may then change core input bytes.
    pub(crate) fn normalized_sha1(&self) -> &str {
        &self.normalized_sha1
    }

    pub(crate) fn removed_header_bytes(&self) -> usize {
        self.removed_header_bytes
    }

    pub(crate) fn smd_deinterleaved(&self) -> bool {
        self.smd_deinterleaved
    }
}
