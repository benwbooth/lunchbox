use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use lunchbox_controller_probe::retroarch_frontend_autoconfig::{
    FrontendAutoconfigHost, FrontendAutoconfigProfileSpec, FrontendPortMode,
    MacOsInstallDisposition, NESTOPIA_NES_FOUR_PLAYER_PROFILE, NESTOPIA_NES_TWO_PLAYER_PROFILE,
    NativeRetroArchPathRequest, attach_frontend_autoconfig, build_frontend_autoconfig,
    ensure_no_physical_mapping_keys, prepare_frontend_autoconfig_session,
    prepare_pinned_frontend_autoconfig_session, resolve_effective_core_options_path,
    resolve_macos_install_disposition, resolve_native_retroarch_paths,
    validate_unmodified_launch_context,
};

fn nestopia_profile(id: &str, players: usize) -> FrontendAutoconfigProfileSpec {
    FrontendAutoconfigProfileSpec {
        id: id.to_owned(),
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
        content_extensions: BTreeSet::from(["nes".to_owned(), "unf".to_owned(), "unif".to_owned()]),
        frontend_ports: 4,
        max_players: players,
        default_device: 257,
        port_devices: BTreeMap::new(),
        core_options: BTreeMap::from([("nestopia_button_shift".to_owned(), "disabled".to_owned())]),
        content_guard: None,
        requires_fresh_start: false,
        has_player_topology: false,
        dynamic_profile: false,
        special_preparation: false,
    }
}

fn bundled_profile(id: &str) -> FrontendAutoconfigProfileSpec {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../lunchbox-app/data/controllers/catalog.json"
    ))
    .unwrap();
    let profile = catalog["emulator_profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["id"] == id)
        .unwrap();
    let launch = &profile["retroarch_launch"];
    let strings = |field: &str| {
        profile[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect()
    };
    let map = |field: &str| {
        profile[field]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.as_str().unwrap().to_owned()))
            .collect()
    };
    FrontendAutoconfigProfileSpec {
        id: profile["id"].as_str().unwrap().to_owned(),
        core: profile["core"].as_str().unwrap().to_owned(),
        target_layout: profile["target_layout"].as_str().unwrap().to_owned(),
        transport: profile["transport"].as_str().unwrap().to_owned(),
        retroarch_library: profile["retroarch_library"].as_str().map(str::to_owned),
        explicit_selection: profile["explicit_selection"].as_bool().unwrap_or(false),
        platforms: launch["platforms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect(),
        content_extensions: strings("content_extensions"),
        frontend_ports: profile["frontend_ports"].as_u64().unwrap() as usize,
        max_players: launch["max_players"].as_u64().unwrap() as usize,
        default_device: launch["device"].as_u64().unwrap() as u32,
        port_devices: BTreeMap::new(),
        core_options: map("core_options"),
        content_guard: profile["content_guard"].as_str().map(str::to_owned),
        requires_fresh_start: profile["requires_fresh_start"].as_bool().unwrap_or(false),
        has_player_topology: launch.get("player_topology").is_some(),
        dynamic_profile: false,
        special_preparation: false,
    }
}

