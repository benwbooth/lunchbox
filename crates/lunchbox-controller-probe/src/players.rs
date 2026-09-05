//! Event-order player-slot projection for a verified DuckStation input contract.
//! A separate helper process cannot guarantee a later process sees the same
//! topology. The actual emulator startup still needs to confirm this projection.
use crate::{Device, GetString, string};
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::time::{Duration, Instant};

pub const CONTRACT: &str = "0a53bc47c";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub instance_id: u32,
    pub path: Option<String>,
    pub name: Option<String>,
    pub is_gamepad: bool,
    /// Queried from the open device, not the pre-open inventory hint.
    pub opened_player_index: i32,
    pub projected_player_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProbe {
    pub duckstation_revision: String,
    /// In observed successful-open event order, including non-gamepad joysticks.
    pub assignments: Vec<Assignment>,
    pub events_processed: u32,
}

impl PlayerProbe {
    pub fn at_path(&self, path: &str) -> Result<&Assignment> {
        ensure!(!path.is_empty(), "Nonempty player device path required");
        let mut matches = self
            .assignments
            .iter()
            .filter(|a| a.path.as_deref() == Some(path));
        let assignment = matches
            .next()
            .context("No player assignment for exact path")?;
        ensure!(matches.next().is_none(), "Ambiguous player device path");
        Ok(assignment)
    }

    /// Confirm one startup against the actual revision's verbose device log.
    /// This is not identity verification across future reconnects.
    pub fn verify_startup_log(&self, log: &str) -> Result<()> {
        ensure!(
            self.duckstation_revision == CONTRACT,
            "Unknown player log contract"
        );
        ensure!(
            !self.assignments.is_empty(),
            "No devices to verify in emulator startup"
        );
        let log = plain_log(log);
        let mut actual = Vec::new();
        for line in log.lines() {
            let Some(offset) = line.find("Opened ") else {
                continue;
            };
            let tail = &line[offset + 7..];
            let (is_gamepad, tail) = if let Some(t) = tail.strip_prefix("game controller ") {
                (true, t)
            } else if let Some(t) = tail.strip_prefix("joystick ") {
                (false, t)
            } else {
                continue;
            };
            let (index, tail) = tail
                .split_once(" (instance id ")
                .context("Malformed device-open log")?;
            let (instance, tail) = tail
                .split_once(", player id ")
                .context("Malformed instance log")?;
            let (player, name) = tail.split_once("): ").context("Malformed player log")?;
            let instance: u32 = instance.parse()?;
            ensure!(
                index.parse::<u32>()? == instance,
                "Device-open index and instance disagree"
            );
            actual.push((instance, player.parse::<u32>()?, is_gamepad, name));
        }
        ensure!(
            actual.len() == self.assignments.len(),
            "Expected {} device opens; emulator logged {}",
            self.assignments.len(),
            actual.len()
        );
        for (expected, actual) in self.assignments.iter().zip(actual) {
            ensure!(
                actual
                    == (
                        expected.instance_id,
                        expected.projected_player_id,
                        expected.is_gamepad,
                        expected.name.as_deref().unwrap_or("Unknown Device")
                    ),
                "Actual emulator player assignment differs for instance {}",
                expected.instance_id
            );
        }
        Ok(())
    }
}

