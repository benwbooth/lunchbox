import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: setup
    required property var settingsModel
    required property var gamepad
    Dialog {
        id: mameNativeEditor
        property var reviews: []
        readonly property var catalog: JSON.parse(setup.settingsModel.controller_catalog_json())
        title: "Standalone MAME panels — partial native Linux"
        modal: true
        width: Math.min(800, setup.Window.width - 40)
        standardButtons: Dialog.Close
        onVisibleChanged: if (!setup.visible) close()
        ColumnLayout {
            anchors.fill: parent
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Edit a JSON list of emulator_id, executable, executable_sha256, joystick_provider and players. Each player has player (1–8), controller_id, native_device_id, panel (six/eight), and controls for up/down/left/right/start/coin/button1–6 or button1–8. Controls use native MAME items, e.g. {\"kind\":\"button\",\"number\":1} or {\"kind\":\"hat\",\"number\":1,\"direction\":\"up\"}. Native device IDs and item numbers must not be guessed from SDL/joydev. Launch also requires cfg_directory and runtime {probe_program, sdl_library} with absolute paths. threshold_basis_points defaults to 3000 (0.3). Partial raw-SDL dispatch is untested."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 240
                TextArea { id: mameNativeText; wrapMode: TextEdit.Wrap; selectByMouse: true; onTextChanged: mameNativeEditor.reviews = [] }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Optional per-player source_controls links targets to saved physical layout IDs, e.g. {\"button1\":\"b\",\"coin\":\"select\"}. Review shows these declared links on both diagrams; native correspondence remains unverified."
            }
            RowLayout {
                Button {
                    text: "Review declarations"
                    onClicked: {
                        const result = JSON.parse(setup.settingsModel.review_mame_native_setups(mameNativeText.text))
                        mameNativeResult.text = result.error || "Declared native routes only. Physical source-to-token correspondence and launch readiness are not verified."
                        mameNativeEditor.reviews = result.error ? [] : result.setups.reduce((players, item) => players.concat(item.players), [])
                    }
                }
                Button {
                    text: "Stage setups"
                    onClicked: {
                        const error = setup.settingsModel.stage_mame_native_setups(mameNativeText.text)
                        mameNativeResult.text = error || "Staged. Save settings on the main page. Native Linux raw SDL dispatch is partial and untested; no device discovery or emulator launch was performed."
                    }
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 280
                ColumnLayout {
                    width: parent.width
                    Repeater {
                        model: mameNativeEditor.reviews
                        delegate: ColumnLayout {
                            id: mamePanelReview
                            required property var modelData
                            Layout.fillWidth: true
                            Label { text: "Player " + modelData.player + " · " + modelData.controller_id + " · declared native assignments" }
                            ControllerMappingView {
                                Layout.fillWidth: true
                                settingsModel: setup.settingsModel
                                destinationLayout: mameNativeEditor.catalog.layouts.find(layout => layout.id === mamePanelReview.modelData.target_layout) || null
                                sourceLayout: mameNativeEditor.catalog.layouts.find(layout => layout.id === mamePanelReview.modelData.source_layout) || null
                                rows: Object.keys(mamePanelReview.modelData.bindings).map(control => ({
                                    target_id: control === "coin" && destinationLayout && destinationLayout.controls.some(item => item.id === "select") ? "select" : control,
                                    target: control === "coin" ? "Coin" : control,
                                    physical_id: mamePanelReview.modelData.source_controls[control] || null,
                                    physical: mamePanelReview.modelData.source_labels[mamePanelReview.modelData.source_controls[control]] || "Source control not linked",
                                    output: mamePanelReview.modelData.bindings[control],
                                    input: null,
                                    reason: "User-declared source link; native token correspondence not verified"
                                }))
                            }
                        }
                    }
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 180
                TextArea { id: mameNativeResult; readOnly: true; wrapMode: TextEdit.Wrap; selectByMouse: true }
            }
        }
    }
    Button {
        text: "Standalone MAME panels…"
        onClicked: {
            mameNativeText.text = setup.settingsModel.mame_native_setups_json()
            mameNativeResult.text = "Partial native Linux raw SDL dispatch. Set cfg_directory and runtime {probe_program, sdl_library} to absolute paths. Plain machine arguments and unique controller GUIDs are required. Changes to MAME cfg settings during play remain session-local."
            mameNativeEditor.open()
        }
    }
    property bool testInput: false
    onVisibleChanged: { if (!visible) { mameNativeEditor.close(); duckstationSetups.close(); nativeCapture.close(); nativeRuntime.close(); fbneoSetups.close(); mameSetups.close(); relativeSettings.close(); relativeForm.close(); absoluteSettings.close(); layoutExplorer.close(); arcadePreview.close() } }
    readonly property bool calibrationActive: calibration.visible || nativeCapture.visible
    readonly property var calibrationContentItem: nativeCapture.visible ? nativeCapture.contentItem : calibration.contentItem
    readonly property string calibratedCoverage: {
        if (calibration.catalog.host_os !== "linux")
            return "Calibrated launch adapters for this OS are not implemented yet."
        const profiles = calibration.catalog.emulator_profiles.filter(profile => profile.retroarch_launch)
        return "Linux RetroArch (native or Flatpak): " + profiles.map(profile => profile.name.replace("RetroArch · ", "")).join(", ") + "."
    }
    function openCalibrationFor(index, layoutId) {
        calibration.openFor(settingsModel.controller_key_at(index), settingsModel.controller_name_at(index))
        if (layoutId) {
            const choice = calibration.catalog.layouts.findIndex(item => item.id === layoutId)
            if (choice >= 0) calibration.resetLayout(choice)
        }
    }
    ControllerCalibrationWizard {
        id: calibration
        settingsModel: setup.settingsModel
        gamepad: setup.gamepad
    }
    ControllerCoverageDialog {
        id: coverage
        settingsModel: setup.settingsModel
        onNativeRuntimeRequested: nativeRuntime.loadAndOpen()
        onPerGameSetupRequested: function(core) {
            if (core === "mame") mameSetups.loadAndOpen()
            else if (core === "fbneo") fbneoSetups.loadAndOpen()
        }
    }
    ControllerNativeRuntimeDialog {
        id: nativeRuntime
        settingsModel: setup.settingsModel
    }
    Button {
        text: "xemu Xbox setups…"
        onClicked: {
            duckstationSetups.adapter = "xemu-native"
            duckstationEditor.text = setup.settingsModel.xemu_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch writes a private -config_path file with the declared boot ROM, flash image and game; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "BlastEm Genesis setups…"
        onClicked: {
            duckstationSetups.adapter = "blastem-native"
            duckstationEditor.text = setup.settingsModel.blastem_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch isolates HOME and binds the selected SDL devices to gamepad ports; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Mesen2 NES setups…"
        onClicked: {
            duckstationSetups.adapter = "mesen2-native"
            duckstationEditor.text = setup.settingsModel.mesen2_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch isolates XDG_DATA_HOME and verifies the sole qualifying event device; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Hatari joystick setups…"
        onClicked: {
            duckstationSetups.adapter = "hatari-native"
            duckstationEditor.text = setup.settingsModel.hatari_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch isolates HOME and passes a private -c configuration with the declared TOS image; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "VICE joystick setups…"
        onClicked: {
            duckstationSetups.adapter = "vice-native"
            duckstationEditor.text = setup.settingsModel.vice_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch uses a private -config/-joymap pair with SDL2 probing; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Stella standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "stella-native"
            duckstationEditor.text = setup.settingsModel.stella_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch uses a private -basedir and the SDL classic backend; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "bsnes standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "bsnes"
            duckstationEditor.text = setup.settingsModel.bsnes_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch writes a private settings.bml through --settings; runtime verification is deferred."
            duckstationSetups.open()
        }
    }
    Button {
        text: "DuckStation standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "duckstation"
            duckstationEditor.text = setup.settingsModel.duckstation_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Native Linux launch requires a saved trusted runtime; tests and runtime verification are deferred."
            duckstationSetups.open()
        }
    }
    Dialog {
        id: duckstationSetups
        property string adapter: "duckstation"
        readonly property bool ppsspp: adapter === "ppsspp"
        readonly property bool mgba: adapter === "mgba"
        readonly property bool dolphin: adapter === "dolphin"
        readonly property bool snes9x: adapter === "snes9x"
        readonly property bool fceux: adapter === "fceux"
        readonly property bool sameboy: adapter === "sameboy"
        readonly property bool bsnes: adapter === "bsnes"
        readonly property bool stellaNative: adapter === "stella-native"
        readonly property bool viceNative: adapter === "vice-native"
        readonly property bool mesen2Native: adapter === "mesen2-native"
        readonly property bool blastemNative: adapter === "blastem-native"
        readonly property bool xemuNative: adapter === "xemu-native"
        readonly property bool hatariNative: adapter === "hatari-native"
        readonly property bool mednafen: adapter === "mednafen"
        readonly property bool flycastNative: adapter === "flycast-native"
        readonly property bool melonds: adapter === "melonds"
        readonly property bool rpcs3: adapter === "rpcs3"
        readonly property bool pcsx2: adapter === "pcsx2"
        readonly property var catalog: JSON.parse(setup.settingsModel.controller_catalog_json())
        title: melonds ? "melonDS controller setups — partial native Linux" : rpcs3 ? "RPCS3 controller setups — partial native Linux" : pcsx2 ? "PCSX2 DualShock 2 — partial native Linux" : flycastNative ? "Standalone Flycast panels — partial native Linux" : mednafen ? "Mednafen setups — partial native Linux" : sameboy ? "SameBoy SDL setups — partial native Linux" : bsnes ? "bsnes SNES setups — partial native Linux" : stellaNative ? "Stella Atari 2600 setups — partial native Linux" : viceNative ? "VICE Commodore joystick setups — partial native Linux" : hatariNative ? "Hatari Atari ST joystick setups — partial native Linux" : mesen2Native ? "Mesen2 NES setups — partial native Linux" : blastemNative ? "BlastEm Genesis setups — partial native Linux" : xemuNative ? "xemu Xbox setups — partial native Linux" : fceux ? "FCEUX Qt setups — partial native Linux" : snes9x ? "Snes9x GTK setups — native Linux" : dolphin ? "Dolphin GameCube setups — native Linux" : mgba ? "mGBA SDL controller setups — native Linux" : ppsspp ? "PPSSPP controller setups — native Linux SDL2" : "DuckStation controller setups — native Linux"
        width: Math.min(900, setup.width)
        height: 640
        modal: true
        property var review: []
        contentItem: ColumnLayout {
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: duckstationSetups.melonds
                    ? "melonDS standard controls: JSON setups require emulator_id, content, executable_sha256, source_config, probe_program, sdl_library, bubblewrap_program and players. runtime_libraries must be empty for the SDL2 probe. Supply one player entry with player: 1, controller_id and source_controls for a/b/x/y, up/down/left/right, start/select and l/r. Paths must be absolute. Review shows calibrated source links and the DS destination layout; stylus, lid and microphone controls are not implemented by this adapter. Native Linux launch integration is partial and untested."
                    : duckstationSetups.rpcs3
                    ? "RPCS3 standard pads: JSON setups require emulator_id, content, executable_sha256, source_config, probe_program, sdl_library, optional runtime_libraries, and players. Each player has player (1–7), controller_id and source_controls linking all 24 destination visual IDs to calibrated physical controls. Paths must be absolute. Review displays source and destination layouts without opening devices. Native Linux file-boot dispatch is connected but untested. Directory boot targets and other backends remain pending."
                    : duckstationSetups.pcsx2
                    ? "PCSX2 DualShock 2: JSON setups require emulator_id, content, executable_sha256, source_config, probe_program, sdl_library, runtime_libraries (explicit libudev dependency), native (data_root, serial and integer disc crc), multitaps ([false,false] by default), and players. Each player has player, controller_id and source_controls. Source links use destination visual IDs: up/down/left/right, a/b/x/y, select/start, l/r/l2/r2/l3/r3, stick_up/down/left/right and right_stick_up/down/left/right. Players follow port-one slots then port-two slots. Paths must be absolute. Review shows both layouts without opening devices. Native Linux dispatch is connected for SDL 3.2.20 classic backend. Runtime/internal routing remain unverified."
                    : duckstationSetups.flycastNative
                    ? "Standalone Flycast: edit a JSON list with emulator_id, content, game_id (native ID, not library title), executable_sha256, source_config, probe_program, sdl_library and players. Paths must be absolute. Each player has player (1–4), controller_id, panel (six by default or eight), and source_controls linking every target to a calibrated physical layout ID. Targets: up/down/left/right/start/coin/button1–6 or button1–8. Example source_controls entry: button1 maps to b. Review displays source/destination diagrams. Partial native Linux launch dispatch is connected. Startup checks can reject mismatched controllers. Internal routing and runtime compatibility remain unverified. Review opens no devices."
                    : duckstationSetups.mednafen
                    ? "Mednafen: edit a JSON list with emulator_id, content, base_directory, bubblewrap_program, executable_sha256, gamepad (game-boy, game-boy-advance, lynx, neo-geo-pocket, wonder-swan, virtual-boy, game-gear, master-system, pce-two, pce-six, pce-fast-two, pce-fast-six, nes-two, nes-four-score, nes-famicom-four, snes, snes-faust, md-three, md-six, saturn-digital play-station-digital or play-station-dual-analog), and players containing player and controller_id (handhelds: player 1; Master System: ports 1–2; PC Engine: ports 1–5; NES: 1–2 or 1–4 according to adapter; SNES/SNES Faust: 1–8, with 3–5 on the port-two tap and 6–8 on the port-one tap). Each player may optionally specify gamepad: pce-two or pce-six (or pce-fast-two/pce-fast-six for the fast module) to override the default within the same native module. Genesis: md_tap is none (default), port-one, port-two, dual or four-way; player ports are sequential native virtual ports, up to 2/5/5/8/4 respectively. Per-player md-three/md-six overrides allow mixed pads. Saturn: saturn_multitaps is [port1-enabled, port2-enabled], default [false, false], with 2/7/12 sequential virtual slots. PlayStation: psx_multitaps is [port1-enabled, port2-enabled], default [false, false], with 2/5/8 sequential virtual slots. Per-player play-station-digital/play-station-dual-analog overrides permit mixed pads. Dual Analog uses four centered proportional axes, no rumble or mode switch. PlayStation, Saturn and PC Engine accept .ccd and UTF-8 .cue discs with companion-file tracking. Native CD firmware is required. TOC/M3U and firmware identity remain incomplete. Paths must be absolute. Native GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine joydev launch dispatch is connected. Child internal IDs and other native input drivers remain unverified. NES currently accepts raw iNES .nes and conventional UNIF .unf/.unif content and rejects ROM-device conflicts. Review opens no devices."
                    : duckstationSetups.xemuNative
                    ? "xemu: edit a JSON list with emulator_id, content (absolute XISO path), mcpx_bootrom and flashrom (absolute declared images), probe_program, sdl_library (the SDL3 library the xemu build links), executable_sha256, and players (1–4). Paths must be absolute. Launch passes a private -config_path file that mounts the declared boot ROM, flash image and game, disables auto-bind and binds each SDL GUID to its port with standard-index controller mappings; the probe and child both run with SDL_JOYSTICK_LINUX_CLASSIC=1. Same-GUID devices cannot be distinguished; Steel Battalion and the S controller driver are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.blastemNative
                    ? "BlastEm: edit a JSON list with emulator_id, content (absolute ROM path), probe_program, sdl_library (the SDL2 library the BlastEm build links), executable_sha256, and players. Each player has player (1–2) and controller_id. Paths must be absolute. Launch isolates HOME and writes the private blastem.cfg tern config binding each SDL device to its gamepad port with the six-button Genesis pad targets. Mice, the tee-input adapter, hotkeys and analog stick translation are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.mesen2Native
                    ? "Mesen2: edit a JSON list with emulator_id, content (absolute ROM path), controller_id, probe_program (absolute lunchbox-controller-probe path), and executable_sha256. Paths must be absolute. Launch isolates XDG_DATA_HOME and writes the private settings.json NES Port1 mapping; the selected controller must be the only qualifying /dev/input gamepad (BTN_GAMEPAD or ABS_X), which fixes its pad slot at zero. Zapper, Power Pad, Four Score, multitaps and non-NES systems are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.hatariNative
                    ? "Hatari: edit a JSON list with emulator_id, content (absolute disk/program path), tos_image (absolute EmuTOS/TOS image), probe_program, sdl_library (the SDL library the Hatari build links), executable_sha256, and players. Each player has player (1–2) and controller_id. Paths must be absolute. Launch isolates HOME, passes a private -c configuration and selects the declared TOS image. Directions must live on SDL axes 0/1 or hat 0; fire 2/3 are optional joyport expansions. Mouse, analog paddles and joypad emulation are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.viceNative
                    ? "VICE 3.x+: edit a JSON list with emulator_id, content (absolute ROM path), probe_program, sdl_library (the SDL2 library the VICE build links), executable_sha256, and players. Each player has player (1–2) and controller_id. Paths must be absolute. Launch passes a private -config and -joymap pair; the user's own vicerc is never touched. Digital joystick pins plus fire2/fire3 are mapped on the control ports; keysets, paddles, potentiometers, mouse and userport adapters are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.stellaNative
                    ? "Stella 7.x: edit a JSON list with emulator_id, content (absolute ROM path), base_directory (persistent private -basedir holding stella.sqlite3 and native saves), probe_program, sdl_library (the SDL3 library the Stella build links), executable_sha256, and players. Each player has player (1–2) and controller_id. Paths must be absolute. Launch runs Stella with -basedir, SDL_JOYSTICK_LINUX_CLASSIC=1 and a private settings database; the user's own Stella configuration is never touched. Joystick, Booster Grip/Genesis buttons and console switches are mapped; paddles, driving controllers, keypads and Stelladaptors are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.bsnes
                    ? "bsnes v115+ settings.bml: edit a JSON list with emulator_id, content (absolute ROM path), probe_program, sdl_library (the SDL2 library the bsnes build links), executable_sha256, and players. Each player has player (1–2) and controller_id. Paths must be absolute. Launch writes a private settings.bml via --settings and probes SDL2 numbering at launch with the trusted library; the user's own settings file is never touched. Two controller ports; Mouse, Super Multitap, Super Scope and Justifier targets are not covered. Runtime testing remains deferred; review opens no devices."
                    : duckstationSetups.sameboy
                    ? "SameBoy SDL v1.0.3: edit a JSON list with emulator_id, content, source_config (binary preferences file), probe_program, sdl_library, bubblewrap_program, executable_sha256, runtime, and players containing exactly one entry with player: 1 and controller_id. All paths must be absolute. Native SDL device zero must be the selected controller. Runtime declares abi: sdl103-enums32-bool8 and data_directory: {kind: not-compiled} or {kind: compiled, path: absolute-directory}. These must describe the trusted executable build; they are not automatically verified. Native dispatch is connected; tilt-game axis behavior remains unresolved. Review opens no devices."
                    : duckstationSetups.fceux
                    ? "Edit a JSON list with emulator_id, content, base_directory (native FCEUX data/config root), probe_program, sdl_library, bubblewrap_program, executable_sha256, and players. Each player has player (1–4) and controller_id. Paths must be absolute and runtime files trusted. This targets FCEUX Qt 2.6.6 standard NES pads. Native launch dispatch is connected but ROM-selected device overrides may replace standard pads. Review opens no devices; runtime support is untested."
                    : duckstationSetups.snes9x
                    ? "Edit a JSON list with emulator_id, content, source_config (native snes9x.conf), probe_program, sdl_library, bubblewrap_program, executable_sha256, and players. Each player has player (1–5) and controller_id. Paths must be absolute and runtime files trusted. This targets Snes9x GTK 1.63, not Qt. Review uses saved calibration only; native controller and runtime checks run at launch. Runtime testing remains deferred."
                    : duckstationSetups.dolphin
                    ? "Edit a JSON list with emulator_id, content, game_id (six-character disc ID), revision, user_directory, system_directory, bubblewrap_program, executable_sha256, and players. Each player has port (1–4), controller_id and device_qualifier (native evdev/id/name). All paths must be absolute. This targets standard GameCube pads in Dolphin 2606. Review checks saved measurements only; launch checks run only when starting a game. System-data path is user-declared."
                    : duckstationSetups.mgba
                    ? "Edit a JSON list with emulator_id, content, handheld (gba or gameboy), controller_id, source_config (native config.ini), probe_program, sdl_library, bubblewrap_program, and executable_sha256. Paths must be absolute and executables/libraries trusted. This targets mGBA 0.10.5's native Linux SDL frontend, not Qt. Launch checks the runtime and isolated config handoff; review/staging open no devices. Runtime testing remains deferred."
                    : duckstationSetups.ppsspp
                    ? "Edit a JSON list with emulator_id, content (absolute path), game_id, source_system (native PSP/SYSTEM path), controller_id, probe_program, sdl_library, mapping_database (PPSSPP's bundled gamecontrollerdb.txt), bubblewrap_program, and executable_sha256. All paths must be absolute and executables/libraries trusted. Native Linux SDL2 launches use a private SYSTEM overlay; saves stay in their native location. The child must confirm its runtime and mapping order. This implementation has not been runtime-tested."
                    : "Edit a JSON list with emulator_id, content (absolute ROM path), data_root, serial, first_disc_serial (null only for a confirmed single-disc game), and players. Players need pad (1–8), controller_id and controller_type (DigitalController or AnalogController). For native launch add runtime: {probe_program, sdl_library, runtime_libraries: [absolute dependency paths], executable_sha256}. These must belong to trusted DuckStation 0a53bc47c / SDL 3.2.20. Existing controller types are preserved; Flatpak/Wine are not enabled."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 190
                TextArea {
                    id: duckstationEditor
                    font.family: "monospace"
                    wrapMode: TextEdit.Wrap
                    onTextChanged: duckstationSetups.review = []
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                ColumnLayout {
                    width: parent.width
                    Repeater {
                        model: duckstationSetups.review
                        delegate: ColumnLayout {
                            id: nativeReviewPlayer
                            required property var modelData
                            Layout.fillWidth: true
                            Label { text: "Pad " + modelData.pad + ": " + modelData.source_layout + " → " + modelData.target_layout }
                            ControllerMappingView {
                                Layout.fillWidth: true
                                settingsModel: setup.settingsModel
                                sourceLayout: duckstationSetups.catalog.layouts.find(layout => layout.id === nativeReviewPlayer.modelData.source_layout) || null
                                destinationLayout: duckstationSetups.catalog.layouts.find(layout => layout.id === nativeReviewPlayer.modelData.target_layout) || null
                                rows: nativeReviewPlayer.modelData.mapping.rows
                            }
                        }
                    }
                }
            }
            Label { id: duckstationStatus; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            RowLayout {
                Button {
                    text: "Review mappings"
                    onClicked: {
                        const result = JSON.parse(duckstationSetups.melonds
                            ? setup.settingsModel.review_melonds_setups(duckstationEditor.text)
                            : duckstationSetups.rpcs3
                            ? setup.settingsModel.review_rpcs3_setups(duckstationEditor.text)
                            : duckstationSetups.pcsx2
                            ? setup.settingsModel.review_pcsx2_setups(duckstationEditor.text)
                            : duckstationSetups.flycastNative
                            ? setup.settingsModel.review_flycast_native_setups(duckstationEditor.text)
                            : duckstationSetups.mednafen
                            ? setup.settingsModel.review_mednafen_setups(duckstationEditor.text)
                            : duckstationSetups.xemuNative
                            ? setup.settingsModel.review_xemu_native_setups(duckstationEditor.text)
                            : duckstationSetups.blastemNative
                            ? setup.settingsModel.review_blastem_native_setups(duckstationEditor.text)
                            : duckstationSetups.mesen2Native
                            ? setup.settingsModel.review_mesen2_native_setups(duckstationEditor.text)
                            : duckstationSetups.hatariNative
                            ? setup.settingsModel.review_hatari_native_setups(duckstationEditor.text)
                            : duckstationSetups.viceNative
                            ? setup.settingsModel.review_vice_native_setups(duckstationEditor.text)
                            : duckstationSetups.stellaNative
                            ? setup.settingsModel.review_stella_native_setups(duckstationEditor.text)
                            : duckstationSetups.bsnes
                            ? setup.settingsModel.review_bsnes_setups(duckstationEditor.text)
                            : duckstationSetups.sameboy
                            ? setup.settingsModel.review_sameboy_setups(duckstationEditor.text)
                            : duckstationSetups.fceux
                            ? setup.settingsModel.review_fceux_setups(duckstationEditor.text)
                            : duckstationSetups.snes9x
                            ? setup.settingsModel.review_snes9x_setups(duckstationEditor.text)
                            : duckstationSetups.dolphin
                            ? setup.settingsModel.review_dolphin_setups(duckstationEditor.text)
                            : duckstationSetups.mgba
                            ? setup.settingsModel.review_mgba_setups(duckstationEditor.text)
                            : duckstationSetups.ppsspp
                            ? setup.settingsModel.review_ppsspp_setups(duckstationEditor.text)
                            : setup.settingsModel.review_duckstation_setups(duckstationEditor.text))
                        let players = []
                        if (!result.error) for (const item of result.setups) players = players.concat(item.players)
                        duckstationSetups.review = players
                        duckstationStatus.text = result.error || result.setups.map(function(item) {
                            return item.detail || "Mapping review only; runtime readiness is not verified."
                        }).join("\n") || "No saved setups to review."
                    }
                }
                Button {
                    text: "Stage setups"
                    onClicked: {
                        const error = duckstationSetups.melonds
                            ? setup.settingsModel.stage_melonds_setups(duckstationEditor.text)
                            : duckstationSetups.rpcs3
                            ? setup.settingsModel.stage_rpcs3_setups(duckstationEditor.text)
                            : duckstationSetups.pcsx2
                            ? setup.settingsModel.stage_pcsx2_setups(duckstationEditor.text)
                            : duckstationSetups.flycastNative
                            ? setup.settingsModel.stage_flycast_native_setups(duckstationEditor.text)
                            : duckstationSetups.mednafen
                            ? setup.settingsModel.stage_mednafen_setups(duckstationEditor.text)
                            : duckstationSetups.xemuNative
                            ? setup.settingsModel.stage_xemu_native_setups(duckstationEditor.text)
                            : duckstationSetups.blastemNative
                            ? setup.settingsModel.stage_blastem_native_setups(duckstationEditor.text)
                            : duckstationSetups.mesen2Native
                            ? setup.settingsModel.stage_mesen2_native_setups(duckstationEditor.text)
                            : duckstationSetups.hatariNative
                            ? setup.settingsModel.stage_hatari_native_setups(duckstationEditor.text)
                            : duckstationSetups.viceNative
                            ? setup.settingsModel.stage_vice_native_setups(duckstationEditor.text)
                            : duckstationSetups.stellaNative
                            ? setup.settingsModel.stage_stella_native_setups(duckstationEditor.text)
                            : duckstationSetups.bsnes
                            ? setup.settingsModel.stage_bsnes_setups(duckstationEditor.text)
                            : duckstationSetups.sameboy
                            ? setup.settingsModel.stage_sameboy_setups(duckstationEditor.text)
                            : duckstationSetups.fceux
                            ? setup.settingsModel.stage_fceux_setups(duckstationEditor.text)
                            : duckstationSetups.snes9x
                            ? setup.settingsModel.stage_snes9x_setups(duckstationEditor.text)
                            : duckstationSetups.dolphin
                            ? setup.settingsModel.stage_dolphin_setups(duckstationEditor.text)
                            : duckstationSetups.mgba
                            ? setup.settingsModel.stage_mgba_setups(duckstationEditor.text)
                            : duckstationSetups.ppsspp
                            ? setup.settingsModel.stage_ppsspp_setups(duckstationEditor.text)
                            : setup.settingsModel.stage_duckstation_setups(duckstationEditor.text)
                        duckstationStatus.text = error || (duckstationSetups.melonds
                            ? "melonDS setups staged. Save settings on the main page. Native Linux launch integration is partial and untested; no devices opened."
                            : duckstationSetups.rpcs3
                            ? "RPCS3 setups staged. Save settings on the main page. Native integration is partial and untested; no devices opened."
                            : duckstationSetups.pcsx2
                            ? "PCSX2 setups staged. Save settings on the main page. Native launch integration is partial and untested; no devices opened."
                            : duckstationSetups.flycastNative
                            ? "Staged. Save settings on the main page. Standalone Flycast uses partial native Linux launch mapping; no devices were opened during review."
                            : duckstationSetups.mednafen
                            ? "Staged. Save settings in the main page. Mednafen GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine native dispatch is partial and untested; no devices were opened."
                            : duckstationSetups.xemuNative
                            ? "Staged. Save settings in the main page. xemu native dispatch writes a private -config_path configuration at launch; no devices were opened."
                            : duckstationSetups.blastemNative
                            ? "Staged. Save settings in the main page. BlastEm native dispatch writes a private blastem.cfg under an isolated HOME at launch; no devices were opened."
                            : duckstationSetups.mesen2Native
                            ? "Staged. Save settings in the main page. Mesen2 native dispatch writes a private settings.json under an isolated XDG_DATA_HOME at launch; no devices were opened."
                            : duckstationSetups.hatariNative
                            ? "Staged. Save settings in the main page. Hatari native dispatch stages a private HOME and -c configuration at launch; no devices were opened."
                            : duckstationSetups.viceNative
                            ? "Staged. Save settings in the main page. VICE native dispatch stages a private -config/-joymap pair at launch; no devices were opened."
                            : duckstationSetups.stellaNative
                            ? "Staged. Save settings in the main page. Stella native dispatch writes a private stella.sqlite3 under its -basedir at launch; no devices were opened."
                            : duckstationSetups.bsnes
                            ? "Staged. Save settings in the main page. bsnes native dispatch writes a private settings.bml at launch; no devices were opened."
                            : duckstationSetups.sameboy
                            ? "Staged. Save settings in the main page. SameBoy native dispatch is partial; runtime details are user-declared and tilt-game axes remain unresolved; no devices were opened."
                            : duckstationSetups.fceux
                            ? "Staged. Save settings in the main page. FCEUX native dispatch is partial: ROM device overrides remain unresolved. No devices were opened."
                            : duckstationSetups.snes9x
                            ? "Staged. Save settings in the main page. Native Snes9x launch checks runtime and controller routing; no devices were opened while staging."
                            : duckstationSetups.dolphin
                            ? "Staged. Save settings in the main page. No devices were opened; native launch checks run when starting a game."
                            : duckstationSetups.mgba
                            ? "Staged. Save settings in the main page. Native mGBA launch will resolve and check the controller; no devices were opened while staging."
                            : duckstationSetups.ppsspp
                            ? "Staged. Save settings in the main page. Native PPSSPP launch will capture and confirm the mapping; no devices were opened while staging."
                            : "Staged. Save settings in the main page. Native launch validates the configured runtime and confirms actual startup routing; staging opens no devices.")
                    }
                }
                Button { text: "Close"; onClicked: duckstationSetups.close() }
            }
        }
    }
    Button {
        text: "PPSSPP standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "ppsspp"
            duckstationEditor.text = setup.settingsModel.ppsspp_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Review checks saved intent only. Native runtime and controller checks occur when launching."
            duckstationSetups.open()
        }
    }
    Button {
        text: "mGBA SDL standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "mgba"
            duckstationEditor.text = setup.settingsModel.mgba_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Saved mapping review only. Native SDL runtime, device and config checks occur at launch."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Dolphin GameCube standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "dolphin"
            duckstationEditor.text = setup.settingsModel.dolphin_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Review shows saved physical mappings only. Native Dolphin launch checks run when starting a game."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Snes9x GTK standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "snes9x"
            duckstationEditor.text = setup.settingsModel.snes9x_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Saved mapping review only. Native Snes9x GTK checks controller routing at launch."
            duckstationSetups.open()
        }
    }
    Button {
        text: "FCEUX Qt standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "fceux"
            duckstationEditor.text = setup.settingsModel.fceux_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Saved mapping review only. FCEUX Qt dispatch is partial; ROM device overrides remain unresolved."
            duckstationSetups.open()
        }
    }
    Button {
        text: "SameBoy SDL standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "sameboy"
            duckstationEditor.text = setup.settingsModel.sameboy_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "SameBoy native dispatch is partial; runtime details are user-declared and tilt-game axes remain unresolved. Review opens no devices."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Mednafen standalone setups…"
        onClicked: {
            duckstationSetups.adapter = "mednafen"
            duckstationEditor.text = setup.settingsModel.mednafen_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Mednafen GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine native dispatch is partial and untested. Review opens no devices."
            duckstationSetups.open()
        }
    }
    Button {
        text: "Standalone Flycast panels…"
        onClicked: {
            duckstationSetups.adapter = "flycast-native"
            duckstationEditor.text = setup.settingsModel.flycast_native_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Standard six/eight-button panels: partial native Linux dispatch connected; runtime behavior remains untested."
            duckstationSetups.open()
        }
    }
    Button {
        text: "PCSX2 controller setups…"
        onClicked: {
            duckstationSetups.adapter = "pcsx2"
            duckstationEditor.text = setup.settingsModel.pcsx2_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "DualShock 2 native Linux dispatch is partial and untested; review opens no devices."
            duckstationSetups.open()
        }
    }
    Button {
        text: "RPCS3 controller setups…"
        onClicked: {
            duckstationSetups.adapter = "rpcs3"
            duckstationEditor.text = setup.settingsModel.rpcs3_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Standard pads: partial native Linux file-boot dispatch; runtime behavior untested."
            duckstationSetups.open()
        }
    }
    Button {
        text: "melonDS controller setups…"
        onClicked: {
            duckstationSetups.adapter = "melonds"
            duckstationEditor.text = setup.settingsModel.melonds_setups_json()
            duckstationSetups.review = []
            duckstationStatus.text = "Twelve standard DS controls; partial native Linux launch integration, untested."
            duckstationSetups.open()
        }
    }
    ControllerFbneoSetupDialog {
        // Standard native PCSX2 uses the shared editor above.
        // Flycast standalone setup is separate from libretro FBNeo.
        // Native mapping editor is above; arcade settings stay independent.
        // Separate per-game arcade editor; native standalone setups above share
        // only the presentation, never their saved records or input protocols.
        id: fbneoSetups
        settingsModel: setup.settingsModel
    }
    ControllerMameSetupDialog {
        id: mameSetups
        settingsModel: setup.settingsModel
        onCalibrationRequested: function(controllerId, sourceLayout, physicalControl) {
            const matches = []
            for (let index = 0; index < setup.settingsModel.controller_count(); ++index) {
                if (setup.settingsModel.controller_key_at(index) === controllerId) matches.push(index)
            }
            if (matches.length !== 1) {
                mameSetups.statusText = "Cannot open calibration: this saved controller identity is missing or ambiguous in the current inventory. Reconnect it or choose another player controller. The game draft is unchanged."
                return
            }
            try {
                calibration.openFor(controllerId, setup.settingsModel.controller_name_at(matches[0]))
                if (physicalControl && !calibration.focusSavedControl(sourceLayout, physicalControl)) {
                    calibration.status = "The requested physical control no longer matches this saved layout. Recording is paused. Choose the correct layout/control or explicitly resume; existing bindings have not been reset."
                }
            } catch (error) {
                mameSetups.statusText = "Cannot open this controller's calibration: " + error + ". The game draft is unchanged."
            }
        }
    }
    ControllerLayoutExplorer { id: layoutExplorer; settingsModel: setup.settingsModel }
    Button { text: "Explore source → destination layouts…"; onClicked: layoutExplorer.open() }
    Button {
        text: "Relative mouse / trackball settings…"
        onClicked: {
            relativeEditor.text = setup.settingsModel.relative_device_settings_json()
            relativeStatus.text = ""
            relativeSettings.open()
        }
    }
    Button {
        text: "Absolute calibration records (advanced)…"
        onClicked: absoluteSettings.loadAndOpen()
    }
    ControllerAbsoluteSettingsDialog {
        id: absoluteSettings
        settingsModel: setup.settingsModel
    }
    ControllerRelativeDeviceForm {
        id: relativeForm
        onDeviceAdded: device => {
            try {
                const devices = JSON.parse(relativeEditor.text)
                if (!Array.isArray(devices)) throw new Error("The draft must be a JSON list.")
                if (devices.length >= 16) throw new Error("At most 16 devices may be saved.")
                if (devices.some(existing => existing.event_path === device.event_path || existing.input_identity === device.input_identity))
                    throw new Error("That event path or physical identity is already in the draft. Edit its existing entry.")
                devices.push(device)
                relativeEditor.text = JSON.stringify(devices, null, 2)
                relativeStatus.text = "Added to draft only. Stage settings to validate the complete list."
            } catch (error) {
                relativeStatus.text = "Device was not added: " + error.message
            }
        }
    }
    Dialog {
        id: relativeSettings
        onClosed: relativeForm.close()
        title: "Saved relative devices"
        modal: true
        width: 780
        height: 540
        closePolicy: Popup.NoAutoClose
        contentItem: ColumnLayout {
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Edit the device list as JSON. Each entry needs event_path, input_identity, axes and buttons; motion contains x_percent, y_percent, invert_x, invert_y and swap_xy. Axes: 0=X, 1=Y, 6=horizontal wheel, 8=vertical wheel. Buttons are [physical, output] code pairs (272=left, 273=right, 274=middle). An empty list removes saved entries."
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "These paths must identify the exact device and may need updating after reconnect/reboot. Staging checks the data only. Frontend routing is unfinished: saving these settings does not enable emulator mouse input or start capture."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                TextArea { id: relativeEditor; selectByMouse: true; font.family: "monospace"; wrapMode: TextEdit.NoWrap }
            }
            Label { id: relativeStatus; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            RowLayout {
                Button {
                    text: "Add device with form…"
                    onClicked: relativeForm.open()
                }
                Button {
                    text: "Stage settings"
                    onClicked: {
                        const error = setup.settingsModel.stage_relative_device_settings(relativeEditor.text)
                        relativeStatus.text = error || "Staged. Save settings in the main page to persist this list. No device was opened."
                    }
                }
                Button { text: "Close"; onClicked: relativeSettings.close() }
            }
        }
    }
    RowLayout {
        Layout.fillWidth: true
        Label { text: "MAME arcade default" }
        ComboBox {
            id: mameDefault
            Layout.fillWidth: true
            readonly property var ids: ["automatic", "six_button", "eight_button", "neo_geo", "fixed_channels", "disabled"]
            model: ["Standard six-button arcade (no game inspection)", "Six buttons: 1 2 3 / 4 5 6", "Eight buttons: 1 2 3 7 / 4 5 6 8", "Advanced per-game Neo Geo", "Advanced per-game fixed channels", "Per-game setups only"]
            currentIndex: { setup.revision; return ids.indexOf(setup.settingsModel.mame_arcade_layout()) }
            onActivated: {
                const error = setup.settingsModel.choose_mame_arcade_layout(ids[index])
                mameDefaultStatus.text = error || "Arcade default staged. Save settings to persist it."
            }
        }
        Button {
            text: "Preview layout…"
            enabled: mameDefault.currentIndex >= 0 && mameDefault.ids[mameDefault.currentIndex] !== "disabled"
            onClicked: arcadePreview.open()
        }
    }
    Dialog {
        id: arcadePreview
        title: "MAME arcade button positions"
        width: Math.min(740, setup.width)
        modal: true
        standardButtons: Dialog.Close
        readonly property var panels: {
            const choice = mameDefault.ids[mameDefault.currentIndex]
            const six = {id: "arcade-six-button", name: "Six-button arcade"}
            const eight = {id: "arcade-eight-button", name: "Eight-button arcade"}
            if (choice === "automatic") return [six]
            if (choice === "six_button") return [six]
            if (choice === "eight_button") return [eight]
            if (choice === "neo_geo") return [{id: "neogeo", name: "Neo Geo"}]
            if (choice === "fixed_channels") return [{id: "mame-fixed-digital", name: "Fixed frontend channels"}]
            return []
        }
        contentItem: ColumnLayout {
            Repeater {
                model: arcadePreview.panels
                delegate: ColumnLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    Label { text: modelData.name }
                    Image {
                        Layout.fillWidth: true
                        Layout.preferredHeight: Math.min(220, width * 500 / 900)
                        fillMode: Image.PreserveAspectFit
                        source: arcadePreview.visible ? setup.settingsModel.controller_diagram(modelData.id, "") : ""
                        sourceSize.width: Math.max(1, Math.ceil(width * Screen.devicePixelRatio))
                        sourceSize.height: Math.max(1, Math.ceil(height * Screen.devicePixelRatio))
                        Accessible.name: modelData.name + " button positions"
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Six and eight buttons use the same first-six positions; eight adds a rightmost column. Only controls used by the game need mapping. Automatic may choose a different panel for twin sticks or unusual inputs. This preview shows positions, not a verified per-game mapping."
            }
        }
    }
    Label {
        id: mameDefaultStatus
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: "Standard six/eight-button arcade layouts require no per-game input inspection. Saved advanced setups still win; dependency discovery below applies to advanced inspected setups. Cabinet-specific layouts and peripherals are outside this mapping scope. Not runtime-tested yet."
    }
    CheckBox {
        text: "Discover MAME dependencies beside the selected archive"
        checked: { setup.revision; return setup.settingsModel.mame_dependency_discovery() }
        onToggled: {
            setup.settingsModel.choose_mame_dependency_discovery(checked)
            mameDefaultStatus.text = "Dependency discovery staged. Each required ROM set needs its own archive in that folder; CHDs may use set/parent subfolders. Merged archives need an explicit setup. Save settings to persist this choice."
        }
    }
    Button {
        text: "MAME per-game setups (advanced)…"
        onClicked: mameSetups.loadAndOpen()
    }
    Button {
        text: "FBNeo per-game setups (advanced)…"
        onClicked: fbneoSetups.loadAndOpen()
    }
    Dialog {
        id: nativeCapture
        property string controllerId: ""
        property string errorText: ""
        property int selectedChange: -1
        property var targetControls: []
        property string runtimeSelectionKey: ""
        readonly property var runtimeChoices: {
            setup.revision
            return JSON.parse(setup.settingsModel.native_controller_runtimes_json())
        }
        onRuntimeChoicesChanged: {
            setup.settingsModel.cancel_native_controller_capture()
            captureRuntimeChoice.currentIndex = runtimeChoices.findIndex(value => JSON.stringify(value.key) === runtimeSelectionKey)
        }
        readonly property var savedBindings: {
            setup.revision
            const saved = JSON.parse(setup.settingsModel.scoped_native_controller_calibration_json(controllerId, runtimeSelectionKey))
            return Object.entries(saved.bindings || {}).map(([control, gesture]) => ({ control: control, gesture: gesture }))
        }
        readonly property var changes: {
            try { return JSON.parse(setup.settingsModel.native_capture_results) }
            catch (_) { return [] }
        }
        function openFor(id, name) {
            setup.settingsModel.cancel_native_controller_capture()
            controllerId = id
            title = "Native SDL2 gesture — " + name
            errorText = ""
            selectedChange = -1
            runtimeSelectionKey = runtimeChoices.length === 1 ? JSON.stringify(runtimeChoices[0].key) : ""
            captureRuntimeChoice.currentIndex = runtimeChoices.length === 1 ? 0 : -1
            const saved = JSON.parse(setup.settingsModel.controller_calibration_json(id))
            const layout = calibration.catalog.layouts.find(item => item.id === saved.layout)
            targetControls = layout ? layout.controls.filter(control => saved.bindings && saved.bindings[control.id]) : []
            open()
        }
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(650, parent ? parent.width - 40 : 650)
        height: Math.min(540, parent ? parent.height - 40 : 540)
        modal: true
        standardButtons: Dialog.Close
        onClosed: { clearNativeBindings.close(); setup.settingsModel.cancel_native_controller_capture() }
        onChangesChanged: selectedChange = -1
        contentItem: ScrollView {
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: parent.width
                spacing: 10
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: "Requires a configured trusted native BizHawk runtime. Release all controls and capture the released state. Then hold one control and capture its pressed state. Use the mouse to operate these buttons so controller navigation does not interrupt the gesture."
                }
                ComboBox {
                    id: captureRuntimeChoice
                    Layout.fillWidth: true
                    model: nativeCapture.runtimeChoices
                    textRole: "name"
                    currentIndex: -1
                    displayText: currentIndex >= 0 ? currentText : "Select the native runtime to calibrate"
                    onActivated: {
                        nativeCapture.runtimeSelectionKey = JSON.stringify(nativeCapture.runtimeChoices[currentIndex].key)
                        setup.settingsModel.cancel_native_controller_capture()
                    }
                    Accessible.name: "Native capture runtime and SDL library"
                }
                RowLayout {
                    Button {
                        text: "1. Capture released"
                        enabled: !setup.settingsModel.native_capture_busy
                        onClicked: {
                            if (captureRuntimeChoice.currentIndex < 0 || captureRuntimeChoice.currentIndex >= nativeCapture.runtimeChoices.length) {
                                nativeCapture.errorText = "Choose the saved runtime whose SDL library should be calibrated."
                                return
                            }
                            nativeCapture.errorText = setup.settingsModel.begin_native_controller_capture(nativeCapture.controllerId,
                                JSON.stringify(nativeCapture.runtimeChoices[captureRuntimeChoice.currentIndex].key))
                        }
                    }
                    Button {
                        text: "2. Capture pressed"
                        enabled: setup.settingsModel.native_capture_ready && !setup.settingsModel.native_capture_busy
                        onClicked: nativeCapture.errorText = setup.settingsModel.finish_native_controller_capture()
                    }
                    Button {
                        text: "Cancel capture"
                        onClicked: {
                            setup.settingsModel.cancel_native_controller_capture()
                            nativeCapture.errorText = ""
                        }
                    }
                }
                BusyIndicator {
                    running: setup.settingsModel.native_capture_busy
                    visible: running
                    Layout.alignment: Qt.AlignHCenter
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: nativeCapture.errorText || setup.settingsModel.native_capture_status
                    color: nativeCapture.errorText ? "#ffb454" : "#95a2b6"
                }
                ButtonGroup { id: nativeChanges }
                Repeater {
                    model: nativeCapture.changes
                    RadioButton {
                        required property int index
                        required property var modelData
                        ButtonGroup.group: nativeChanges
                        checked: nativeCapture.selectedChange === index
                        text: modelData.output + (modelData.analog ? " (axis)" : " (button)")
                            + ": " + modelData.released + " → " + modelData.pressed
                        onClicked: nativeCapture.selectedChange = index
                    }
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: nativeCapture.targetControls.length
                        ? "Choose which calibrated physical control this gesture represents. Use Save settings to keep recorded bindings. Native BizHawk digital pads and directly representable DualShock sticks use these records; asymmetric or composite stick mappings still need normalized transport. This implementation has not been tested yet."
                        : "Choose a layout and save physical calibration first, then reopen this dialog to record logical bindings."
                    color: "#ffb454"
                }
                ComboBox {
                    id: nativeTarget
                    Layout.fillWidth: true
                    model: nativeCapture.targetControls
                    textRole: "label"
                    Accessible.name: "Calibrated control represented by this gesture"
                }
                Button {
                    text: "Record selected logical binding"
                    enabled: nativeCapture.selectedChange >= 0 && nativeTarget.currentIndex >= 0
                        && !setup.settingsModel.native_capture_busy
                    onClicked: nativeCapture.errorText = setup.settingsModel.save_native_controller_gesture(
                        nativeCapture.controllerId, nativeCapture.targetControls[nativeTarget.currentIndex].id,
                        nativeCapture.selectedChange)
                }
                Label {
                    text: "Recorded logical bindings: " + nativeCapture.savedBindings.length
                    font.bold: true
                }
                Repeater {
                    model: nativeCapture.savedBindings
                    Label {
                        required property var modelData
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: modelData.control + " → " + modelData.gesture.output
                            + " (" + modelData.gesture.released + " → " + modelData.gesture.pressed + ")"
                    }
                }
                Button {
                    text: "Clear this runtime's recorded bindings…"
                    enabled: nativeCapture.savedBindings.length > 0 && !setup.settingsModel.native_capture_busy
                    onClicked: { clearNativeBindings.controllerId = nativeCapture.controllerId; clearNativeBindings.runtimeKey = nativeCapture.runtimeSelectionKey; clearNativeBindings.open() }
                }
            }
        }
    }
    Dialog {
        id: clearNativeBindings
        property string controllerId: ""
        property string runtimeKey: ""
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(450, parent ? parent.width - 40 : 450)
        modal: true
        title: "Clear this controller's logical bindings?"
        standardButtons: Dialog.Ok | Dialog.Cancel
        Label {
            width: parent.width
            wrapMode: Text.WordWrap
            text: "This removes scoped SDL2 bindings for this controller and runtime only. Other scopes and legacy fallback bindings remain unchanged. Legacy bindings may apply again after clearing this scope. The change reaches disk only when you save settings."
        }
        onAccepted: setup.settingsModel.clear_scoped_native_controller_calibration(controllerId, runtimeKey)
    }
    readonly property int revision: settingsModel.controller_revision
    readonly property var profiles: {
        revision
        let choices = [{ id: "", name: "Automatic layout" },
                       { id: "none", name: "Keep native mapping" },
                       { id: "two-button-clockwise", name: "Two-button: left run / bottom jump" }]
        for (let index = 0; index < settingsModel.custom_controller_profile_count(); ++index)
            choices.push({ id: settingsModel.custom_controller_profile_id_at(index),
                           name: settingsModel.custom_controller_profile_name_at(index) })
        return choices
    }
    readonly property var layouts: [
        { id: "auto", name: "Unverified — choose a layout" },
        { id: "diamond", name: "Standard diamond (PS5 / Steam / Xbox)" },
        { id: "horizontal", name: "Horizontal: left B / right A" },
        { id: "horizontal-swapped", name: "Horizontal: left A / right B (swap)" },
        { id: "nintendo", name: "Nintendo diamond: swap A/B and X/Y" }
    ]
    spacing: 10

    Label {
        text: "Smart controller setup"
        font.pixelSize: 20
        font.bold: true
    }
    Button {
        text: "Controller coverage — implemented / remaining…"
        onClicked: coverage.open()
    }
    Button {
        visible: Qt.platform.os === "linux"
        text: "Native BizHawk runtime and player setup…"
        onClicked: nativeRuntime.loadAndOpen()
    }

    Switch {
        text: "Automatic selection for existing launch profiles"
        checked: setup.settingsModel.controller_automatic
        onToggled: setup.settingsModel.set_controller_automatic_enabled(checked)
    }
    Label {
        Layout.fillWidth: true
        text: setup.calibratedCoverage + " Automatic RetroArch overrides/remaps are suspended for the session. Contracts that require core options also suspend per-game core options; saved files are unchanged. Other cores/emulators still need adapters. Disable Apply saved calibrations to keep native setup."
        color: "#ffb454"
        wrapMode: Text.WordWrap
    }
    Label {
        Layout.fillWidth: true
        visible: !setup.settingsModel.controller_remapping_available
        text: "Advanced virtual-controller routing below needs a supported, managed InputPlumber device on Linux. Calibrated RetroArch launch and Lunchbox navigation work independently of InputPlumber."
        wrapMode: Text.WordWrap
        color: "#ffb454"
    }
    RowLayout {
        visible: Qt.platform.os === "linux"
        Button {
            text: "Enable Linux controller routing…"
            enabled: !setup.settingsModel.controller_busy
            onClicked: { routingConfirmation.enableRouting = true; routingConfirmation.open() }
        }
        Button {
            text: "Use native controller routing…"
            enabled: !setup.settingsModel.controller_busy
            onClicked: { routingConfirmation.enableRouting = false; routingConfirmation.open() }
        }
    }
    Dialog {
        id: routingConfirmation
        property bool enableRouting: true
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(520, parent ? parent.width - 40 : 520)
        modal: true
        title: enableRouting ? "Enable system-wide controller routing?" : "Return to native routing?"
        standardButtons: Dialog.Ok | Dialog.Cancel
        Label {
            width: parent.width
            wrapMode: Text.WordWrap
            text: "This changes the running InputPlumber service for all supported controllers, including other applications. It can change Steam Input routing. Close running games first. No system packages or drivers will be installed."
        }
        onAccepted: setup.settingsModel.configure_controller_routing(enableRouting)
    }
    Label {
        Layout.fillWidth: true
        text: "For two-button systems, horizontal pads keep their comfortable left/right layout; diamond pads use left for run and bottom for jump. Save settings after changes."
        wrapMode: Text.WordWrap
        color: "#95a2b6"
    }
    Label {
        Layout.fillWidth: true
        text: setup.gamepad.last_input.length ? "Input check: " + setup.gamepad.last_input
                                             : "Press a controller button to see what it reports."
        wrapMode: Text.WordWrap
        color: "#62d6c6"
    }
    Switch {
        text: "Test controller input — pause menu navigation"
        checked: setup.testInput
        onToggled: setup.testInput = checked
    }
    Switch {
        text: "Apply saved controller mappings at emulator launch"
        Accessible.description: "Apply saved gamepad calibrations and per-game setups, including relative-only MAME and FBNeo input. Runtime requirements still apply."
        checked: setup.settingsModel.controller_calibrated_launch
        onToggled: setup.settingsModel.set_controller_calibrated_launch_enabled(checked)
    }
    Label {
        Layout.fillWidth: true
        text: "Press a button: its controller row flashes and shows the reported control. South/East/West/North are the reported positions, not necessarily the labels printed on your pad. Test mode prevents presses from activating settings."
        wrapMode: Text.WordWrap
        color: "#95a2b6"
    }
    CheckBox {
        id: showVirtualControllers
        text: "Show virtual controllers"
        checked: false
    }
    Repeater {
        model: { setup.revision; return setup.settingsModel.controller_count() }
        delegate: ColumnLayout {
            id: device
            visible: {
                setup.revision
                const review = JSON.parse(setup.settingsModel.controller_model_review(
                    setup.settingsModel.controller_key_at(index), ""))
                return showVirtualControllers.checked || !review.steam_virtual
            }
            required property int index
            Layout.fillWidth: true
            ControllerInputFeedback {
                Layout.fillWidth: true
                gamepad: setup.gamepad
                controllerName: { setup.revision; return setup.settingsModel.controller_name_at(device.index) }
                matchesInput: {
                    setup.revision
                    return setup.settingsModel.controller_receives_input(device.index, setup.gamepad.last_device_key)
                }
            }
            TextField {
                Layout.fillWidth: true
                text: { setup.revision; return setup.settingsModel.controller_alias_at(device.index) }
                placeholderText: "Name this controller (e.g. N30 or Retro Fighters)"
                maximumLength: 80
                onEditingFinished: setup.settingsModel.rename_controller(device.index, text)
                Accessible.name: "Rename " + setup.settingsModel.controller_name_at(device.index)
            }
            Button {
                text: "Choose layout and calibrate…"
                onClicked: calibration.openFor(setup.settingsModel.controller_key_at(device.index),
                    setup.settingsModel.controller_name_at(device.index))
            }
            ColumnLayout {
                visible: { setup.revision; return setup.settingsModel.controller_key_at(device.index).startsWith("sdl3:") }
                Layout.fillWidth: true
                Button {
                    text: "Use native SDL3 mapping"
                    enabled: !setup.settingsModel.busy
                    onClicked: {
                        const error = setup.settingsModel.use_sdl3_controller_mapping(setup.settingsModel.controller_key_at(device.index))
                        nativeSdlResult.text = error || "SDL3 mapping saved. Choose layout and calibrate to review or edit it."
                    }
                }
                Label { id: nativeSdlResult; Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: text.length > 0 }
            }
            Button {
                visible: Qt.platform.os === "linux"
                text: "Capture native SDL2 gesture…"
                onClicked: nativeCapture.openFor(setup.settingsModel.controller_key_at(device.index),
                    setup.settingsModel.controller_name_at(device.index))
            }
            ComboBox {
                Layout.fillWidth: true
                model: setup.layouts
                textRole: "name"
                currentIndex: {
                    setup.revision
                    const layout = setup.settingsModel.controller_layout_at(device.index)
                    return Math.max(0, setup.layouts.findIndex(option => option.id === layout))
                }
                onActivated: setup.settingsModel.choose_controller_layout(device.index, setup.layouts[currentIndex].id)
                Accessible.name: "Physical layout for " + setup.settingsModel.controller_name_at(device.index)
            }
        }
    }
    Label {
        Layout.fillWidth: true
        text: "N64 / unusual controllers: use the profile editor below to assign physical controls. C-buttons reported as right-stick directions are supported. Do not select a wiring preset unless the input check agrees."
        wrapMode: Text.WordWrap
        color: "#95a2b6"
    }
    Label {
        Layout.fillWidth: true
        text: "Some USB adapters identify as Xbox controllers regardless of their physical layout. Verify those once. Devices without a unique serial are remembered by USB port; keep each controller on its configured port."
        wrapMode: Text.WordWrap
        color: "#95a2b6"
    }
    Repeater {
        model: [
            { id: "two-button", name: "NES / Game Boy / PC Engine" },
            { id: "n64", name: "Nintendo 64" },
            { id: "six-button", name: "Mega Drive / Saturn / Arcade" },
            { id: "modern", name: "SNES / modern systems" }
        ]
        delegate: RowLayout {
            required property var modelData
            Layout.fillWidth: true
            Label { text: parent.modelData.name; Layout.preferredWidth: 220; wrapMode: Text.WordWrap }
            ComboBox {
                Layout.fillWidth: true
                model: {
                    setup.revision
                    let names = ["Automatic / first available"]
                    for (let index = 0; index < setup.settingsModel.controller_count(); ++index)
                        names.push(setup.settingsModel.controller_name_at(index))
                    return names
                }
                currentIndex: { setup.revision; return setup.settingsModel.controller_preference_at(parent.modelData.id) }
                onActivated: setup.settingsModel.choose_preferred_controller(parent.modelData.id, currentIndex)
                Accessible.name: "Preferred controller for " + parent.modelData.name
            }
            ComboBox {
                Layout.fillWidth: true
                model: setup.profiles
                textRole: "name"
                enabled: { setup.revision; return setup.settingsModel.controller_preference_at(parent.modelData.id) > 0 }
                currentIndex: {
                    setup.revision
                    const profile = setup.settingsModel.preferred_controller_profile(parent.modelData.id)
                    return Math.max(0, setup.profiles.findIndex(option => option.id === profile))
                }
                onActivated: setup.settingsModel.choose_preferred_controller_profile(parent.modelData.id, setup.profiles[currentIndex].id)
                Accessible.name: "Controller layout for " + parent.modelData.name
            }
        }
    }
    Label {
        Layout.fillWidth: true
        text: "Choose a preferred controller to assign a different custom profile for each system family. Explicit player assignments and game overrides below take priority. Disconnected preferences are retained."
        color: "#95a2b6"
        wrapMode: Text.WordWrap
    }
}