#[test]
fn bundled_nestopia_profiles_match_the_closed_allowlist() {
    let root = tempfile::tempdir().unwrap();
    for (id, players) in [
        (NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        (NESTOPIA_NES_FOUR_PLAYER_PROFILE, 4),
    ] {
        let profile = bundled_profile(id);
        assert_eq!(profile.max_players, players);
        build_frontend_autoconfig(
            FrontendAutoconfigHost::Windows,
            &profile,
            &root.path().join("game.nes"),
            "",
            &root.path().join(id.replace(':', "-")),
        )
        .unwrap();
    }
}

#[test]
fn windows_paths_follow_native_config_precedence_without_moving_defaults() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let home = root.path().join("home");
    let app_data = root.path().join("AppData/Roaming");
    let local = executable.parent().unwrap().join("retroarch.cfg");
    let fallback = app_data.join("retroarch.cfg");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::write(&fallback, "fallback").unwrap();
    let request = NativeRetroArchPathRequest::Windows {
        executable: executable.clone(),
        home: Some(home),
        roaming_app_data: app_data.clone(),
    };

    let paths = resolve_native_retroarch_paths(&request, Path::is_file).unwrap();
    assert_eq!(paths.main_config, fallback);
    assert_eq!(paths.application_directory, executable.parent().unwrap());
    assert_eq!(paths.application_data_directory, app_data);
    assert_eq!(
        paths.default_core_options_path(),
        fallback.with_file_name("retroarch-core-options.cfg")
    );
    assert_eq!(
        paths.resolve_configured_path(":/config/Nestopia").unwrap(),
        executable.parent().unwrap().join("config/Nestopia")
    );

    std::fs::write(&local, "local").unwrap();
    let paths = resolve_native_retroarch_paths(&request, Path::is_file).unwrap();
    assert_eq!(paths.main_config, local);
    assert_eq!(
        paths.application_data_directory,
        executable.parent().unwrap()
    );
    assert_eq!(
        paths.default_core_options_path(),
        executable
            .parent()
            .unwrap()
            .join("retroarch-core-options.cfg")
    );
}

#[test]
fn windows_first_run_uses_executable_directory() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let request = NativeRetroArchPathRequest::Windows {
        executable: executable.clone(),
        home: Some(root.path().join("home")),
        roaming_app_data: root.path().join("AppData/Roaming"),
    };
    let paths = resolve_native_retroarch_paths(&request, |_| false).unwrap();
    assert_eq!(
        paths.main_config,
        executable.parent().unwrap().join("retroarch.cfg")
    );
}

