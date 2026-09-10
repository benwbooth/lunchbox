//! Guided targets backed by the existing standalone writers, never RetroPad
//! aliases. Keep native vocabulary in the writer's single route table.
use crate::controller_catalog::{Catalog, EmulatorProfile};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) fn add_profiles(db: &mut Catalog) -> Result<()> {
    let mut ds = db
        .layout("nds-stylus-controls")
        .context("Missing DS geometry")?
        .clone();
    ds.id = "nds-native-buttons".into();
    ds.name = "Nintendo DS — buttons (touchscreen uses mouse)".into();
    ds.notes = "Native melonDS button controls. Touchscreen, microphone and lid are not mapped by this contract.".into();
    let routes = crate::controller_melonds::visual_routes();
    ds.controls
        .retain(|control| routes.contains_key(control.id.as_str()));
    db.layouts.push(ds);
    let mut msx = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    msx.id = "openmsx-native-joystick".into();
    msx.name = "MSX — digital joystick".into();
    msx.family = "two-button".into();
    msx.source = "https://openmsx.org/".into();
    msx.notes = "Native openMSX control-port joystick with two triggers mapped through the msxjoystickN_config dicts.".into();
    msx.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("a", "Trigger A", 70.0, 50.0, "face", true),
        ("b", "Trigger B", 80.0, 35.0, "face", false),
    ]
    .into_iter()
    .map(
        |(id, label, x, y, group, required)| crate::controller_catalog::Control {
            id: id.into(),
            label: label.into(),
            x,
            y,
            group: group.into(),
            optional: !required,
            analog: false,
            pressure: false,
            repeat_of: None,
        },
    )
    .collect();
    db.layouts.push(msx);
    let mut st = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    st.id = "hatari-native-joystick".into();
    st.name = "Atari ST — digital joystick".into();
    st.family = "two-button".into();
    st.source = "https://hatari-emu.org/".into();
    st.notes = "Native Hatari control-port joystick. Directions are pinned to SDL axes 0/1 with hat 0 override; fire 2/3 are joyport expansions.".into();
    st.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("fire", "Fire", 70.0, 50.0, "face", true),
        ("fire2", "Fire 2", 80.0, 35.0, "face", false),
        ("fire3", "Fire 3", 80.0, 65.0, "face", false),
    ]
    .into_iter()
    .map(
        |(id, label, x, y, group, required)| crate::controller_catalog::Control {
            id: id.into(),
            label: label.into(),
            x,
            y,
            group: group.into(),
            optional: !required,
            analog: false,
            pressure: false,
            repeat_of: None,
        },
    )
    .collect();
    db.layouts.push(st);
    let mut joystick = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    joystick.id = "vice-joystick-panel".into();
    joystick.name = "Commodore — digital joystick".into();
    joystick.family = "two-button".into();
    joystick.source = "https://sourceforge.net/p/vice-emu/".into();
    joystick.notes = "Native VICE control-port joystick pins. Fire2/fire3 serve the extra buttons of joyport expansions; keysets, paddles and potentiometer axes are separate contracts.".into();
    joystick.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("fire", "Fire", 70.0, 50.0, "face", true),
        ("fire2", "Fire 2", 80.0, 35.0, "face", false),
        ("fire3", "Fire 3", 80.0, 65.0, "face", false),
    ]
    .into_iter()
    .map(
        |(id, label, x, y, group, required)| crate::controller_catalog::Control {
            id: id.into(),
            label: label.into(),
            x,
            y,
            group: group.into(),
            optional: !required,
            analog: false,
            pressure: false,
            repeat_of: None,
        },
    )
    .collect();
    db.layouts.push(joystick);

    for (core, layout, players, platforms, source) in [
        (
            "mesen2",
            "nes",
            1,
            vec![
                "Nintendo Entertainment System",
                "Nintendo Famicom Disk System",
            ],
            "https://github.com/SourMesen/Mesen2/blob/b9fa69ddc6d0a331fb103fdb5eef6904305703c2/Linux/LinuxGameController.cpp",
        ),
        (
            "openmsx",
            "openmsx-native-joystick",
            2,
            vec![
                "Microsoft MSX",
                "Microsoft MSX2",
                "Microsoft MSX2+",
                "Spectravideo",
            ],
            "https://github.com/openMSX/openMSX/blob/25179d6b8d5ec69ad68252f3854721c9a02594eb/src/input/MSXJoystick.cc",
        ),
        (
            "desmume",
            "nds-native-buttons",
            1,
            vec!["Nintendo DS"],
            "https://github.com/TASEmulators/desmume/blob/b3915949700be824253a35affa7f7b8248e84e46/desmume/src/frontend/posix/shared/ctrlssdl.cpp",
        ),
        (
            "hatari",
            "hatari-native-joystick",
            2,
            vec!["Atari ST"],
            "https://github.com/hatari/hatari/blob/11964da62914bf232ca84eacf0cedf1d25223e08/src/sdl/joy_ui.c",
        ),
        (
            "vice",
            "vice-joystick-panel",
            2,
            vec![
                "Commodore 64",
                "Commodore 128",
                "Commodore Plus 4",
                "Commodore VIC-20",
                "Commodore PET",
                "Commodore MAX Machine",
            ],
            "https://sourceforge.net/p/vice-emu/code/HEAD/tree/trunk/vice/src/joyport/joystick.c",
        ),
        (
            "stella",
            "atari2600-stella-panel",
            2,
            vec!["Atari 2600"],
            "https://github.com/stella-emu/stella/blob/c65c845c8686c81698ffbd2fc9dfc5ccea5b32a1/src/common/PJoystickHandler.cxx",
        ),
        (
            "bsnes",
            "snes",
            2,
            vec!["Super Nintendo Entertainment System"],
            "https://github.com/bsnes-emu/bsnes/blob/7d5aa1e656b9171524d01b1b22917197d8121cb4/bsnes/target-bsnes/input/input.cpp",
        ),
        (
            "pcsx2",
            "dualshock",
            8,
            vec!["Sony Playstation 2"],
            "https://github.com/PCSX2/pcsx2/tree/98697735f1bb1a1452d975251269abd1019876d1/pcsx2/SIO/Pad",
        ),
        (
            "rpcs3",
            "dualshock",
            7,
            vec!["Sony Playstation 3"],
            "https://github.com/RPCS3/rpcs3/tree/54014a7de4b2ccec98c9c0cb7dbebec0606c5cd6/rpcs3/Input",
        ),
        (
            "melonds",
            "nds-native-buttons",
            1,
            vec!["Nintendo DS"],
            "https://github.com/melonDS-emu/melonDS/tree/906e9ebb27da8c6a715cd7abab4abfe8a8d29427/src/frontend/qt_sdl",
        ),
    ] {
        add(db, core, layout, players, &platforms, source)?;
    }
    for layout in ["arcade-six-button", "arcade-eight-button"] {
        add(
            db,
            "flycast",
            layout,
            4,
            &["Arcade", "Sega Naomi", "Sega Naomi 2", "Sammy Atomiswave"],
            "https://github.com/flyinghead/flycast/tree/fb286f777ce690ef8acf3359a75ab84b61566ad9/core/input",
        )?;
        add(
            db,
            "mame",
            layout,
            8,
            &["Arcade"],
            "https://github.com/mamedev/mame/blob/ec9abd86c6c9029f67e9cf4908ef5426b78d3eab/docs/source/advanced/ctrlr_config.rst",
        )?;
    }
    crate::controller_bizhawk::guided::add_profiles(db)?;
    Ok(())
}

