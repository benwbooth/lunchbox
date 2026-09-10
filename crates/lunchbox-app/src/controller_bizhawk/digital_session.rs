//! Native digital-core session ownership and routing checks.
use anyhow::{Result, ensure};

pub(crate) struct PlayerRequest<'a> {
    /// One-based native slot: compacted for Nintendo/GPGX, fixed for SMS/PCE.
    pub player: u8,
    pub calibration: &'a crate::controller_catalog::Calibration,
    pub logical: Option<&'a super::LogicalCalibration>,
    pub normalized_input: bool,
    pub runtime_path: &'a str,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DigitalDeck {
    Snes9x([super::snes9x::PadPort; 2]),
    NesHawk([super::neshawk::PadPort; 2]),
    SmsHawk(super::smshawk::System),
    PceHawk([bool; 5]),
    TurboNyma { ports: [bool; 5], multitap: bool },
    Gpgx(super::gpgx::Topology),
}

/// Independently derived expectations, not values copied from the response.
pub(crate) struct DefinitionContent<'a> {
    pub system: &'a str,
    pub rom_hash: &'a str,
    pub medium: DefinitionMedium,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DefinitionMedium {
    Cartridge,
    Discs,
    Nes(super::neshawk::ConsoleControls),
}

/// Private fields keep the validated request paired with its prepared files.
#[cfg(target_os = "linux")]
pub(crate) struct PreparedDeckCapture {
    deck: DigitalDeck,
    system: String,
    rom_hash: String,
    medium: DefinitionMedium,
    prepared: super::definition_capture::PreparedCapture,
    exe_directory: std::path::PathBuf,
    working_directory: std::path::PathBuf,
    content_path: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
pub(crate) struct CapturedDeckDefinition {
    definition: super::definition_capture::Definition,
    deck: DigitalDeck,
    medium: DefinitionMedium,
    configuration_sha256: [u8; 32],
    cartridge_identity: Option<super::cartridge_identity::CaptureIdentity>,
    runtime_artifacts: Vec<super::RuntimeArtifact>,
    program: std::path::PathBuf,
    assembly: Option<std::path::PathBuf>,
    working_directory: std::path::PathBuf,
    working_directory_target: std::path::PathBuf,
    environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    content_path: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
pub(crate) struct CartridgeHandoffRequest<'a> {
    pub program: &'a std::path::Path,
    pub assembly: Option<&'a std::path::Path>,
    pub working_directory: &'a std::path::Path,
    pub environment: &'a [(std::ffi::OsString, std::ffi::OsString)],
    /// Excludes the executable/Mono assembly prefix.
    pub arguments: &'a [std::ffi::OsString],
    pub deck: DigitalDeck,
    pub content: DefinitionContent<'a>,
}

/// Owns checked cartridge handoff inputs and the temporary config, not a running
/// emulator. Loaded-core/build policy remains the caller's responsibility.
#[cfg(target_os = "linux")]
pub(crate) struct PreparedCartridgeHandoff {
    captured: CapturedDeckDefinition,
    configuration: super::RuntimeArtifact,
    configuration_owner: super::PreparedConfig,
    arguments: Vec<std::ffi::OsString>,
}

#[cfg(target_os = "linux")]
impl PreparedCartridgeHandoff {
    pub(crate) fn verify(&self) -> Result<()> {
        self.configuration_owner.verify_source()?;
        self.configuration.verify()?;
        self.captured.verify_cartridge_identity()?;
        self.captured.verify_runtime_files()
    }

    pub(crate) fn application_arguments(&self) -> Result<&[std::ffi::OsString]> {
        self.verify()?;
        Ok(&self.arguments)
    }

