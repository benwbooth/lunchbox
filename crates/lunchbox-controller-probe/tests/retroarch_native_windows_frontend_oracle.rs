#[cfg(target_os = "windows")]
mod windows_oracle {
    use anyhow::{Context, Result, bail, ensure};
    use lunchbox_controller_probe::file_hash;
    use lunchbox_controller_probe::retroarch_frontend_autoconfig::{
        FrontendAutoconfigProfileSpec, FrontendAutoconfigSession, NESTOPIA_NES_FOUR_PLAYER_PROFILE,
        NESTOPIA_NES_TWO_PLAYER_PROFILE, NativeRetroArchPathRequest, WINDOWS_NESTOPIA_SHA256,
        WINDOWS_RETROARCH_1_19_1_DISTRIBUTION, WINDOWS_RETROARCH_1_19_1_SHA256,
        prepare_pinned_frontend_autoconfig_session,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::OsString;
    use std::io::ErrorKind;
    use std::net::UdpSocket;
    use std::path::{Component, Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::time::{Duration, Instant};

    const SOURCE_ENV: &str = "LUNCHBOX_RETROARCH_WINDOWS_ORACLE_SOURCE";
    const RUN_ROOT_ENV: &str = "LUNCHBOX_RETROARCH_WINDOWS_ORACLE_RUN_ROOT";
    const REPORT_SCHEMA: u32 = 2;
    const COMMAND_PORT: u16 = 55355;
    const DISTRIBUTION_MANIFEST_BYTES: u64 = 9_323;
    const DISTRIBUTION_MANIFEST_SHA256: &str =
        "a4b5dcede243a70b9b6583f76d73d730b51fcd10d7b3adfbaad784c2067ebef0";
    const FRONTEND_BYTES: u64 = 16_128_258;
    const CORE_BYTES: u64 = 6_630_377;
    const CORE_INFO_BYTES: u64 = 1_394;
    const CORE_INFO_SHA256: &str =
        "3aedf30570ef2f751eee31d06022243ec5ae13205b1ab17b880a077651c27e87";
    const CONTROLLER_ROM_BYTES: u64 = 24_592;
    const CONTROLLER_ROM_SHA256: &str =
        "0aa0053c7932bf5ba3db562229310f01e9e0b4697db8389e515a37d599e95f19";
    const PERSISTENCE_ROM_BYTES: u64 = 24_592;
    const PERSISTENCE_ROM_SHA256: &str =
        "0ae807dfe80d7178f082e0d6297e94b145d2880ff5a974fa5c1b26f6bf1beaaa";
    const FIRST_SAVE_SHA256: &str =
        "35f57192c251084b91b1a5fbfda019c4294db3266df377689c96a5769317745e";
    const SECOND_SAVE_SHA256: &str =
        "de5232a4b6205e474f7c169fda3dc0f1a22f3edd770c798340f7ee4571d2e124";
    const STATE_CORE_BYTES: usize = 21_781;
    const STATE_FILE_BYTES: u64 = 21_808;
    const STATE_MARKER: &[u8; 8] = b"LBSTATE1";
    const MUTATED_MARKER: &[u8; 8] = b"MUTATED!";
    const FIRST_SAVE_MARKER: &[u8; 5] = b"LBSR\x01";
    const SECOND_SAVE_MARKER: &[u8; 5] = b"LBSR\x02";
    const SOURCE_OPTIONS: &[u8] = b"unrelated_oracle_option = \"preserved\"\n\
nestopia_button_shift = \"enabled\"\n\
nestopia_select_adapter = \"ntsc\"\n";

    const TEST_INPUTS: &[TestInput] = &[
        TestInput {
            name: "baseline",
            file: "test-input-baseline.ratst",
            contents: "[\n  { \"frame\": 0, \"action\": 1, \"param_num\": 0, \"param_str\": \"(045e:028e) Synthetic oracle pad 1\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 1, \"param_str\": \"(045e:028e) Synthetic oracle pad 2\" }\n]\n",
            bytes: 199,
            sha256: "8d92a1ddbe06f586724c89a6b2373b8c2297babae593c17457487ba87b8b8a47",
            player1: 0xff,
            player2: 0xff,
        },
        TestInput {
            name: "player1-a",
            file: "test-input-player1-a.ratst",
            contents: "[\n  { \"frame\": 0, \"action\": 1, \"param_num\": 0, \"param_str\": \"(045e:028e) Synthetic oracle pad 1\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 1, \"param_str\": \"(045e:028e) Synthetic oracle pad 2\" },\n  { \"frame\": 0, \"action\": 16, \"param_num\": 256 }\n]\n",
            bytes: 249,
            sha256: "647be90b570ddf5e8420129bfaa57621432e3d7dc7e0f856d95d576b39d0a56c",
            player1: 0xfe,
            player2: 0xff,
        },
        TestInput {
            name: "player2-b",
            file: "test-input-player2-b.ratst",
            contents: "[\n  { \"frame\": 0, \"action\": 1, \"param_num\": 0, \"param_str\": \"(045e:028e) Synthetic oracle pad 1\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 1, \"param_str\": \"(045e:028e) Synthetic oracle pad 2\" },\n  { \"frame\": 0, \"action\": 17, \"param_num\": 1 }\n]\n",
            bytes: 247,
            sha256: "b9ab7ddd4c0153e9a1ebb2a6f239be719d51fd4635d7d8812dd188880ae52b27",
            player1: 0xff,
            player2: 0xfd,
        },
        TestInput {
            name: "both",
            file: "test-input-both.ratst",
            contents: "[\n  { \"frame\": 0, \"action\": 1, \"param_num\": 0, \"param_str\": \"(045e:028e) Synthetic oracle pad 1\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 1, \"param_str\": \"(045e:028e) Synthetic oracle pad 2\" },\n  { \"frame\": 0, \"action\": 16, \"param_num\": 256 },\n  { \"frame\": 0, \"action\": 17, \"param_num\": 1 }\n]\n",
            bytes: 297,
            sha256: "2c93c98f8edfa7075d809ce24005a1b89a616860f103dce7bdc4c5d6b3cbfdad",
            player1: 0xfe,
            player2: 0xfd,
        },
    ];

    const FOUR_PLAYER_TEST_INPUT: TestInput = TestInput {
        name: "four-score-all-players",
        file: "test-input-four-score-all-players.ratst",
        contents: "[\n  { \"frame\": 0, \"action\": 1, \"param_num\": 0, \"param_str\": \"(045e:028e) Synthetic oracle pad 1\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 1, \"param_str\": \"(045e:028e) Synthetic oracle pad 2\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 2, \"param_str\": \"(045e:028e) Synthetic oracle pad 3\" },\n  { \"frame\": 0, \"action\": 1, \"param_num\": 3, \"param_str\": \"(045e:028e) Synthetic oracle pad 4\" },\n  { \"frame\": 0, \"action\": 16, \"param_num\": 256 },\n  { \"frame\": 0, \"action\": 17, \"param_num\": 1 },\n  { \"frame\": 0, \"action\": 18, \"param_num\": 256 },\n  { \"frame\": 0, \"action\": 19, \"param_num\": 1 }\n]\n",
        bytes: 591,
        sha256: "1c8fb2811d8dc41e37f3f5f0b36f3646b91842bf00e66f29ec0181e304c8d572",
        player1: 0xfe,
        player2: 0xfd,
    };

    struct TestInput {
        name: &'static str,
        file: &'static str,
        contents: &'static str,
        bytes: u64,
        sha256: &'static str,
        player1: u8,
        player2: u8,
    }

    #[derive(Debug, Deserialize)]
    struct DistributionManifest {
        schema_version: u32,
        file_count: usize,
        files: Vec<DistributionFile>,
    }

    #[derive(Debug, Deserialize)]
    struct DistributionFile {
        path: String,
        bytes: u64,
        sha256: String,
    }

    #[derive(Debug, Serialize)]
    struct OracleReport {
        schema_version: u32,
        status: &'static str,
        host: &'static str,
        frontend: ArtifactIdentity,
        core: ArtifactIdentity,
        content: Vec<ArtifactIdentity>,
        production_contract: ProductionContractReport,
        deterministic_input: Vec<InputObservation>,
        four_player_input: FourPlayerInputObservation,
        persistence: PersistenceReport,
        negative_drift_cases: Vec<&'static str>,
        cleanup: CleanupReport,
    }

    #[derive(Debug, Serialize)]
    struct FailureReport<'a> {
        schema_version: u32,
        status: &'static str,
        host: &'static str,
        error: &'a str,
    }

    #[derive(Debug, Serialize)]
    struct ArtifactIdentity {
        name: &'static str,
        version: &'static str,
        path: PathBuf,
        bytes: u64,
        sha256: String,
    }

    #[derive(Debug, Serialize)]
    struct ProductionContractReport {
        two_player_profile: &'static str,
        four_player_profile: &'static str,
        argv_prefix: [&'static str; 2],
        source_option: &'static str,
        private_option: &'static str,
        preserved_adapter_option: &'static str,
        two_player_private_port_modes: [u32; 4],
        four_player_private_port_modes: [u32; 4],
        prepared_environment_empty: bool,
        source_configs_unchanged: bool,
        private_sessions_removed: usize,
    }

    #[derive(Debug, Serialize)]
    struct InputObservation {
        case: &'static str,
        player1: String,
        player2: String,
        status: String,
        log: PathBuf,
    }

    #[derive(Debug, Serialize)]
    struct FourPlayerInputObservation {
        case: &'static str,
        profile: &'static str,
        player1: String,
        player2: String,
        player3: String,
        player4: String,
        source_adapter_option: &'static str,
        private_adapter_option: &'static str,
        status: String,
        log: PathBuf,
    }

    #[derive(Debug, Serialize)]
    struct PersistenceReport {
        first_save: FileEvidence,
        fresh_process_save: FileEvidence,
        state: StateEvidence,
        same_process_restored: String,
        fresh_process_before: String,
        fresh_process_restored: String,
    }

    #[derive(Debug, Serialize)]
    struct FileEvidence {
        path: PathBuf,
        bytes: u64,
        sha256: String,
    }

    #[derive(Debug, Serialize)]
    struct StateEvidence {
        path: PathBuf,
        bytes: u64,
        sha256: String,
        format: &'static str,
        format_version: u8,
        core_payload_bytes: usize,
        core_payload_sha256: String,
        alignment_padding_bytes: usize,
        terminator: &'static str,
    }

    #[derive(Debug, Serialize)]
    struct CleanupReport {
        all_children_reaped: bool,
        all_private_sessions_removed: bool,
        retained_run_root: PathBuf,
    }

    struct RuntimePayload {
        executable: PathBuf,
        core: PathBuf,
        core_info: PathBuf,
        controller_rom: PathBuf,
        persistence_rom: PathBuf,
    }

    struct OwnedChild {
        child: Child,
    }

    impl OwnedChild {
        fn spawn(
            executable: &Path,
            arguments: &[OsString],
            working_directory: &Path,
        ) -> Result<Self> {
            let child = Command::new(executable)
                .args(arguments)
                .current_dir(working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .with_context(|| format!("Starting {}", executable.display()))?;
            Ok(Self { child })
        }

        fn ensure_running(&mut self) -> Result<()> {
            ensure!(
                self.child.try_wait()?.is_none(),
                "RetroArch exited before completing an oracle assertion"
            );
            Ok(())
        }

        fn stop_gracefully(&mut self) -> Result<ExitStatus> {
            if let Some(status) = self.child.try_wait()? {
                ensure!(status.success(), "RetroArch exited with {status}");
                return Ok(status);
            }
            send_command("QUIT", false)?;
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                if let Some(status) = self.child.try_wait()? {
                    ensure!(status.success(), "RetroArch exited with {status}");
                    return Ok(status);
                }
                ensure!(
                    Instant::now() < deadline,
                    "RetroArch did not exit after the QUIT command"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    struct PreparedProcess {
        child: OwnedChild,
        session: FrontendAutoconfigSession,
        session_root: PathBuf,
        source_main: Vec<u8>,
        source_options: Vec<u8>,
        main_config: PathBuf,
        options_config: PathBuf,
    }

    impl PreparedProcess {
        fn verify_sources(&self) -> Result<()> {
            ensure!(
                std::fs::read(&self.main_config)? == self.source_main,
                "RetroArch changed the user's selected main configuration"
            );
            ensure!(
                std::fs::read(&self.options_config)? == self.source_options,
                "RetroArch changed the user's selected core-options file"
            );
            Ok(())
        }

        fn stop_and_cleanup(mut self, removed_roots: &mut Vec<PathBuf>) -> Result<()> {
            self.child.stop_gracefully()?;
            self.verify_sources()?;
            verify_private_runtime_writes(&self.session)?;
            let root = self.session_root.clone();
            drop(self.session);
            ensure!(
                !root.exists(),
                "Private frontend_autoconfig session remained after teardown: {}",
                root.display()
            );
            removed_roots.push(root);
            Ok(())
        }
    }

    fn verify_private_runtime_writes(session: &FrontendAutoconfigSession) -> Result<()> {
        ensure!(
            std::fs::read_to_string(&session.artifacts.append_config_path)?
                == session.artifacts.append_config,
            "RetroArch changed the private append configuration"
        );
        ensure!(
            std::fs::read_to_string(&session.artifacts.core_options_path)?
                == session.artifacts.core_options,
            "RetroArch changed the private source core-options snapshot"
        );
        let allowed = BTreeSet::from([
            session.artifacts.append_config_path.clone(),
            session.artifacts.core_options_path.clone(),
            session.root().join("config/Nestopia/Nestopia.opt"),
        ]);
        let files = regular_files(session.root())?;
        ensure!(
            files.iter().all(|path| allowed.contains(path)),
            "RetroArch wrote an unexpected file inside the private frontend_autoconfig root"
        );
        if let Some(options) = files
            .iter()
            .find(|path| path.ends_with("config/Nestopia/Nestopia.opt"))
        {
            let text = std::fs::read_to_string(options)?;
            ensure!(
                text.contains("nestopia_button_shift = \"disabled\"")
                    && !text.contains("nestopia_button_shift = \"enabled\"")
                    && text.contains("nestopia_select_adapter = \"ntsc\""),
                "RetroArch did not persist the private button-shift and adapter options"
            );
        }
        Ok(())
    }

    #[test]
    #[ignore = "requires the pinned official RetroArch 1.19.1 payload and a native Windows VM"]
    fn production_frontend_autoconfig_reaches_windows_retroarch_and_persists() {
        let source = required_absolute_directory(SOURCE_ENV).unwrap();
        let run_root = required_fresh_run_root(RUN_ROOT_ENV).unwrap();
        let report_path = run_root.join("retroarch-native-windows-report.json");
        match run_oracle(&source, &run_root) {
            Ok(report) => {
                write_json(&report_path, &report).unwrap();
                println!("PASS: native Windows RetroArch production frontend_autoconfig oracle");
                println!("Report: {}", report_path.display());
            }
            Err(error) => {
                let message = format!("{error:#}");
                let failure = FailureReport {
                    schema_version: REPORT_SCHEMA,
                    status: "fail",
                    host: "Windows x86_64 native process",
                    error: &message,
                };
                let _ = write_json(&report_path, &failure);
                panic!("{message}; report: {}", report_path.display());
            }
        }
    }

    fn run_oracle(source: &Path, run_root: &Path) -> Result<OracleReport> {
        let payload = stage_verified_payload(source, run_root)?;
        let mut removed_roots = Vec::new();
        let deterministic_input =
            run_deterministic_input_cases(&payload, run_root, &mut removed_roots)?;
        let four_player_input = run_four_player_input_case(&payload, run_root, &mut removed_roots)?;
        let persistence = run_persistence_cases(&payload, run_root, &mut removed_roots)?;
        let negative_drift_cases =
            run_negative_drift_cases(&payload, run_root, &mut removed_roots)?;

        let baseline_options = run_root.join("selected-retroarch-core-options.cfg");
        ensure!(
            std::fs::read(&baseline_options)? == SOURCE_OPTIONS,
            "Selected source core-options bytes changed"
        );
        ensure!(
            removed_roots.iter().all(|path| !path.exists()),
            "At least one private frontend_autoconfig session survived teardown"
        );

        Ok(OracleReport {
            schema_version: REPORT_SCHEMA,
            status: "pass",
            host: "Windows x86_64 native process",
            frontend: artifact_identity(
                "RetroArch",
                "1.19.1",
                &payload.executable,
                FRONTEND_BYTES,
                WINDOWS_RETROARCH_1_19_1_SHA256,
            )?,
            core: artifact_identity(
                "Nestopia",
                "1.99.0 5265136",
                &payload.core,
                CORE_BYTES,
                WINDOWS_NESTOPIA_SHA256,
            )?,
            content: vec![
                artifact_identity(
                    "controller-diagnostic",
                    "original NROM oracle",
                    &payload.controller_rom,
                    CONTROLLER_ROM_BYTES,
                    CONTROLLER_ROM_SHA256,
                )?,
                artifact_identity(
                    "persistence-diagnostic",
                    "original battery-backed NROM oracle",
                    &payload.persistence_rom,
                    PERSISTENCE_ROM_BYTES,
                    PERSISTENCE_ROM_SHA256,
                )?,
            ],
            production_contract: ProductionContractReport {
                two_player_profile: NESTOPIA_NES_TWO_PLAYER_PROFILE,
                four_player_profile: NESTOPIA_NES_FOUR_PLAYER_PROFILE,
                argv_prefix: ["--appendconfig", "--device=PORT:ID"],
                source_option: "nestopia_button_shift = enabled",
                private_option: "nestopia_button_shift = disabled",
                preserved_adapter_option: "nestopia_select_adapter = ntsc",
                two_player_private_port_modes: [257, 257, 0, 0],
                four_player_private_port_modes: [257, 257, 257, 257],
                prepared_environment_empty: true,
                source_configs_unchanged: true,
                private_sessions_removed: removed_roots.len(),
            },
            deterministic_input,
            four_player_input,
            persistence,
            negative_drift_cases,
            cleanup: CleanupReport {
                all_children_reaped: true,
                all_private_sessions_removed: true,
                retained_run_root: run_root.to_path_buf(),
            },
        })
    }

    fn stage_verified_payload(source: &Path, run_root: &Path) -> Result<RuntimePayload> {
        let source_payload = source.join("payload");
        let source_manifest = source_payload.join("retroarch-dist-manifest.json");
        assert_file(
            &source_manifest,
            DISTRIBUTION_MANIFEST_BYTES,
            DISTRIBUTION_MANIFEST_SHA256,
        )?;
        let manifest: DistributionManifest =
            serde_json::from_slice(&std::fs::read(&source_manifest)?)?;
        ensure!(
            manifest.schema_version == 1,
            "Unsupported distribution manifest"
        );
        ensure!(
            manifest.file_count == 59 && manifest.files.len() == 59,
            "Distribution manifest does not contain exactly 59 files"
        );
        ensure!(
            manifest.files.len() == WINDOWS_RETROARCH_1_19_1_DISTRIBUTION.len()
                && manifest
                    .files
                    .iter()
                    .zip(WINDOWS_RETROARCH_1_19_1_DISTRIBUTION)
                    .all(|(actual, expected)| {
                        actual.path == expected.relative_path
                            && actual.bytes == expected.bytes
                            && actual.sha256 == expected.sha256
                    }),
            "External distribution manifest differs from the production 59-file audit closure"
        );
        let source_distribution = source_payload.join("retroarch-dist");
        ensure!(
            regular_files(&source_distribution)?.len() == 59,
            "Source RetroArch runtime closure does not contain exactly 59 files"
        );
        let destination_payload = run_root.join("payload");
        let destination_distribution = destination_payload.join("retroarch-dist");
        std::fs::create_dir(&destination_payload)?;
        std::fs::create_dir(&destination_distribution)?;
        for entry in &manifest.files {
            let relative = checked_relative_path(&entry.path)?;
            let source_file = source_distribution.join(&relative);
            let destination = destination_distribution.join(&relative);
            copy_verified_file(&source_file, &destination, entry.bytes, &entry.sha256)?;
        }
        ensure!(
            regular_files(&destination_distribution)?.len() == 59,
            "Copied RetroArch runtime closure does not contain exactly 59 files"
        );

        let core = destination_payload.join("nestopia_libretro.dll");
        copy_verified_file(
            &source_payload.join("nestopia_libretro.dll"),
            &core,
            CORE_BYTES,
            WINDOWS_NESTOPIA_SHA256,
        )?;
        let core_info = destination_payload.join("info/nestopia_libretro.info");
        copy_verified_file(
            &source_payload.join("info/nestopia_libretro.info"),
            &core_info,
            CORE_INFO_BYTES,
            CORE_INFO_SHA256,
        )?;
        let controller_rom = destination_payload.join("controller-diagnostic.nes");
        let controller_bytes = lunchbox_controller_probe::libretro_input::nes_diagnostic_rom();
        ensure!(
            controller_bytes.len() as u64 == CONTROLLER_ROM_BYTES,
            "Generated controller diagnostic has the wrong size"
        );
        std::fs::write(&controller_rom, controller_bytes)?;
        assert_file(&controller_rom, CONTROLLER_ROM_BYTES, CONTROLLER_ROM_SHA256)?;
        let persistence_rom = destination_payload.join("persistence-diagnostic.nes");
        let persistence_bytes =
            lunchbox_controller_probe::libretro_persistence::nes_persistence_rom();
        ensure!(
            persistence_bytes.len() as u64 == PERSISTENCE_ROM_BYTES,
            "Generated persistence diagnostic has the wrong size"
        );
        std::fs::write(&persistence_rom, persistence_bytes)?;
        assert_file(
            &persistence_rom,
            PERSISTENCE_ROM_BYTES,
            PERSISTENCE_ROM_SHA256,
        )?;
        let executable = destination_distribution.join("retroarch.exe");
        assert_file(&executable, FRONTEND_BYTES, WINDOWS_RETROARCH_1_19_1_SHA256)?;
        Ok(RuntimePayload {
            executable,
            core,
            core_info,
            controller_rom,
            persistence_rom,
        })
    }

    fn run_negative_drift_cases(
        payload: &RuntimePayload,
        run_root: &Path,
        removed_roots: &mut Vec<PathBuf>,
    ) -> Result<Vec<&'static str>> {
        let case_root = run_root.join("negative-drift");
        std::fs::create_dir(&case_root)?;
        let log = case_root.join("retroarch.log");
        let options = seed_source_options(run_root)?;
        let config = write_main_config(payload, &case_root, &log, &options, None)?;
        let config_before = std::fs::read(&config)?;
        let arguments = production_arguments(&payload.core, &payload.controller_rom);
        let request = path_request(payload, &case_root);

        let wrong_frontend = prepare_pinned_frontend_autoconfig_session(
            &request,
            &payload.core,
            &"0".repeat(64),
            WINDOWS_NESTOPIA_SHA256,
            &profile(),
            &payload.controller_rom,
            &arguments,
            &[],
            &case_root.join("wrong-frontend-cache"),
        );
        ensure!(
            wrong_frontend.is_err(),
            "Frontend digest drift was accepted"
        );

        let wrong_core = prepare_pinned_frontend_autoconfig_session(
            &request,
            &payload.core,
            WINDOWS_RETROARCH_1_19_1_SHA256,
            &"0".repeat(64),
            &profile(),
            &payload.controller_rom,
            &arguments,
            &[],
            &case_root.join("wrong-core-cache"),
        );
        ensure!(wrong_core.is_err(), "Core digest drift was accepted");

        let injected_loadable = payload
            .executable
            .parent()
            .context("Staged RetroArch executable has no parent")?
            .join("winmm.dll");
        std::fs::write(&injected_loadable, b"MZ injected oracle fixture")?;
        let injected_at_prepare = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("injected-loadable-prepare-cache"),
        );
        std::fs::remove_file(&injected_loadable)?;
        ensure!(
            injected_at_prepare.is_err(),
            "Unexpected runtime DLL was accepted during preparation"
        );

        let runtime_file = payload
            .executable
            .parent()
            .context("Staged RetroArch executable has no parent")?
            .join("libssp-0.dll");
        let runtime_original = std::fs::read(&runtime_file)?;
        let (session, prepared) = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("runtime-closure-cache"),
        )?;
        assert_prepared_contract(&session, &arguments, &prepared, &options)?;
        let session_root = session.root().to_path_buf();
        let mut changed_runtime = runtime_original.clone();
        changed_runtime[0] ^= 0xff;
        let runtime_permissions = std::fs::metadata(&runtime_file)?.permissions();
        let mut writable_runtime_permissions = runtime_permissions.clone();
        writable_runtime_permissions.set_readonly(false);
        std::fs::set_permissions(&runtime_file, writable_runtime_permissions)?;
        std::fs::write(&runtime_file, changed_runtime)?;
        let runtime_drift_rejected = session
            .verify_launch_identity(
                &payload.executable,
                &payload.core,
                &payload.controller_rom,
                &prepared,
                &[],
            )
            .is_err();
        std::fs::write(&runtime_file, &runtime_original)?;
        std::fs::set_permissions(&runtime_file, runtime_permissions)?;
        ensure!(
            runtime_drift_rejected,
            "RetroArch runtime-closure drift was accepted"
        );
        std::fs::write(&injected_loadable, b"MZ injected oracle fixture")?;
        let injected_after_prepare = session
            .verify_launch_identity(
                &payload.executable,
                &payload.core,
                &payload.controller_rom,
                &prepared,
                &[],
            )
            .is_err();
        std::fs::remove_file(&injected_loadable)?;
        ensure!(
            injected_after_prepare,
            "Unexpected runtime DLL was accepted after preparation"
        );
        session.verify_launch_identity(
            &payload.executable,
            &payload.core,
            &payload.controller_rom,
            &prepared,
            &[],
        )?;
        drop(session);
        ensure!(
            !session_root.exists(),
            "Runtime-closure session survived teardown"
        );
        removed_roots.push(session_root);

        let content_copy = case_root.join("content-drift.nes");
        copy_verified_file(
            &payload.controller_rom,
            &content_copy,
            CONTROLLER_ROM_BYTES,
            CONTROLLER_ROM_SHA256,
        )?;
        let content_arguments = production_arguments(&payload.core, &content_copy);
        let (session, prepared) = prepare_session(
            &request,
            payload,
            &content_copy,
            &content_arguments,
            &case_root.join("content-cache"),
        )?;
        assert_prepared_contract(&session, &content_arguments, &prepared, &options)?;
        let session_root = session.root().to_path_buf();
        let mut content_bytes = std::fs::read(&content_copy)?;
        content_bytes[0] ^= 0xff;
        std::fs::write(&content_copy, &content_bytes)?;
        ensure!(session.verify().is_err(), "Content drift was accepted");
        drop(session);
        ensure!(
            !session_root.exists(),
            "Content-drift session survived teardown"
        );
        removed_roots.push(session_root);

        let (session, prepared) = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("argv-cache"),
        )?;
        assert_prepared_contract(&session, &arguments, &prepared, &options)?;
        let mut changed_arguments = prepared.clone();
        changed_arguments.push(OsString::from("--fullscreen"));
        ensure!(
            session
                .verify_launch_identity(
                    &payload.executable,
                    &payload.core,
                    &payload.controller_rom,
                    &changed_arguments,
                    &[],
                )
                .is_err(),
            "Prepared argv drift was accepted"
        );
        let session_root = session.root().to_path_buf();
        drop(session);
        ensure!(
            !session_root.exists(),
            "Argument-drift session survived teardown"
        );
        removed_roots.push(session_root);

        let (session, prepared) = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("environment-cache"),
        )?;
        assert_prepared_contract(&session, &arguments, &prepared, &options)?;
        for environment in [
            vec![(
                OsString::from("HOME"),
                case_root.join("late-home").into_os_string(),
            )],
            vec![(
                OsString::from("APPDATA"),
                case_root.join("late-appdata").into_os_string(),
            )],
            vec![(OsString::from("ORACLE_CUSTOM_ENV"), OsString::from("1"))],
        ] {
            ensure!(
                session
                    .verify_launch_identity(
                        &payload.executable,
                        &payload.core,
                        &payload.controller_rom,
                        &prepared,
                        &environment,
                    )
                    .is_err(),
                "Late launch-environment drift was accepted"
            );
        }
        session.verify_launch_identity(
            &payload.executable,
            &payload.core,
            &payload.controller_rom,
            &prepared,
            &[],
        )?;
        let session_root = session.root().to_path_buf();
        drop(session);
        ensure!(
            !session_root.exists(),
            "Environment-drift session survived teardown"
        );
        removed_roots.push(session_root);

        let (session, prepared) = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("source-config-cache"),
        )?;
        assert_prepared_contract(&session, &arguments, &prepared, &options)?;
        let session_root = session.root().to_path_buf();
        let mut changed_config = config_before.clone();
        changed_config.extend_from_slice(b"# source drift\n");
        std::fs::write(&config, &changed_config)?;
        ensure!(
            session.verify().is_err(),
            "Source-config drift was accepted"
        );
        drop(session);
        ensure!(
            !session_root.exists(),
            "Source-config session survived teardown"
        );
        removed_roots.push(session_root);
        std::fs::write(&config, &config_before)?;

        let (session, prepared) = prepare_session(
            &request,
            payload,
            &payload.controller_rom,
            &arguments,
            &case_root.join("private-config-cache"),
        )?;
        assert_prepared_contract(&session, &arguments, &prepared, &options)?;
        let session_root = session.root().to_path_buf();
        let mut private = std::fs::read(&session.artifacts.append_config_path)?;
        private.extend_from_slice(b"# private drift\n");
        std::fs::write(&session.artifacts.append_config_path, private)?;
        ensure!(
            session.verify().is_err(),
            "Private-config drift was accepted"
        );
        drop(session);
        ensure!(
            !session_root.exists(),
            "Private-config session survived teardown"
        );
        removed_roots.push(session_root);

        ensure!(
            std::fs::read(&config)? == config_before,
            "Negative drift tests did not restore the selected main config"
        );
        ensure!(
            std::fs::read(&options)? == SOURCE_OPTIONS,
            "Negative drift tests changed the selected options"
        );
        Ok(vec![
            "frontend-sha256",
            "core-sha256",
            "unexpected-runtime-dll-at-prepare",
            "unexpected-runtime-dll-after-prepare",
            "runtime-closure-bytes-after-prepare",
            "content-bytes-after-prepare",
            "prepared-argv-after-prepare",
            "launch-environment-after-prepare",
            "source-config-after-prepare",
            "private-config-after-prepare",
        ])
    }