pub(crate) fn add(
    db: &mut Catalog,
    core: &str,
    layout: &str,
    players: usize,
    platforms: &[&str],
    source: &str,
) -> Result<()> {
    let bindings = routes(core, layout).context("Unknown native target contract")?;
    let name = db
        .layout(layout)
        .context("Missing native target layout")?
        .name
        .clone();
    db.emulator_profiles.push(serde_json::from_value(serde_json::json!({
        "id": format!("{core}:standalone-{layout}"),
        "name": format!("{core} · {name}"), "core": core, "target_layout": layout,
        "transport": format!("{core}-native-settings"), "status": "documented",
        "source": source, "native_launch": {"platforms": platforms, "max_players": players},
        "conditions": ["Guided players and button choices feed the native writer. A matching Linux runtime setup and physical calibration are required. Source implementation only; runtime compatibility is unverified."],
        "bindings": bindings
    }))?);
    Ok(())
}

/// Preview labels for the VICE joystick pins, kept as static strings so the
/// route table stays one type across adapters.
const VICE_PIN_ROUTES: [(&str, &str); 7] = [
    ("up", "pin 1 Up"),
    ("down", "pin 2 Down"),
    ("left", "pin 4 Left"),
    ("right", "pin 8 Right"),
    ("fire", "pin 16 Fire"),
    ("fire2", "pin 32 Fire 2"),
    ("fire3", "pin 64 Fire 3"),
];

