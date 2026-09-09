//! SDL3 native input is namespaced independently of GilRs/evdev calibration.
use crate::controller_catalog::{Calibration, InputBinding};
use crate::controllers::ControllerDevice;
use lunchbox_controller_probe::live_sdl3::{Frame, Pad};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

pub const BACKEND: &str = "sdl3-gamepad";
pub const MODEL_ID: &str = "lunchbox:steam-controller-2026:sdl3";
const BUTTON_CODE: u32 = 0x53440000;
const AXIS_CODE: u32 = 0x53450000;

#[derive(Default)]
struct Inventory {
    pads: Vec<(String, Pad)>,
    status: String,
}
fn inventory() -> &'static Mutex<Inventory> {
    static STATE: OnceLock<Mutex<Inventory>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(Inventory::default()))
}

pub fn runtime_path() -> std::path::PathBuf {
    std::env::var_os("LUNCHBOX_SDL3_LIBRARY")
        .map(Into::into)
        .or_else(|| option_env!("LUNCHBOX_SDL3_LIBRARY").map(Into::into))
        .or_else(|| {
            let filename = if cfg!(target_os = "windows") {
                "SDL3.dll"
            } else if cfg!(target_os = "macos") {
                "libSDL3.dylib"
            } else {
                "libSDL3.so.0"
            };
            let path = std::env::current_exe().ok()?.parent()?.join(filename);
            path.is_file().then_some(path)
        })
        .unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                "SDL3.dll"
            } else if cfg!(target_os = "macos") {
                "libSDL3.dylib"
            } else {
                "libSDL3.so.0"
            }
            .into()
        })
}

fn key(pad: &Pad, pads: &[Pad]) -> String {
    if let Some(serial) = pad.serial.as_ref().filter(|s| !s.is_empty())
        && pads
            .iter()
            .filter(|p| p.serial.as_ref() == Some(serial))
            .count()
            == 1
    {
        return format!("sdl3:serial:{}", hex::encode(serial.as_bytes()));
    }
    // An instance ID is explicitly session-local; do not masquerade as a serial.
    format!("sdl3:session:{}:{}", std::process::id(), pad.instance)
}

pub fn devices() -> (Vec<ControllerDevice>, String) {
    let state = inventory().lock().unwrap();
    (
        state
            .pads
            .iter()
            .map(|(key, pad)| ControllerDevice {
                stable_id: key.clone(),
                name: pad.name.clone(),
                device_path: key.into(),
                event_paths: vec![],
                vendor_id: Some(format!("{:04x}", pad.vendor)),
                product_id: Some(format!("{:04x}", pad.product)),
                version: None,
                bus_type: None,
                physical_path: pad.path.clone(),
                unique_id: pad.serial.clone(),
                is_virtual: false,
            })
            .collect(),
        state.status.clone(),
    )
}

pub fn connected(id: &str) -> bool {
    inventory()
        .lock()
        .unwrap()
        .pads
        .iter()
        .any(|(key, _)| key == id)
}
pub fn mapping(id: &str) -> Option<String> {
    inventory()
        .lock()
        .unwrap()
        .pads
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, p)| p.mapping.clone())
}
pub fn is_binding(input: &InputBinding) -> bool {
    matches!(input.code & 0xffff0000, BUTTON_CODE | AXIS_CODE)
}
pub fn valid_binding(input: &InputBinding) -> bool {
    let index = (input.code & 0xffff) as usize;
    let expected = match input.code & 0xffff0000 {
        BUTTON_CODE if index < 22 && input.kind == "button" && input.direction == 0 => {
            binding(index, 0)
        }
        AXIS_CODE if index < 6 && input.kind == "axis" && matches!(input.direction, -1 | 1) => {
            binding(index, input.direction)
        }
        _ => return false,
    };
    input.logical == expected.logical && input.native.is_none() && input.axis.is_none()
}
pub fn backend(bindings: &BTreeMap<String, InputBinding>) -> &'static str {
    if bindings.values().any(is_binding) {
        BACKEND
    } else {
        "gilrs-0.11"
    }
}