    fn run_deterministic_input_cases(
        payload: &RuntimePayload,
        run_root: &Path,
        removed_roots: &mut Vec<PathBuf>,
    ) -> Result<Vec<InputObservation>> {
        let root = run_root.join("deterministic-input");
        std::fs::create_dir(&root)?;
        let options = seed_source_options(run_root)?;
        let mut observations = Vec::new();
        for case in TEST_INPUTS {
            let case_root = root.join(case.name);
            std::fs::create_dir(&case_root)?;
            let test_input = case_root.join(case.file);
            std::fs::write(&test_input, case.contents.as_bytes())?;
            assert_file(&test_input, case.bytes, case.sha256)?;
            let log = case_root.join("retroarch.log");
            let mut process = launch_production(
                payload,
                &payload.controller_rom,
                &case_root,
                &log,
                &options,
                Some(&test_input),
            )?;
            let status = wait_for_retroarch(&mut process.child)?;
            assert_log_identity(&log)?;
            let observed = wait_nes_memory(
                &mut process.child,
                case.player1,
                case.player2,
                Duration::from_secs(10),
            )?;
            ensure!(
                observed[0] == case.player1 && observed[4] == case.player2,
                "Wrong deterministic input observation"
            );
            process.stop_and_cleanup(removed_roots)?;
            observations.push(InputObservation {
                case: case.name,
                player1: format!("{:02X}", observed[0]),
                player2: format!("{:02X}", observed[4]),
                status,
                log,
            });
        }
        Ok(observations)
    }