fn routes(core: &str, layout: &str) -> Option<BTreeMap<String, String>> {
    if core == "bizhawk" {
        return crate::controller_bizhawk::guided::routes(layout);
    }
    if core == "mame" {
        let panel = match layout {
            "arcade-six-button" => crate::controller_mame_native::Panel::Six,
            "arcade-eight-button" => crate::controller_mame_native::Panel::Eight,
            _ => return None,
        };
        // Preview shows the P1 vocabulary; the writer expands the same routes
        // with the actual assigned player number for every native port.
        return Some(
            panel
                .routes(1)
                .into_iter()
                .map(|(target, output)| {
                    (
                        if target == "coin" {
                            "select".into()
                        } else {
                            target
                        },
                        output,
                    )
                })
                .collect(),
        );
    }
    if core == "flycast" {
        let panel = match layout {
            "arcade-six-button" => crate::controller_flycast_native::arcade::Panel::Six,
            "arcade-eight-button" => crate::controller_flycast_native::arcade::Panel::Eight,
            _ => return None,
        };
        return Some(
            panel
                .routes()
                .into_iter()
                .map(|(target, output)| {
                    (
                        if target == "coin" {
                            "select".into()
                        } else {
                            target
                        },
                        output,
                    )
                })
                .collect(),
        );
    }
    let routes = match (core, layout) {
        ("bsnes", "snes") => crate::controller_bsnes::CONTROLS
            .into_iter()
            .collect::<BTreeMap<&str, &str>>(),
        ("stella", "atari2600-stella-panel") => crate::controller_stella_native::CONTROLS
            .into_iter()
            .collect::<BTreeMap<&str, &str>>(),
        ("mesen2", "nes") => crate::controller_mesen2_native::CONTROLS
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("openmsx", "openmsx-native-joystick") => crate::controller_openmsx_native::CONTROLS
            .iter()
            .map(|(target, key, _)| (*target, *key))
            .collect::<BTreeMap<&str, &str>>(),
        ("desmume", "nds-native-buttons") => crate::controller_desmume_native::KEYS
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("hatari", "hatari-native-joystick") => crate::controller_hatari_native::CONTROLS
            .iter()
            .map(|(target, output, _)| (*target, *output))
            .collect::<BTreeMap<&str, &str>>(),
        ("vice", "vice-joystick-panel") => VICE_PIN_ROUTES
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("pcsx2", "dualshock") => crate::controller_pcsx2::visual_routes(),
        ("rpcs3", "dualshock") => crate::controller_rpcs3::visual_routes(),
        ("melonds", "nds-native-buttons") => crate::controller_melonds::visual_routes(),
        _ => return None,
    };
    Some(
        routes
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
    )
}

pub(crate) fn validate(profile: &EmulatorProfile) -> Result<()> {
    ensure!(
        profile.transport == format!("{}-native-settings", profile.core)
            && profile.native_launch.is_some()
            && profile.retroarch_launch.is_none(),
        "Native target has the wrong transport"
    );
    ensure!(
        routes(&profile.core, &profile.target_layout).as_ref() == Some(&profile.bindings),
        "Native target must exactly match its writer's control routes"
    );
    Ok(())
}

pub(crate) fn valid_output(profile: &EmulatorProfile, target: &str, output: &str) -> bool {
    routes(&profile.core, &profile.target_layout)
        .and_then(|routes| routes.get(target).cloned())
        .as_deref()
        == Some(output)
}
