//! Standard native handheld gamepads from the pinned module IDII/PortInfo.
use super::{Input, Polarity, binding};
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Gamepad {
    GameBoy,
    GameBoyAdvance,
    Lynx,
    NeoGeoPocket,
    WonderSwan,
    VirtualBoy,
    GameGear,
    MasterSystem,
    PceTwo,
    PceSix,
    PceFastTwo,
    PceFastSix,
    NesTwo,
    NesFourScore,
    NesFamicomFour,
    Snes,
    SnesFaust,
    MdThree,
    MdSix,
    SaturnDigital,
    PlayStationDigital,
    PlayStationDualAnalog,
}

impl Gamepad {
    pub(crate) fn from_profile_id(id: &str) -> Option<Self> {
        [
            Self::GameBoy,
            Self::GameBoyAdvance,
            Self::Lynx,
            Self::NeoGeoPocket,
            Self::WonderSwan,
            Self::VirtualBoy,
            Self::GameGear,
            Self::MasterSystem,
            Self::PceTwo,
            Self::PceSix,
            Self::PceFastTwo,
            Self::PceFastSix,
            Self::NesTwo,
            Self::Snes,
            Self::SnesFaust,
            Self::MdThree,
            Self::MdSix,
            Self::SaturnDigital,
            Self::PlayStationDigital,
            Self::PlayStationDualAnalog,
        ]
        .into_iter()
        .find(|mode| mode.profile_id() == id)
    }
    pub(crate) fn system(self) -> &'static str {
        match self {
            Self::SaturnDigital => "ss",
            Self::PlayStationDigital | Self::PlayStationDualAnalog => "psx",
            Self::MdThree | Self::MdSix => "md",
            Self::Snes => "snes",
            Self::SnesFaust => "snes_faust",
            Self::NesTwo | Self::NesFourScore | Self::NesFamicomFour => "nes",
            Self::GameBoy => "gb",
            Self::GameBoyAdvance => "gba",
            Self::Lynx => "lynx",
            Self::NeoGeoPocket => "ngp",
            Self::WonderSwan => "wswan",
            Self::VirtualBoy => "vb",
            Self::GameGear => "gg",
            Self::MasterSystem => "sms",
            Self::PceTwo | Self::PceSix => "pce",
            Self::PceFastTwo | Self::PceFastSix => "pce_fast",
        }
    }
    pub(crate) fn profile_id(self) -> &'static str {
        match self {
            Self::SaturnDigital => "mednafen:standalone-saturn",
            Self::PlayStationDigital => "mednafen:standalone-psx",
            Self::PlayStationDualAnalog => "mednafen:standalone-psx-dualanalog",
            Self::MdThree => "mednafen:standalone-md3",
            Self::MdSix => "mednafen:standalone-md6",
            Self::Snes => "mednafen:standalone-snes",
            Self::SnesFaust => "mednafen:standalone-snes-faust",
            Self::NesTwo | Self::NesFourScore | Self::NesFamicomFour => "mednafen:standalone-nes",
            Self::GameBoy => "mednafen:standalone-gb",
            Self::GameBoyAdvance => "mednafen:standalone-gba",
            Self::Lynx => "mednafen:standalone-lynx",
            Self::NeoGeoPocket => "mednafen:standalone-ngp",
            Self::WonderSwan => "mednafen:standalone-wswan",
            Self::VirtualBoy => "mednafen:standalone-vb",
            Self::GameGear => "mednafen:standalone-gg",
            Self::MasterSystem => "mednafen:standalone-sms",
            Self::PceTwo => "mednafen:standalone-pce2",
            Self::PceSix => "mednafen:standalone-pce6",
            Self::PceFastTwo => "mednafen:standalone-pce-fast2",
            Self::PceFastSix => "mednafen:standalone-pce-fast6",
        }
    }

    pub(crate) fn controls(self) -> &'static [&'static str] {
        match self {
            Self::SaturnDigital => super::saturn::CONTROLS,
            Self::PlayStationDigital => super::psx::CONTROLS,
            Self::PlayStationDualAnalog => super::psx::ANALOG_CONTROLS,
            Self::MdThree => super::md::Pad::Three.controls(),
            Self::MdSix => super::md::Pad::Six.controls(),
            Self::Snes | Self::SnesFaust => super::snes::CONTROLS,
            Self::NesTwo | Self::NesFourScore | Self::NesFamicomFour => super::nes::CONTROLS,
            Self::PceTwo | Self::PceFastTwo => super::pce::Mode::Two.controls(),
            Self::PceSix | Self::PceFastSix => super::pce::Mode::Six.controls(),
            Self::MasterSystem => &["up", "down", "left", "right", "fire1", "fire2", "pause"],
            Self::GameGear => &["up", "down", "left", "right", "button1", "button2", "start"],
            Self::VirtualBoy => &[
                "up-l", "down-l", "left-l", "right-l", "up-r", "down-r", "left-r", "right-r", "a",
                "b", "lt", "rt", "select", "start",
            ],
            Self::WonderSwan => &[
                "up-x", "down-x", "left-x", "right-x", "up-y", "down-y", "left-y", "right-y", "a",
                "b", "start",
            ],
            Self::NeoGeoPocket => &["up", "down", "left", "right", "a", "b", "option"],
            Self::Lynx => &[
                "up", "down", "left", "right", "a", "b", "option_1", "option_2", "pause",
            ],
            Self::GameBoy => &["up", "down", "left", "right", "a", "b", "select", "start"],
            Self::GameBoyAdvance => &[
                "up",
                "down",
                "left",
                "right",
                "a",
                "b",
                "select",
                "start",
                "shoulder_l",
                "shoulder_r",
            ],
        }
    }

    pub(crate) fn assignments(
        self,
        native_id: &str,
        controls: &BTreeMap<String, Input>,
    ) -> Result<BTreeMap<String, String>> {
        self.assignments_for_player(1, native_id, controls)
    }

    pub(crate) fn max_players(self) -> u8 {
        match self {
            Self::SaturnDigital => 12,
            Self::PlayStationDigital | Self::PlayStationDualAnalog => 8,
            Self::MdThree | Self::MdSix => 8,
            Self::Snes | Self::SnesFaust => 8,
            Self::NesTwo => 2,
            Self::NesFourScore | Self::NesFamicomFour => 4,
            Self::MasterSystem => 2,
            Self::PceTwo | Self::PceSix | Self::PceFastTwo | Self::PceFastSix => 5,
            _ => 1,
        }
    }

    fn port(self, player: u8) -> Result<&'static str> {
        ensure!(
            (1..=self.max_players()).contains(&player),
            "Invalid Mednafen player port"
        );
        Ok(if self == Self::MasterSystem {
            if player == 1 { "port1" } else { "port2" }
        } else {
            "builtin"
        })
    }

    pub(crate) fn assignments_for_player(
        self,
        player: u8,
        native_id: &str,
        controls: &BTreeMap<String, Input>,
    ) -> Result<BTreeMap<String, String>> {
        if self == Self::PlayStationDigital {
            return super::psx::assignments(
                [true; 2],
                &[super::psx::Player {
                    port: player,
                    native_id,
                    controls,
                }],
            );
        }
        if self == Self::PlayStationDualAnalog {
            return super::psx::dual_analog_assignments(
                [true; 2],
                &[super::psx::Player {
                    port: player,
                    native_id,
                    controls,
                }],
            );
        }
        if self == Self::SaturnDigital {
            return super::saturn::assignments(
                [true, true],
                &[super::saturn::Player {
                    port: player,
                    native_id,
                    controls,
                }],
            );
        }
        if matches!(self, Self::MdThree | Self::MdSix) {
            return super::md::assignments(
                super::md::Tap::Dual,
                &[super::md::Player {
                    port: player,
                    pad: if self == Self::MdThree {
                        super::md::Pad::Three
                    } else {
                        super::md::Pad::Six
                    },
                    native_id,
                    controls,
                }],
            );
        }
        if self == Self::SnesFaust {
            return super::snes::faust_assignments(&[super::snes::Player {
                port: player,
                native_id,
                controls,
            }]);
        }
        if self == Self::Snes {
            return super::snes::assignments(&[super::snes::Player {
                port: player,
                native_id,
                controls,
            }]);
        }
        if self.system() == "nes" {
            let adapter = match self {
                Self::NesTwo => super::nes::Adapter::TwoPlayer,
                Self::NesFourScore => super::nes::Adapter::FourScore,
                _ => super::nes::Adapter::FamicomFourPlayer,
            };
            let prefix = format!("nes.input.port{player}");
            return Ok(super::nes::assignments(
                adapter,
                &[super::nes::Player {
                    port: player,
                    native_id,
                    controls,
                }],
            )?
            .into_iter()
            .filter(|(key, _)| key == &prefix || key.starts_with(&format!("{prefix}.")))
            .collect());
        }
        if matches!(
            self,
            Self::PceTwo | Self::PceSix | Self::PceFastTwo | Self::PceFastSix
        ) {
            let mode = if matches!(self, Self::PceTwo | Self::PceFastTwo) {
                super::pce::Mode::Two
            } else {
                super::pce::Mode::Six
            };
            let prefix = format!("pce.input.port{player}");
            return Ok(super::pce::assignments(&[super::pce::Player {
                port: player,
                native_id,
                mode,
                controls,
            }])?
            .into_iter()
            .filter(|(key, _)| key == &prefix || key.starts_with(&format!("{prefix}.")))
            // Both pinned modules declare the same pad/port contract. The
            // fast module has no pce.input.multitap setting to translate.
            .map(|(key, value)| {
                (
                    key.replacen("pce.", &format!("{}.", self.system()), 1),
                    value,
                )
            })
            .collect());
        }
        let port = self.port(player)?;
        ensure!(
            controls.len() == self.controls().len()
                && self
                    .controls()
                    .iter()
                    .all(|key| controls.contains_key(*key)),
            "Mednafen gamepad requires every native gameplay control"
        );
        let system = self.system();
        let mut result = BTreeMap::new();
        if self == Self::WonderSwan {
            // Explicitly select physical X/Y identities, not gamepadraa's
            // rotation-adjusted A'/B' device, in every private config layer.
            result.insert("wswan.input.builtin".into(), "gamepad".into());
        }
        let mut owners = BTreeSet::new();
        for key in self.controls() {
            let input = controls[*key];
            ensure!(
                !matches!(
                    input,
                    Input::Absolute {
                        polarity: Polarity::Full | Polarity::FullReversed,
                        ..
                    }
                ),
                "Mednafen digital gamepad requires a signed half-axis, not a full-range axis"
            );
            ensure!(
                owners.insert(input),
                "Mednafen gameplay input has competing owners"
            );
            result.insert(
                format!("{system}.input.{port}.gamepad.{key}"),
                binding(native_id, input, 4096)?,
            );
        }
        // ButtonCR declares rapid-fire settings, including both Lynx options.
        let rapid: &[&str] = match self {
            Self::MasterSystem => &["rapid_fire1", "rapid_fire2"],
            Self::GameGear => &["rapid_button1", "rapid_button2"],
            Self::Lynx => &["rapid_a", "rapid_b", "rapid_option_1", "rapid_option_2"],
            _ => &["rapid_a", "rapid_b"],
        };
        for key in rapid {
            result.insert(
                format!("{system}.input.{port}.gamepad.{key}"),
                String::new(),
            );
        }
        Ok(result)
    }

    /// SMS has no disconnected device choice. Clear every gameplay binding on
    /// an unused port so native defaults cannot share an active controller.
    pub(crate) fn unused_port(self, player: u8) -> Result<BTreeMap<String, String>> {
        if self.system() == "nes" {
            ensure!((1..=4).contains(&player), "Invalid NES port");
            return Ok(BTreeMap::from([(
                format!("nes.input.port{player}"),
                "none".into(),
            )]));
        }
        if matches!(
            self,
            Self::PceTwo | Self::PceSix | Self::PceFastTwo | Self::PceFastSix
        ) {
            ensure!((1..=5).contains(&player), "Invalid PCE player port");
            return Ok(BTreeMap::from([(
                format!("{}.input.port{player}", self.system()),
                "none".into(),
            )]));
        }
        ensure!(
            self == Self::MasterSystem,
            "Only SMS has optional native ports"
        );
        let port = self.port(player)?;
        Ok(self
            .controls()
            .iter()
            .copied()
            .chain(["rapid_fire1", "rapid_fire2"])
            .map(|key| (format!("sms.input.{port}.gamepad.{key}"), String::new()))
            .collect())
    }
}