#[test]
fn windows_tilde_paths_require_the_exact_inherited_home() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let without_home = resolve_native_retroarch_paths(
        &NativeRetroArchPathRequest::Windows {
            executable: executable.clone(),
            home: None,
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        |_| false,
    )
    .unwrap();
    assert!(
        without_home
            .resolve_configured_path("~/options.cfg")
            .is_err()
    );
    let inherited_home = root.path().join("different-inherited-home");
    let with_home = resolve_native_retroarch_paths(
        &NativeRetroArchPathRequest::Windows {
            executable,
            home: Some(inherited_home.clone()),
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        |_| false,
    )
    .unwrap();
    assert_eq!(
        with_home.resolve_configured_path("~/options.cfg").unwrap(),
        inherited_home.join("options.cfg")
    );
}

#[test]
fn macos_standard_portable_and_ambiguous_roots_are_distinct() {
    let root = tempfile::tempdir().unwrap();
    let application_bundle = root.path().join("Applications/RetroArch.app");
    let executable = application_bundle.join("Contents/MacOS/RetroArch");
    let home = root.path().join("Users/tester");
    let support = home.join("Library/Application Support");
    let standard = NativeRetroArchPathRequest::MacOs {
        executable: executable.clone(),
        application_bundle: application_bundle.clone(),
        home: home.clone(),
        application_support: support.clone(),
        install: MacOsInstallDisposition::Standard,
    };
    let paths = resolve_native_retroarch_paths(&standard, |_| false).unwrap();
    assert_eq!(paths.application_directory, application_bundle);
    assert_eq!(paths.application_data_directory, support.join("RetroArch"));
    assert_eq!(
        paths.default_config_directory(),
        support.join("RetroArch/config")
    );
    assert_eq!(
        paths.main_config,
        support.join("RetroArch/config/retroarch.cfg")
    );
    assert_eq!(
        paths.default_core_options_path(),
        support.join("RetroArch/config/retroarch-core-options.cfg")
    );
    assert_eq!(
        paths.resolve_configured_path(":/config/Nestopia").unwrap(),
        root.path()
            .join("Applications/RetroArch.app/config/Nestopia")
    );

    let portable_root = root.path().join("Portable RetroArch");
    let portable_bundle = portable_root.join("RetroArch.app");
    let portable = NativeRetroArchPathRequest::MacOs {
        executable: portable_bundle.join("Contents/MacOS/RetroArch"),
        application_bundle: portable_bundle.clone(),
        home: home.clone(),
        application_support: support.clone(),
        install: MacOsInstallDisposition::Portable {
            application_root: portable_root.clone(),
        },
    };
    let paths = resolve_native_retroarch_paths(&portable, |_| false).unwrap();
    assert_eq!(paths.application_directory, portable_bundle);
    assert_eq!(paths.application_data_directory, portable_root);
    assert_eq!(
        paths.main_config,
        paths
            .application_data_directory
            .join("config/retroarch.cfg")
    );
    assert_eq!(
        paths.default_core_options_path(),
        paths
            .application_data_directory
            .join("config/retroarch-core-options.cfg")
    );

    let mismatched_portable = NativeRetroArchPathRequest::MacOs {
        executable: executable.clone(),
        application_bundle: application_bundle.clone(),
        home: home.clone(),
        application_support: support.clone(),
        install: MacOsInstallDisposition::Portable {
            application_root: portable_root,
        },
    };
    assert!(resolve_native_retroarch_paths(&mismatched_portable, |_| false).is_err());

    let ambiguous = NativeRetroArchPathRequest::MacOs {
        executable,
        application_bundle,
        home,
        application_support: support,
        install: MacOsInstallDisposition::Ambiguous,
    };
    assert!(
        resolve_native_retroarch_paths(&ambiguous, |_| false)
            .unwrap_err()
            .to_string()
            .contains("Ambiguous macOS")
    );
}

#[test]
fn macos_executable_must_be_directly_inside_the_declared_app_bundle() {
    let root = tempfile::tempdir().unwrap();
    let bundle = root.path().join("Applications/RetroArch.app");
    let home = root.path().join("Users/tester");
    let support = home.join("Library/Application Support");
    for executable in [
        root.path().join("Applications/RetroArch"),
        bundle.join("Contents/Helpers/RetroArch"),
        bundle.join("Contents/MacOS/Nested/RetroArch"),
    ] {
        let request = NativeRetroArchPathRequest::MacOs {
            executable,
            application_bundle: bundle.clone(),
            home: home.clone(),
            application_support: support.clone(),
            install: MacOsInstallDisposition::Standard,
        };
        assert!(
            resolve_native_retroarch_paths(&request, |_| false)
                .unwrap_err()
                .to_string()
                .contains("<bundle>/Contents/MacOS")
        );
    }

    let request = NativeRetroArchPathRequest::MacOs {
        executable: bundle.join("Contents/MacOS/RetroArch"),
        application_bundle: root.path().join("Applications/RetroArch.bundle"),
        home,
        application_support: support,
        install: MacOsInstallDisposition::Standard,
    };
    assert!(
        resolve_native_retroarch_paths(&request, |_| false)
            .unwrap_err()
            .to_string()
            .contains(".app extension")
    );
}

#[test]
fn macos_paths_reject_a_missing_or_relative_inherited_home() {
    let root = tempfile::tempdir().unwrap();
    let bundle = root.path().join("Applications/RetroArch.app");
    let request = NativeRetroArchPathRequest::MacOs {
        executable: bundle.join("Contents/MacOS/RetroArch"),
        application_bundle: bundle,
        home: PathBuf::from("relative-home"),
        application_support: root.path().join("Library/Application Support"),
        install: MacOsInstallDisposition::Standard,
    };
    assert!(
        resolve_native_retroarch_paths(&request, |_| false)
            .unwrap_err()
            .to_string()
            .contains("home directory")
    );
}

#[test]
fn macos_portable_policy_reads_the_adjacent_marker_and_plist_boolean() {
    let root = tempfile::tempdir().unwrap();
    let bundle = root.path().join("Applications/RetroArch.app");
    std::fs::create_dir_all(bundle.join("Contents")).unwrap();
    let plist = |value: &str| {
        format!(
            "<?xml version=\"1.0\"?><plist><dict><key>RAPortableInstall</key><{value}/></dict></plist>"
        )
    };
    std::fs::write(bundle.join("Contents/Info.plist"), plist("false")).unwrap();
    assert_eq!(
        resolve_macos_install_disposition(&bundle).unwrap(),
        MacOsInstallDisposition::Standard
    );
    std::fs::write(bundle.join("Contents/Info.plist"), plist("true")).unwrap();
    assert_eq!(
        resolve_macos_install_disposition(&bundle).unwrap(),
        MacOsInstallDisposition::Portable {
            application_root: bundle.parent().unwrap().to_path_buf()
        }
    );
    std::fs::write(bundle.parent().unwrap().join("portable.txt"), []).unwrap();
    std::fs::write(bundle.join("Contents/Info.plist"), b"bplist00").unwrap();
    assert!(matches!(
        resolve_macos_install_disposition(&bundle).unwrap(),
        MacOsInstallDisposition::Portable { .. }
    ));
    std::fs::remove_file(bundle.parent().unwrap().join("portable.txt")).unwrap();
    assert_eq!(
        resolve_macos_install_disposition(&bundle).unwrap(),
        MacOsInstallDisposition::Ambiguous
    );
}

#[test]
fn effective_options_resolve_game_core_and_special_roots() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let request = NativeRetroArchPathRequest::Windows {
        executable,
        home: Some(root.path().join("home")),
        roaming_app_data: root.path().join("AppData/Roaming"),
    };
    let paths = resolve_native_retroarch_paths(&request, |_| false).unwrap();
    let content = root.path().join("roms/NES/Test Game.nes");
    let game_options = paths
        .application_directory
        .join("config/Nestopia/Test Game.opt");
    std::fs::create_dir_all(game_options.parent().unwrap()).unwrap();
    std::fs::write(&game_options, "game").unwrap();
    let config = "rgui_config_directory = \":/config\"\n";
    assert_eq!(
        resolve_effective_core_options_path(&paths, config, "Nestopia", &content, Path::is_file,)
            .unwrap(),
        game_options
    );

    std::fs::remove_file(&game_options).unwrap();
    let per_core = paths
        .application_directory
        .join("config/Nestopia/Nestopia.opt");
    std::fs::write(&per_core, "core").unwrap();
    assert_eq!(
        resolve_effective_core_options_path(&paths, config, "Nestopia", &content, Path::is_file,)
            .unwrap(),
        per_core
    );

    std::fs::remove_file(&per_core).unwrap();
    let custom = root.path().join("custom/options.cfg");
    let config = format!(
        "game_specific_options = \"false\"\nglobal_core_options = \"true\"\ncore_options_path = \"{}\"\n",
        custom.display()
    );
    assert_eq!(
        resolve_effective_core_options_path(&paths, &config, "Nestopia", &content, |_| false,)
            .unwrap(),
        custom
    );
}