    /// Recheck a final launch plan against the retained handoff, not only
    /// against the original observation's paths and environment.
    pub(crate) fn verify_request(&self, request: &CartridgeHandoffRequest<'_>) -> Result<()> {
        ensure!(
            request.arguments == self.arguments.as_slice(),
            "Launch arguments changed after handoff preparation"
        );
        self.captured.verify_invocation(
            request.program,
            request.assembly,
            request.working_directory,
            request.environment,
        )?;
        self.captured
            .verify_request(request.deck, &request.content)?;
        self.verify()
    }
}

#[cfg(target_os = "linux")]
impl CapturedDeckDefinition {
    pub(crate) fn prepare_cartridge_handoff(
        self,
        request: &CartridgeHandoffRequest<'_>,
        configuration_owner: super::PreparedConfig,
    ) -> Result<PreparedCartridgeHandoff> {
        use sha2::{Digest, Sha256};
        self.verify_invocation(
            request.program,
            request.assembly,
            request.working_directory,
            request.environment,
        )?;
        let path = self.handoff_config_path(request.arguments)?;
        ensure!(
            path == configuration_owner.path(),
            "Handoff arguments do not select the supplied owned configuration"
        );
        configuration_owner.verify_source()?;
        let configuration = super::RuntimeArtifact::capture(&path)?;
        let text = super::read_config(&path)?;
        let digest: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        ensure!(
            digest == configuration.sha256,
            "Handoff configuration changed while reading"
        );
        configuration.verify()?;
        self.verify_cartridge_handoff_configuration(&text, request.deck, &request.content)?;
        let handoff = PreparedCartridgeHandoff {
            captured: self,
            configuration,
            configuration_owner,
            arguments: request.arguments.to_vec(),
        };
        handoff.verify_request(request)?;
        Ok(handoff)
    }