    fn run_four_player_input_case(
        payload: &RuntimePayload,
        run_root: &Path,
        removed_roots: &mut Vec<PathBuf>,
    ) -> Result<FourPlayerInputObservation> {
        let case = &FOUR_PLAYER_TEST_INPUT;
        let case_root = run_root.join("four-player-input").join(case.name);
        std::fs::create_dir_all(&case_root)?;
        let options = seed_source_options(run_root)?;
        let test_input = case_root.join(case.file);
        std::fs::write(&test_input, case.contents.as_bytes())?;
        assert_file(&test_input, case.bytes, case.sha256)?;
        let log = case_root.join("retroarch.log");
        let four_player_profile = four_player_profile();
        let mut process = launch_production_with_profile(
            payload,
            &payload.controller_rom,
            &case_root,
            &log,
            &options,
            Some(&test_input),
            &four_player_profile,
        )?;
        let status = wait_for_retroarch(&mut process.child)?;
        assert_log_identity(&log)?;
        let observed = wait_nes_four_player_memory(
            &mut process.child,
            [case.player1, case.player2, 0xfe, 0xfd],
            Duration::from_secs(10),
        )?;
        process.stop_and_cleanup(removed_roots)?;
        Ok(FourPlayerInputObservation {
            case: case.name,
            profile: NESTOPIA_NES_FOUR_PLAYER_PROFILE,
            player1: format!("{:02X}", observed[0]),
            player2: format!("{:02X}", observed[4]),
            player3: format!("{:02X}", observed[5]),
            player4: format!("{:02X}", observed[6]),
            source_adapter_option: "nestopia_select_adapter = ntsc",
            private_adapter_option: "nestopia_select_adapter = ntsc",
            status,
            log,
        })
    }

