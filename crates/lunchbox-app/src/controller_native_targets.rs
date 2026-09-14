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
    let mut pointer = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    pointer.id = "scummvm-default-actions".into();
    pointer.name = "ScummVM — default actions".into();
    pointer.family = "two-button".into();
    pointer.source = "https://scummvm.org/".into();
    pointer.notes = "Native ScummVM engine-default game keymap actions driven through the SDL standard gamepad.".into();
    pointer.controls = [
        ("a", "Left click (LCLK)", 70.0, 50.0, "face", true),
        ("b", "Right click (RCLK)", 60.0, 65.0, "face", true),
        ("x", "Skip line (SKLI)", 80.0, 35.0, "face", true),
        ("y", "Skip (SKIP)", 90.0, 50.0, "face", true),
        ("l", "Game menu (MENU)", 5.0, 35.0, "shoulder", true),
        ("start", "Confirm (RETURN)", 30.0, 90.0, "menu", true),
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
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
    db.layouts.push(pointer);
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
    let mut atari_plus_plus = db
        .layout("hatari-native-joystick")
        .context("Missing digital joystick reference layout")?
        .clone();
    atari_plus_plus.id = "atari-plus-plus-native-joystick".into();
    atari_plus_plus.name = "Atari 8-bit — Atari++ AnalogJoystick".into();
    atari_plus_plus.source = "https://sources.debian.org/src/atari++/1.85-1/".into();
    atari_plus_plus.notes = "Atari++ native Linux AnalogJoystick input. Directions require two joydev axes among 0..3; four source-defined abstract buttons are mapped explicitly. SDL, keyboard, paddle and 5200 analog modes are separate contracts.".into();
    atari_plus_plus
        .controls
        .push(crate::controller_catalog::Control {
            id: "fire4".into(),
            label: "Fire 4".into(),
            x: 90.0,
            y: 50.0,
            group: "face".into(),
            optional: false,
            analog: false,
            pressure: false,
            repeat_of: None,
        });
    for control in &mut atari_plus_plus.controls {
        control.optional = false;
    }
    db.layouts.push(atari_plus_plus);
    let mut aranym = db
        .layout("hatari-native-joystick")
        .context("Missing digital joystick reference layout")?
        .clone();
    aranym.id = "aranym-ikbd-joystick".into();
    aranym.name = "Atari ST — ARAnyM IKBD joystick".into();
    aranym.source = "https://github.com/aranym/aranym".into();
    aranym.notes = "ARAnyM native IKBD joystick. The pinned source accepts only SDL axes 0/1 or a cardinal hat for directions and treats every physical button as Fire; Jaguar joypads are a separate contract.".into();
    aranym.controls.retain(|control| {
        matches!(
            control.id.as_str(),
            "up" | "down" | "left" | "right" | "fire"
        )
    });
    for control in &mut aranym.controls {
        control.optional = false;
    }
    db.layouts.push(aranym);
    let mut atari800 = db
        .layout("aranym-ikbd-joystick")
        .context("Missing five-control joystick reference layout")?
        .clone();
    atari800.id = "atari800-native-joystick".into();
    atari800.name = "Atari 8-bit — Atari800 digital joystick".into();
    atari800.source = "https://github.com/atari800/atari800".into();
    atari800.notes = "Atari800 native SDL2 host joystick. Directions must resolve to source-supported axes 0/1 or 2/3, or cardinal hat 0; Fire must be raw button 0..14. The runtime selects a physical device by exact SDL name plus duplicate-name slot.".into();
    db.layouts.push(atari800);
    let mut nanoboyadvance = db
        .layout("gba")
        .context("Missing Game Boy Advance reference layout")?
        .clone();
    nanoboyadvance.id = "gba-controller".into();
    nanoboyadvance.name = "Game Boy Advance — NanoBoyAdvance controller".into();
    nanoboyadvance.source = format!(
        "https://github.com/nba-emu/NanoBoyAdvance/tree/{}",
        crate::controller_nanoboyadvance_native::SOURCE_COMMIT
    );
    nanoboyadvance.notes = "NanoBoyAdvance native SDL3 joystick input. One uniquely matched SDL GUID selects the device; every GBA control is written as a raw button, axis half, or cardinal hat while keyboard bindings remain intact.".into();
    db.layouts.push(nanoboyadvance);
    let mut vba_m = db
        .layout("gba")
        .context("Missing Game Boy Advance reference layout")?
        .clone();
    vba_m.id = "vba-m-gba-controller".into();
    vba_m.name = "Game Boy Advance — VBA-M controller".into();
    vba_m.source = format!(
        "https://github.com/visualboyadvance-m/visualboyadvance-m/tree/{}",
        crate::controller_vba_m_native::SOURCE_COMMIT
    );
    vba_m.notes = "VBA-M native Qt/wx SDL joystick input. The private configuration disables GameController translation and maps all ten ordinary GBA controls through exact raw SDL2 or SDL3 joystick numbering.".into();
    db.layouts.push(vba_m);
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
    let mut xroar = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    xroar.id = "xroar-analog-joystick".into();
    xroar.name = "Dragon/CoCo — analog joystick".into();
    xroar.family = "two-button".into();
    xroar.source =
        "https://www.6809.org.uk/git/xroar.git/commit/?id=0229f97a636c3c80d51fd27e7d145d792f0a8932"
            .into();
    xroar.notes = "XRoar native right/left joystick ports. Guided directions must resolve to opposite halves of two SDL gamepad axes; button-backed D-pads cannot be represented by this physical-axis grammar.".into();
    xroar.controls.retain(|control| {
        matches!(
            control.id.as_str(),
            "up" | "down" | "left" | "right" | "a" | "b"
        )
    });
    for control in &mut xroar.controls {
        match control.id.as_str() {
            "b" => control.label = "Fire 1".into(),
            "a" => control.label = "Fire 2".into(),
            _ => {}
        }
    }
    db.layouts.push(xroar);

    for (core, layout, players, platforms, source) in [
        (
            "scummvm",
            "scummvm-default-actions",
            1,
            vec!["ScummVM"],
            "https://github.com/scummvm/scummvm/blob/3f6428df202e2c044ef53208acba0f665ea92096/engines/metaengine.cpp",
        ),
        (
            "jgenesis",
            "genesis-6",
            2,
            vec![
                "Sega Genesis",
                "Sega CD",
                "Sega 32X",
                "Sega Master System",
                "Sega Game Gear",
                "Nintendo Entertainment System",
                "Super Nintendo Entertainment System",
                "Nintendo Game Boy",
                "Nintendo Game Boy Color",
                "Nintendo Game Boy Advance",
                "NEC TurboGrafx-16",
            ],
            "https://github.com/jsgroth/jgenesis/blob/cbe7f129e3f5c805a2a2e4318981834192116e90/frontend/jgenesis-native-config/src/input/mappings.rs",
        ),
        (
            "gopher64",
            "n64",
            4,
            vec!["Nintendo 64"],
            "https://github.com/gopher64/gopher64/blob/0bb9fbba638f5cebe3b8a3c1c245abcfec8b0132/src/ui/input.rs",
        ),
        (
            "gearsystem",
            "gamegear",
            1,
            vec!["Sega Game Gear"],
            "https://github.com/drhelius/Gearsystem/blob/253752954d5237a30b60c40789117ded345bcd48/platforms/shared/desktop/events.cpp",
        ),
        (
            "gearsystem",
            "master-system",
            2,
            vec!["Sega Master System", "Sega SG-1000", "Othello Multivision"],
            "https://github.com/drhelius/Gearsystem/blob/253752954d5237a30b60c40789117ded345bcd48/platforms/shared/desktop/events.cpp",
        ),
        (
            "gearcoleco",
            "colecovision",
            2,
            vec!["ColecoVision", "Coleco ColecoVision"],
            "https://github.com/drhelius/Gearcoleco/blob/8ad5f92c45e7ca616535a057495557c2352a9115/platforms/shared/desktop/events.cpp",
        ),
        (
            "rmg",
            "n64",
            4,
            vec!["Nintendo 64", "Nintendo 64DD"],
            "https://github.com/Rosalie241/RMG/blob/3e8b366be91ea96329db0567b038a31785f33468/Source/RMG-Input/main.cpp",
        ),
        (
            "simple64",
            "n64",
            4,
            vec!["Nintendo 64"],
            "https://github.com/simple64/simple64/blob/d8c969c7b932e3d76e6a25549d76348839dbaefd/simple64-input-qt/main.cpp",
        ),
        (
            "xemu",
            "xbox",
            4,
            vec!["Microsoft Xbox"],
            "https://github.com/mborgerson/xemu/blob/fd0ae0c0a189d56e87f8e46073b15b287e4a1e1a/ui/xemu-input.c",
        ),
        (
            "blastem",
            "genesis-6",
            2,
            vec!["Sega Genesis", "Sega CD", "Sega 32X"],
            "https://github.com/libretro/blastem/blob/b4d75247ebad8852fd9bc385b423df704c6c5af5/bindings.c",
        ),
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
        (
            "yaba-sanshiro",
            "saturn-digital",
            2,
            vec!["Sega Saturn"],
            "https://d1t36rsydvwkyk.cloudfront.net/yabasanshiro-src-1.20.37.tar.gz",
        ),
        (
            "kronos",
            "saturn-digital",
            4,
            vec!["Sega Saturn", "Sega ST-V"],
            "https://github.com/FCare/Kronos/tree/d451a55253e2e75bcef704ec8ade2085d298212c",
        ),
        (
            "xroar",
            "xroar-analog-joystick",
            2,
            vec!["Dragon 32/64", "TRS-80 Color Computer"],
            "https://www.6809.org.uk/git/xroar.git/commit/?id=0229f97a636c3c80d51fd27e7d145d792f0a8932",
        ),
        (
            "zesarux",
            "spectrum-joystick",
            1,
            vec![
                "Sinclair ZX Spectrum",
                "Sinclair ZX80",
                "Sinclair ZX81",
                "Jupiter Ace",
            ],
            "https://github.com/chernandezba/zesarux/tree/2e529034957cb61dec35d52d0008f03dcd35c019",
        ),
        (
            "oricutron",
            "spectrum-joystick",
            2,
            vec!["Oric Atmos"],
            "https://github.com/pete-gordon/oricutron/tree/002279fce9fa756d1d63cdc40ae97939eb7de7ed",
        ),
        (
            "atari-plus-plus",
            "atari-plus-plus-native-joystick",
            4,
            vec!["Atari 800", "Atari XEGS"],
            "https://sources.debian.org/src/atari++/1.85-1/",
        ),
        (
            "aranym",
            "aranym-ikbd-joystick",
            2,
            vec!["Atari ST"],
            "https://github.com/aranym/aranym/tree/5f4ebed6b039ddf42eef1122a315ad9608d20f6e",
        ),
        (
            "atari800",
            "atari800-native-joystick",
            4,
            vec!["Atari 800", "Atari XEGS"],
            "https://github.com/atari800/atari800/tree/fe1d2890d9f05fcecb2fd033d09a5c43f534bebf",
        ),
        (
            "nanoboyadvance",
            "gba-controller",
            1,
            vec![
                "Nintendo Game Boy Advance",
                "Nintendo - Game Boy Advance",
                "Game Boy Advance",
                "GBA",
            ],
            "https://github.com/nba-emu/NanoBoyAdvance/tree/55b5cf0ae3d929582ac5bfd486558173502b8354",
        ),
        (
            "vba-m",
            "vba-m-gba-controller",
            1,
            vec!["Nintendo e-Reader", "Nintendo - e-Reader"],
            "https://github.com/visualboyadvance-m/visualboyadvance-m/tree/fd13034143c128c8b68133a7a18bc785178ec4e4",
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

/// Preview labels for the ScummVM engine-default actions.
const SCUMMVM_ACTION_ROUTES: [(&str, &str); 10] = [
    ("a", "LCLK (Left click)"),
    ("b", "RCLK (Right click)"),
    ("x", "SKLI (Skip line)"),
    ("y", "SKIP"),
    ("l", "MENU (Game menu)"),
    ("start", "RETURN (Confirm)"),
    ("up", "UP"),
    ("down", "DOWN"),
    ("left", "LEFT"),
    ("right", "RIGHT"),
];

/// Preview labels for the xemu controller_mapping fields.
const XEMU_FIELD_ROUTES: [(&str, &str); 24] = [
    ("a", "a"),
    ("b", "b"),
    ("x", "x"),
    ("y", "y"),
    ("select", "back"),
    ("start", "start"),
    ("l", "lshoulder"),
    ("r", "rshoulder"),
    ("l3", "lstick_btn"),
    ("r3", "rstick_btn"),
    ("up", "dpad_up"),
    ("down", "dpad_down"),
    ("left", "dpad_left"),
    ("right", "dpad_right"),
    ("l2", "axis_trigger_left"),
    ("r2", "axis_trigger_right"),
    ("stick_left", "axis_left_x (negative half)"),
    ("stick_right", "axis_left_x (positive half)"),
    ("stick_up", "axis_left_y (negative half)"),
    ("stick_down", "axis_left_y (positive half)"),
    ("right_stick_left", "axis_right_x (negative half)"),
    ("right_stick_right", "axis_right_x (positive half)"),
    ("right_stick_up", "axis_right_y (negative half)"),
    ("right_stick_down", "axis_right_y (positive half)"),
];

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
    if (core, layout) == ("gopher64", "n64") {
        return Some(
            crate::controller_gopher64_native::CONTROLS
                .iter()
                .map(|(target, index)| {
                    (
                        (*target).to_owned(),
                        format!("input_profiles[].inputs[{index}]"),
                    )
                })
                .collect(),
        );
    }
    if (core, layout) == ("rmg", "n64") {
        return Some(
            crate::controller_rmg_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("simple64", "n64") {
        return Some(
            crate::controller_simple64_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("gearsystem", "gamegear") {
        return Some(
            crate::controller_gearsystem_standalone::GAME_GEAR_ROUTES
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("gearsystem", "master-system") {
        return Some(
            crate::controller_gearsystem_standalone::MASTER_SYSTEM_ROUTES
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("gearcoleco", "colecovision") {
        return Some(
            crate::controller_gearcoleco_standalone::STANDARD_ROUTES
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("xroar", "xroar-analog-joystick") {
        return Some(
            crate::controller_xroar_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("zesarux", "spectrum-joystick") {
        return Some(
            crate::controller_zesarux_native::CONTROLS
                .iter()
                .map(|target| ((*target).to_owned(), format!("--joystickevent {target}")))
                .collect(),
        );
    }
    if (core, layout) == ("oricutron", "spectrum-joystick") {
        return Some(
            crate::controller_oricutron_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("atari-plus-plus", "atari-plus-plus-native-joystick") {
        return Some(
            crate::controller_atari_plus_plus_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("aranym", "aranym-ikbd-joystick") {
        return Some(
            crate::controller_aranym_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("atari800", "atari800-native-joystick") {
        return Some(
            crate::controller_atari800_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("nanoboyadvance", "gba-controller") {
        return Some(
            crate::controller_nanoboyadvance_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("vba-m", "vba-m-gba-controller") {
        return Some(
            crate::controller_vba_m_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
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
        ("scummvm", "scummvm-default-actions") => SCUMMVM_ACTION_ROUTES
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("jgenesis", "genesis-6") => crate::controller_jgenesis_native::CONTROLS
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("xemu", "xbox") => XEMU_FIELD_ROUTES
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("blastem", "genesis-6") => crate::controller_blastem_native::CONTROLS
            .iter()
            .copied()
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
        ("yaba-sanshiro", "saturn-digital") => crate::controller_yaba_sanshiro::SATURN_ROUTES
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
        ("kronos", "saturn-digital") => crate::controller_kronos::SATURN_ROUTES
            .iter()
            .copied()
            .collect::<BTreeMap<&str, &str>>(),
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