#[test]
fn explicit_default_config_directory_follows_the_loaded_main_config() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let app_data = root.path().join("AppData/Roaming");
    let fallback = app_data.join("retroarch.cfg");
    let paths = resolve_native_retroarch_paths(
        &NativeRetroArchPathRequest::Windows {
            executable: executable.clone(),
            home: Some(root.path().join("home")),
            roaming_app_data: app_data,
        },
        |path| path == fallback,
    )
    .unwrap();
    let content = root.path().join("roms/NES/game.nes");

    let platform_default = executable
        .parent()
        .unwrap()
        .join("config/Nestopia/Nestopia.opt");
    assert_eq!(
        resolve_effective_core_options_path(&paths, "", "Nestopia", &content, |path| {
            path == platform_default
        })
        .unwrap(),
        platform_default
    );

    let main_config_default = fallback.parent().unwrap().join("Nestopia/Nestopia.opt");
    for value in ["default", ""] {
        let config = format!("rgui_config_directory = \"{value}\"\n");
        assert_eq!(
            resolve_effective_core_options_path(&paths, &config, "Nestopia", &content, |path| path
                == main_config_default,)
            .unwrap(),
            main_config_default
        );
    }

    assert!(
        resolve_effective_core_options_path(
            &paths,
            "global_core_options = \"true\"\ncore_options_path = \"default\"\n",
            "Nestopia",
            &content,
            |_| false,
        )
        .is_err()
    );
}