    fn run_persistence_cases(
        payload: &RuntimePayload,
        run_root: &Path,
        removed_roots: &mut Vec<PathBuf>,
    ) -> Result<PersistenceReport> {
        let root = run_root.join("persistence");
        std::fs::create_dir(&root)?;
        let options = seed_source_options(run_root)?;

        let save_root = root.join("save-case");
        std::fs::create_dir(&save_root)?;
        let first_log = save_root.join("first-process/retroarch.log");
        let mut first = launch_production(
            payload,
            &payload.persistence_rom,
            &save_root,
            &first_log,
            &options,
            None,
        )?;
        wait_for_retroarch(&mut first.child)?;
        assert_log_identity(&first_log)?;
        wait_bytes(
            &mut first.child,
            0,
            FIRST_SAVE_MARKER,
            Duration::from_secs(10),
        )?;
        first.stop_and_cleanup(removed_roots)?;
        let first_save = wait_file(
            &save_root,
            |path, metadata| {
                path.extension().is_some_and(|extension| extension == "srm")
                    && metadata.len() == 8_192
                    && file_hash(path).is_ok_and(|hash| hash == FIRST_SAVE_SHA256)
            },
            Duration::from_secs(15),
            "first exact 8 KiB save RAM",
        )?;
        let first_save_evidence = file_evidence(&first_save)?;

        let second_log = save_root.join("second-process/retroarch.log");
        let mut second = launch_production(
            payload,
            &payload.persistence_rom,
            &save_root,
            &second_log,
            &options,
            None,
        )?;
        wait_for_retroarch(&mut second.child)?;
        assert_log_identity(&second_log)?;
        wait_bytes(
            &mut second.child,
            0,
            SECOND_SAVE_MARKER,
            Duration::from_secs(10),
        )?;
        second.stop_and_cleanup(removed_roots)?;
        let second_save = wait_file(
            &save_root,
            |path, metadata| {
                path.extension().is_some_and(|extension| extension == "srm")
                    && metadata.len() == 8_192
                    && file_hash(path).is_ok_and(|hash| hash == SECOND_SAVE_SHA256)
            },
            Duration::from_secs(15),
            "fresh-process exact 8 KiB save RAM",
        )?;
        let second_save_evidence = file_evidence(&second_save)?;

        let state_root = root.join("state-case");
        std::fs::create_dir(&state_root)?;
        let state_first_log = state_root.join("first-process/retroarch.log");
        let mut state_first = launch_production(
            payload,
            &payload.persistence_rom,
            &state_root,
            &state_first_log,
            &options,
            None,
        )?;
        wait_for_retroarch(&mut state_first.child)?;
        assert_log_identity(&state_first_log)?;
        wait_bytes(
            &mut state_first.child,
            0,
            FIRST_SAVE_MARKER,
            Duration::from_secs(10),
        )?;
        write_core_memory(0x100, STATE_MARKER)?;
        wait_bytes(
            &mut state_first.child,
            0x100,
            STATE_MARKER,
            Duration::from_secs(5),
        )?;
        send_command("SAVE_STATE", false)?;
        let state_path = wait_file(
            &state_root,
            |path, metadata| {
                state_file_name(path)
                    && metadata.len() == STATE_FILE_BYTES
                    && validate_state_file(path).is_ok()
            },
            Duration::from_secs(15),
            "exact RASTATE1 file",
        )?;
        let state_evidence = validate_state_file(&state_path)?;
        write_core_memory(0x100, MUTATED_MARKER)?;
        wait_bytes(
            &mut state_first.child,
            0x100,
            MUTATED_MARKER,
            Duration::from_secs(5),
        )?;
        send_command("LOAD_STATE", false)?;
        let same_process = wait_bytes(
            &mut state_first.child,
            0x100,
            STATE_MARKER,
            Duration::from_secs(10),
        )?;
        state_first.stop_and_cleanup(removed_roots)?;

        let state_second_log = state_root.join("second-process/retroarch.log");
        let mut state_second = launch_production(
            payload,
            &payload.persistence_rom,
            &state_root,
            &state_second_log,
            &options,
            None,
        )?;
        wait_for_retroarch(&mut state_second.child)?;
        assert_log_identity(&state_second_log)?;
        wait_bytes(
            &mut state_second.child,
            0,
            SECOND_SAVE_MARKER,
            Duration::from_secs(10),
        )?;
        let before = read_core_memory(0x100, STATE_MARKER.len())?;
        ensure!(
            before != STATE_MARKER,
            "Fresh process already contained LBSTATE1"
        );
        send_command("LOAD_STATE", false)?;
        let fresh_process = wait_bytes(
            &mut state_second.child,
            0x100,
            STATE_MARKER,
            Duration::from_secs(10),
        )?;
        state_second.stop_and_cleanup(removed_roots)?;

        Ok(PersistenceReport {
            first_save: first_save_evidence,
            fresh_process_save: second_save_evidence,
            state: state_evidence,
            same_process_restored: bytes_hex(&same_process),
            fresh_process_before: bytes_hex(&before),
            fresh_process_restored: bytes_hex(&fresh_process),
        })
    }