    /// Validate application CLI separately from the executable/assembly prefix.
    /// The returned config still needs fingerprinting and handoff validation.
    pub(crate) fn handoff_config_path(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<std::path::PathBuf> {
        super::definition_capture::handoff_config_path(arguments, &self.content_path)
    }

    /// Exact explicit invocation comparison. This intentionally does not
    /// normalize aliases or reorder duplicate environment keys. Inherited
    /// environment and application arguments need separate launch ownership.
    pub(crate) fn verify_invocation(
        &self,
        program: &std::path::Path,
        assembly: Option<&std::path::Path>,
        working_directory: &std::path::Path,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<()> {
        ensure!(
            program == self.program
                && assembly == self.assembly.as_deref()
                && working_directory == self.working_directory
                && environment == self.environment.as_slice(),
            "Proposed runtime invocation differs from the captured setup"
        );
        self.verify_runtime_files()
    }

    /// Bind reuse to the original core/deck topology and medium, then repeat
    /// the full definition/content comparison against the proposed request.
    pub(crate) fn verify_request(
        &self,
        deck: DigitalDeck,
        content: &DefinitionContent<'_>,
    ) -> Result<()> {
        ensure!(
            deck == self.deck && content.medium == self.medium,
            "Captured controller deck or media mode differs from the proposed request"
        );
        deck.validate_definition(self.definition()?, content)
    }

    /// Exact capture configuration bytes, not a comparison to a normal launch
    /// config whose pause/input policies intentionally differ from capture.
    pub(crate) fn verify_capture_configuration(&self, configuration: &str) -> Result<()> {
        use sha2::{Digest, Sha256};
        ensure!(
            configuration.len() <= 16 * 1024 * 1024,
            "Capture config exceeds 16 MiB"
        );
        let digest: [u8; 32] = Sha256::digest(configuration.as_bytes()).into();
        ensure!(
            digest == self.configuration_sha256,
            "Configuration differs from the captured controller setup"
        );
        Ok(())
    }

    /// Compare the candidate after applying the SAME capture-only overrides
    /// used for the observed configuration. This does not validate actions
    /// suppressed by projection (autoload, tools, single-instance forwarding).
    /// Normal launch policy must check those separately before handoff.
    pub(crate) fn verify_capture_projection(
        &self,
        configuration: &str,
        deck: DigitalDeck,
        content: &DefinitionContent<'_>,
    ) -> Result<()> {
        self.verify_request(deck, content)?;
        let projected = super::definition_capture::encode_capture_config(configuration)?;
        self.verify_capture_configuration(&projected)?;
        deck.validate_capture_config(&projected, content)
    }

    /// Combined cartridge configuration gate. Callers still own executable,
    /// arguments/environment, loaded-core provenance and final launch lifetime.
    pub(crate) fn verify_cartridge_handoff_configuration(
        &self,
        configuration: &str,
        deck: DigitalDeck,
        content: &DefinitionContent<'_>,
    ) -> Result<()> {
        super::definition_capture::validate_handoff_startup(configuration)?;
        self.verify_cartridge_identity()?;
        self.verify_capture_projection(configuration, deck, content)
    }

    /// Read-only observation with any retained cartridge inputs rechecked.
    /// Disc observations still lack independent content provenance; reading
    /// this definition is not approval for a launch or a loaded-core identity.
    pub(crate) fn definition(&self) -> Result<&super::definition_capture::Definition> {
        self.verify_runtime_files()?;
        if let Some(identity) = &self.cartridge_identity {
            identity.verify()?;
        }
        Ok(&self.definition)
    }

    /// Recheck selected executable/assembly/Waterbox files, not all loader
    /// dependencies and not authentication of the actual loaded core.
    pub(crate) fn verify_runtime_files(&self) -> Result<()> {
        ensure!(
            self.working_directory.is_dir()
                && self.working_directory.canonicalize()? == self.working_directory_target,
            "Capture working directory changed before handoff"
        );
        for artifact in &self.runtime_artifacts {
            artifact.verify()?;
        }
        Ok(())
    }

    /// A launch handoff must explicitly require the evidence it needs. Do not
    /// treat an absent cartridge snapshot as a successfully verified snapshot.
    pub(crate) fn verify_cartridge_identity(&self) -> Result<()> {
        let Some(identity) = &self.cartridge_identity else {
            anyhow::bail!("Captured definition has no independent cartridge identity snapshot");
        };
        identity.verify()
    }
}

#[cfg(target_os = "linux")]
impl PreparedDeckCapture {
    /// Worker-thread operation. Runtime/core provenance is still the caller's
    /// responsibility; the result only matches the retained definition request.
    pub(crate) fn capture(
        mut self,
        program: &std::path::Path,
        assembly: Option<&std::path::Path>,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
        timeout: std::time::Duration,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<CapturedDeckDefinition> {
        ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Native definition capture cancelled before spawn"
        );
        ensure!(
            !timeout.is_zero() && timeout <= std::time::Duration::from_secs(60),
            "Capture timeout must be greater than zero and at most 60 seconds"
        );
        let content = DefinitionContent {
            system: &self.system,
            rom_hash: &self.rom_hash,
            medium: self.medium,
        };
        self.deck.validate_definition_request(&content)?;
        let working_directory_target = self.working_directory.canonicalize()?;
        let configuration = self.prepared.configuration()?;
        use sha2::{Digest, Sha256};
        let configuration_sha256: [u8; 32] = Sha256::digest(configuration.as_bytes()).into();
        self.deck
            .validate_capture_config(&configuration, &content)?;
        let mut runtime_artifacts = vec![super::RuntimeArtifact::capture(program)?];
        if let Some(assembly) = assembly {
            runtime_artifacts.push(super::RuntimeArtifact::capture(assembly)?);
        }
        let cartridge_identity = match self.medium {
            DefinitionMedium::Cartridge | DefinitionMedium::Nes(_) => {
                Some(super::cartridge_identity::CaptureIdentity::prepare(
                    &self.content_path,
                    &self.exe_directory,
                    environment,
                    &configuration,
                    content.system,
                    content.rom_hash,
                )?)
            }
            DefinitionMedium::Discs => None,
        };
        if let Some(artifact) = self.deck.waterbox_artifact(
            &self.exe_directory,
            &self.working_directory,
            environment,
        )? {
            runtime_artifacts.push(artifact);
        }
        for artifact in &runtime_artifacts {
            // Child preparation and the returned result share the same
            // baseline; do not recapture a new baseline after the child exits.
            self.prepared.retain_artifact(artifact.clone());
        }
        if let Some(identity) = &cartridge_identity {
            identity.verify()?;
        }
        ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Native definition capture cancelled during identity preparation"
        );
        let process = self.prepared.spawn(program, assembly, environment)?;
        let result = self
            .deck
            .capture_definition(process, &content, timeout, cancel);
        if let Some(identity) = &cartridge_identity {
            identity.verify()?;
        }
        for artifact in &runtime_artifacts {
            artifact.verify()?;
        }
        ensure!(
            self.working_directory.is_dir()
                && self.working_directory.canonicalize()? == working_directory_target,
            "Capture working directory changed during observation"
        );
        Ok(CapturedDeckDefinition {
            definition: result?,
            deck: self.deck,
            medium: self.medium,
            configuration_sha256,
            cartridge_identity,
            runtime_artifacts,
            program: program.to_owned(),
            assembly: assembly.map(std::path::Path::to_owned),
            working_directory: self.working_directory,
            working_directory_target,
            environment: environment.to_vec(),
            content_path: self.content_path,
        })
    }
}

impl DigitalDeck {
    pub(crate) fn validate_capture_config(
        self,
        source: &str,
        content: &DefinitionContent<'_>,
    ) -> Result<()> {
        self.validate_definition_request(content)?;
        match self {
            Self::Snes9x(ports) => super::snes9x::validate_capture_config(source, ports),
            Self::NesHawk(ports) => super::neshawk::validate_capture_config(source, ports),
            Self::SmsHawk(system) => super::smshawk::validate_capture_config(source, system),
            Self::PceHawk(ports) => {
                super::pcehawk::validate_capture_config(source, ports, content.system)
            }
            Self::TurboNyma { ports, multitap } => super::turbonyma::validate_capture_config(
                source,
                super::turbonyma::Topology { ports, multitap },
                content.system,
            ),
            Self::Gpgx(topology) => super::gpgx::validate_capture_config(source, topology),
        }
    }
    pub(crate) fn validate_definition_request(self, content: &DefinitionContent<'_>) -> Result<()> {
        ensure!(
            !content.rom_hash.is_empty()
                && content.rom_hash.len() <= 256
                && !content.rom_hash.chars().any(char::is_control),
            "Expected native content hash is missing or invalid"
        );
        match self {
            Self::PceHawk(ports) | Self::TurboNyma { ports, .. } => {
                self.validate_players(
                    ports
                        .into_iter()
                        .enumerate()
                        .filter(|(_, enabled)| *enabled)
                        .map(|(index, _)| index as u8 + 1),
                )?;
            }
            _ => self.validate_players(1..=self.players())?,
        }
        let matches = match (self, content.medium) {
            (Self::Snes9x(_), DefinitionMedium::Cartridge) => content.system == "SNES",
            (Self::NesHawk(_), DefinitionMedium::Nes(console)) => {
                ensure!(
                    console
                        .fds_sides
                        .is_none_or(|sides| (1..=256).contains(&sides)),
                    "FDS side count exceeds the capture contract"
                );
                content.system == "NES"
            }
            (Self::SmsHawk(system), DefinitionMedium::Cartridge) => {
                content.system
                    == match system {
                        super::smshawk::System::MasterSystem => "SMS",
                        super::smshawk::System::GameGear => "GG",
                        super::smshawk::System::Sg1000 => "SG",
                    }
            }
            (Self::PceHawk(_), DefinitionMedium::Discs) => content.system == "PCECD",
            (Self::PceHawk(_) | Self::TurboNyma { .. }, DefinitionMedium::Cartridge) => {
                matches!(content.system, "PCE" | "SGX")
            }
            (Self::TurboNyma { .. }, DefinitionMedium::Discs) => {
                matches!(content.system, "PCECD" | "SGXCD")
            }
            (Self::Gpgx(_), DefinitionMedium::Cartridge | DefinitionMedium::Discs) => {
                content.system == "GEN"
            }
            _ => false,
        };
        ensure!(
            matches,
            "Native deck, medium and expected runtime system disagree"
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn prepare_definition_capture(
        self,
        options: &[std::ffi::OsString],
        working_directory: &std::path::Path,
        exe_directory: &std::path::Path,
        content_path: &std::path::Path,
        content: &DefinitionContent<'_>,
    ) -> Result<PreparedDeckCapture> {
        self.validate_definition_request(content)?;
        let prepared = super::definition_capture::PreparedCapture::prepare(
            options,
            working_directory,
            exe_directory,
            content_path,
        )?;
        self.validate_capture_config(&prepared.configuration()?, content)?;
        Ok(PreparedDeckCapture {
            deck: self,
            system: content.system.to_owned(),
            rom_hash: content.rom_hash.to_owned(),
            medium: content.medium,
            prepared,
            exe_directory: exe_directory.to_owned(),
            working_directory: working_directory.to_owned(),
            content_path: content_path.to_owned(),
        })
    }

    pub(crate) fn validate_definition(
        self,
        definition: &super::definition_capture::Definition,
        content: &DefinitionContent<'_>,
    ) -> Result<()> {
        self.validate_definition_request(content)?;
        ensure!(
            definition.system == content.system,
            "Captured system differs from the independently requested system"
        );
        match (self, content.medium) {
            (Self::Snes9x(ports), DefinitionMedium::Cartridge) => {
                super::snes9x::validate_definition(definition, ports, content.rom_hash)
            }
            (Self::NesHawk(ports), DefinitionMedium::Nes(console)) => {
                super::neshawk::validate_definition(definition, ports, console, content.rom_hash)
            }
            (Self::SmsHawk(system), DefinitionMedium::Cartridge) => {
                super::smshawk::validate_definition(definition, system, content.rom_hash)
            }
            (
                Self::PceHawk(ports),
                medium @ (DefinitionMedium::Cartridge | DefinitionMedium::Discs),
            ) => {
                let discs = matches!(medium, DefinitionMedium::Discs);
                // PCEHawk's disc constructor reports PCECD even for an SGXCD
                // load request. This is expected runtime identity, not a label.
                ensure!(
                    if discs {
                        content.system == "PCECD"
                    } else {
                        matches!(content.system, "PCE" | "SGX")
                    },
                    "PCEHawk content medium and expected runtime system disagree"
                );
                super::pcehawk::validate_definition(
                    definition,
                    ports,
                    content.system,
                    content.rom_hash,
                )
            }
            (
                Self::TurboNyma { ports, multitap },
                medium @ (DefinitionMedium::Cartridge | DefinitionMedium::Discs),
            ) => super::turbonyma::validate_definition(
                definition,
                super::turbonyma::Topology { ports, multitap },
                content.system,
                content.rom_hash,
                matches!(medium, DefinitionMedium::Discs),
            ),
            (
                Self::Gpgx(topology),
                medium @ (DefinitionMedium::Cartridge | DefinitionMedium::Discs),
            ) => super::gpgx::validate_definition(
                definition,
                topology,
                content.rom_hash,
                matches!(medium, DefinitionMedium::Discs),
            ),
            _ => anyhow::bail!(
                "Content-definition expectations do not match the selected native deck"
            ),
        }
    }

    /// Shared worker-thread comparison after owned-child cleanup. A match is
    /// not loaded-core provenance or launch readiness.
    #[cfg(target_os = "linux")]
    pub(crate) fn capture_definition(
        self,
        process: super::definition_capture::CaptureProcess,
        content: &DefinitionContent<'_>,
        timeout: std::time::Duration,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<super::definition_capture::Definition> {
        self.validate_definition_request(content)?;
        let definition = process.wait_for_definition(timeout, cancel)?;
        self.validate_definition(&definition, content)?;
        ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Native digital definition comparison cancelled"
        );
        Ok(definition)
    }

    fn waterbox_filename(self) -> Option<&'static str> {
        match self {
            Self::Snes9x(_) => Some("snes9x.wbx"),
            Self::TurboNyma { .. } => Some("turbo.wbx"),
            Self::Gpgx(_) => Some("gpgx.wbx"),
            Self::NesHawk(_) | Self::SmsHawk(_) | Self::PceHawk(_) => None,
        }
    }
    fn waterbox_artifact(
        self,
        exe_directory: &std::path::Path,
        working_directory: &std::path::Path,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<Option<super::RuntimeArtifact>> {
        let Some(filename) = self.waterbox_filename() else {
            return Ok(None);
        };
        // PathUtils selects BIZHAWK_HOME/dll, not the native library path.
        let selected_home = environment
            .iter()
            .rev()
            .find(|(key, _)| key == std::ffi::OsStr::new("BIZHAWK_HOME"))
            .map(|(_, value)| value.clone())
            .or_else(|| std::env::var_os("BIZHAWK_HOME"));
        if let Some(home) = selected_home.filter(|value| !value.is_empty()) {
            let home = std::path::PathBuf::from(home);
            let home = if home.is_absolute() {
                home
            } else {
                working_directory.join(home)
            };
            ensure!(
                home.canonicalize()? == exe_directory.canonicalize()?,
                "Native BIZHAWK_HOME differs from the configured installation"
            );
        }
        Ok(Some(super::RuntimeArtifact::capture(
            &exe_directory.join("dll").join(filename),
        )?))
    }
    pub(crate) fn players(self) -> u8 {
        match self {
            Self::Snes9x(ports) => ports.iter().map(|port| port.players()).sum(),
            Self::NesHawk(ports) => ports.iter().map(|port| port.players()).sum(),
            Self::SmsHawk(system) => system.capacity(),
            // This is the slot-ID bound, not the count of connected ports.
            Self::PceHawk(_) => 5,
            Self::TurboNyma { .. } => 5,
            Self::Gpgx(topology) => topology.players(),
        }
    }
    pub(crate) fn layout_id(self, player: u8) -> Result<&'static str> {
        match self {
            Self::Snes9x(_) => Ok("snes"),
            Self::NesHawk(ports) => Ok(super::neshawk::player_port(ports, player)?.layout_id()),
            Self::SmsHawk(system) => {
                system.validate_players([player])?;
                Ok(system.layout_id())
            }
            Self::PceHawk(ports) => {
                ensure!(
                    (1..=5).contains(&player) && ports[usize::from(player - 1)],
                    "Invalid or disconnected PCEHawk player {player}"
                );
                Ok("pce-2")
            }
            Self::TurboNyma { ports, multitap } => {
                ensure!(
                    (1..=5).contains(&player)
                        && ports[usize::from(player - 1)]
                        && (player == 1 || multitap),
                    "Invalid, disconnected or hidden TurboNyma player {player}"
                );
                Ok("pce-turbonyma")
            }
            Self::Gpgx(topology) => topology.layout_id(player),
        }
    }
    pub(crate) fn validate_players(self, players: impl IntoIterator<Item = u8>) -> Result<()> {
        if let Self::SmsHawk(system) = self {
            return system.validate_players(players);
        }
        if let Self::PceHawk(ports) = self {
            return super::pcehawk::validate_players(ports, players);
        }
        if let Self::TurboNyma { ports, multitap } = self {
            return super::turbonyma::validate_players(ports, multitap, players);
        }
        if let Self::Gpgx(topology) = self {
            return topology.validate_players(players);
        }
        let count = self.players();
        let mut selected = std::collections::BTreeSet::new();
        for player in players {
            ensure!(
                (1..=count).contains(&player) && selected.insert(player),
                "Invalid or duplicate native digital player {player}"
            );
        }
        ensure!(
            count > 0 && !selected.is_empty(),
            "Select at least one configured native digital slot"
        );
        Ok(())
    }
}

/// Native session transaction; invoked by launch routing, never by previews.
#[cfg(target_os = "linux")]
pub(crate) fn prepare_session(
    plan: &mut crate::emulator::LaunchPlan,
    exe_directory: &std::path::Path,
    probe_program: &std::path::Path,
    sdl_library: &std::path::Path,
    deck: DigitalDeck,
    requests: &[PlayerRequest<'_>],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<crate::controller_launch::CalibratedLaunch> {
    prepare_session_inputs(
        plan,
        exe_directory,
        probe_program,
        sdl_library,
        deck,
        requests,
        cancel,
    )?
    .finish(plan)
}

/// Shared output of physical routing and native binding preparation. Keeping
/// these owners together permits capture between preparation and final commit.
#[cfg(target_os = "linux")]
pub(crate) struct PreparedDigitalSession {
    config: super::PreparedConfig,
    topology: crate::controller_bizhawk_guard::InputTopology,
    description: String,
    arguments: Vec<std::ffi::OsString>,
    deck: DigitalDeck,
}

#[cfg(target_os = "linux")]
pub(crate) struct SessionCaptureRequest<'a> {
    pub exe_directory: &'a std::path::Path,
    pub assembly: Option<&'a std::path::Path>,
    pub content_path: &'a std::path::Path,
    pub content: DefinitionContent<'a>,
    pub timeout: std::time::Duration,
}

#[cfg(target_os = "linux")]
impl PreparedDigitalSession {
    /// Explicit worker operation, never a preview. The caller must establish
    /// supported build/core policy before opting into this capture transaction.
    pub(crate) fn capture_and_finish(
        self,
        plan: &mut crate::emulator::LaunchPlan,
        request: SessionCaptureRequest<'_>,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<crate::controller_launch::CalibratedLaunch> {
        use std::sync::atomic::Ordering;
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Digital session capture cancelled"
        );
        ensure!(
            !matches!(request.content.medium, DefinitionMedium::Discs),
            "Disc capture handoff needs independent disc provenance"
        );
        self.topology.verify()?;
        self.config.verify_source()?;
        let arguments = self.application_arguments(request.assembly)?;
        let config_path =
            super::definition_capture::handoff_config_path(arguments, request.content_path)?;
        ensure!(
            config_path == self.config.path(),
            "Capture does not select the prepared controller config"
        );
        let configuration = super::read_config(&config_path)?;
        super::definition_capture::validate_handoff_startup(&configuration)?;
        let prepared = self.deck.prepare_definition_capture(
            &arguments[..arguments.len() - 1],
            &plan.current_directory,
            request.exe_directory,
            request.content_path,
            &request.content,
        )?;
        let captured = prepared.capture(
            &plan.program,
            request.assembly,
            &plan.environment,
            request.timeout,
            cancel,
        )?;
        self.topology.verify()?;
        self.config.verify_source()?;
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Digital session cancelled before captured handoff"
        );
        self.finish_with_capture(plan, captured, request.assembly, request.content)
    }