#[test]
fn unresolved_paths_includes_arguments_and_environment_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let paths = resolve_native_retroarch_paths(
        &NativeRetroArchPathRequest::Windows {
            executable: root.path().join("RetroArch/retroarch.exe"),
            home: Some(root.path().join("home")),
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        |_| false,
    )
    .unwrap();
    assert!(
        paths
            .resolve_configured_path("relative/options.cfg")
            .is_err()
    );
    assert!(
        resolve_effective_core_options_path(
            &paths,
            "#include \"other.cfg\"\n",
            "Nestopia",
            &root.path().join("game.nes"),
            |_| false,
        )
        .is_err()
    );
    for argument in [
        "--config",
        "--config=custom.cfg",
        "-c",
        "-c=custom.cfg",
        "-ccustom.cfg",
        "--appendconfig",
        "--appendconfig=custom.cfg",
    ] {
        assert!(
            validate_unmodified_launch_context(&[OsString::from(argument)], &[]).is_err(),
            "{argument}"
        );
    }
    for argument in [
        "--device",
        "--device=1:257",
        "-d",
        "-d1:257",
        "--nodevice",
        "--nodevice=3",
        "-N",
        "-N3",
        "--dualanalog",
        "--dualanalog=1",
        "-A",
        "-A1",
    ] {
        assert!(
            validate_unmodified_launch_context(&[OsString::from(argument)], &[]).is_err(),
            "{argument}"
        );
    }
    validate_unmodified_launch_context(
        &[
            OsString::from("--"),
            OsString::from("--device=ordinary-option-looking-content.nes"),
        ],
        &[],
    )
    .unwrap();
    assert!(
        validate_unmodified_launch_context(
            &[],
            &[(OsString::from("APPDATA"), OsString::from("override"))]
        )
        .is_err()
    );
}