    fn launch_production(
        payload: &RuntimePayload,
        content: &Path,
        case_root: &Path,
        log: &Path,
        options: &Path,
        test_input: Option<&Path>,
    ) -> Result<PreparedProcess> {
        launch_production_with_profile(
            payload,
            content,
            case_root,
            log,
            options,
            test_input,
            &profile(),
        )
    }

    fn launch_production_with_profile(
        payload: &RuntimePayload,
        content: &Path,
        case_root: &Path,
        log: &Path,
        options: &Path,
        test_input: Option<&Path>,
        profile: &FrontendAutoconfigProfileSpec,
    ) -> Result<PreparedProcess> {
        let main_config = write_main_config(payload, case_root, log, options, test_input)?;
        let source_main = std::fs::read(&main_config)?;
        let source_options = std::fs::read(options)?;
        let arguments = production_arguments(&payload.core, content);
        let request = path_request(payload, case_root);
        let cache = case_root.join("frontend-autoconfig-cache");
        let (session, prepared) =
            prepare_session_with_profile(&request, payload, content, &arguments, &cache, profile)?;
        assert_prepared_contract_for_profile(&session, &arguments, &prepared, options, profile)?;
        session.verify_launch_identity(
            &payload.executable,
            &payload.core,
            content,
            &prepared,
            &[],
        )?;
        let session_root = session.root().to_path_buf();
        let child = OwnedChild::spawn(&payload.executable, &prepared, case_root)?;
        Ok(PreparedProcess {
            child,
            session,
            session_root,
            source_main,
            source_options,
            main_config,
            options_config: options.to_path_buf(),
        })
    }

