//! Launch-scoped Steam Controller 2 routing for RetroArch. Prefer Steam's
//! existing virtual Xbox pad in gamepad mode. When Steam has not created one,
//! forward SDL3 native input through an owned uinput pad instead.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::live_sdl3::Pad;

use crate::controller_axis::{GamepadAxis, GamepadControl, GamepadFrame, VirtualGamepad};

const BUTTONS: &[(usize, u16)] = &[
    (0, 0x130),  // South / BTN_SOUTH
    (1, 0x131),  // East
    (2, 0x134),  // West
    (3, 0x133),  // North
    (9, 0x136),  // Left shoulder
    (10, 0x137), // Right shoulder
    (4, 0x13a),  // Back
    (6, 0x13b),  // Start
    (5, 0x13c),  // Steam / Guide
    (7, 0x13d),  // Left stick click
    (8, 0x13e),  // Right stick click
    (11, 0x220), // D-pad up
    (12, 0x221), // D-pad down
    (13, 0x222), // D-pad left
    (14, 0x223), // D-pad right
];

pub struct RetroArchControllerSession {
    stop: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
    steam_virtual_event: Option<PathBuf>,
    pub joypad_index: usize,
}

impl RetroArchControllerSession {
    /// None means no unique SC2 is currently reported by Lunchbox's live SDL3
    /// helper; other controllers and normal RetroArch setup remain untouched.
    pub fn start() -> Result<Option<Self>> {
        let Some((identity, first_pad)) = crate::controller_sdl3::unique_pad() else {
            return Ok(None);
        };
        ensure!(
            lunchbox_controller_probe::live_sdl3::is_sc2(first_pad.vendor, first_pad.product),
            "SDL3 device is not a Steam Controller 2"
        );
        if let Some(event) = steam_virtual_event()?
            && let Some(joypad_index) = udev_joypad_index(&event)?
        {
            return Ok(Some(Self {
                stop: None,
                worker: None,
                failure: Arc::new(Mutex::new(None)),
                steam_virtual_event: Some(event),
                joypad_index,
            }));
        }
        let frame = gamepad_contract()?;
        let publisher = VirtualGamepad::create(&frame)?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let joypad_index = loop {
            if let Some(event_path) = publisher.event_path()?
                && let Some(index) = udev_joypad_index(&event_path)?
            {
                break index;
            }
            ensure!(
                Instant::now() < deadline,
                "RetroArch did not discover the owned virtual gamepad within two seconds"
            );
            thread::sleep(Duration::from_millis(20));
        };
        let (stop, stopping) = mpsc::channel();
        let failure = Arc::new(Mutex::new(None));
        let worker_failure = Arc::clone(&failure);
        let worker = thread::Builder::new()
            .name("lunchbox-steam-retroarch".into())
            .spawn(move || {
                if let Err(error) = forward(&identity, publisher, stopping) {
                    if let Ok(mut state) = worker_failure.lock() {
                        *state = Some(format!("{error:#}"));
                    }
                }
            })?;
        Ok(Some(Self {
            stop: Some(stop),
            worker: Some(worker),
            failure,
            steam_virtual_event: None,
            joypad_index,
        }))
    }

    pub fn config(&self) -> String {
        if self.steam_virtual_event.is_some() {
            return format!(
                "# Lunchbox uses Steam's existing virtual Xbox pad for this session.\n\
                 input_joypad_driver = \"udev\"\n\
                 input_autodetect_enable = \"true\"\n\
                 input_player1_joypad_index = \"{}\"\n\
                 config_save_on_exit = \"false\"\n",
                self.joypad_index
            );
        }
        format!(
            "# Lunchbox session-local Steam Controller 2 bridge.\n\
             input_joypad_driver = \"udev\"\n\
             input_autodetect_enable = \"false\"\n\
             input_player1_joypad_index = \"{}\"\n\
             input_player1_b_btn = \"0\"\n\
             input_player1_a_btn = \"1\"\n\
             input_player1_x_btn = \"2\"\n\
             input_player1_y_btn = \"3\"\n\
             input_player1_l_btn = \"4\"\n\
             input_player1_r_btn = \"5\"\n\
             input_player1_select_btn = \"6\"\n\
             input_player1_start_btn = \"7\"\n\
             input_menu_toggle_btn = \"8\"\n\
             input_player1_l3_btn = \"9\"\n\
             input_player1_r3_btn = \"10\"\n\
             input_player1_up_btn = \"11\"\n\
             input_player1_down_btn = \"12\"\n\
             input_player1_left_btn = \"13\"\n\
             input_player1_right_btn = \"14\"\n\
             input_player1_l2_axis = \"+4\"\n\
             input_player1_r2_axis = \"+5\"\n\
             input_player1_l_x_plus_axis = \"+0\"\n\
             input_player1_l_x_minus_axis = \"-0\"\n\
             input_player1_l_y_plus_axis = \"+1\"\n\
             input_player1_l_y_minus_axis = \"-1\"\n\
             input_player1_r_x_plus_axis = \"+2\"\n\
             input_player1_r_x_minus_axis = \"-2\"\n\
             input_player1_r_y_plus_axis = \"+3\"\n\
             input_player1_r_y_minus_axis = \"-3\"\n\
             config_save_on_exit = \"false\"\n",
            self.joypad_index
        )
    }

