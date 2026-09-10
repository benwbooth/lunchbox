//! Opt-in real kernel -> generated config -> RetroArch -> emulated hardware test.
//! Includes an opt-in saved-calibration -> discovery -> prepare integration path.
//! Virtual fixtures are not evidence of physical Brawler64 calibration capture.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

struct VirtualPad(File);

impl VirtualPad {
    fn create(calibration: &Calibration, discoverable: bool) -> Result<(Self, PathBuf)> {
        let pad = Self(OpenOptions::new().write(true).open("/dev/uinput")?);
        let fd = pad.0.as_raw_fd();
        for (request, value) in [
            (0x40045564, 1),
            (0x40045564, 3),
            (0x40045567, 0),
            (0x40045567, 1),
        ] {
            ensure!(
                unsafe { libc::ioctl(fd, request as libc::c_ulong, value as libc::c_int) } >= 0,
                "uinput capability: {}",
                std::io::Error::last_os_error()
            );
        }
        for binding in calibration.bindings.values().filter(|b| b.kind == "button") {
            let code = binding.native.as_ref().unwrap().code & 0xffff;
            ensure!(
                unsafe { libc::ioctl(fd, 0x40045565 as libc::c_ulong, code as libc::c_int) } >= 0,
                "uinput button: {}",
                std::io::Error::last_os_error()
            );
        }
        // Linux uinput_setup ABI: input_id (8), name (80), ff_effects_max (4).
        static NEXT_PAD: AtomicU64 = AtomicU64::new(0);
        let instance = NEXT_PAD.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "Lunchbox {} gamepad oracle {}-{instance}",
            if discoverable {
                "Steam-compatible virtual"
            } else {
                "virtual"
            },
            std::process::id()
        );
        ensure!(name.len() < 80, "Oracle device name exceeds uinput ABI");
        let mut setup = [0u8; 92];
        setup[..2].copy_from_slice(&0x06u16.to_ne_bytes()); // BUS_VIRTUAL
        if discoverable {
            // Exercise the existing Steam-compatible virtual-pad discovery class,
            // not a test-only discovery override or a claim of physical hardware.
            setup[2..4].copy_from_slice(&0x28deu16.to_ne_bytes());
            setup[4..6].copy_from_slice(&0x11ffu16.to_ne_bytes());
        }
        setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
        ensure!(
            unsafe { libc::ioctl(fd, 0x405c5503 as libc::c_ulong, setup.as_ptr()) } >= 0,
            "uinput setup: {}",
            std::io::Error::last_os_error()
        );
        for axis in [0u16, 1] {
            let mut abs = [0u8; 28]; // uinput_abs_setup, including alignment padding
            abs[..2].copy_from_slice(&axis.to_ne_bytes());
            abs[8..12].copy_from_slice(&(-32767i32).to_ne_bytes());
            abs[12..16].copy_from_slice(&32767i32.to_ne_bytes());
            ensure!(
                unsafe { libc::ioctl(fd, 0x401c5504 as libc::c_ulong, abs.as_ptr()) } >= 0,
                "uinput axis: {}",
                std::io::Error::last_os_error()
            );
        }
        ensure!(
            unsafe { libc::ioctl(fd, 0x5501 as libc::c_ulong) } >= 0,
            "uinput create: {}",
            std::io::Error::last_os_error()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            for entry in fs::read_dir("/sys/class/input")? {
                let entry = entry?;
                if !entry.file_name().to_string_lossy().starts_with("js") {
                    continue;
                }
                if fs::read_to_string(entry.path().join("device/name"))
                    .ok()
                    .as_deref()
                    .map(str::trim)
                    == Some(&name)
                {
                    let path = Path::new("/dev/input").join(entry.file_name());
                    if File::open(&path).is_ok() {
                        return Ok((pad, path));
                    }
                }
            }
            ensure!(
                Instant::now() < deadline,
                "No readable joydev node for {name}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn button(&mut self, code: u16, pressed: bool) -> Result<()> {
        for (kind, code, value) in [(1, code, i32::from(pressed)), (0, 0, 0)] {
            let event = libc::input_event {
                time: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                type_: kind,
                code,
                value,
            };
            // input_event is a C POD; all fields, including timeval, initialized.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (&event as *const libc::input_event).cast::<u8>(),
                    std::mem::size_of_val(&event),
                )
            };
            self.0.write_all(bytes)?;
        }
        Ok(())
    }
}