    fn prepare_session(
        request: &NativeRetroArchPathRequest,
        payload: &RuntimePayload,
        content: &Path,
        arguments: &[OsString],
        cache: &Path,
    ) -> Result<(FrontendAutoconfigSession, Vec<OsString>)> {
        prepare_session_with_profile(request, payload, content, arguments, cache, &profile())
    }

    fn prepare_session_with_profile(
        request: &NativeRetroArchPathRequest,
        payload: &RuntimePayload,
        content: &Path,
        arguments: &[OsString],
        cache: &Path,
        profile: &FrontendAutoconfigProfileSpec,
    ) -> Result<(FrontendAutoconfigSession, Vec<OsString>)> {
        prepare_pinned_frontend_autoconfig_session(
            request,
            &payload.core,
            WINDOWS_RETROARCH_1_19_1_SHA256,
            WINDOWS_NESTOPIA_SHA256,
            profile,
            content,
            arguments,
            &[],
            cache,
        )
    }

    fn assert_prepared_contract(
        session: &FrontendAutoconfigSession,
        original: &[OsString],
        prepared: &[OsString],
        source_options: &Path,
    ) -> Result<()> {
        assert_prepared_contract_for_profile(
            session,
            original,
            prepared,
            source_options,
            &profile(),
        )
    }

    fn assert_prepared_contract_for_profile(
        session: &FrontendAutoconfigSession,
        original: &[OsString],
        prepared: &[OsString],
        source_options: &Path,
        profile: &FrontendAutoconfigProfileSpec,
    ) -> Result<()> {
        ensure!(
            prepared.len() == original.len() + 6
                && prepared[0] == "--appendconfig"
                && prepared[1].to_string_lossy()
                    == session
                        .artifacts
                        .append_config_path
                        .to_string_lossy()
                        .replace('\\', "/")
                && &prepared[6..] == original,
            "Production helper did not prepend the private config and four device overrides"
        );
        ensure!(
            original.iter().all(|argument| {
                argument != "--config"
                    && argument != "-c"
                    && argument != "--appendconfig"
                    && argument.to_str().is_none_or(|text| {
                        !text.starts_with("--config=") && !text.starts_with("--appendconfig=")
                    })
            }),
            "Oracle launch accidentally bypasses native main-config selection"
        );
        ensure!(
            session.effective_core_options_path.canonicalize()? == source_options.canonicalize()?,
            "Production helper selected a different source core-options file"
        );
        let append = &session.artifacts.append_config;
        for port in 1..=4 {
            let device = if port <= profile.max_players { 257 } else { 0 };
            let expected = format!("input_libretro_device_p{port} = \"{device}\"");
            ensure!(
                append.contains(&expected),
                "Private append config omitted {expected}"
            );
            ensure!(
                prepared[port + 1] == OsString::from(format!("--device={port}:{device}")),
                "Prepared argv omitted the exact port-{port} device override"
            );
        }
        let expected_max_users = format!("input_max_users = \"{}\"", profile.max_players);
        ensure!(
            append.contains(&expected_max_users),
            "Private append config omitted {expected_max_users}"
        );
        for expected in [
            "auto_remaps_enable = \"false\"",
            "auto_overrides_enable = \"false\"",
            "config_save_on_exit = \"false\"",
        ] {
            ensure!(
                append.contains(expected),
                "Private append config omitted {expected}"
            );
        }
        ensure!(
            session
                .artifacts
                .core_options
                .contains("unrelated_oracle_option = \"preserved\"")
                && session
                    .artifacts
                    .core_options
                    .contains("nestopia_button_shift = \"disabled\"")
                && !session
                    .artifacts
                    .core_options
                    .contains("nestopia_button_shift = \"enabled\""),
            "Private core-options snapshot did not replace the seeded button-shift value"
        );
        ensure!(
            session
                .artifacts
                .core_options
                .contains("nestopia_select_adapter = \"ntsc\""),
            "Private core-options snapshot did not preserve the effective NTSC adapter option"
        );
        session.verify()?;
        Ok(())
    }