    pub fn check_health(&self) -> Result<()> {
        if let Some(event) = &self.steam_virtual_event {
            ensure!(event.exists(), "Steam's virtual Xbox pad disappeared");
            return Ok(());
        }
        let failure = self
            .failure
            .lock()
            .map_err(|_| anyhow::anyhow!("Controller bridge status unavailable"))?;
        if let Some(error) = failure.as_ref() {
            anyhow::bail!("{error}");
        }
        ensure!(
            self.worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished()),
            "Steam Controller bridge stopped unexpectedly"
        );
        Ok(())
    }
}

impl Drop for RetroArchControllerSession {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Steam's Linux uinput pad is 28de:11ff and lives under /devices/virtual.
/// The raw SC2 HID itself exposes keyboard/mouse nodes, not a gamepad node.
/// Requiring a unique virtual pad avoids guessing which of several Steam
/// controllers should own player one.
fn steam_virtual_event() -> Result<Option<PathBuf>> {
    let report = fs::read_to_string("/proc/bus/input/devices")?;
    Ok(parse_steam_virtual_event(&report))
}

fn parse_steam_virtual_event(report: &str) -> Option<PathBuf> {
    let mut events = report.split("\n\n").filter_map(|record| {
        let identity = record.lines().find(|line| line.starts_with("I: "))?;
        let virtual_path = record
            .lines()
            .find_map(|line| line.strip_prefix("S: Sysfs="))?;
        if !identity.contains("Vendor=28de Product=11ff")
            || !virtual_path.starts_with("/devices/virtual/input/")
        {
            return None;
        }
        let handlers = record
            .lines()
            .find_map(|line| line.strip_prefix("H: Handlers="))?;
        let event = handlers
            .split_whitespace()
            .find(|handler| handler.starts_with("event"))?;
        Some(Path::new("/dev/input").join(event))
    });
    let first = events.next();
    if events.next().is_none() { first } else { None }
}

fn gamepad_contract() -> Result<GamepadFrame> {
    GamepadFrame::new(
        BUTTONS.iter().map(|(_, code)| *code),
        (0..6).map(|code| {
            (
                code,
                GamepadAxis::Passthrough {
                    minimum: if code < 4 { -32768 } else { 0 },
                    maximum: 32767,
                    neutral: 0,
                },
            )
        }),
    )
}

fn frame_from_pad(pad: &Pad) -> Result<BTreeMap<GamepadControl, i32>> {
    ensure!(
        lunchbox_controller_probe::live_sdl3::is_sc2(pad.vendor, pad.product)
            && pad.buttons.len() == 26
            && pad.axes.len() == 6,
        "Steam Controller 2 input frame changed shape"
    );
    let mut frame = BTreeMap::new();
    for (source, code) in BUTTONS {
        frame.insert(
            GamepadControl::Button(*code),
            i32::from(pad.buttons[*source]),
        );
    }
    for (code, value) in pad.axes.iter().enumerate() {
        frame.insert(GamepadControl::Axis(code as u16), i32::from(*value));
    }
    Ok(frame)
}

fn forward(
    identity: &str,
    mut publisher: VirtualGamepad,
    stopping: mpsc::Receiver<()>,
) -> Result<()> {
    let mut last = None;
    loop {
        if stopping.recv_timeout(Duration::from_millis(16)).is_ok() {
            return Ok(());
        }
        let pad = crate::controller_sdl3::pad_by_id(identity)
            .context("Steam Controller 2 disconnected from Lunchbox SDL3 input")?;
        let frame = frame_from_pad(&pad)?;
        if last.as_ref() != Some(&frame) {
            publisher.publish(&frame)?;
            last = Some(frame);
        }
    }
}

fn udev_joypad_index(event_path: &Path) -> Result<Option<usize>> {
    let output = Command::new("udevadm")
        .args(["info", "--export-db"])
        .output()
        .context("reading udev gamepad order")?;
    ensure!(output.status.success(), "udev gamepad enumeration failed");
    Ok(parse_udev_joypad_index(
        &String::from_utf8_lossy(&output.stdout),
        event_path,
    ))
}

fn parse_udev_joypad_index(database: &str, event_path: &Path) -> Option<usize> {
    let target = event_path.strip_prefix("/dev/").ok()?.to_str()?;
    let mut index = 0;
    for record in database.split("\n\n") {
        if !record.lines().any(|line| line == "E: ID_INPUT_JOYSTICK=1") {
            continue;
        }
        let Some(node) = record.lines().find_map(|line| line.strip_prefix("N: ")) else {
            continue;
        };
        if !node.starts_with("input/event") {
            continue;
        }
        if node == target {
            return Some(index);
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_unique_steam_virtual_xbox_pad_is_reused() {
        let physical = "I: Bus=0003 Vendor=045e Product=028e Version=0110\nS: Sysfs=/devices/pci0000:00/input/input75\nH: Handlers=event262 js4 \n";
        let steam = "I: Bus=0003 Vendor=28de Product=11ff Version=0001\nN: Name=\"Microsoft X-Box 360 pad 0\"\nS: Sysfs=/devices/virtual/input/input92\nH: Handlers=event263 js5 \n";
        assert_eq!(
            parse_steam_virtual_event(&format!("{physical}\n{steam}\n")),
            Some(PathBuf::from("/dev/input/event263"))
        );
        assert!(parse_steam_virtual_event(physical).is_none());
        assert!(parse_steam_virtual_event(&format!("{steam}\n{steam}")).is_none());
    }

    #[test]
    fn normalized_sc2_frame_has_gamepad_controls_only() {
        let mut pad = Pad {
            instance: 1,
            name: "Steam Controller 2".into(),
            path: None,
            serial: None,
            vendor: 0x28de,
            product: 0x1302,
            mapping: String::new(),
            buttons: vec![false; 26],
            axes: vec![0; 6],
        };
        pad.buttons[0] = true;
        pad.buttons[2] = true;
        pad.buttons[3] = true;
        pad.buttons[11] = true;
        pad.axes[4] = 32767;
        let frame = frame_from_pad(&pad).unwrap();
        assert_eq!(frame[&GamepadControl::Button(0x130)], 1);
        assert_eq!(frame[&GamepadControl::Button(0x134)], 1);
        assert_eq!(frame[&GamepadControl::Button(0x133)], 1);
        assert_eq!(frame[&GamepadControl::Button(0x220)], 1);
        assert_eq!(frame[&GamepadControl::Axis(4)], 32767);
        assert_eq!(frame.len(), BUTTONS.len() + 6);
        assert!(gamepad_contract().is_ok());
    }

    #[test]
    fn udev_order_selects_owned_event_not_a_guessed_slot() {
        let database = "N: input/event262\nE: ID_INPUT_JOYSTICK=1\n\nN: input/js3\nE: ID_INPUT_JOYSTICK=1\n\nN: input/event257\nE: ID_INPUT_JOYSTICK=1\n\nN: input/event300\nE: ID_INPUT_JOYSTICK=1\n\n";
        assert_eq!(
            parse_udev_joypad_index(database, Path::new("/dev/input/event300")),
            Some(2)
        );
        assert_eq!(
            parse_udev_joypad_index(database, Path::new("/dev/input/event301")),
            None
        );
    }

    #[test]
    #[ignore = "requires writable /dev/uinput and the live udev service"]
    fn owned_virtual_pad_is_visible_at_an_exact_udev_slot() {
        let pad = VirtualGamepad::create(&gamepad_contract().unwrap()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let index = loop {
            if let Some(event) = pad.event_path().unwrap()
                && let Some(index) = udev_joypad_index(&event).unwrap()
            {
                break index;
            }
            assert!(
                Instant::now() < deadline,
                "owned gamepad was not enumerated"
            );
            thread::sleep(Duration::from_millis(20));
        };
        assert!(index < 16);
    }
}