/// DuckStation writes ANSI SGR sequences even when stdout is redirected.
fn plain_log(log: &str) -> String {
    let mut output = String::with_capacity(log.len());
    let mut chars = log.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

fn assign_slot(used: &BTreeSet<u32>, reported: i32) -> Result<u32> {
    if let Ok(reported) = u32::try_from(reported) {
        ensure!(reported <= 1024, "Unexpected SDL player index");
        if !used.contains(&reported) {
            return Ok(reported);
        }
    }
    (0..=1024)
        .find(|id| !used.contains(id))
        .context("Too many player slots")
}

// SDL_Event is a 128-byte union; these are the public common/device fields.
// Alignment is at least 8 on all supported 64-bit targets. SDL3 keeps this ABI.
#[repr(C, align(8))]
struct Event {
    bytes: [u8; 128],
}
impl Event {
    fn kind(&self) -> u32 {
        u32::from_ne_bytes(self.bytes[0..4].try_into().unwrap())
    }
    fn instance(&self) -> u32 {
        u32::from_ne_bytes(self.bytes[16..20].try_into().unwrap())
    }
}
const _: () = assert!(std::mem::size_of::<Event>() == 128);

struct Opened {
    pointer: *mut c_void,
    close: unsafe extern "C" fn(*mut c_void),
}
impl Drop for Opened {
    fn drop(&mut self) {
        unsafe { (self.close)(self.pointer) }
    }
}

pub(crate) struct ProbeSession {
    pub report: PlayerProbe,
    // Keep every opened device alive through subsequent binding queries.
    _devices: Vec<Opened>,
}

/// Library and SDL subsystems must outlive the returned session. Call before
/// any other device opens or event polling in this fresh helper process.
pub(crate) unsafe fn observe(
    library: &Library,
    devices: &[Device],
    get_error: GetString,
) -> Result<ProbeSession> {
    unsafe {
        let poll = *library.get::<unsafe extern "C" fn(*mut Event) -> bool>(b"SDL_PollEvent\0")?;
        let open_gamepad =
            *library.get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"SDL_OpenGamepad\0")?;
        let close_gamepad =
            *library.get::<unsafe extern "C" fn(*mut c_void)>(b"SDL_CloseGamepad\0")?;
        let open_joystick =
            *library.get::<unsafe extern "C" fn(u32) -> *mut c_void>(b"SDL_OpenJoystick\0")?;
        let close_joystick =
            *library.get::<unsafe extern "C" fn(*mut c_void)>(b"SDL_CloseJoystick\0")?;
        let gamepad_player = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_GetGamepadPlayerIndex\0")?;
        let joystick_player = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"SDL_GetJoystickPlayerIndex\0")?;
        let is_gamepad = *library.get::<unsafe extern "C" fn(u32) -> bool>(b"SDL_IsGamepad\0")?;
        let gamepad_name = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> *const std::ffi::c_char>(
                b"SDL_GetGamepadName\0",
            )?;
        let joystick_name = *library
            .get::<unsafe extern "C" fn(*mut c_void) -> *const std::ffi::c_char>(
                b"SDL_GetJoystickName\0",
            )?;
        let mut result = ProbeSession {
            report: PlayerProbe {
                duckstation_revision: CONTRACT.into(),
                assignments: vec![],
                events_processed: 0,
            },
            _devices: vec![],
        };
        let mut opened_ids = BTreeSet::new();
        let mut used_slots = BTreeSet::new();
        let started = Instant::now();
        let mut last_topology = started;
        loop {
            ensure!(
                started.elapsed() < Duration::from_secs(2),
                "SDL player topology did not settle; retry with a fresh snapshot"
            );
            let mut event = Event { bytes: [0; 128] };
            if !poll(&mut event) {
                if opened_ids.len() == devices.len()
                    && last_topology.elapsed() >= Duration::from_millis(50)
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            result.report.events_processed += 1;
            ensure!(
                result.report.events_processed < 100_000,
                "SDL event flood while resolving players"
            );
            let kind = event.kind();
            // Removal or remapping invalidates this entire startup projection.
            ensure!(
                !matches!(kind, 0x606 | 0x654 | 0x655),
                "SDL device removed or remapped during player probe; retry"
            );
            if !matches!(kind, 0x605 | 0x653) {
                continue;
            }
            last_topology = Instant::now();
            let id = event.instance();
            let device = devices
                .iter()
                .find(|d| d.instance_id == id)
                .context("New SDL device appeared during player probe; retry")?;
            let gamepad = is_gamepad(id);
            ensure!(
                gamepad == device.is_gamepad,
                "SDL device classification changed during player probe"
            );
            if kind == 0x605 && gamepad {
                continue;
            }
            ensure!((kind == 0x653) == gamepad, "Inconsistent SDL added event");
            ensure!(
                !opened_ids.contains(&id),
                "Duplicate SDL added event; cannot establish player identity"
            );
            let (open, close, player) = if gamepad {
                (open_gamepad, close_gamepad, gamepad_player)
            } else {
                (open_joystick, close_joystick, joystick_player)
            };
            let pointer = open(id);
            ensure!(
                !pointer.is_null(),
                "Opening device {id} for player projection failed: {:?}",
                string(get_error())?
            );
            let opened = Opened { pointer, close };
            let reported = player(pointer);
            let slot = assign_slot(&used_slots, reported)?;
            used_slots.insert(slot);
            opened_ids.insert(id);
            result.report.assignments.push(Assignment {
                instance_id: id,
                path: device.path.clone(),
                name: string(if gamepad {
                    gamepad_name(pointer)
                } else {
                    joystick_name(pointer)
                })?,
                is_gamepad: gamepad,
                opened_player_index: reported,
                projected_player_id: slot,
            });
            result._devices.push(opened);
        }
        Ok(result)
    }
}