    fn write_main_config(
        payload: &RuntimePayload,
        case_root: &Path,
        log: &Path,
        options: &Path,
        test_input: Option<&Path>,
    ) -> Result<PathBuf> {
        let main_config = payload
            .executable
            .parent()
            .context("RetroArch executable has no directory")?
            .join("retroarch.cfg");
        let save = case_root.join("saves");
        let states = case_root.join("states");
        let system = case_root.join("system");
        let assets = case_root.join("assets");
        let menu_config = case_root.join("config");
        let log_directory = log.parent().context("Log has no parent directory")?;
        for directory in [
            &save,
            &states,
            &system,
            &assets,
            &menu_config,
            log_directory,
        ] {
            std::fs::create_dir_all(directory)?;
        }
        let input = test_input
            .map(|path| format!("test_input_file_joypad = \"{}\"\n", config_path(path)))
            .unwrap_or_default();
        let config = format!(
            "config_save_on_exit = \"false\"\n\
history_list_enable = \"false\"\n\
content_history_size = \"0\"\n\
network_cmd_enable = \"true\"\n\
network_cmd_port = \"{COMMAND_PORT}\"\n\
quit_press_twice = \"false\"\n\
input_driver = \"dinput\"\n\
input_joypad_driver = \"test\"\n\
input_autodetect_enable = \"false\"\n\
input_player1_device = \"1\"\n\
input_player2_device = \"1\"\n\
input_player3_device = \"1\"\n\
input_player4_device = \"1\"\n\
input_player1_joypad_index = \"0\"\n\
input_player2_joypad_index = \"1\"\n\
input_player3_joypad_index = \"2\"\n\
input_player4_joypad_index = \"3\"\n\
input_player1_a_btn = \"0\"\n\
input_player1_b_btn = \"1\"\n\
input_player2_a_btn = \"0\"\n\
input_player2_b_btn = \"1\"\n\
input_player3_a_btn = \"0\"\n\
input_player3_b_btn = \"1\"\n\
input_player4_a_btn = \"0\"\n\
input_player4_b_btn = \"1\"\n\
{input}\
video_driver = \"null\"\n\
audio_driver = \"null\"\n\
menu_driver = \"null\"\n\
video_fullscreen = \"false\"\n\
video_vsync = \"true\"\n\
audio_sync = \"true\"\n\
savefile_directory = \"{}\"\n\
savestate_directory = \"{}\"\n\
save_file_compression = \"false\"\n\
savestate_file_compression = \"false\"\n\
system_directory = \"{}\"\n\
core_assets_directory = \"{}\"\n\
core_options_path = \"{}\"\n\
global_core_options = \"true\"\n\
game_specific_options = \"false\"\n\
rgui_config_directory = \"{}\"\n\
libretro_directory = \"{}\"\n\
libretro_info_path = \"{}\"\n\
state_slot = \"0\"\n\
frontend_log_level = \"0\"\n\
libretro_log_level = \"0\"\n\
log_verbosity = \"true\"\n\
log_to_file = \"true\"\n\
log_to_file_timestamp = \"false\"\n\
log_dir = \"{}\"\n",
            config_path(&save),
            config_path(&states),
            config_path(&system),
            config_path(&assets),
            config_path(options),
            config_path(&menu_config),
            config_path(payload.core.parent().unwrap()),
            config_path(payload.core_info.parent().unwrap()),
            config_path(log_directory),
        );
        std::fs::write(&main_config, config.as_bytes())?;
        Ok(main_config)
    }

