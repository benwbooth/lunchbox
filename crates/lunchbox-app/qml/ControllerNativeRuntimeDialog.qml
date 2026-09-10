import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: runtime
    required property var settingsModel
    property var players: []
    property string errorText: ""
    property var emulatorChoices: []
    property var savedSetups: []
    property string loadedSetup: "null"
    property string catalogError: ""
    property var snes9xPorts: null
    property var neshawkPorts: null
    property var smshawkSystem: null
    property var pcehawkPorts: null
    property var turbonymaTopology: null
    property var gpgxTopology: null
    readonly property bool isGpgx: gpgxTopology !== null
    onGpgxTopologyChanged: nativeLayouts.close()
    readonly property bool gpgxTopologyValid: !isGpgx || ((gpgxTopology.team_players || [false, false]).every((tap, port) => !tap || gpgxTopology.ports[port])
        && (!gpgxTopology.wayplay || (gpgxTopology.ports.every(Boolean) && !(gpgxTopology.team_players || []).some(Boolean)))
        && (gpgxTopology.activators || [false, false]).every((active, port) => !active || (gpgxTopology.ports[port] && !(gpgxTopology.team_players || [false, false])[port] && !gpgxTopology.wayplay)))
    readonly property bool gpgxPlayersComplete: !isGpgx || (gpgxTopologyValid && players.length === portCapacity && unassignedPorts.length === 0)
    readonly property var gpgxConnectors: {
        if (!isGpgx) return []
        if (gpgxTopology.wayplay) return [0, 1, 2, 3].map(slot => "4-Way Play socket " + (slot + 1) + " (both connectors)")
        let slots = []
        const taps = gpgxTopology.team_players || [false, false]
        for (let port = 0; port < 2; ++port) {
            if (!gpgxTopology.ports[port]) continue
            for (let socket = 0; socket < (taps[port] ? 4 : 1); ++socket)
                slots.push((port === 0 ? "left" : "right") + (taps[port] ? " Team Player socket " + (socket + 1) : (gpgxTopology.activators || [false, false])[port] ? " Activator" : " normal pad"))
        }
        return slots
    }
    function editGpgxTopology(change) {
        gpgxTopology = Object.assign({ team_players: [false, false], wayplay: false, activators: [false, false] }, gpgxTopology, change)
    }
    readonly property bool isTurboNyma: turbonymaTopology !== null
    readonly property var fixedPcePorts: isPceHawk ? pcehawkPorts : isTurboNyma ? turbonymaTopology.ports : null
    readonly property bool turboTopologyValid: !isTurboNyma || turbonymaTopology.multitap || !turbonymaTopology.ports.slice(1).some(Boolean)
    onTurbonymaTopologyChanged: nativeLayouts.close()
    readonly property bool isPceHawk: pcehawkPorts !== null
    onPcehawkPortsChanged: nativeLayouts.close()
    readonly property bool pcePlayersComplete: fixedPcePorts === null || (players.length === fixedPcePorts.filter(Boolean).length
        && fixedPcePorts.every((connected, port) => players.filter(player => player.virtual_port === port).length === (connected ? 1 : 0)))
    readonly property bool isSnes9x: snes9xPorts !== null
    readonly property bool isNesHawk: neshawkPorts !== null
    readonly property bool isNintendo: isSnes9x || isNesHawk
    readonly property bool isSmsHawk: smshawkSystem !== null
    readonly property bool isDigitalCore: isNintendo || isSmsHawk || isPceHawk || isTurboNyma || isGpgx
    onSmshawkSystemChanged: nativeLayouts.close()
    readonly property string previewCore: isSnes9x ? "snes9x" : isNesHawk ? "neshawk" : isSmsHawk ? (smshawkSystem === "game_gear" ? "smshawk-gg" : smshawkSystem === "sg1000" ? "smshawk-sg" : "smshawk-sms") : isPceHawk ? "pcehawk" : isTurboNyma ? "turbonyma" : isGpgx ? (gpgxTopology.pad === "six_button" ? "gpgx-six" : "gpgx-three") : "nymashock"
    function snesPortCapacity(port) { return port === "multitap" ? 4 : port === "joypad" ? 1 : 0 }
    readonly property int portCapacity: isSnes9x
        ? snesPortCapacity(snes9xPorts[0]) + snesPortCapacity(snes9xPorts[1])
        : isNesHawk ? neshawkPorts.reduce((count, port) => count + nesPortCapacity(port), 0)
        : isSmsHawk ? (smshawkSystem === "game_gear" ? 1 : 2)
        : fixedPcePorts !== null ? 5
        : isGpgx ? gpgxConnectors.length
        : 2 + (firstTap.checked ? 3 : 0) + (secondTap.checked ? 3 : 0)
    function editSnesPort(index, value) {
        let copy = snes9xPorts.slice()
        copy[index] = value
        snes9xPorts = copy
    }
    onSnes9xPortsChanged: nativeLayouts.close()
    onNeshawkPortsChanged: nativeLayouts.close()
    function editNesPort(index, value) {
        let copy = neshawkPorts.slice()
        copy[index] = value
        neshawkPorts = copy
    }
    function nesPortCapacity(port) { return port === "four_score" ? 2 : (port === "joypad" || port === "snes_joypad" || port === "power_pad") ? 1 : 0 }
    function playerPreviewMode(player) {
        if (isGpgx) {
            if (!gpgxTopologyValid || player.virtual_port >= portCapacity) return "invalid-gpgx-topology"
            if (gpgxTopology.wayplay) return previewCore
            let remaining = player.virtual_port
            for (let port = 0; port < 2; ++port) {
                if (!gpgxTopology.ports[port]) continue
                const count = (gpgxTopology.team_players || [false, false])[port] ? 4 : 1
                if (remaining < count) return (gpgxTopology.activators || [false, false])[port] ? "gpgx-activator" : previewCore
                remaining -= count
            }
            return "invalid-gpgx-topology"
        }
        if (!isNintendo) return previewCore
        const slot = nintendoSlots.find(value => value.player === player.virtual_port)
        if (!slot) throw new Error("This player is outside the configured Nintendo deck.")
        if (isSnes9x) return previewCore
        return slot.device === "snes_joypad" ? "neshawk-snespad" : slot.device === "power_pad" ? "neshawk-powerpad" : "neshawk"
    }
    readonly property var nintendoSlots: {
        if (!isNintendo) return []
        const ports = isSnes9x ? snes9xPorts : neshawkPorts
        const names = { joypad: "Joypad", multitap: "Multitap", four_score: "Four Score half", snes_joypad: "SNES adapter", power_pad: "Power Pad" }
        let slots = []
        for (let port = 0; port < ports.length; ++port) {
            const device = ports[port]
            const count = isSnes9x ? snesPortCapacity(device) : nesPortCapacity(device)
            for (let index = 0; index < count; ++index) {
                slots.push({ player: slots.length, device: device,
                    label: (port === 0 ? "Left port" : "Right port") + " · " + names[device]
                        + (count > 1 ? " · core slot " + (index + 1) : "") })
            }
        }
        return slots
    }
    readonly property var missingNintendoSlots: nintendoSlots.filter(slot => !players.some(player => player.virtual_port === slot.player))
    readonly property var unassignedPorts: {
        let ports = []
        for (let port = 0; port < portCapacity; ++port) {
            if (fixedPcePorts !== null && (!fixedPcePorts[port] || (isTurboNyma && port > 0 && !turbonymaTopology.multitap))) continue
            if (!players.some(player => player.virtual_port === port)) ports.push(port)
        }
        return ports
    }
    readonly property var controllers: {
        settingsModel.controller_revision
        let values = []
        for (let index = 0; index < settingsModel.controller_count(); ++index)
            values.push({ id: settingsModel.controller_key_at(index), name: settingsModel.controller_name_at(index) })
        for (const saved of JSON.parse(settingsModel.saved_controller_choices_json()))
            if (saved.valid && !values.some(value => value.id === saved.id))
                values.push({ id: saved.id, name: saved.name + " — " + saved.layout + " (saved; not in current inventory)" })
        for (const player of players)
            if (!values.some(value => value.id === player.controller_id))
                values.push({ id: player.controller_id, name: player.controller_id + " (saved player; calibration/device unavailable)" })
        return values
    }
    function editPlayer(index, key, value) {
        let copy = players.slice()
        copy[index] = Object.assign({}, copy[index])
        copy[index][key] = value
        if ((key === "dualshock" || key === "dualanalog") && value) copy[index].analog_joystick = false
        if (key === "analog_joystick" && value) {
            copy[index].dualshock = false
            copy[index].dualanalog = false
            copy[index].analog_toggle_id = null
            copy[index].rumble = false
        }
        if (key === "dualshock" && value) copy[index].dualanalog = false
        if (key === "dualanalog" && value) {
            copy[index].dualshock = false
            copy[index].analog_toggle_id = null
            copy[index].rumble = false
        }
        if (key === "dualshock" && !value) {
            copy[index].analog_toggle_id = null
            copy[index].rumble = false
        }
        if (key === "controller_id") copy[index].analog_toggle_id = null
        if (key === "normalized_input" && value) copy[index].rumble = false
        players = copy
    }
    function editMode(index, mode) {
        let copy = players.slice()
        copy[index] = Object.assign({}, copy[index])
        copy[index].dualshock = mode === 1
        copy[index].dualanalog = mode === 2
        copy[index].analog_joystick = mode === 3
        copy[index].negcon = mode === 6
        copy[index].pointer = mode === 7 ? "mouse" : mode === 8 ? "gun_con" : mode === 9 ? "justifier" : null
        if (mode !== 8 && mode !== 9) copy[index].desktop_cursor = false
        copy[index].rhythm = mode === 4 ? "dance_pad" : mode === 5 ? "popn_music" : null
        if (mode !== 1) {
            copy[index].analog_toggle_id = null
            copy[index].rumble = false
        }
        players = copy
    }
    function loadAndOpen() {
        savedSetups = JSON.parse(settingsModel.native_controller_runtimes_json())
        const saved = JSON.parse(settingsModel.native_controller_runtime_json())
        loadSetup(saved)
    }
    function loadSetup(saved) {
        loadedSetup = JSON.stringify(saved)
        const available = JSON.parse(settingsModel.native_controller_emulators_json())
        emulatorChoices = available.choices
        catalogError = available.error
        if (saved && !emulatorChoices.some(choice => choice.id === saved.emulator_id))
            emulatorChoices = emulatorChoices.concat([{ id: saved.emulator_id, name: "Saved emulator (not in current catalog)" }])
        emulatorId.text = saved ? saved.emulator_id : ""
        emulatorChoice.currentIndex = emulatorChoices.findIndex(choice => choice.id === emulatorId.text)
        exeDirectory.text = saved ? saved.exe_directory : ""
        probeProgram.text = saved ? saved.probe_program : ""
        sdlLibrary.text = saved ? saved.sdl_library : ""
        directMono.checked = !!(saved && saved.mono_program)
        monoProgram.text = saved && saved.mono_program ? saved.mono_program : ""
        libraryDirectories.text = saved && saved.library_paths ? saved.library_paths.join("\n") : ""
        unmanagedDirectory.text = saved && saved.unmanaged_directory ? saved.unmanaged_directory : ""
        managedDirectories.text = saved && saved.managed_paths ? saved.managed_paths.join("\n") : ""
        dataDirectory.text = saved && saved.data_directory ? saved.data_directory : ""
        baseConfig.text = saved && saved.base_config ? saved.base_config : ""
        firstTap.checked = saved ? saved.multitaps[0] : false
        secondTap.checked = saved ? saved.multitaps[1] : false
        snes9xPorts = saved ? saved.snes9x_ports || null : null
        neshawkPorts = saved ? saved.neshawk_ports || null : null
        smshawkSystem = saved ? saved.smshawk_system || null : null
        pcehawkPorts = saved ? saved.pcehawk_ports || null : null
        turbonymaTopology = saved ? saved.turbonyma_topology || null : null
        gpgxTopology = saved ? saved.gpgx_topology || null : null
        players = saved ? saved.players : []
        errorText = ""
        open()
    }
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(760, parent ? parent.width - 40 : 760)
    height: Math.min(720, parent ? parent.height - 40 : 720)
    modal: true
    title: "Native BizHawk controller runtime"
    standardButtons: Dialog.Close
    onClosed: { nativeLayouts.close(); removeSavedSetup.close() }
    onPlayersChanged: nativeLayouts.close()
    readonly property int mappingRevision: settingsModel.controller_revision
    onMappingRevisionChanged: {
        nativeLayouts.close()
        savedSetups = JSON.parse(settingsModel.native_controller_runtimes_json())
        savedSetupChoice.currentIndex = -1
    }
    Dialog {
        id: removeSavedSetup
        property string expected: ""
        property string setupName: ""
        title: "Remove selected native setup?"
        modal: true
        anchors.centerIn: parent
        width: Math.min(500, runtime.width - 30)
        standardButtons: Dialog.Ok | Dialog.Cancel
        Label {
            width: parent.width
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            text: "Remove " + removeSavedSetup.setupName + " from staged settings? Other setups and all calibration records stay unchanged. The current draft is retained, but start a new draft if you later want to recreate a removed setup."
        }
        onAccepted: runtime.errorText = runtime.settingsModel.remove_native_controller_runtime(expected)
    }
    function previewPlayerLayouts(player) {
        try {
            const result = JSON.parse(settingsModel.native_controller_player_layout_json(JSON.stringify(player), runtime.playerPreviewMode(player), emulatorId.text))
            if (result.error) throw new Error(result.error)
            const catalog = JSON.parse(settingsModel.controller_catalog_json())
            const source = catalog.layouts.find(layout => layout.id === result.source_layout)
            const target = catalog.layouts.find(layout => layout.id === result.target_layout)
            if (!source || !target) throw new Error("A controller layout is missing from the catalog.")
            nativeLayouts.panels = [{role: "Source controller", layout: source}, {role: "Emulated controller", layout: target}]
            nativeLayouts.rows = result.rows || []
            nativeLayouts.logicalRows = result.logical_rows || []
            nativeLayouts.logicalError = result.logical_error || ""
            nativeLayouts.exclusions = result.exclusions || []
            savedSdlPreview.checked = false
            nativeLayouts.open()
        } catch (error) { errorText = "Cannot preview controller layouts: " + error }
    }
    Dialog {
        id: nativeLayouts
        property var panels: []
        property var rows: []
        property var logicalRows: []
        property string logicalError: ""
        property var exclusions: []
        title: runtime.isSnes9x ? "Native SNES controller layouts" : runtime.isNesHawk ? "Native NES controller layouts" : runtime.isSmsHawk ? "Native Sega controller layouts" : "Native PlayStation controller layouts"
        width: runtime.width - 40
        height: runtime.height - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ScrollView {
            id: nativePreviewScroll
            clip: true
            contentWidth: availableWidth
            ColumnLayout {
                width: nativePreviewScroll.availableWidth
            CheckBox {
                id: savedSdlPreview
                text: "Restrict to saved logical SDL calibration"
                enabled: nativeLayouts.logicalError.length === 0
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                visible: nativeLayouts.logicalError.length > 0
                text: "Saved SDL view unavailable: " + nativeLayouts.logicalError
            }
            ControllerMappingView {
                Layout.fillWidth: true
                settingsModel: runtime.settingsModel
                sourceLayout: nativeLayouts.panels.length ? nativeLayouts.panels[0].layout : null
                destinationLayout: nativeLayouts.panels.length > 1 ? nativeLayouts.panels[1].layout : null
                rows: savedSdlPreview.checked ? nativeLayouts.logicalRows : nativeLayouts.rows
            }
            Repeater {
                model: nativeLayouts.exclusions
                Label {
                    required property string modelData
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    text: modelData
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Preliminary " + (savedSdlPreview.checked ? "physical + saved SDL" : "physical-calibration") + " mapping, not final native bindings. The mode toggle is reserved; desktop-cursor axes are excluded. Saved SDL mode uses the intersection of both calibrations, but does not verify that the live SDL mapping still matches. Normalized sessions use a newly created virtual device rather than these saved source SDL bindings. Native gesture/axis translation and runtime eligibility remain unchecked."
            }
            }
        }
    }
    contentItem: ScrollView {
        clip: true
        contentWidth: availableWidth
        ColumnLayout {
            width: parent.width
            spacing: 10
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Linux native BizHawk controller setup. Choose a trusted probe executable and the SDL2 library used by this installation. Capturing a gesture loads that library in a separate process. Staging these settings does not execute either file. Wine and Flatpak runtimes need separate adapters. Native controller implementations remain untested."
                color: "#ffb454"
            }
            Label { text: "BizHawk catalog entry" }
            ComboBox {
                id: savedSetupChoice
                Layout.fillWidth: true
                model: runtime.savedSetups
                textRole: "name"
                currentIndex: -1
                displayText: currentIndex >= 0 ? currentText : "Choose a saved emulator / system setup"
                Accessible.name: "Saved native controller setup"
            }
            RowLayout {
                Button {
                    text: "Load selected setup — replace draft"
                    enabled: savedSetupChoice.currentIndex >= 0 && savedSetupChoice.currentIndex < runtime.savedSetups.length
                    onClicked: {
                        const selected = runtime.savedSetups[savedSetupChoice.currentIndex]
                        const fresh = JSON.parse(runtime.settingsModel.native_controller_runtimes_json())
                        const saved = fresh.find(value => JSON.stringify(value.key) === JSON.stringify(selected.key))
                        if (!saved) { runtime.errorText = "This setup is no longer saved. Reopen the editor to refresh choices."; return }
                        runtime.loadSetup(saved.configuration)
                    }
                }
                Button {
                    text: "New setup — replace draft"
                    onClicked: runtime.loadSetup(null)
                }
            }
            Button {
                text: "Remove selected saved setup…"
                enabled: savedSetupChoice.currentIndex >= 0 && savedSetupChoice.currentIndex < runtime.savedSetups.length
                onClicked: {
                    const selected = runtime.savedSetups[savedSetupChoice.currentIndex]
                    removeSavedSetup.expected = JSON.stringify(selected.configuration)
                    removeSavedSetup.setupName = selected.name
                    removeSavedSetup.open()
                }
            }
            ComboBox {
                id: emulatorChoice
                Layout.fillWidth: true
                model: runtime.emulatorChoices
                textRole: "name"
                onActivated: emulatorId.text = runtime.emulatorChoices[currentIndex].id
                Accessible.name: "BizHawk catalog entry"
            }
            TextField { id: emulatorId; Layout.fillWidth: true; readOnly: true; Accessible.name: "Selected BizHawk emulator ID" }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: runtime.catalogError || "Catalog presence is not proof of installation. Supply this native installation's actual paths below."
                color: "#95a2b6"
            }
            Label { text: "Actual EmuHawk executable directory" }
            TextField { id: exeDirectory; Layout.fillWidth: true; placeholderText: "Absolute directory containing EmuHawk.exe"; Accessible.name: "EmuHawk directory" }
            Label { text: "Trusted controller-probe executable" }
            TextField { id: probeProgram; Layout.fillWidth: true; placeholderText: "Absolute path to lunchbox-controller-probe"; Accessible.name: "Controller probe executable" }
            Label { text: "This runtime's SDL2 shared library" }
            TextField { id: sdlLibrary; Layout.fillWidth: true; placeholderText: "Absolute SDL2 library path"; Accessible.name: "SDL2 library" }
            CheckBox { id: directMono; text: "Invoke Mono directly with a shared probe/emulator environment" }
            Label {
                Layout.fillWidth: true
                visible: directMono.checked
                wrapMode: Text.WordWrap
                text: "This explicitly bypasses the selected EmuHawk shell wrapper. Supply the real Mono ELF executable and dependency library directories. Split installations can specify their existing native-library directory, writable data directory and source configuration below. Assets must already be installed; wrapper asset copying and theme setup are not reproduced automatically."
                color: "#ffb454"
            }
            TextField {
                id: monoProgram
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Absolute path to the real Mono executable (not a wrapper)"
                Accessible.name: "Direct Mono executable"
            }
            TextArea {
                id: libraryDirectories
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Additional native dependency directories, one absolute path per line"
                Accessible.name: "Native dependency library directories"
            }
            TextField {
                id: unmanagedDirectory
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Optional native BizHawk library directory (default: installation/dll)"
                Accessible.name: "Native BizHawk library directory"
            }
            TextArea {
                id: managedDirectories
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Additional managed assembly directories, one absolute path per line (optional)"
                Accessible.name: "Managed assembly search directories"
            }
            TextField {
                id: dataDirectory
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Optional existing data/working directory (default: installation)"
                Accessible.name: "BizHawk data directory"
            }
            TextField {
                id: baseConfig
                Layout.fillWidth: true
                visible: directMono.checked
                placeholderText: "Optional existing source configuration (copied privately for launch)"
                Accessible.name: "Source BizHawk configuration"
            }
            ComboBox {
                Layout.fillWidth: true
                model: ["PlayStation — Nymashock", "Super Nintendo — Snes9x", "Nintendo — NesHawk", "Master System — SMSHawk", "Game Gear — SMSHawk", "SG-1000 — SMSHawk", "PC Engine / TurboGrafx — PCEHawk (two-button)", "PC Engine / TurboGrafx — TurboNyma (six-button controls)", "Genesis / Sega CD — GPGX"]
                currentIndex: runtime.isSnes9x ? 1 : runtime.isNesHawk ? 2 : runtime.isSmsHawk ? (runtime.smshawkSystem === "game_gear" ? 4 : runtime.smshawkSystem === "sg1000" ? 5 : 3) : runtime.isPceHawk ? 6 : runtime.isTurboNyma ? 7 : runtime.isGpgx ? 8 : 0
                enabled: runtime.players.length === 0
                onActivated: {
                    const selected = currentIndex
                    runtime.snes9xPorts = selected === 1 ? ["joypad", "joypad"] : null
                    runtime.neshawkPorts = selected === 2 ? ["joypad", "none"] : null
                    runtime.smshawkSystem = selected === 3 ? "master_system" : selected === 4 ? "game_gear" : selected === 5 ? "sg1000" : null
                    runtime.pcehawkPorts = selected === 6 ? [true, false, false, false, false] : null
                    runtime.turbonymaTopology = selected === 7 ? { ports: [true, false, false, false, false], multitap: false } : null
                    runtime.gpgxTopology = selected === 8 ? { ports: [true, false], pad: "three_button" } : null
                    firstTap.checked = false
                    secondTap.checked = false
                }
                Accessible.name: "Native BizHawk core"
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Remove existing players before changing cores. Port changes retain assignments; reassign or remove any players outside the new deck."
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isSmsHawk
                wrapMode: Text.WordWrap
                text: runtime.smshawkSystem === "game_gear"
                    ? "Game Gear exposes player one only: D-pad, buttons 1/2 and Start. Reset keeps its existing console binding."
                    : "Master System / SG-1000 exposes two standard pad ports: D-pad and buttons 1/2. Assign either or both ports. Unassigned ports have no host bindings; Pause and Reset keep existing core console bindings."
            }
            RowLayout {
                visible: !runtime.isDigitalCore
                CheckBox { id: firstTap; text: "Physical port 1 multitap" }
                CheckBox { id: secondTap; text: "Physical port 2 multitap" }
            }
            RowLayout {
                visible: runtime.fixedPcePorts !== null
                Repeater {
                    model: runtime.fixedPcePorts !== null ? 5 : 0
                    delegate: CheckBox {
                        required property int index
                        text: "P" + (index + 1) + " connected"
                        checked: runtime.fixedPcePorts ? runtime.fixedPcePorts[index] : false
                        onToggled: {
                            let ports = runtime.fixedPcePorts.slice()
                            ports[index] = checked
                            if (runtime.isPceHawk) runtime.pcehawkPorts = ports
                            else runtime.turbonymaTopology = { ports: ports, multitap: runtime.turbonymaTopology.multitap }
                        }
                    }
                }
            }
            RowLayout {
                visible: runtime.isGpgx
                Repeater {
                    model: runtime.isGpgx ? 2 : 0
                    delegate: CheckBox {
                        required property int index
                        text: index === 0 ? "Left pad connected" : "Right pad connected"
                        checked: runtime.isGpgx && runtime.gpgxTopology.ports[index]
                        onToggled: {
                            let ports = runtime.gpgxTopology.ports.slice()
                            ports[index] = checked
                            runtime.editGpgxTopology({ ports: ports })
                        }
                    }
                }
                ComboBox {
                    model: ["Three-button pads", "Six-button pads"]
                    currentIndex: runtime.isGpgx && runtime.gpgxTopology.pad === "six_button" ? 1 : 0
                    onActivated: runtime.editGpgxTopology({ pad: currentIndex === 1 ? "six_button" : "three_button" })
                    Accessible.name: "GPGX pad mode for both connectors"
                }
            }
            RowLayout {
                visible: runtime.isGpgx
                Repeater {
                    model: runtime.isGpgx ? 2 : 0
                    delegate: CheckBox {
                        required property int index
                        text: index === 0 ? "Left Team Player" : "Right Team Player"
                        checked: runtime.isGpgx && !!(runtime.gpgxTopology.team_players || [false, false])[index]
                        onToggled: {
                            let taps = (runtime.gpgxTopology.team_players || [false, false]).slice()
                            taps[index] = checked
                            runtime.editGpgxTopology({ team_players: taps })
                        }
                    }
                }
                CheckBox {
                    text: "4-Way Play (both connectors)"
                    checked: runtime.isGpgx && !!runtime.gpgxTopology.wayplay
                    onToggled: runtime.editGpgxTopology({ wayplay: checked })
                }
            }
            RowLayout {
                visible: runtime.isGpgx
                Repeater {
                    model: runtime.isGpgx ? 2 : 0
                    delegate: CheckBox {
                        required property int index
                        text: index === 0 ? "Left Activator" : "Right Activator"
                        checked: runtime.isGpgx && !!(runtime.gpgxTopology.activators || [false, false])[index]
                        onToggled: {
                            let active = (runtime.gpgxTopology.activators || [false, false]).slice()
                            active[index] = checked
                            runtime.editGpgxTopology({ activators: active })
                        }
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isGpgx && (runtime.gpgxTopology.activators || []).some(Boolean)
                wrapMode: Text.WordWrap
                text: "Each Activator requires sixteen independent calibrated sensor inputs and an enabled connector without a multitap. The preview is a channel grid, not physical ring orientation or verified Activator hardware support."
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isGpgx
                wrapMode: Text.WordWrap
                text: "GPGX uses one pad mode for all normal controllers. Players compact past disconnected ports: a right-only pad is P1. Requested routing: "
                    + runtime.gpgxConnectors.map((label, player) => "P" + (player + 1) + " → " + label).join(", ")
                    + ". Team Player needs its connector enabled; 4-Way Play needs both connectors and no Team Players. Some games override controllers; loaded-game behavior is unverified. J-Cart and special peripherals remain unfinished."
                    + (runtime.gpgxTopologyValid ? "" : " Adapter/connector choices conflict; recording is blocked.")
                    + (runtime.gpgxPlayersComplete ? "" : " Fill every connected pad slot before recording.")
            }
            CheckBox {
                visible: runtime.isTurboNyma
                text: "TurboNyma multitap (required for P2–P5)"
                checked: runtime.isTurboNyma && runtime.turbonymaTopology.multitap
                onToggled: runtime.turbonymaTopology = { ports: runtime.turbonymaTopology.ports.slice(), multitap: checked }
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isTurboNyma
                wrapMode: Text.WordWrap
                text: "TurboNyma: I–VI, D-pad, Select/Run and separate Set 2-button / Set 6-button inputs. Six-button mode requires compatible games; no initial mode is forced. P1–P5 retain fixed numbers. Use a distinct emulator ID or remove a conflicting PCEHawk setup."
                    + (runtime.turboTopologyValid ? "" : " P2–P5 are connected but the multitap is off; recording is blocked.")
                    + (runtime.pcePlayersComplete ? "" : " Assign exactly one player to each connected port before recording.")
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isPceHawk
                wrapMode: Text.WordWrap
                text: "Two-button pads only: D-pad, I, II, Select and Run. P1–P5 keep their numbers when earlier ports are disconnected. Assign exactly one player to each connected port. Six-button pads are not supported by this core contract."
                    + (runtime.pcePlayersComplete ? "" : " Connected ports and player assignments do not yet match; recording is blocked.")
            }
            RowLayout {
                visible: runtime.isSnes9x
                Repeater {
                    model: 2
                    ColumnLayout {
                        required property int index
                        Label { text: index === 0 ? "SNES left port" : "SNES right port" }
                        ComboBox {
                            model: ["none", "joypad", "multitap"]
                            currentIndex: runtime.isSnes9x ? model.indexOf(runtime.snes9xPorts[parent.index]) : 0
                            onActivated: runtime.editSnesPort(parent.index, currentText)
                            Accessible.name: parent.index === 0 ? "SNES left port device" : "SNES right port device"
                        }
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isSnes9x
                wrapMode: Text.WordWrap
                text: "SNES joypad = 1 player; multitap = 4. Fill every connected slot, left port first and then right. Empty physical ports do not reserve a player number. Mouse and light-gun modes are not implemented here."
            }
            RowLayout {
                visible: runtime.isNesHawk
                Repeater {
                    model: 2
                    ColumnLayout {
                        id: nesPortRow
                        required property int index
                        Label { text: nesPortRow.index === 0 ? "NES left port" : "NES right port" }
                        ComboBox {
                            model: ["none", "joypad", "four_score", "snes_joypad", "power_pad"]
                            currentIndex: runtime.isNesHawk ? model.indexOf(runtime.neshawkPorts[nesPortRow.index]) : 0
                            onActivated: runtime.editNesPort(nesPortRow.index, currentText)
                            Accessible.name: nesPortRow.index === 0 ? "NES left port device" : "NES right port device"
                        }
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isNesHawk
                wrapMode: Text.WordWrap
                text: "NES/SNES-adapter joypad or Power Pad = 1 player; each Four Score half = 2. Fill every connected slot in the core's left-then-right order. Power Pad needs twelve independently pressable inputs. Famicom expansion and light-gun modes are not implemented here."
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.players.some(player => player.virtual_port >= runtime.portCapacity
                    || (runtime.fixedPcePorts !== null && !runtime.fixedPcePorts[player.virtual_port]))
                wrapMode: Text.WordWrap
                text: "A saved player is outside the current deck or assigned to a disconnected port. Restore its port topology or explicitly reassign/remove the player before recording setup."
                color: "#ffb454"
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.isNintendo && runtime.missingNintendoSlots.length > 0
                wrapMode: Text.WordWrap
                text: "Unassigned core slots: " + runtime.missingNintendoSlots.map(slot => "P" + (slot.player + 1) + " (" + slot.label + ")").join(", ")
                color: "#ffb454"
            }
            Label {
                Layout.fillWidth: true
                visible: runtime.fixedPcePorts !== null && runtime.unassignedPorts.length > 0
                wrapMode: Text.WordWrap
                text: "Connected ports needing players: " + runtime.unassignedPorts.map(port => "P" + (port + 1)).join(", ")
                color: "#ffb454"
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Save physical calibration for each controller first. Virtual ports follow the core's ordered deck: each enabled multitap adds three slots. A controller may occupy only one slot. DualShock needs a separately calibrated analog-mode toggle. Normalized input creates a gamepad only for the launch session and requires existing read access to the controller plus write access to /dev/uinput; no permissions are changed. It does not forward rumble."
            }
            Repeater {
                model: runtime.players
                Frame {
                    id: playerRow
                    required property int index
                    required property var modelData
                    readonly property var controls: {
                        runtime.settingsModel.controller_revision
                        const saved = JSON.parse(runtime.settingsModel.controller_calibration_json(modelData.controller_id))
                        return Object.keys(saved.bindings || {})
                    }
                    Layout.fillWidth: true
                    ColumnLayout {
                        anchors.fill: parent
                        RowLayout {
                            Label { text: runtime.isGpgx ? "GPGX socket" : runtime.isNintendo ? "Logical player" : "Virtual port" }
                            SpinBox {
                                visible: !runtime.isGpgx
                                from: 1; to: Math.max(1, runtime.portCapacity)
                                enabled: runtime.portCapacity > 0
                                value: playerRow.modelData.virtual_port + 1
                                onValueModified: runtime.editPlayer(playerRow.index, "virtual_port", value - 1)
                            }
                            ComboBox {
                                Layout.fillWidth: true
                                visible: runtime.isGpgx
                                model: runtime.gpgxConnectors.map((label, player) => "P" + (player + 1) + " — " + label
                                    + (runtime.players.some((entry, index) => index !== playerRow.index && entry.virtual_port === player) ? " (assigned to another controller)" : ""))
                                currentIndex: playerRow.modelData.virtual_port < runtime.gpgxConnectors.length ? playerRow.modelData.virtual_port : -1
                                displayText: currentIndex < 0 ? "Saved P" + (playerRow.modelData.virtual_port + 1) + " is outside this topology — choose a socket" : currentText
                                onActivated: runtime.editPlayer(playerRow.index, "virtual_port", currentIndex)
                                Accessible.name: "GPGX connector and socket for this controller"
                            }
                            Button {
                                text: "Remove player"
                                onClicked: { let copy = runtime.players.slice(); copy.splice(playerRow.index, 1); runtime.players = copy }
                            }
                        }
                        ComboBox {
                            Layout.fillWidth: true
                            model: runtime.controllers
                            textRole: "name"
                            currentIndex: runtime.controllers.findIndex(value => value.id === playerRow.modelData.controller_id)
                            onActivated: runtime.editPlayer(playerRow.index, "controller_id", runtime.controllers[currentIndex].id)
                            Accessible.name: "Physical controller for virtual port " + (playerRow.modelData.virtual_port + 1)
                        }
                        Label {
                            Layout.fillWidth: true
                            visible: runtime.isNintendo
                            wrapMode: Text.WordWrap
                            text: {
                                const slot = runtime.nintendoSlots.find(value => value.player === playerRow.modelData.virtual_port)
                                return slot ? "P" + (slot.player + 1) + " → " + slot.label : "Outside the configured core deck"
                            }
                        }
                        RowLayout {
                            visible: !runtime.isDigitalCore
                            ComboBox {
                                Layout.fillWidth: true
                                model: ["Digital gamepad", "DualShock", "Dual Analog (SCPH-1180)", "Analog Joystick (SCPH-1110)", "Dance Pad", "Pop'n Music", "neGcon (NPC-101)", "Mouse (controller motion)", "GunCon (controller aim)", "Justifier (controller aim)"]
                                currentIndex: playerRow.modelData.pointer === "mouse" ? 7 : playerRow.modelData.pointer === "gun_con" ? 8 : playerRow.modelData.pointer === "justifier" ? 9 : playerRow.modelData.negcon ? 6 : playerRow.modelData.rhythm === "dance_pad" ? 4 : playerRow.modelData.rhythm === "popn_music" ? 5 : playerRow.modelData.analog_joystick ? 3 : playerRow.modelData.dualanalog ? 2 : playerRow.modelData.dualshock ? 1 : 0
                                onActivated: runtime.editMode(playerRow.index, currentIndex)
                                Accessible.name: "Emulated PlayStation controller mode"
                            }
                            CheckBox {
                                text: "Rumble"
                                enabled: playerRow.modelData.dualshock && !playerRow.modelData.normalized_input
                                checked: playerRow.modelData.rumble
                                onToggled: runtime.editPlayer(playerRow.index, "rumble", checked)
                            }
                            Label { text: "Deadzone (%)" }
                            SpinBox {
                                from: 0; to: 99
                                value: Math.round(playerRow.modelData.deadzone_basis_points / 100)
                                enabled: playerRow.modelData.dualshock || !!playerRow.modelData.dualanalog || !!playerRow.modelData.analog_joystick || !!playerRow.modelData.negcon || !!playerRow.modelData.pointer
                                onValueModified: runtime.editPlayer(playerRow.index, "deadzone_basis_points", value * 100)
                            }
                        }
                        Button {
                            text: "Preview source / destination layouts…"
                            onClicked: runtime.previewPlayerLayouts(playerRow.modelData)
                        }
                        Label {
                            Layout.fillWidth: true
                            visible: runtime.isSnes9x
                            wrapMode: Text.WordWrap
                            text: "SNES joypad: D-pad, A/B/X/Y, L/R, Start and Select. Preview shows calibration candidates, not verified native bindings."
                        }
                        CheckBox {
                            text: "Normalize through a session virtual gamepad (no rumble)"
                            checked: !!playerRow.modelData.normalized_input
                            onToggled: runtime.editPlayer(playerRow.index, "normalized_input", checked)
                        }
                        RowLayout {
                            visible: !runtime.isDigitalCore && playerRow.modelData.pointer === "mouse"
                            Label { text: "Mouse motion gain (%)" }
                            SpinBox {
                                from: 1; to: 400
                                value: Math.round((playerRow.modelData.mouse_speed_basis_points === undefined ? 10000 : playerRow.modelData.mouse_speed_basis_points) / 100)
                                onValueModified: runtime.editPlayer(playerRow.index, "mouse_speed_basis_points", value * 100)
                                Accessible.name: "Controller-driven mouse motion gain"
                            }
                            Label { text: "Core movement limits still apply" }
                        }
                        CheckBox {
                            visible: !runtime.isDigitalCore && (playerRow.modelData.pointer === "gun_con" || playerRow.modelData.pointer === "justifier")
                            text: "Aim with the desktop cursor; keep buttons on this controller (one player only)"
                            checked: !!playerRow.modelData.desktop_cursor
                            onToggled: runtime.editPlayer(playerRow.index, "desktop_cursor", checked)
                        }
                        ComboBox {
                            Layout.fillWidth: true
                            visible: !runtime.isDigitalCore && playerRow.modelData.dualshock
                            model: playerRow.controls
                            currentIndex: playerRow.controls.indexOf(playerRow.modelData.analog_toggle_id)
                            displayText: currentIndex >= 0 ? currentText : "Choose a separate analog-mode toggle"
                            onActivated: runtime.editPlayer(playerRow.index, "analog_toggle_id", playerRow.controls[currentIndex])
                            Accessible.name: "DualShock analog-mode toggle"
                        }
                    }
                }
            }
            Button {
                text: "Add player"
                enabled: runtime.unassignedPorts.length > 0 && runtime.controllers.length > 0
                onClicked: {
                    const port = runtime.unassignedPorts[0]
                    if (port === undefined) { runtime.errorText = "No unassigned connected port remains. Update this core's port topology first."; return }
                    const controller = runtime.controllers.find(value => !runtime.players.some(player => player.controller_id === value.id))
                    if (!controller) { runtime.errorText = "Connect another uniquely identified controller."; return }
                    runtime.players = runtime.players.concat([{ controller_id: controller.id, virtual_port: port,
                        dualshock: false, dualanalog: false, analog_joystick: false, rhythm: null, negcon: false, pointer: null, desktop_cursor: false, mouse_speed_basis_points: 10000, analog_toggle_id: null, deadzone_basis_points: 1000, rumble: false, normalized_input: false }])
                }
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: runtime.errorText; color: "#ffb454" }
            Button {
                text: "Record runtime setup"
                enabled: runtime.players.length > 0 && runtime.gpgxPlayersComplete && runtime.pcePlayersComplete && runtime.turboTopologyValid && (!runtime.isNintendo || (runtime.players.length === runtime.portCapacity && runtime.missingNintendoSlots.length === 0))
                onClicked: {
                    runtime.errorText = runtime.settingsModel.save_native_controller_runtime(JSON.stringify({
                        emulator_id: emulatorId.text.trim(), exe_directory: exeDirectory.text.trim(),
                        probe_program: probeProgram.text.trim(), sdl_library: sdlLibrary.text.trim(),
                        mono_program: directMono.checked ? monoProgram.text.trim() : null,
                        library_paths: directMono.checked ? libraryDirectories.text.split("\n").map(path => path.trim()).filter(path => path.length) : [],
                        managed_paths: directMono.checked ? managedDirectories.text.split("\n").map(path => path.trim()).filter(path => path.length) : [],
                        unmanaged_directory: directMono.checked && unmanagedDirectory.text.trim() ? unmanagedDirectory.text.trim() : null,
                        data_directory: directMono.checked && dataDirectory.text.trim() ? dataDirectory.text.trim() : null,
                        base_config: directMono.checked && baseConfig.text.trim() ? baseConfig.text.trim() : null,
                        snes9x_ports: runtime.snes9xPorts,
                        neshawk_ports: runtime.neshawkPorts,
                        smshawk_system: runtime.smshawkSystem,
                        pcehawk_ports: runtime.pcehawkPorts,
                        turbonyma_topology: runtime.turbonymaTopology,
                        gpgx_topology: runtime.gpgxTopology,
                        multitaps: runtime.isDigitalCore ? [false, false] : [firstTap.checked, secondTap.checked], players: runtime.players }), runtime.loadedSetup)
                    if (!runtime.errorText.length) runtime.close()
                }
            }
            Button {
                text: "Disable all native controller setups in staged settings"
                onClicked: {
                    runtime.errorText = runtime.settingsModel.save_native_controller_runtime("null", "null")
                    if (!runtime.errorText.length) runtime.close()
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Use Save settings to keep changes. Disabling the adapter preserves physical and logical calibration records. This implementation has not been tested yet."
                color: "#95a2b6"
            }
        }
    }
}