const BUTTONS: [&str; 26] = [
    "South",
    "East",
    "West",
    "North",
    "Back",
    "Guide",
    "Start",
    "LeftStick",
    "RightStick",
    "LeftBumper",
    "RightBumper",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
    "QuickAccess",
    "RightGrip1",
    "LeftGrip1",
    "RightGrip2",
    "LeftGrip2",
    "LeftTouchpadClick",
    "RightTouchpadClick",
    "RightStickTouch",
    "LeftStickTouch",
    "RightGripSense",
    "LeftGripSense",
];
pub fn binding(index: usize, direction: i8) -> InputBinding {
    let logical = if direction == 0 {
        BUTTONS[index].to_owned()
    } else {
        match index {
            0 => {
                if direction < 0 {
                    "LeftStickLeft"
                } else {
                    "LeftStickRight"
                }
            }
            1 => {
                if direction < 0 {
                    "LeftStickUp"
                } else {
                    "LeftStickDown"
                }
            }
            2 => {
                if direction < 0 {
                    "RightStickLeft"
                } else {
                    "RightStickRight"
                }
            }
            3 => {
                if direction < 0 {
                    "RightStickUp"
                } else {
                    "RightStickDown"
                }
            }
            4 => "LeftTrigger",
            _ => "RightTrigger",
        }
        .into()
    };
    InputBinding {
        code: if direction == 0 {
            BUTTON_CODE
        } else {
            AXIS_CODE
        } + index as u32,
        kind: if direction == 0 { "button" } else { "axis" }.into(),
        direction,
        logical,
        native: None,
        axis: None,
    }
}

pub fn standard_calibration(id: &str) -> anyhow::Result<Calibration> {
    anyhow::ensure!(connected(id), "Reconnect the native SDL3 controller first");
    calibration()
}

fn calibration() -> anyhow::Result<Calibration> {
    let buttons = [
        ("b", 0),
        ("a", 1),
        ("y", 2),
        ("x", 3),
        ("select", 4),
        ("guide", 5),
        ("start", 6),
        ("l3", 7),
        ("r3", 8),
        ("l", 9),
        ("r", 10),
        ("up", 11),
        ("down", 12),
        ("left", 13),
        ("right", 14),
        ("quick_access", 15),
        ("right_grip1", 16),
        ("left_grip1", 17),
        ("right_grip2", 18),
        ("left_grip2", 19),
        ("left_pad_click", 20),
        ("right_pad_click", 21),
    ];
    let mut bindings: BTreeMap<_, _> = buttons
        .into_iter()
        .map(|(id, index)| (id.to_owned(), binding(index, 0)))
        .collect();
    for (id, index, direction) in [
        ("stick_left", 0, -1),
        ("stick_right", 0, 1),
        ("stick_up", 1, -1),
        ("stick_down", 1, 1),
        ("right_stick_left", 2, -1),
        ("right_stick_right", 2, 1),
        ("right_stick_up", 3, -1),
        ("right_stick_down", 3, 1),
        ("l2", 4, 1),
        ("r2", 5, 1),
    ] {
        bindings.insert(id.into(), binding(index, direction));
    }
    // Deserialize the persisted schema so optional calibration metadata keeps
    // its schema defaults instead of copying another controller's settings.
    let calibration: Calibration = serde_json::from_value(serde_json::json!({
        "layout":"steam-controller-2026", "os":std::env::consts::OS,
        "backend":BACKEND, "bindings":bindings
    }))?;
    calibration.validate()?;
    Ok(calibration)
}