    fn seed_source_options(run_root: &Path) -> Result<PathBuf> {
        let path = run_root.join("selected-retroarch-core-options.cfg");
        match std::fs::read(&path) {
            Ok(bytes) => ensure!(bytes == SOURCE_OPTIONS, "Selected source options changed"),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                std::fs::write(&path, SOURCE_OPTIONS)?
            }
            Err(error) => return Err(error.into()),
        }
        Ok(path)
    }

    fn production_arguments(core: &Path, content: &Path) -> Vec<OsString> {
        vec![
            OsString::from("--verbose"),
            OsString::from("--libretro"),
            core.as_os_str().to_owned(),
            content.as_os_str().to_owned(),
        ]
    }

    fn path_request(payload: &RuntimePayload, case_root: &Path) -> NativeRetroArchPathRequest {
        NativeRetroArchPathRequest::Windows {
            executable: payload.executable.clone(),
            home: std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .filter(|path| path.is_absolute()),
            roaming_app_data: case_root.join("roaming-app-data"),
        }
    }

    fn profile() -> FrontendAutoconfigProfileSpec {
        FrontendAutoconfigProfileSpec {
            id: NESTOPIA_NES_TWO_PLAYER_PROFILE.to_owned(),
            core: "nestopia".to_owned(),
            target_layout: "nes".to_owned(),
            transport: "retropad".to_owned(),
            retroarch_library: Some("Nestopia".to_owned()),
            explicit_selection: true,
            platforms: BTreeSet::from([
                "NES".to_owned(),
                "Nintendo NES".to_owned(),
                "Nintendo Entertainment System".to_owned(),
                "Nintendo - Nintendo Entertainment System".to_owned(),
                "Nintendo Famicom".to_owned(),
            ]),
            content_extensions: BTreeSet::from([
                "nes".to_owned(),
                "unf".to_owned(),
                "unif".to_owned(),
            ]),
            frontend_ports: 4,
            max_players: 2,
            default_device: 257,
            port_devices: BTreeMap::new(),
            core_options: BTreeMap::from([(
                "nestopia_button_shift".to_owned(),
                "disabled".to_owned(),
            )]),
            content_guard: None,
            requires_fresh_start: false,
            has_player_topology: false,
            dynamic_profile: false,
            special_preparation: false,
        }
    }

    fn four_player_profile() -> FrontendAutoconfigProfileSpec {
        let mut profile = profile();
        profile.id = NESTOPIA_NES_FOUR_PLAYER_PROFILE.to_owned();
        profile.max_players = 4;
        profile
    }

    fn wait_for_retroarch(child: &mut OwnedChild) -> Result<String> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            child.ensure_running()?;
            let observation = match send_command("GET_STATUS", true) {
                Ok(reply)
                    if reply.starts_with("GET_STATUS PLAYING nes,")
                        || reply.starts_with("GET_STATUS PAUSED nes,") =>
                {
                    return Ok(reply);
                }
                Ok(reply) => format!("unexpected reply {reply:?}"),
                Err(error) => format!("socket error {error:#}"),
            };
            ensure!(
                Instant::now() < deadline,
                "RetroArch did not expose the audited Nestopia system id on UDP {COMMAND_PORT}; last observation: {observation}"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn send_command(command: &str, expect_reply: bool) -> Result<String> {
        let socket = UdpSocket::bind(("127.0.0.1", 0))?;
        socket.connect(("127.0.0.1", COMMAND_PORT))?;
        socket.set_read_timeout(Some(Duration::from_millis(750)))?;
        let command = format!("{command}\n");
        socket.send(command.as_bytes())?;
        if !expect_reply {
            return Ok(String::new());
        }
        let mut buffer = [0_u8; 4_096];
        let count = socket.recv(&mut buffer)?;
        Ok(std::str::from_utf8(&buffer[..count])?.trim().to_owned())
    }

    fn read_core_memory(address: usize, count: usize) -> Result<Vec<u8>> {
        let address_text = format!("{address:x}");
        let reply = send_command(&format!("READ_CORE_MEMORY {address_text} {count}"), true)?;
        let prefix = format!("READ_CORE_MEMORY {address_text}");
        let tail = reply
            .strip_prefix(&prefix)
            .context("Unexpected core-memory reply")?
            .trim();
        ensure!(
            !tail.starts_with("-1"),
            "Core memory is unavailable: {reply}"
        );
        let bytes = tail
            .split_ascii_whitespace()
            .map(|value| u8::from_str_radix(value, 16).context("Invalid memory byte"))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            bytes.len() == count,
            "Core-memory reply has the wrong length"
        );
        Ok(bytes)
    }

    fn write_core_memory(address: usize, bytes: &[u8]) -> Result<()> {
        let address_text = format!("{address:x}");
        let values = bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let reply = send_command(&format!("WRITE_CORE_MEMORY {address_text} {values}"), true)?;
        ensure!(
            reply == format!("WRITE_CORE_MEMORY {address_text} {}", bytes.len()),
            "Unexpected core-memory write reply: {reply}"
        );
        Ok(())
    }

    fn wait_nes_memory(
        child: &mut OwnedChild,
        player1: u8,
        player2: u8,
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            child.ensure_running()?;
            if let Ok(bytes) = read_core_memory(0, 7)
                && bytes.len() == 7
                && bytes[0] == player1
                && bytes[1..4] == *b"LBN"
                && bytes[4] == player2
            {
                return Ok(bytes);
            }
            ensure!(
                Instant::now() < deadline,
                "Timed out waiting for NES bytes P1={player1:02X}, P2={player2:02X}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn wait_nes_four_player_memory(
        child: &mut OwnedChild,
        players: [u8; 4],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            child.ensure_running()?;
            if let Ok(bytes) = read_core_memory(0, 7)
                && bytes.len() == 7
                && bytes[0] == players[0]
                && bytes[1..4] == *b"LBN"
                && bytes[4] == players[1]
                && bytes[5] == players[2]
                && bytes[6] == players[3]
            {
                return Ok(bytes);
            }
            ensure!(
                Instant::now() < deadline,
                "Timed out waiting for Four Score bytes P1={:02X}, P2={:02X}, P3={:02X}, P4={:02X}",
                players[0],
                players[1],
                players[2],
                players[3]
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn wait_bytes(
        child: &mut OwnedChild,
        address: usize,
        expected: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        loop {
            child.ensure_running()?;
            if let Ok(bytes) = read_core_memory(address, expected.len())
                && bytes == expected
            {
                return Ok(bytes);
            }
            ensure!(
                Instant::now() < deadline,
                "Timed out waiting for core memory at 0x{address:x}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn assert_log_identity(log: &Path) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(text) = std::fs::read_to_string(log)
                && text.contains("Version: 1.19.1")
                && text.contains("Found joypad driver: \"test\"")
            {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Log did not prove RetroArch 1.19.1 and the test joypad driver: {}",
                log.display()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn validate_state_file(path: &Path) -> Result<StateEvidence> {
        let data = std::fs::read(path)?;
        ensure!(
            data.len() as u64 == STATE_FILE_BYTES,
            "Wrong RASTATE length"
        );
        ensure!(&data[..7] == b"RASTATE", "Missing RASTATE identifier");
        ensure!(data[7] == 1, "Unsupported RASTATE version");
        ensure!(&data[8..12] == b"MEM ", "First state block is not MEM");
        let declared = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        ensure!(
            declared == STATE_CORE_BYTES,
            "Wrong RASTATE core payload length"
        );
        let padding = (8 - (STATE_CORE_BYTES % 8)) % 8;
        let payload_end = 16 + STATE_CORE_BYTES;
        let end = payload_end + padding;
        ensure!(
            data[payload_end..end].iter().all(|byte| *byte == 0),
            "Nonzero RASTATE alignment byte"
        );
        ensure!(&data[end..end + 4] == b"END ", "Missing RASTATE terminator");
        ensure!(
            u32::from_le_bytes(data[end + 4..end + 8].try_into().unwrap()) == 0,
            "RASTATE terminator has a nonzero length"
        );
        use sha2::{Digest, Sha256};
        let core_payload_sha256 = format!("{:x}", Sha256::digest(&data[16..payload_end]));
        Ok(StateEvidence {
            path: path.to_path_buf(),
            bytes: data.len() as u64,
            sha256: file_hash(path)?,
            format: "RASTATE",
            format_version: 1,
            core_payload_bytes: STATE_CORE_BYTES,
            core_payload_sha256,
            alignment_padding_bytes: padding,
            terminator: "END ",
        })
    }

    fn state_file_name(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.rsplit_once(".state")
                    .is_some_and(|(_, suffix)| suffix.bytes().all(|byte| byte.is_ascii_digit()))
            })
    }

    fn wait_file(
        root: &Path,
        predicate: impl Fn(&Path, &std::fs::Metadata) -> bool,
        timeout: Duration,
        description: &str,
    ) -> Result<PathBuf> {
        let deadline = Instant::now() + timeout;
        loop {
            let matches = regular_files(root)?
                .into_iter()
                .filter(|path| {
                    path.metadata()
                        .is_ok_and(|metadata| predicate(path, &metadata))
                })
                .collect::<Vec<_>>();
            if matches.len() == 1 {
                return Ok(matches[0].clone());
            }
            ensure!(
                Instant::now() < deadline,
                "Did not find exactly one {description} under {}",
                root.display()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn regular_files(root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory)
                .with_context(|| format!("Reading {}", directory.display()))?
            {
                let entry = entry?;
                let kind = entry.file_type()?;
                ensure!(!kind.is_symlink(), "Oracle roots may not contain symlinks");
                if kind.is_dir() {
                    pending.push(entry.path());
                } else if kind.is_file() {
                    files.push(entry.path());
                } else {
                    bail!("Unsupported artifact type: {}", entry.path().display());
                }
            }
        }
        files.sort();
        Ok(files)
    }

    fn checked_relative_path(text: &str) -> Result<PathBuf> {
        let path = PathBuf::from(text);
        ensure!(
            !path.as_os_str().is_empty()
                && path
                    .components()
                    .all(|component| matches!(component, Component::Normal(_))),
            "Distribution manifest contains an unsafe path"
        );
        Ok(path)
    }

    fn copy_verified_file(
        source: &Path,
        destination: &Path,
        expected_bytes: u64,
        expected_sha256: &str,
    ) -> Result<()> {
        assert_file(source, expected_bytes, expected_sha256)?;
        ensure!(
            !destination.exists(),
            "Refusing to overwrite {}",
            destination.display()
        );
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, destination)?;
        assert_file(destination, expected_bytes, expected_sha256)
    }

    fn assert_file(path: &Path, expected_bytes: u64, expected_sha256: &str) -> Result<()> {
        let metadata = path
            .metadata()
            .with_context(|| format!("Reading artifact metadata for {}", path.display()))?;
        ensure!(
            metadata.is_file(),
            "Artifact is not a file: {}",
            path.display()
        );
        ensure!(
            metadata.len() == expected_bytes,
            "Wrong byte length for {}: {}, expected {expected_bytes}",
            path.display(),
            metadata.len()
        );
        let hash = file_hash(path)?;
        ensure!(
            hash.eq_ignore_ascii_case(expected_sha256),
            "Wrong SHA-256 for {}: {hash}",
            path.display()
        );
        Ok(())
    }

    fn artifact_identity(
        name: &'static str,
        version: &'static str,
        path: &Path,
        bytes: u64,
        expected_sha256: &str,
    ) -> Result<ArtifactIdentity> {
        assert_file(path, bytes, expected_sha256)?;
        Ok(ArtifactIdentity {
            name,
            version,
            path: path.to_path_buf(),
            bytes,
            sha256: file_hash(path)?,
        })
    }

    fn file_evidence(path: &Path) -> Result<FileEvidence> {
        Ok(FileEvidence {
            path: path.to_path_buf(),
            bytes: path.metadata()?.len(),
            sha256: file_hash(path)?,
        })
    }

    fn required_absolute_directory(name: &str) -> Result<PathBuf> {
        let path = PathBuf::from(
            std::env::var_os(name).with_context(|| format!("Set {name} to the v8 oracle root"))?,
        );
        ensure!(path.is_absolute(), "{name} must be absolute");
        let path = path.canonicalize()?;
        ensure!(path.is_dir(), "{name} must select a directory");
        Ok(path)
    }

    fn required_fresh_run_root(name: &str) -> Result<PathBuf> {
        let path = PathBuf::from(
            std::env::var_os(name).with_context(|| format!("Set {name} to a new output path"))?,
        );
        ensure!(path.is_absolute(), "{name} must be absolute");
        ensure!(
            !path.exists(),
            "Refusing to reuse or overwrite run root {}",
            path.display()
        );
        let parent = path.parent().context("Run root has no parent directory")?;
        ensure!(parent.is_dir(), "Run-root parent does not exist");
        std::fs::create_dir(&path)?;
        // Keep the ordinary drive-qualified spelling used by real launch plans.
        // Windows canonicalization adds a `\\?\` prefix; feeding that spelling
        // back through RetroArch config paths changes its core-info matching.
        Ok(path)
    }

    fn config_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn bytes_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02X}")).collect()
    }

    fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
        let mut bytes = serde_json::to_vec_pretty(value)?;
        bytes.push(b'\n');
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
#[ignore = "native Windows VM only"]
fn production_frontend_autoconfig_reaches_windows_retroarch_and_persists() {
    panic!("this ignored oracle must be built and run on native Windows");
}
