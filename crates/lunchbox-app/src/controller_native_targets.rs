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
    let mut skyemu = db
        .layout("nds-stylus-controls")
        .context("Missing DS geometry")?
        .clone();
    skyemu.id = "skyemu-ds-buttons".into();
    skyemu.name = "Nintendo DS — buttons (SkyEmu native)".into();
    skyemu.source =
        "https://github.com/skylersaleh/SkyEmu/tree/01516d6798e3652b583e6a366085bb51c43b528d"
            .into();
    skyemu.notes = "Native SkyEmu button controls. Touchscreen, microphone and lid are not mapped by this contract.".into();
    skyemu.controls.retain(|control| {
        matches!(
            control.id.as_str(),
            "a" | "b"
                | "x"
                | "y"
                | "up"
                | "down"
                | "left"
                | "right"
                | "l"
                | "r"
                | "start"
                | "select"
        )
    });
    for control in &mut skyemu.controls {
        control.optional = false;
    }
    db.layouts.push(skyemu);
    let mut caprice32 = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    caprice32.id = "caprice32-cpc".into();
    caprice32.name = "Amstrad CPC — Caprice32 joystick".into();
    caprice32.family = "two-button".into();
    caprice32.source =
        "https://github.com/ColinPitrat/caprice32/tree/6c12c4c92360065cdc229ac9ada7551f941436b8"
            .into();
    caprice32.notes = "Caprice32 fixed CPC joystick ports. Directions and fire buttons are source-fixed; start/select supply the two global menu buttons.".into();
    caprice32.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("a", "Fire 1", 70.0, 50.0, "face", true),
        ("b", "Fire 2", 80.0, 35.0, "face", true),
        ("start", "Menu", 30.0, 90.0, "menu", true),
        ("select", "Virtual keyboard", 70.0, 90.0, "menu", true),
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
    db.layouts.push(caprice32);
    let mut vita3k = db
        .layout("vita")
        .context("Missing Vita reference layout")?
        .clone();
    vita3k.id = "vita3k-vita".into();
    vita3k.name = "PlayStation Vita — Vita3K gamepad".into();
    vita3k.family = "diamond".into();
    vita3k.source = "https://github.com/Vita3K/Vita3K/tree/84184a363aa99c7f331a7e75bdd75f43ff63db08/vita3k/gui-qt/src/controls_dialog.cpp".into();
    vita3k.notes = "Vita3K native SDL gamepad. Buttons use vita_button-order binds; sticks pair into four axes with identity triggers.".into();
    for control in &mut vita3k.controls {
        control.optional = false;
    }
    db.layouts.push(vita3k);
    let mut gbe_plus = db
        .layout("gba")
        .context("Missing Game Boy Advance reference layout")?
        .clone();
    gbe_plus.id = "gbe-plus-gamepad".into();
    gbe_plus.name = "Game Boy Advance — GBE+ gamepad".into();
    gbe_plus.source =
        "https://github.com/shonumi/gbe-plus/tree/05a05e931b3993ff3e6316b0d841a1fb4d3ac7a7".into();
    gbe_plus.notes = "GBE+ twelve-control gamepad. X/Y extend the GBA set; directions may be axes, hats, or buttons.".into();
    gbe_plus.controls.push(crate::controller_catalog::Control {
        id: "x".into(),
        label: "X".into(),
        x: 66.0,
        y: 36.0,
        group: "face".into(),
        optional: false,
        analog: false,
        pressure: false,
        repeat_of: None,
    });
    gbe_plus.controls.push(crate::controller_catalog::Control {
        id: "y".into(),
        label: "Y".into(),
        x: 54.0,
        y: 48.0,
        group: "face".into(),
        optional: false,
        analog: false,
        pressure: false,
        repeat_of: None,
    });
    for control in &mut gbe_plus.controls {
        control.optional = false;
    }
    db.layouts.push(gbe_plus);
    let mut eka2l1 = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    eka2l1.id = "eka2l1-phone".into();
    eka2l1.name = "Symbian phone — EKA2L1 gamepad".into();
    eka2l1.family = "two-button".into();
    eka2l1.source = "https://github.com/EKA2L1/EKA2L1/tree/8dd86cffc59d12c59661acecdfddfab5ffc810db/src/emu/qt/src/settings_dialog.cpp".into();
    eka2l1.notes = "EKA2L1 phone gamepad. Directions drive arrows; face buttons drive softkeys, middle select, and call keys.".into();
    eka2l1.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("a", "Select (middle)", 70.0, 50.0, "face", true),
        ("b", "Right softkey", 80.0, 35.0, "face", true),
        ("x", "Left softkey", 60.0, 35.0, "face", true),
        ("y", "Call (green)", 90.0, 50.0, "face", true),
        ("l", "Star", 5.0, 35.0, "shoulder", true),
        ("r", "Hash", 95.0, 35.0, "shoulder", true),
        ("start", "End (red)", 30.0, 90.0, "menu", true),
        ("select", "Clear", 70.0, 90.0, "menu", true),
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
    db.layouts.push(eka2l1);
    let mut cemu = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    cemu.id = "cemu-vpad".into();
    cemu.name = "Wii U GamePad — Cemu VPAD".into();
    cemu.family = "two-button".into();
    cemu.source = "https://github.com/cemu-project/Cemu/tree/3310f3b8b184d64a62b89fd59088c799432badf5/src/input".into();
    cemu.notes =
        "Cemu Wii U GamePad. Face buttons, triggers, plus/minus, dpad, and the left stick.".into();
    cemu.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("a", "A", 90.0, 48.0, "face", true),
        ("b", "B", 78.0, 60.0, "face", true),
        ("x", "X", 78.0, 36.0, "face", true),
        ("y", "Y", 66.0, 48.0, "face", true),
        ("l", "L", 20.0, 12.0, "shoulder", true),
        ("r", "R", 80.0, 12.0, "shoulder", true),
        ("zl", "ZL", 20.0, 24.0, "shoulder", true),
        ("zr", "ZR", 80.0, 24.0, "shoulder", true),
        ("plus", "Plus", 60.0, 88.0, "menu", true),
        ("minus", "Minus", 47.0, 88.0, "menu", true),
        ("stick_up", "Left stick up", 20.0, 70.0, "stick", true),
        ("stick_down", "Left stick down", 20.0, 90.0, "stick", true),
        ("stick_left", "Left stick left", 10.0, 80.0, "stick", true),
        ("stick_right", "Left stick right", 30.0, 80.0, "stick", true),
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
    db.layouts.push(cemu);
    let mut amiberry = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    amiberry.id = "amiberry-amiga".into();
    amiberry.name = "Commodore Amiga — Amiberry joystick".into();
    amiberry.family = "two-button".into();
    amiberry.source =
        "https://github.com/BlitterStudio/amiberry/tree/06ff25093b620deef734a395189a1c564ed8beac"
            .into();
    amiberry.notes = "Amiberry fixed-dpad directions with raw-button fire on SDL gamepads; the gamecontrollerdb line translates physical controls.".into();
    amiberry.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("fire", "Fire", 70.0, 50.0, "face", true),
        ("fire2", "Fire 2", 80.0, 35.0, "face", true),
        ("fire3", "Fire 3", 80.0, 65.0, "face", true),
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
    db.layouts.push(amiberry);
    let mut fuse = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    fuse.id = "fuse-spectrum".into();
    fuse.name = "ZX Spectrum — Fuse joystick".into();
    fuse.family = "two-button".into();
    fuse.source =
        "https://github.com/fuse-emulator/fuse/tree/5ba7804a44483466d7403a6e646a228da562ed5d"
            .into();
    fuse.notes = "Fuse fixed-slot Spectrum joystick. Directions are source-fixed; fire lands on its own raw button index.".into();
    fuse.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("fire", "Fire", 70.0, 50.0, "face", true),
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
    db.layouts.push(fuse);
    let mut linapple = db
        .layout("atari7800")
        .context("Missing two-button stick reference layout")?
        .clone();
    linapple.id = "linapple-joystick".into();
    linapple.name = "Apple II — LinApple joystick".into();
    linapple.family = "two-button".into();
    linapple.source =
        "https://github.com/linappleii/linapple/tree/fa31e11b579edec32dd431c8b400a04e60a21dab"
            .into();
    linapple.notes = "LinApple native SDL joystick. Directions require one shared analog axis per pair with opposite polarity; two raw buttons; hats cannot drive the analog Axis fields.".into();
    linapple.controls = [
        ("up", "Up", 30.0, 30.0, "dpad", true),
        ("down", "Down", 30.0, 70.0, "dpad", true),
        ("left", "Left", 10.0, 50.0, "dpad", true),
        ("right", "Right", 50.0, 50.0, "dpad", true),
        ("fire", "Button 1", 70.0, 50.0, "face", true),
        ("fire2", "Button 2", 80.0, 35.0, "face", true),
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
    db.layouts.push(linapple);
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
    let mut eighty_six_box = db
        .layout("atari7800")
        .context("Missing two-button joystick reference layout")?
        .clone();
    eighty_six_box.id = "86box-2axis-2button".into();
    eighty_six_box.name = "PC gameport — 2-axis, 2-button joystick".into();
    eighty_six_box.family = "two-button".into();
    eighty_six_box.source = format!(
        "https://github.com/86Box/86Box/tree/{}",
        crate::controller_86box_native::SOURCE_COMMIT
    );
    eighty_six_box.notes = "86Box native SDL2 gameport input. The selected machine config is overlaid at its original path and forced to the source-defined 2axis_2button topology; directions require two raw SDL axes or cardinal hats and buttons remain raw.".into();
    eighty_six_box.controls.retain(|control| {
        matches!(
            control.id.as_str(),
            "up" | "down" | "left" | "right" | "a" | "b"
        )
    });
    for control in &mut eighty_six_box.controls {
        control.optional = false;
    }
    db.layouts.push(eighty_six_box);
    let mut a7800 = db
        .layout("atari7800")
        .context("Missing Atari 7800 controller reference layout")?
        .clone();
    a7800.id = "a7800-proline".into();
    a7800.name = "Atari 7800 — A7800 Pro-Line joystick".into();
    a7800.family = "two-button".into();
    a7800.source = format!(
        "https://github.com/7800-devtools/a7800/tree/{}",
        crate::controller_a7800_native::SOURCE_COMMIT
    );
    a7800.notes = "A7800 5.2 raw SDL2 provider. Native mapdevice identity is the SDL joystick name with whitespace removed; ambiguous names are refused. Only the base NTSC/PAL machines and two-button Pro-Line ports are covered.".into();
    a7800.controls.retain(|control| {
        matches!(
            control.id.as_str(),
            "up" | "down" | "left" | "right" | "a" | "b"
        )
    });
    for control in &mut a7800.controls {
        control.optional = false;
    }
    db.layouts.push(a7800);
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
        (
            "86box",
            "86box-2axis-2button",
            2,
            vec!["MS-DOS", "Windows", "Windows 3.X"],
            "https://github.com/86Box/86Box/tree/189d9d003ad9670853cec6edac8db7d6ff63550f",
        ),
        (
            "a7800",
            "a7800-proline",
            2,
            vec!["Atari 7800"],
            "https://github.com/7800-devtools/a7800/tree/7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8",
        ),
        (
            "picodrive",
            "genesis-6",
            2,
            vec!["Sega Genesis", "Sega CD", "Sega 32X"],
            "https://github.com/notaz/picodrive/tree/26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee",
        ),
        (
            "nestopia-ue",
            "nes",
            2,
            vec![
                "Nintendo Entertainment System",
                "Nintendo Famicom Disk System",
            ],
            "https://github.com/0ldsk00l/nestopia/tree/1.53.2",
        ),
        (
            "gambatte",
            "gameboy",
            1,
            vec!["Nintendo Game Boy", "Nintendo Game Boy Color"],
            "https://github.com/EclipseEmu/gambatte/tree/04e7ddf85ff23032cb7132f155c42c7d1857f474",
        ),
        (
            "skyemu",
            "skyemu-ds-buttons",
            1,
            vec!["Nintendo DS"],
            "https://github.com/skylersaleh/SkyEmu/tree/01516d6798e3652b583e6a366085bb51c43b528d",
        ),
        (
            "linapple",
            "linapple-joystick",
            2,
            vec!["Apple II"],
            "https://github.com/linappleii/linapple/tree/fa31e11b579edec32dd431c8b400a04e60a21dab",
        ),
        (
            "caprice32",
            "caprice32-cpc",
            2,
            vec!["Amstrad CPC"],
            "https://github.com/ColinPitrat/caprice32/tree/6c12c4c92360065cdc229ac9ada7551f941436b8",
        ),
        (
            "fuse",
            "fuse-spectrum",
            2,
            vec!["ZX Spectrum"],
            "https://github.com/fuse-emulator/fuse/tree/5ba7804a44483466d7403a6e646a228da562ed5d",
        ),
        (
            "amiberry",
            "amiberry-amiga",
            2,
            vec!["Commodore Amiga"],
            "https://github.com/BlitterStudio/amiberry/tree/06ff25093b620deef734a395189a1c564ed8beac",
        ),
        (
            "vita3k",
            "vita3k-vita",
            1,
            vec!["PlayStation Vita"],
            "https://github.com/Vita3K/Vita3K/tree/84184a363aa99c7f331a7e75bdd75f43ff63db08",
        ),
        (
            "gbe-plus",
            "gbe-plus-gamepad",
            1,
            vec!["Nintendo Game Boy Advance"],
            "https://github.com/shonumi/gbe-plus/tree/05a05e931b3993ff3e6316b0d841a1fb4d3ac7a7",
        ),
        (
            "pokemini",
            "pokemini",
            1,
            vec!["Nintendo Pokemon Mini"],
            "https://sourceforge.net/p/pokemini/code/ci/15cc97ff70d6d9d749287ac55cb68198708564f3/tree/",
        ),
        (
            "uzem",
            "snes",
            2,
            vec!["Uzebox"],
            "https://github.com/Uzebox/uzebox/tree/abf5125847e68a6b7c4432f7849cd5baf717bba5/tools/uzem",
        ),
        (
            "eka2l1",
            "eka2l1-phone",
            1,
            vec!["Symbian"],
            "https://github.com/EKA2L1/EKA2L1/tree/8dd86cffc59d12c59661acecdfddfab5ffc810db",
        ),
        (
            "cemu",
            "cemu-vpad",
            1,
            vec!["Nintendo Wii U"],
            "https://github.com/cemu-project/Cemu/tree/3310f3b8b184d64a62b89fd59088c799432badf5",
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
    if (core, layout) == ("86box", "86box-2axis-2button") {
        return Some(
            crate::controller_86box_native::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("gambatte", "gameboy") {
        return Some(
            crate::controller_gambatte_standalone::CONTROLS
                .iter()
                .map(|(target, field)| ((*target).to_owned(), (*field).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("caprice32", "caprice32-cpc") {
        return Some(
            crate::controller_caprice32_standalone::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("vita3k", "vita3k-vita") {
        return Some(
            crate::controller_vita3k_native::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("cemu", "cemu-vpad") {
        return Some(
            crate::controller_cemu_native::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("eka2l1", "eka2l1-phone") {
        return Some(
            crate::controller_eka2l1_native::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("uzem", "snes") {
        return Some(
            crate::controller_uzem_standalone::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("pokemini", "pokemini") {
        return Some(
            crate::controller_pokemini_standalone::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("gbe-plus", "gbe-plus-gamepad") {
        return Some(
            crate::controller_gbe_plus_standalone::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("amiberry", "amiberry-amiga") {
        return Some(
            crate::controller_amiberry_native::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("fuse", "fuse-spectrum") {
        return Some(
            crate::controller_fuse_standalone::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("linapple", "linapple-joystick") {
        return Some(
            crate::controller_linapple_native::ROUTES
                .iter()
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("skyemu", "skyemu-ds-buttons") {
        return Some(
            crate::controller_skyemu_native::PROFILE_CONTROLS
                .iter()
                .map(|target| ((*target).to_owned(), (*target).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("nestopia-ue", "nes") {
        return Some(
            crate::controller_nestopia_ue_native::CONTROLS
                .iter()
                .filter(|(target, _)| {
                    crate::controller_nestopia_ue_native::STANDARD_CONTROLS.contains(target)
                })
                .map(|(target, label)| ((*target).to_owned(), (*label).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("picodrive", "genesis-6") {
        return Some(
            crate::controller_picodrive_native::ROUTES
                .iter()
                .map(|(target, action)| ((*target).to_owned(), (*action).to_owned()))
                .collect(),
        );
    }
    if (core, layout) == ("a7800", "a7800-proline") {
        return Some(
            crate::controller_a7800_native::CONTROLS
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