#[test]
fn nestopia_two_player_builder_only_serializes_frontend_profile_state() {
    let root = tempfile::tempdir().unwrap();
    let content = root.path().join("game.nes");
    let session = root.path().join("session");
    let profile = nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2);
    let artifacts = build_frontend_autoconfig(
        FrontendAutoconfigHost::Windows,
        &profile,
        &content,
        "unrelated = \"keep\"\nnestopia_button_shift = \"enabled\"\n",
        &session,
    )
    .unwrap();

    assert!(artifacts.core_options.contains("unrelated = \"keep\""));
    assert_eq!(
        artifacts
            .core_options
            .matches("nestopia_button_shift")
            .count(),
        1
    );
    assert!(
        artifacts
            .core_options
            .contains("nestopia_button_shift = \"disabled\"")
    );
    assert!(
        artifacts
            .append_config
            .contains("input_libretro_device_p1 = \"257\"")
    );
    assert!(
        artifacts
            .append_config
            .contains("input_libretro_device_p2 = \"257\"")
    );
    assert!(
        artifacts
            .append_config
            .contains("input_libretro_device_p3 = \"0\"")
    );
    assert!(
        artifacts
            .append_config
            .contains("input_libretro_device_p4 = \"0\"")
    );
    assert!(artifacts.append_config.contains("input_max_users = \"2\""));
    assert!(artifacts.description.contains("frontend_autoconfig"));
    assert!(artifacts.description.contains("not verified"));
    ensure_no_physical_mapping_keys(&artifacts.append_config).unwrap();
    for forbidden in [
        "linuxraw",
        "input_joypad_driver",
        "input_autodetect_enable",
        "joypad_index",
        "_btn",
        "_axis",
        "savefile_directory",
        "savestate_directory",
        "savefiles_in_content_dir",
        "savestates_in_content_dir",
        "sort_savefiles_enable",
        "sort_savestates_enable",
        "savestate_auto_load",
        "savestate_auto_save",
    ] {
        assert!(!artifacts.append_config.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn nestopia_four_player_builder_serializes_all_four_devices() {
    let root = tempfile::tempdir().unwrap();
    let profile = nestopia_profile(NESTOPIA_NES_FOUR_PLAYER_PROFILE, 4);
    let artifacts = build_frontend_autoconfig(
        FrontendAutoconfigHost::MacOs,
        &profile,
        &root.path().join("game.unif"),
        "",
        &root.path().join("session"),
    )
    .unwrap();
    assert_eq!(
        artifacts
            .port_modes
            .iter()
            .map(|mode| mode.libretro_device)
            .collect::<Vec<_>>(),
        vec![257, 257, 257, 257]
    );
    assert!(artifacts.append_config.contains("input_max_users = \"4\""));
}

#[test]
fn append_config_is_native_private_and_precedes_the_original_plan() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("session/frontend-autoconfig.cfg");
    let original = vec![
        OsString::from("--verbose"),
        OsString::from("-L"),
        OsString::from("core.dll"),
        OsString::from("game.nes"),
    ];
    let port_modes = vec![
        FrontendPortMode {
            frontend_port: 1,
            libretro_device: 257,
        },
        FrontendPortMode {
            frontend_port: 2,
            libretro_device: 257,
        },
        FrontendPortMode {
            frontend_port: 3,
            libretro_device: 0,
        },
        FrontendPortMode {
            frontend_port: 4,
            libretro_device: 0,
        },
    ];
    let attached = attach_frontend_autoconfig(
        FrontendAutoconfigHost::Windows,
        &original,
        &[],
        &config,
        &port_modes,
    )
    .unwrap();
    assert_eq!(attached[0], "--appendconfig");
    assert_eq!(PathBuf::from(&attached[1]), config);
    assert_eq!(
        &attached[2..6],
        [
            "--device=1:257",
            "--device=2:257",
            "--device=3:0",
            "--device=4:0",
        ]
    );
    assert_eq!(&attached[6..], original);
}

#[test]
fn production_session_snapshots_effective_options_and_retains_user_configuration() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let home = root.path().join("home");
    let app_data = root.path().join("AppData/Roaming");
    let main_config = executable.parent().unwrap().join("retroarch.cfg");
    let options = executable
        .parent()
        .unwrap()
        .join("config/Nestopia/Nestopia.opt");
    std::fs::create_dir_all(options.parent().unwrap()).unwrap();
    let main_before = "rgui_config_directory = \":/config\"\ninput_joypad_driver = \"dinput\"\nsavefile_directory = \"D:/Saves\"\n";
    let options_before = "unrelated = \"keep\"\nnestopia_button_shift = \"enabled\"\n";
    std::fs::write(&main_config, main_before).unwrap();
    std::fs::write(&options, options_before).unwrap();
    let content = root.path().join("roms/game.nes");
    let arguments = vec![
        OsString::from("--verbose"),
        OsString::from("-L"),
        root.path()
            .join("cores/nestopia_libretro.dll")
            .into_os_string(),
        content.clone().into_os_string(),
    ];
    let (session, prepared) = prepare_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::Windows {
            executable,
            home: Some(home),
            roaming_app_data: app_data,
        },
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &arguments,
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    assert_eq!(session.paths.main_config, main_config);
    assert_eq!(session.effective_core_options_path, options);
    assert_eq!(prepared[0], "--appendconfig");
    assert_eq!(
        &prepared[2..6],
        [
            "--device=1:257",
            "--device=2:257",
            "--device=3:0",
            "--device=4:0",
        ]
    );
    assert_eq!(&prepared[6..], arguments);
    assert_eq!(std::fs::read_to_string(&main_config).unwrap(), main_before);
    assert_eq!(std::fs::read_to_string(&options).unwrap(), options_before);
    assert!(
        session
            .artifacts
            .core_options
            .contains("unrelated = \"keep\"")
    );
    assert!(
        session
            .artifacts
            .core_options
            .contains("nestopia_button_shift = \"disabled\"")
    );
    assert!(
        !session
            .artifacts
            .append_config
            .contains("savefile_directory")
    );
    assert!(
        !session
            .artifacts
            .append_config
            .contains("input_joypad_driver")
    );
    session.verify().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(session.root())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        for path in [
            &session.artifacts.append_config_path,
            &session.artifacts.core_options_path,
        ] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    std::fs::write(
        &main_config,
        format!("{main_before}menu_driver = \"xmb\"\n"),
    )
    .unwrap();
    assert!(
        session
            .verify()
            .unwrap_err()
            .to_string()
            .contains("changed")
    );
}