pub fn add_layout(catalog: &mut crate::controller_catalog::Catalog) {
    let mut layout = catalog
        .layouts
        .iter()
        .find(|l| l.id == "xbox")
        .expect("standard gamepad layout")
        .clone();
    layout.id = "steam-controller-2026".into();
    layout.name = "Steam Controller 2 (2026) — SDL3 native controls".into();
    layout.source = "https://github.com/libsdl-org/SDL/blob/release-3.4.12/src/joystick/hidapi/SDL_hidapi_steam_triton.c".into();
    layout.notes = "Standard gamepad axes/buttons and additional grip/pad clicks. Touch coordinates, gyro, haptics and pressure sensing are not represented by this mapping.".into();
    for (i, (id, label)) in [
        ("guide", "Steam"),
        ("quick_access", "Quick access"),
        ("right_grip1", "R4"),
        ("left_grip1", "L4"),
        ("right_grip2", "R5"),
        ("left_grip2", "L5"),
        ("left_pad_click", "Left pad click"),
        ("right_pad_click", "Right pad click"),
    ]
    .into_iter()
    .enumerate()
    {
        layout.controls.push(
            serde_json::from_value(serde_json::json!({
                "id":id,"label":label,"x":30.0+(i%4) as f64*12.0,
            "y":18.0+(i/4) as f64*16.0,"group":if i<2 {"menu"} else {"auxiliary"},
                "optional":true,"analog":false
            }))
            .expect("native SDL control schema"),
        );
    }
    catalog.layouts.push(layout);
}

pub enum InputEvent {
    Press {
        key: String,
        binding: InputBinding,
        first: bool,
    },
    Neutral {
        key: String,
        binding: Option<InputBinding>,
        error: String,
    },
}
#[derive(Default)]
pub struct Tracker {
    active: BTreeMap<(u32, i8), InputBinding>,
    capture: Option<InputBinding>,
    initialized: bool,
}
impl Tracker {
    pub fn update(&mut self, key: &str, pad: &Pad) -> Vec<InputEvent> {
        let mut active = BTreeMap::new();
        // Capacitive stick/grip sensors remain active while holding the pad.
        // They must not start calibration or prevent a real button's release.
        for (index, pressed) in pad.buttons.iter().take(22).enumerate() {
            if *pressed {
                let b = binding(index, 0);
                active.insert((b.code, 0), b);
            }
        }
        for (index, value) in pad.axes.iter().take(6).enumerate() {
            let dir = if *value < 0 { -1 } else { 1 };
            let b = binding(index, dir);
            let threshold = if self.active.contains_key(&(b.code, dir)) {
                12451
            } else {
                22281
            };
            if i32::from(*value).abs() >= threshold {
                active.insert((b.code, dir), b);
            }
        }
        let mut events = Vec::new();
        if self.initialized {
            for (identity, b) in &active {
                if !self.active.contains_key(identity) {
                    let first = self.capture.is_none();
                    if first {
                        self.capture = Some(b.clone());
                    }
                    events.push(InputEvent::Press {
                        key: key.into(),
                        binding: b.clone(),
                        first,
                    });
                }
            }
            if active.is_empty()
                && let Some(binding) = self.capture.take()
            {
                events.push(InputEvent::Neutral {
                    key: key.into(),
                    binding: Some(binding),
                    error: String::new(),
                });
            }
        }
        self.initialized = true;
        self.active = active;
        events
    }
}