    fn application_arguments(
        &self,
        assembly: Option<&std::path::Path>,
    ) -> Result<&[std::ffi::OsString]> {
        if let Some(assembly) = assembly {
            ensure!(
                self.arguments
                    .first()
                    .is_some_and(|arg| arg == assembly.as_os_str()),
                "Prepared arguments do not begin with the selected Mono assembly"
            );
            Ok(&self.arguments[1..])
        } else {
            Ok(&self.arguments)
        }
    }

    pub(crate) fn configuration_path(&self) -> Result<std::path::PathBuf> {
        self.config.verify_source()?;
        Ok(self.config.path())
    }

    fn finish(
        self,
        plan: &mut crate::emulator::LaunchPlan,
    ) -> Result<crate::controller_launch::CalibratedLaunch> {
        let session = crate::controller_launch::CalibratedLaunch::from_bizhawk(
            self.config,
            self.topology,
            self.description,
        );
        session.check_launch_inputs()?;
        plan.arguments = self.arguments;
        Ok(session)
    }

    /// Caller captures this prepared configuration first and establishes its
    /// supported runtime/build policy. This function does not spawn capture.
    pub(crate) fn finish_with_capture(
        self,
        plan: &mut crate::emulator::LaunchPlan,
        captured: CapturedDeckDefinition,
        assembly: Option<&std::path::Path>,
        content: DefinitionContent<'_>,
    ) -> Result<crate::controller_launch::CalibratedLaunch> {
        // Own the argument slice before moving the config owner into handoff.
        let application_args = self.application_arguments(assembly)?.to_vec();
        let request = CartridgeHandoffRequest {
            program: &plan.program,
            assembly,
            working_directory: &plan.current_directory,
            environment: &plan.environment,
            arguments: &application_args,
            deck: self.deck,
            content,
        };
        let handoff = captured.prepare_cartridge_handoff(&request, self.config)?;
        let session = crate::controller_launch::CalibratedLaunch::from_bizhawk_capture(
            handoff,
            self.topology,
            self.description,
        )?;
        plan.arguments = self.arguments;
        Ok(session)
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn prepare_session_inputs(
    plan: &crate::emulator::LaunchPlan,
    exe_directory: &std::path::Path,
    probe_program: &std::path::Path,
    sdl_library: &std::path::Path,
    deck: DigitalDeck,
    requests: &[PlayerRequest<'_>],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<PreparedDigitalSession> {
    let name = match deck {
        DigitalDeck::Snes9x(_) => "Snes9x",
        DigitalDeck::NesHawk(_) => "NesHawk",
        DigitalDeck::SmsHawk(_) => "SMSHawk",
        DigitalDeck::PceHawk(_) => "PCEHawk",
        DigitalDeck::TurboNyma { .. } => "TurboNyma",
        DigitalDeck::Gpgx(_) => "GPGX",
    };
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "{name} preparation cancelled"
    );
    deck.validate_players(requests.iter().map(|request| request.player))?;
    super::verify_bundled_sdl(exe_directory, sdl_library)?;
    let assembly = exe_directory.join("EmuHawk.exe");
    let mut artifacts = [
        plan.program.as_path(),
        assembly.as_path(),
        sdl_library,
        probe_program,
    ]
    .into_iter()
    .map(super::RuntimeArtifact::capture)
    .collect::<Result<Vec<_>>>()?;
    if let Some(artifact) =
        deck.waterbox_artifact(exe_directory, &plan.current_directory, &plan.environment)?
    {
        artifacts.push(artifact);
    }
    let paths: Vec<_> = requests
        .iter()
        .map(|request| PathBuf::from(request.runtime_path))
        .collect();
    ensure!(
        paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == paths.len(),
        "{name} controllers must have distinct runtime paths"
    );
    let topology = crate::controller_bizhawk_guard::InputTopology::capture(&paths)?;
    let discovery = super::probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &[],
        cancel,
    )?;
    let runtime_paths = paths
        .iter()
        .map(|path| {
            topology.resolve_runtime_path(
                path,
                discovery
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let inventory = super::probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &runtime_paths,
        cancel,
    )?;
    topology.verify()?;
    super::ensure_discovery_routing(discovery, &inventory)?;
    // The launch owner has already created each normalized transport. Derive
    // recognized bindings from its actual SDL mapping, never a saved physical
    // map. Raw virtual joysticks use the same measured raw translator as other
    // raw devices; a session device need not have an SDL GameController mapping.
    let generated = requests
        .iter()
        .zip(&runtime_paths)
        .map(|(request, path)| {
            if request.normalized_input && inventory.device_at_path(path)?.is_game_controller {
                super::normalized_logical_calibration(request.calibration, &inventory, path)
                    .map(Some)
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let resolved: Vec<_> = requests
        .iter()
        .zip(&runtime_paths)
        .zip(&generated)
        .map(|((request, path), generated)| PlayerRequest {
            player: request.player,
            calibration: request.calibration,
            logical: if request.normalized_input {
                generated.as_ref()
            } else {
                request.logical
            },
            normalized_input: request.normalized_input,
            runtime_path: path,
        })
        .collect();
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "{name} preparation cancelled"
    );
    let mut arguments = plan.arguments.clone();
    let (mut config, warnings) = match deck {
        DigitalDeck::Snes9x(ports) => super::snes9x::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            ports,
            &resolved,
            &inventory,
        )?,
        DigitalDeck::NesHawk(ports) => super::neshawk::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            ports,
            &resolved,
            &inventory,
        )?,
        DigitalDeck::SmsHawk(system) => super::smshawk::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            system,
            &resolved,
            &inventory,
        )?,
        DigitalDeck::PceHawk(ports) => super::pcehawk::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            ports,
            &resolved,
            &inventory,
        )?,
        DigitalDeck::TurboNyma { ports, multitap } => super::turbonyma::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            ports,
            multitap,
            &resolved,
            &inventory,
        )?,
        DigitalDeck::Gpgx(topology) => super::gpgx::prepare_arguments(
            &mut arguments,
            &plan.current_directory,
            exe_directory,
            topology,
            &resolved,
            &inventory,
        )?,
    };
    config.runtime_artifacts = artifacts;
    let fresh = super::probe_runtime(
        probe_program,
        sdl_library,
        &plan.current_directory,
        &plan.environment,
        &runtime_paths,
        cancel,
    )?;
    inventory.ensure_same_routing(&fresh)?;
    super::verify_bundled_sdl(exe_directory, sdl_library)?;
    topology.verify()?;
    config.verify_source()?;
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "{name} preparation cancelled"
    );
    let mut description = format!(
        "Prepared {} controller(s) for native BizHawk/{name}",
        requests.len()
    );
    if !warnings.is_empty() {
        description.push_str(&format!(" · {}", warnings.join(" · ")));
    }
    Ok(PreparedDigitalSession {
        config,
        topology,
        description,
        arguments,
        deck,
    })
}