/// Recheck after binding queries, before returning the projection.
pub(crate) unsafe fn ensure_no_topology_events(library: &Library) -> Result<()> {
    unsafe {
        let pump = *library.get::<unsafe extern "C" fn()>(b"SDL_PumpEvents\0")?;
        let has_events =
            *library.get::<unsafe extern "C" fn(u32, u32) -> bool>(b"SDL_HasEvents\0")?;
        pump();
        ensure!(
            !has_events(0x605, 0x606) && !has_events(0x653, 0x655),
            "SDL topology changed after player projection; retry"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_gamepads_and_collisions_take_real_slots() {
        let mut used = BTreeSet::new();
        let observed: Vec<_> = [-1, 0, -1, 1, 2, 3]
            .into_iter()
            .map(|reported| {
                let id = assign_slot(&used, reported).unwrap();
                used.insert(id);
                id
            })
            .collect();
        assert_eq!(observed, [0, 1, 2, 3, 4, 5]);
    }
    #[test]
    fn valid_requested_slots_are_not_replaced_by_enumeration_order() {
        assert_eq!(assign_slot(&BTreeSet::from([0, 1]), 7).unwrap(), 7);
        assert_eq!(assign_slot(&BTreeSet::from([0, 2, 7]), 7).unwrap(), 1);
        assert!(assign_slot(&BTreeSet::new(), 2000).is_err());
    }
    #[test]
    fn sdl_device_event_abi_offsets() {
        let mut event = Event { bytes: [0; 128] };
        event.bytes[0..4].copy_from_slice(&0x653u32.to_ne_bytes());
        event.bytes[16..20].copy_from_slice(&42u32.to_ne_bytes());
        assert_eq!(event.kind(), 0x653);
        assert_eq!(event.instance(), 42);
    }
    #[test]
    fn actual_log_requires_every_device_and_correct_order_and_player() {
        let probe = PlayerProbe {
            duckstation_revision: CONTRACT.into(),
            events_processed: 2,
            assignments: vec![Assignment {
                instance_id: 5,
                path: Some("/dev/input/js4".into()),
                name: Some("Generic Pad".into()),
                is_gamepad: true,
                opened_player_index: 2,
                projected_player_id: 4,
            }],
        };
        let log = "[V/SDL] Opened game controller 5 (instance id 5, player id 4): Generic Pad\n";
        probe.verify_startup_log(log).unwrap();
        probe
            .verify_startup_log(&format!("\u{1b}[32m{log}\u{1b}[0m"))
            .unwrap();
        probe
            .verify_startup_log(&log.replace("Generic Pad", "Generic Pad\u{1b}[0m"))
            .unwrap();
        assert!(
            probe
                .verify_startup_log(&log.replace("player id 4", "player id 2"))
                .is_err()
        );
        assert!(probe.verify_startup_log(&format!("{log}{log}")).is_err());
        assert!(probe.verify_startup_log("").is_err());
        assert!(
            probe
                .verify_startup_log(&log.replace("Generic Pad", "Another Pad"))
                .is_err()
        );
    }

    #[test]
    fn identical_names_require_exact_paths_and_event_order() {
        let mut probe = PlayerProbe {
            duckstation_revision: CONTRACT.into(),
            events_processed: 4,
            assignments: (0..2)
                .map(|i| Assignment {
                    instance_id: i + 1,
                    path: Some(format!("/dev/input/js{}", i + 4)),
                    name: Some("Same Pad".into()),
                    is_gamepad: true,
                    opened_player_index: i as i32,
                    projected_player_id: i,
                })
                .collect(),
        };
        assert_eq!(
            probe.at_path("/dev/input/js5").unwrap().projected_player_id,
            1
        );
        assert!(probe.at_path("Same Pad").is_err());
        assert!(probe.at_path("").is_err());
        let first = "Opened game controller 1 (instance id 1, player id 0): Same Pad\n";
        let second = "Opened game controller 2 (instance id 2, player id 1): Same Pad\n";
        probe
            .verify_startup_log(&format!("{first}{second}"))
            .unwrap();
        assert!(
            probe
                .verify_startup_log(&format!("{second}{first}"))
                .is_err()
        );
        probe.assignments[1].path = probe.assignments[0].path.clone();
        assert!(probe.at_path("/dev/input/js4").is_err());
    }
}