#[cfg(unix)]
#[test]
fn session_rejects_private_file_mode_widening() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let content = root.path().join("game.nes");
    let (session, _) = prepare_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::Windows {
            executable,
            home: None,
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    std::fs::set_permissions(
        &session.artifacts.core_options_path,
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    assert!(session.verify().is_err());
}

#[cfg(unix)]
#[test]
fn windows_main_config_selection_follows_complete_readability() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let local = executable.parent().unwrap().join("retroarch.cfg");
    let app_data = root.path().join("AppData/Roaming");
    let fallback = app_data.join("retroarch.cfg");
    std::fs::create_dir_all(&local).unwrap();
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::write(&fallback, "").unwrap();
    let content = root.path().join("game.nes");
    let request = NativeRetroArchPathRequest::Windows {
        executable: executable.clone(),
        home: None,
        roaming_app_data: app_data,
    };
    let (session, _) = prepare_frontend_autoconfig_session(
        &request,
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache-directory"),
    )
    .unwrap();
    assert_eq!(session.paths.main_config, fallback);
    session.verify().unwrap();
    drop(session);

    std::fs::remove_dir(&local).unwrap();
    let readable = root.path().join("readable.cfg");
    std::fs::write(&readable, "# readable through a symlink\n").unwrap();
    symlink(&readable, &local).unwrap();
    let (session, _) = prepare_frontend_autoconfig_session(
        &request,
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache-symlink"),
    )
    .unwrap();
    assert_eq!(session.paths.main_config, local);
    session.verify().unwrap();
    drop(session);

    std::fs::remove_file(&local).unwrap();
    symlink(root.path().join("missing.cfg"), &local).unwrap();
    let (session, _) = prepare_frontend_autoconfig_session(
        &request,
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache-dangling"),
    )
    .unwrap();
    assert_eq!(session.paths.main_config, fallback);
    session.verify().unwrap();
}

#[test]
fn windows_session_rejects_a_new_higher_precedence_main_config() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let app_data = root.path().join("AppData/Roaming");
    let fallback = app_data.join("retroarch.cfg");
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::write(&fallback, "").unwrap();
    let content = root.path().join("game.nes");
    let (session, _) = prepare_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::Windows {
            executable: executable.clone(),
            home: None,
            roaming_app_data: app_data,
        },
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    assert_eq!(session.paths.main_config, fallback);
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(executable.parent().unwrap().join("retroarch.cfg"), "").unwrap();
    assert!(session.verify().is_err());
}

#[cfg(unix)]
#[test]
fn session_rejects_an_effective_options_symlink_swap() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let options = executable
        .parent()
        .unwrap()
        .join("config/Nestopia/Nestopia.opt");
    std::fs::create_dir_all(options.parent().unwrap()).unwrap();
    let first = root.path().join("first.opt");
    let second = root.path().join("second.opt");
    std::fs::write(&first, "one = \"1\"\n").unwrap();
    std::fs::write(&second, "two = \"2\"\n").unwrap();
    symlink(&first, &options).unwrap();
    let content = root.path().join("game.nes");
    let (session, _) = prepare_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::Windows {
            executable,
            home: None,
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    std::fs::remove_file(&options).unwrap();
    symlink(&second, &options).unwrap();
    assert!(session.verify().is_err());
}