impl Drop for VirtualPad {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.0.as_raw_fd(), 0x5502 as libc::c_ulong);
        }
    }
}

struct RetroArch(Child);
impl Drop for RetroArch {
    fn drop(&mut self) {
        // Flatpak may leave its sandbox child alive after the launcher exits.
        // Only signal the new process group created for this owned invocation.
        unsafe {
            libc::kill(-(self.0.id() as libc::pid_t), libc::SIGKILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn await_keys(
    child: &mut RetroArch,
    replies: &Receiver<String>,
    expected: u16,
    psx: bool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last = String::new();
    loop {
        // Do not reap here: Drop must retain ownership of the numeric process
        // group until it has signalled it. EOF/broken pipe detect early exit.
        child
            .0
            .stdin
            .as_mut()
            .context("Missing stdin")?
            .write_all(if psx {
                b"READ_CORE_MEMORY 00020000 36\n"
            } else {
                b"READ_CORE_MEMORY 02000000 8\n"
            })?;
        let response = replies
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .context("RetroArch memory response timeout")?;
        let values = response
            .split_whitespace()
            .skip(2)
            .map(|s| u8::from_str_radix(s, 16))
            .collect::<std::result::Result<Vec<_>, _>>();
        if let Ok(bytes) = values {
            if psx
                && bytes.len() == 36
                && bytes[..4] == [0x4c, 0x42, 0x50, 0x53]
                && bytes[32..34] == [0, 0x41]
                && u16::from_le_bytes([bytes[34], bytes[35]]) == expected
            {
                return Ok(());
            }
            if !psx
                && bytes.len() == 8
                && bytes[4..] == [0x4e, 0x49, 0x42, 0x4c]
                && u16::from_le_bytes([bytes[0], bytes[1]]) == expected
            {
                return Ok(());
            }
        }
        last.clear();
        last.push_str(&response);
        ensure!(
            Instant::now() < deadline,
            "Expected emulated buttons {expected:04x}, last reply: {last}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn local_display_number(display: &str) -> Result<u32> {
    let display = display
        .strip_prefix(':')
        .context("Expected local X display")?;
    let (number, screen) = display
        .split_once('.')
        .map_or((display, None), |(number, screen)| (number, Some(screen)));
    ensure!(
        !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()),
        "Invalid X display number"
    );
    if let Some(screen) = screen {
        ensure!(
            !screen.is_empty() && screen.bytes().all(|b| b.is_ascii_digit()),
            "Invalid X screen number"
        );
    }
    Ok(number.parse()?)
}

fn validate_private_display(display: &str, desktop: &str) -> Result<()> {
    ensure!(
        !display.contains('.'),
        "Oracle display must be :<number> without a screen suffix"
    );
    ensure!(
        local_display_number(display)? != local_display_number(desktop)?,
        "Use a separate local X display"
    );
    Ok(())
}

#[test]
fn private_display_rejects_desktop_aliases_and_missing_desktop() {
    for (display, desktop) in [
        (":0", ":0"),
        (":0", ":0.0"),
        (":00", ":0.1"),
        (":97", ""),
        (":97.0", ":0"),
        (":97x", ":0"),
        (":97", "host:0"),
    ] {
        assert!(
            validate_private_display(display, desktop).is_err(),
            "{display:?}, {desktop:?}"
        );
    }
    assert!(validate_private_display(":97", ":0.0").is_ok());
    assert!(validate_private_display(":97", ":1").is_ok());
}

#[test]
#[ignore = "requires writable uinput, isolated X display, Flatpak RetroArch, and trusted mGBA core; see docs/CONTROLLER_RETROARCH_ORACLE.md"]
fn brawler64_config_reaches_gba_hardware_through_retroarch() -> Result<()> {
    brawler64_hardware_oracle(false, false)
}

#[test]
#[ignore = "requires writable uinput, isolated X display, Flatpak RetroArch, trusted Beetle PSX core and local BIOS; see docs/CONTROLLER_RETROARCH_ORACLE.md"]
fn brawler64_config_reaches_psx_hardware_through_retroarch() -> Result<()> {
    brawler64_hardware_oracle(true, false)
}

#[test]
#[ignore = "requires writable uinput, isolated X display and trusted mGBA; creates two Steam-compatible virtual pads; see docs/CONTROLLER_RETROARCH_ORACLE.md"]
fn saved_brawler64_calibration_prepares_and_controls_gba_through_retroarch() -> Result<()> {
    brawler64_hardware_oracle(false, true)
}

fn saved_calibration_plan(
    directory: &Path,
    calibration: &Calibration,
    preferred_path: &Path,
    other_path: &Path,
) -> Result<(LaunchPlan, CalibratedLaunch)> {
    let mut warnings = Vec::new();
    let inventory = crate::controllers::list_local_controllers(&mut warnings);
    let find = |path: &Path| -> Result<(usize, &ControllerDevice)> {
        let matches = inventory
            .iter()
            .enumerate()
            .filter(|(_, device)| device.device_path == path)
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "Expected one discovered oracle pad at {}: {warnings:?}",
            path.display()
        );
        let (index, device) = matches[0];
        ensure!(
            device.is_virtual
                && device.vendor_id.as_deref() == Some("28de")
                && device.product_id.as_deref() == Some("11ff")
                && device.bus_type.as_deref() == Some("0006")
                && device
                    .name
                    .starts_with("Lunchbox Steam-compatible virtual gamepad oracle "),
            "Unexpected discovered oracle identity: {device:?}"
        );
        Ok((index, device))
    };
    let (preferred_index, preferred) = find(preferred_path)?;
    let (other_index, other) = find(other_path)?;
    ensure!(
        other_index < preferred_index && other.stable_id != preferred.stable_id,
        "Oracle requires two distinct pads with the preferred pad later in discovery order"
    );

    let store = crate::settings::SettingsStore::at(directory.join("lunchbox-state.db"))?;
    let mut settings = AppSettings::default();
    let mapping = &mut settings.controller_mapping;
    mapping.calibrated_launch = true;
    mapping.player_mappings.clear();
    for device in [other, preferred] {
        mapping
            .calibrations
            .insert(device.stable_id.clone(), calibration.clone());
    }
    let platform = "Nintendo Game Boy Advance";
    mapping.preferred_devices.insert(
        crate::controllers::system_layout(platform).to_owned(),
        preferred.stable_id.clone(),
    );
    store.save(&settings)?;
    drop(settings);
    let settings = store.load()?;
    ensure!(
        serde_json::to_value(&settings.controller_mapping.calibrations[&preferred.stable_id])?
            == serde_json::to_value(calibration)?,
        "Saved calibration changed during persistence"
    );
    let option = RomEmulatorOption::retroarch(
        "oracle-mgba".into(),
        "mGBA".into(),
        "mgba",
        EmulatorExecutable::Flatpak {
            command: "flatpak".into(),
            app_id: "org.libretro.RetroArch".into(),
        },
        directory.join("mgba_libretro.so"),
        true,
    );
    let mut plan =
        crate::emulator::build_rom_launch_plan(&directory.join("input.gba"), platform, &option)?;
    let boundary = plan
        .arguments
        .iter()
        .position(|arg| arg == "org.libretro.RetroArch")
        .context("Missing production Flatpak boundary")?;
    // Only diagnostic I/O and private base-config flags are added by the oracle.
    // prepare must select the pad and attach all controller arguments itself.
    plan.arguments.splice(
        boundary + 1..boundary + 1,
        [
            "-M".into(),
            "noload-nosave".into(),
            "-c".into(),
            directory.join("base.cfg").into_os_string(),
        ],
    );
    for flag in ["--device=1:5", "--nodevice=1", "-vd1:5", "--dev=1:5"] {
        let mut conflicting = plan.clone();
        conflicting.arguments.insert(boundary + 1, flag.into());
        let unchanged = conflicting.clone();
        let error = prepare(&settings, platform, &option, &mut conflicting)
            .err()
            .context("Conflicting device command unexpectedly prepared a calibrated launch")?;
        ensure!(
            error
                .to_string()
                .contains("conflicts with calibrated device")
                || error.to_string().contains("Unresolved RetroArch option"),
            "Unexpected rejection for {flag}: {error}"
        );
        ensure!(conflicting == unchanged, "Rejected launch mutated its plan");
    }
    let first = directory.join("preexisting-a.cfg");
    let second = directory.join("preexisting-b.cfg");
    fs::write(&first, "input_libretro_device_p1 = \"5\"\n")?;
    fs::write(&second, "input_player1_b_btn = \"nul\"\n")?;
    for flags in [
        vec![
            "--appendconfig".into(),
            first.clone().into_os_string(),
            "--appendconfig".into(),
            second.clone().into_os_string(),
        ],
        vec![
            format!("--appendconfig={}", first.display()).into(),
            format!("--appendconfig={}", second.display()).into(),
        ],
    ] {
        let mut conflicting = plan.clone();
        conflicting
            .arguments
            .splice(boundary + 1..boundary + 1, flags);
        let unchanged = conflicting.clone();
        let error = prepare(&settings, platform, &option, &mut conflicting)
            .err()
            .context("Duplicate append configs unexpectedly prepared a launch")?;
        ensure!(
            error.to_string().contains("Multiple --appendconfig"),
            "Unexpected duplicate-config error: {error}"
        );
        ensure!(
            conflicting == unchanged,
            "Rejected append configs mutated the plan"
        );
    }
    plan.arguments.insert(
        boundary + 1,
        format!("--appendconfig={}|{}", first.display(), second.display()).into(),
    );
    // An explicit matching CLI device must still reach the real core. RetroArch
    // applies this after reading Lunchbox's appended configuration.
    plan.arguments.insert(boundary + 1, "--device=1:1".into());
    let session = prepare(&settings, platform, &option, &mut plan)?
        .context("Saved calibration did not prepare a launch")?;
    let directory = session
        ._directory
        .as_ref()
        .context("Expected a generated controller config directory")?;
    let path = directory.path().join("controllers.cfg");
    let config = fs::read_to_string(&path)?;
    let numbering = JoydevMap::read(preferred_path)?;
    ensure!(
        cfg_value(&config, "input_player1_joypad_index")? == Some(numbering.index.to_string()),
        "prepare selected the wrong physical joystick"
    );
    ensure!(
        cfg_value(&config, "input_max_users")?.as_deref() == Some("1"),
        "GBA launch did not restrict the selected calibration to its one port"
    );
    let expected_append = OsString::from(format!(
        "--appendconfig={}|{}|{}",
        first.display(),
        second.display(),
        path.display()
    ));
    ensure!(
        plan.arguments
            .iter()
            .filter(|arg| *arg == &expected_append)
            .count()
            == 1,
        "Production plan did not retain the existing config list with calibrated config last"
    );
    ensure!(
        plan.arguments
            .iter()
            .any(|arg| arg
                == &OsString::from(format!("--filesystem={}", directory.path().display()))),
        "Production plan omitted its controller config filesystem grant"
    );
    ensure!(
        plan.cleanup_paths.is_empty(),
        "Unexpected diagnostic content transformation"
    );
    eprintln!(
        "Saved-calibration oracle: {} -> {} ({})",
        preferred.stable_id,
        preferred_path.display(),
        session.description
    );
    Ok((plan, session))
}

fn brawler64_hardware_oracle(psx: bool, saved_launch: bool) -> Result<()> {
    use sha2::{Digest, Sha256};
    let (core_env, core_name, rom_name, expected_hash) = if psx {
        (
            "LUNCHBOX_ORACLE_PSX_CORE",
            "mednafen_psx_libretro.so",
            "input.exe",
            "767bb60bd96d3f19806a9311d96638c9ca39272d1236035a752952bb4b4c1968",
        )
    } else {
        (
            "LUNCHBOX_ORACLE_MGBA_CORE",
            "mgba_libretro.so",
            "input.gba",
            "768921964037e0a40e8eab9e0d6eccad1b8a13d74bc37e9cae5543bb167d18c4",
        )
    };
    let core = PathBuf::from(std::env::var(core_env).context("Set trusted core path")?);
    ensure!(core.is_absolute(), "Core path must be absolute");
    let core_bytes = fs::read(&core)?;
    ensure!(
        format!("{:x}", Sha256::digest(&core_bytes)) == expected_hash,
        "Unreviewed diagnostic core binary"
    );
    let display =
        std::env::var("LUNCHBOX_ORACLE_DISPLAY").context("Set a private X server display")?;
    validate_private_display(
        &display,
        &std::env::var("DISPLAY")
            .context("Desktop DISPLAY is required to rule out the user's display")?,
    )?;
    let socket = PathBuf::from(format!(
        "/tmp/.X11-unix/X{}",
        local_display_number(&display)?
    ));
    let _display_connection = std::os::unix::net::UnixStream::connect(&socket)
        .with_context(|| format!("Private X server is not listening at {}", socket.display()))?;
    let root = tempfile::Builder::new()
        .prefix("lunchbox-retroarch-oracle-")
        .tempdir()?;
    let dir = root.path();
    for name in [
        "config", "data", "cache", "state", "saves", "states", "system", "logs",
    ] {
        fs::create_dir(dir.join(name))?;
    }
    fs::write(dir.join(core_name), core_bytes)?;
    fs::write(
        dir.join(rom_name),
        if psx {
            lunchbox_controller_probe::libretro_input::psx_diagnostic_exe()
        } else {
            lunchbox_controller_probe::libretro_input::gba_diagnostic_rom()
        },
    )?;
    if psx {
        let bios = PathBuf::from(
            std::env::var("LUNCHBOX_ORACLE_PSX_BIOS_DIR")
                .context("Set local PlayStation BIOS directory")?,
        );
        for name in ["scph5500.bin", "scph5501.bin", "scph5502.bin"] {
            let source = bios.join(name);
            ensure!(
                fs::metadata(&source)?.len() == 512 * 1024,
                "Unexpected BIOS size"
            );
            fs::copy(source, dir.join("system").join(name))?;
        }
    }
    let (mut calibration, _) = super::tests::calibrated_layout("brawler64");
    let mut index = 0;
    for binding in calibration
        .bindings
        .values_mut()
        .filter(|b| b.kind == "button")
    {
        // Nonstandard codes avoid desktop Guide/A/B hotkeys; permute their order
        // so a writer assuming physical-label order cannot accidentally pass.
        let native = NativeInput {
            code: 0x10000 + 0x2c0 + (index * 7 + 3) % 17,
            direction: 0,
        };
        binding.code = native.code;
        binding.native = Some(native);
        index += 1;
    }
    ensure!(index == 17, "Update fixture for changed Brawler64 controls");
    fs::write(
        dir.join("calibration.json"),
        serde_json::to_vec(&calibration)?,
    )?;
    let calibration: Calibration =
        serde_json::from_slice(&fs::read(dir.join("calibration.json"))?)?;
    // Both pads remain alive through readback. Prefer the later discovery entry
    // so the test cannot pass by blindly selecting the first calibrated joystick.
    let mut other_pad = if saved_launch {
        Some(VirtualPad::create(&calibration, true)?)
    } else {
        None
    };
    let (mut pad, mut path) = VirtualPad::create(&calibration, saved_launch)?;
    if let Some((other, other_path)) = &mut other_pad {
        let inventory = crate::controllers::list_local_controllers(&mut Vec::new());
        let position = |path: &Path| {
            inventory
                .iter()
                .position(|device| device.device_path == path)
                .context("Oracle pad missing from production discovery")
        };
        if position(&path)? < position(other_path)? {
            std::mem::swap(&mut pad, other);
            std::mem::swap(&mut path, other_path);
        }
    }
    let numbering = JoydevMap::read(&path)?;
    let profile = if psx {
        catalog().launch_mode("mednafen_psx", "PSX", 1)
    } else {
        contract("mgba", "Nintendo Game Boy Advance")
    }
    .context("Missing diagnostic core contract")?;
    if !saved_launch {
        let mut mapping = player_config(&calibration, profile, &numbering, 1)?;
        if psx {
            mapping.push_str(&write_core_options_snapshot(profile, "", dir)?);
            mapping.push_str("input_libretro_device_p2 = \"0\"\ninput_max_users = \"1\"\n");
        }
        fs::write(dir.join("mapping.cfg"), mapping)?;
    }
    let mut config = String::from(
        "stdin_cmd_enable = \"true\"\ninput_driver = \"x\"\ninput_joypad_driver = \"linuxraw\"\ninput_poll_type_behavior = \"0\"\nvideo_driver = \"glcore\"\naudio_enable = \"false\"\nvideo_fullscreen = \"false\"\npause_nonactive = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\nauto_overrides_enable = \"false\"\nauto_remaps_enable = \"false\"\ninput_autodetect_enable = \"false\"\nhistory_list_enable = \"false\"\ngame_specific_options = \"false\"\ncore_info_cache_enable = \"false\"\n",
    );
    config.push_str("global_core_options = \"true\"\ncontent_runtime_log = \"false\"\ncontent_runtime_log_aggregate = \"false\"\nvideo_context_driver = \"x\"\nvideo_vsync = \"false\"\n");
    config.push_str("ui_companion_enable = \"false\"\nui_companion_start_on_boot = \"false\"\ndesktop_menu_enable = \"false\"\nrgui_show_start_screen = \"false\"\nsuspend_screensaver_enable = \"false\"\nmicrophone_enable = \"false\"\n");
    for (key, path) in [
        ("savefile_directory", "saves"),
        ("savestate_directory", "states"),
        ("system_directory", "system"),
        ("cache_directory", "cache"),
        ("log_dir", "logs"),
        ("rgui_config_directory", "config"),
        ("core_options_path", "config/options.cfg"),
        ("content_history_path", "config/history.lpl"),
        ("content_favorites_path", "config/favorites.lpl"),
        ("content_music_history_path", "config/music.lpl"),
        ("content_video_history_path", "config/video.lpl"),
        ("content_image_history_path", "config/images.lpl"),
        ("runtime_log_directory", "logs"),
        ("input_remapping_directory", "config"),
        ("playlist_directory", "data"),
        ("screenshot_directory", "data"),
        ("recording_output_directory", "data"),
        ("recording_config_directory", "config"),
    ] {
        config.push_str(&format!("{key} = \"{}\"\n", dir.join(path).display()));
    }
    fs::write(dir.join("base.cfg"), config)?;
    let (plan, calibrated_session) = if saved_launch {
        let (_, other_path) = other_pad.as_ref().context("Missing second oracle pad")?;
        let (plan, session) = saved_calibration_plan(dir, &calibration, &path, other_path)?;
        (plan, Some(session))
    } else {
        (
            LaunchPlan {
                emulator_name: "RetroArch input oracle".into(),
                program: "flatpak".into(),
                arguments: vec![
                    "run".into(),
                    format!("--filesystem={}", dir.display()).into(),
                    "org.libretro.RetroArch".into(),
                    "--verbose".into(),
                    "--sram-mode".into(),
                    "noload-nosave".into(),
                    "-c".into(),
                    dir.join("base.cfg").into_os_string(),
                    "--appendconfig".into(),
                    dir.join("mapping.cfg").into_os_string(),
                    "-L".into(),
                    dir.join(core_name).into_os_string(),
                    dir.join(rom_name).into_os_string(),
                ],
                current_directory: dir.to_owned(),
                environment: Vec::new(),
                cleanup_paths: Vec::new(),
                retroarch_content: None,
            },
            None,
        )
    };
    let generated_directory = calibrated_session
        .as_ref()
        .and_then(|session| session._directory.as_ref())
        .map(|directory| directory.path().to_owned());
    let boundary = plan
        .arguments
        .iter()
        .position(|arg| arg == "org.libretro.RetroArch")
        .context("Missing oracle Flatpak boundary")?;
    ensure!(
        plan.arguments.first().is_some_and(|arg| arg == "run"),
        "Unexpected Flatpak plan"
    );
    ensure!(
        plan.environment.is_empty(),
        "Oracle isolation requires an unmodified launch environment"
    );
    let mut command = Command::new(&plan.program);
    command
        // Socket exposure is decided from the launcher's environment, before
        // Flatpak applies --env options or the sandbox's env command runs.
        .env("DISPLAY", &display)
        .args([
            "run",
            "--unshare=network",
            "--nosocket=wayland",
            "--nodevice=all",
            "--device=input",
            "--device=shm",
            "--nofilesystem=host:reset",
            "--nofilesystem=home",
            "--command=env",
        ])
        // Retain the production filesystem grants, including the launch-scoped
        // controller config. Do not repair missing grants in the test wrapper.
        .args(&plan.arguments[1..boundary])
        .arg(format!("--env=DISPLAY={display}"))
        .arg("org.libretro.RetroArch")
        .arg("QT_QPA_PLATFORM=xcb")
        .arg(format!("DISPLAY={display}"));
    for (key, suffix) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
    ] {
        // Flatpak replaces --env=XDG_* before execution; set them after entry.
        command.arg(format!("{key}={}", dir.join(suffix).display()));
    }
    command
        .arg("/app/bin/retroarch")
        .args(&plan.arguments[boundary + 1..])
        .current_dir(&plan.current_directory)
        .envs(plan.environment.iter().cloned())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .process_group(0);
    let mut child = RetroArch(command.spawn()?);
    let output = child.0.stdout.take().context("Missing stdout")?;
    let (sender, replies) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(output)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if line.starts_with("READ_CORE_MEMORY ") && sender.send(line).is_err() {
                break;
            }
        }
    });
    let released = if psx { 0xffff } else { 0x3ff };
    await_keys(&mut child, &replies, released, psx)?;
    if let Some((other, _)) = &mut other_pad {
        // Keep conflicting inputs held across every selected-pad observation.
        // An immediate released read alone could precede event consumption.
        for id in ["a", "l"] {
            other.button(
                (calibration.bindings[id].native.as_ref().unwrap().code & 0xffff) as u16,
                true,
            )?;
        }
    }
    // Expected bits come from console hardware protocols, not generated config.
    let cases = if psx {
        vec![
            (vec!["a"], 1 << 14),      // Cross
            (vec!["b"], 1 << 15),      // Square
            (vec!["c_down"], 1 << 13), // Circle
            (vec!["c_left"], 1 << 12), // Triangle
            (vec!["select"], 1),
            (vec!["start"], 1 << 3),
            (vec!["up"], 1 << 4),
            (vec!["right"], 1 << 5),
            (vec!["down"], 1 << 6),
            (vec!["left"], 1 << 7),
            (vec!["z"], 1 << 8),
            (vec!["z_right"], 1 << 9),
            (vec!["l"], 1 << 10),
            (vec!["r"], 1 << 11),
            (vec!["a", "b"], (1 << 14) | (1 << 15)),
            (vec!["l", "r"], (1 << 10) | (1 << 11)),
        ]
    } else {
        vec![
            (vec!["a"], 1),
            (vec!["b"], 2),
            (vec!["select"], 4),
            (vec!["start"], 8),
            (vec!["right"], 16),
            (vec!["left"], 32),
            (vec!["up"], 64),
            (vec!["down"], 128),
            (vec!["r"], 256),
            (vec!["l"], 512),
            (vec!["a", "b"], 3),
            (vec!["l", "r"], 768),
        ]
    };
    for (controls, bits) in cases {
        for id in &controls {
            pad.button(
                (calibration.bindings[*id].native.as_ref().unwrap().code & 0xffff) as u16,
                true,
            )?;
        }
        await_keys(&mut child, &replies, released & !bits, psx)
            .with_context(|| format!("Pressed {controls:?}"))?;
        for id in &controls {
            pad.button(
                (calibration.bindings[*id].native.as_ref().unwrap().code & 0xffff) as u16,
                false,
            )?;
        }
        await_keys(&mut child, &replies, released, psx)
            .with_context(|| format!("Released {controls:?}"))?;
    }
    drop(child);
    drop(calibrated_session);
    if let Some(directory) = generated_directory {
        ensure!(
            !directory.exists(),
            "Launch-scoped controller config survived teardown"
        );
    }
    Ok(())
}