pub fn run(stop: &AtomicBool, mut publish: impl FnMut(InputEvent)) -> anyhow::Result<()> {
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            // EOF requests normal SDL shutdown and restoration of lizard mode.
            drop(self.0.stdin.take());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while std::time::Instant::now() < deadline {
                if matches!(self.0.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(std::env::current_exe()?)
            .arg("--sdl3-input-stream")
            .env("LUNCHBOX_SDL3_LIBRARY", runtime_path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?,
    );
    let stdout = child.0.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut trackers = BTreeMap::<String, Tracker>::new();
    let mut line = String::new();
    while !stop.load(Ordering::Acquire) {
        line.clear();
        // The helper emits a heartbeat even with no devices, bounding shutdown.
        let bytes = reader.by_ref().take(1024 * 1024).read_line(&mut line)?;
        anyhow::ensure!(
            bytes > 0 && line.ends_with('\n'),
            "SDL3 input helper stopped or sent an oversized frame"
        );
        let frame: Frame = serde_json::from_str(&line)?;
        anyhow::ensure!(
            frame.pads.len() <= 128
                && frame
                    .pads
                    .iter()
                    .all(|p| p.buttons.len() == 26 && p.axes.len() == 6),
            "Invalid SDL3 input frame"
        );
        let pads: Vec<_> = frame
            .pads
            .iter()
            .map(|pad| (key(pad, &frame.pads), pad.clone()))
            .collect();
        {
            let mut state = inventory().lock().unwrap();
            state.pads = pads.clone();
            state.status = format!(
                "SDL3 {} native input · {} Steam Controller 2 devices",
                frame.version,
                pads.len()
            );
        }
        trackers.retain(|key, tracker| {
            if pads.iter().any(|(id, _)| id == key) {
                true
            } else {
                if tracker.capture.take().is_some() {
                    publish(InputEvent::Neutral {
                        key: key.clone(),
                        binding: None,
                        error: "Controller disconnected before release".into(),
                    });
                }
                false
            }
        });
        for (key, pad) in pads {
            for event in trackers.entry(key.clone()).or_default().update(&key, &pad) {
                publish(event);
            }
        }
    }
    inventory().lock().unwrap().pads.clear();
    Ok(())
}

pub fn failed(error: &str) {
    let mut state = inventory().lock().unwrap();
    state.pads.clear();
    state.status = format!("SDL3 native input unavailable: {error}");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pad() -> Pad {
        Pad {
            instance: 1,
            name: "SC2".into(),
            path: None,
            serial: Some("serial".into()),
            vendor: 0x28de,
            product: 0x1302,
            mapping: "a:b0".into(),
            buttons: vec![false; 26],
            axes: vec![0; 6],
        }
    }
    #[test]
    fn sdl_axes_and_buttons_never_become_evdev_codes() {
        for b in [binding(0, 0), binding(1, -1), binding(4, 1)] {
            assert!(is_binding(&b));
            assert!(b.native.is_none());
        }
        assert_eq!(binding(1, -1).logical, "LeftStickUp");
    }
    #[test]
    fn press_release_and_reconnect_do_not_record_held_inputs() {
        let mut tracker = Tracker::default();
        let mut p = pad();
        p.buttons[24] = true; // Holding the grip must not prevent neutral.
        assert!(tracker.update("one", &p).is_empty());
        p.buttons[1] = true;
        assert!(matches!(
            tracker.update("one", &p).as_slice(),
            [InputEvent::Press { first: true, .. }]
        ));
        assert!(tracker.update("one", &p).is_empty());
        p.buttons[1] = false;
        assert!(
            matches!(tracker.update("one",&p).as_slice(),[InputEvent::Neutral{error,..}] if error.is_empty())
        );
        p.buttons[0] = true;
        assert!(Tracker::default().update("one", &p).is_empty());
    }
    #[test]
    fn duplicate_serials_are_not_merged() {
        let a = pad();
        let mut b = a.clone();
        b.instance = 2;
        assert_ne!(
            key(&a, &[a.clone(), b.clone()]),
            key(&b, &[a.clone(), b.clone()])
        );
    }
    #[test]
    fn native_preset_has_all_standard_controls_and_grip_pad_clicks() {
        let preset = calibration().unwrap();
        assert_eq!(preset.backend, BACKEND);
        assert_eq!(preset.bindings.len(), 32);
        assert!(preset.bindings.values().all(valid_binding));
        for control in [
            "left_grip1",
            "right_grip2",
            "left_pad_click",
            "right_pad_click",
        ] {
            assert!(preset.bindings.contains_key(control));
        }
        let mut wrong = preset.clone();
        wrong.backend = "gilrs-0.11".into();
        assert!(wrong.validate().is_err());
    }
}