#[test]
fn session_rejects_a_new_higher_precedence_options_file() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("RetroArch/retroarch.exe");
    let main = executable.parent().unwrap().join("retroarch.cfg");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(&main, "rgui_config_directory = \":/config\"\n").unwrap();
    let content = root.path().join("roms/NES/Test Game.nes");
    let (session, _) = prepare_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::Windows {
            executable: executable.clone(),
            home: None,
            roaming_app_data: root.path().join("AppData/Roaming"),
        },
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &[],
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    let game = executable
        .parent()
        .unwrap()
        .join("config/Nestopia/Test Game.opt");
    std::fs::create_dir_all(game.parent().unwrap()).unwrap();
    std::fs::write(&game, "appeared = \"true\"\n").unwrap();
    assert!(session.verify().is_err());
}

#[test]
fn pinned_session_rejects_same_path_content_replacement() {
    let root = tempfile::tempdir().unwrap();
    let application_bundle = root.path().join("RetroArch.app");
    let executable = application_bundle.join("Contents/MacOS/RetroArch");
    let core = application_bundle.join("Contents/Frameworks/nestopia_libretro.dylib");
    let content = root.path().join("game.nes");
    std::fs::create_dir_all(core.parent().unwrap()).unwrap();
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(
        application_bundle.join("Contents/Info.plist"),
        "<?xml version=\"1.0\"?><plist><dict><key>RAPortableInstall</key><false/></dict></plist>",
    )
    .unwrap();
    std::fs::write(&executable, "frontend fixture").unwrap();
    std::fs::write(&core, "core fixture").unwrap();
    std::fs::write(&content, "content fixture one").unwrap();
    let frontend_hash = lunchbox_controller_probe::file_hash(&executable).unwrap();
    let core_hash = lunchbox_controller_probe::file_hash(&core).unwrap();
    let arguments = vec![
        OsString::from("-L"),
        core.clone().into_os_string(),
        content.clone().into_os_string(),
    ];
    let (session, prepared) = prepare_pinned_frontend_autoconfig_session(
        &NativeRetroArchPathRequest::MacOs {
            executable: executable.clone(),
            application_bundle,
            home: root.path().join("home"),
            application_support: root.path().join("Library/Application Support"),
            install: MacOsInstallDisposition::Standard,
        },
        &core,
        &frontend_hash,
        &core_hash,
        &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
        &content,
        &arguments,
        &[],
        &root.path().join("cache"),
    )
    .unwrap();
    session
        .verify_launch_identity(&executable, &core, &content, &prepared, &[])
        .unwrap();
    let late_environment = vec![(
        OsString::from("HOME"),
        root.path().join("late-home").into_os_string(),
    )];
    assert!(
        session
            .verify_launch_identity(&executable, &core, &content, &prepared, &late_environment,)
            .is_err()
    );
    std::fs::write(&content, "content fixture two").unwrap();
    assert!(session.verify().is_err());
}

#[test]
fn excluded_and_drifted_profiles_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let content = root.path().join("game.nes");
    let session = root.path().join("session");
    let baseline = nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2);
    let mut cases = Vec::new();
    let mut profile = baseline.clone();
    profile.id = "retroarch:fceumm:nes".to_owned();
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.dynamic_profile = true;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.special_preparation = true;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.content_guard = Some("stella_joysticks".to_owned());
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.requires_fresh_start = true;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.has_player_topology = true;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.default_device = 1;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.target_layout = "famicom".to_owned();
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.transport = "keyboard".to_owned();
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.retroarch_library = Some("Nestopia UE".to_owned());
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.explicit_selection = false;
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.platforms.insert("Famicom Disk System".to_owned());
    cases.push(profile);
    let mut profile = baseline.clone();
    profile.content_extensions.insert("fds".to_owned());
    cases.push(profile);
    let mut profile = baseline;
    profile
        .core_options
        .insert("nestopia_select_adapter".to_owned(), "ntsc".to_owned());
    cases.push(profile);

    for profile in cases {
        assert!(
            build_frontend_autoconfig(
                FrontendAutoconfigHost::Windows,
                &profile,
                &content,
                "",
                &session,
            )
            .is_err(),
            "{}",
            profile.id
        );
    }
    assert!(
        build_frontend_autoconfig(
            FrontendAutoconfigHost::Windows,
            &nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE, 2),
            &root.path().join("game.fds"),
            "",
            &session,
        )
        .is_err()
    );
}
