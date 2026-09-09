import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: setup
    required property var settingsModel
    required property var gamepad
    property string gameTitle: ""
    property string gamePlatform: ""
    property string gameEmulator: ""
    property int stage: 0
    onStageChanged: if (stage === 2) selectDevice(playerDevices[selectedPlayer] || "")
    property int selectedPlayer: 0
    property string selectedDevice: ""
    property string selectedProfile: ""
    property var playerDevices: [""]
    property var setupResults: ({})
    property var activeDevices: ({})
    property string lastPressedDevice: ""
    property var calibration: ({})
    property string calibrationBaseline: "{}"
    property var choices: ({})
    property var preview: ({rows: [], error: ""})
    property string status: ""
    property bool dirty: false
    property bool initialized: false
    property string modelDevice: ""
    property string renameDevice: ""
    readonly property bool testInput: visible
    readonly property bool calibrationActive: wizard.visible
    readonly property var calibrationContentItem: wizard.contentItem
    readonly property var catalog: JSON.parse(settingsModel.controller_catalog_json())
    readonly property var profile: catalog.emulator_profiles.find(item => item.id === selectedProfile) || null
    readonly property var sourceLayout: catalog.layouts.find(item => item.id === calibration.layout) || null
    readonly property var targetLayout: profile ? catalog.layouts.find(item => item.id === profile.target_layout) || null : null
    readonly property int playerLimit: profile ? targetFilter.playerLimit(profile) : 16
    readonly property var applicableTargets: targetFilter.applicable(catalog.emulator_profiles, gameEmulator, gamePlatform)
    readonly property var connectedControllers: {
        settingsModel.controller_revision
        const rows = []
        for (let i = 0; i < settingsModel.controller_count(); ++i) {
            const id = settingsModel.controller_key_at(i)
            rows.push({id: id, name: settingsModel.controller_name_at(i), index: i,
                review: JSON.parse(settingsModel.controller_model_review(id, "\u0000"))})
        }
        return rows
    }
    readonly property bool playersReady: {
        settingsModel.controller_revision
        return playerDevices.length > 0 && playerDevices.every(id => id && connected(id) && savedReady(id))
    }
    readonly property int missing: (preview.rows || []).filter(row => !row.physical_id && targetLayout
        && targetLayout.controls.some(control => control.id === row.target_id && !control.optional)).length
    spacing: 16
    ControllerTargetFilter { id: targetFilter }

    function connected(id) { return connectedControllers.find(item => item.id === id) || null }
    function saved(id) { return id ? JSON.parse(settingsModel.controller_calibration_json(id)) : ({}) }
    function savedReady(id) {
        const value = saved(id)
        const layout = catalog.layouts.find(item => item.id === value.layout)
        return !!layout && value.os === catalog.host_os && layout.controls.every(control =>
            control.optional || control.repeat_of || (value.bindings && value.bindings[control.id]))
    }
    function controllerChoices(player) {
        const current = playerDevices[player] || ""
        const rows = [{id: "", name: "Select a connected controller"}]
        for (const item of connectedControllers) {
            if ((showVirtual.checked || !item.review.steam_virtual || current === item.id)
                    && (item.id === current || playerDevices.indexOf(item.id) < 0))
                rows.push({id: item.id, name: item.name})
        }
        if (current && !connected(current)) rows.push({id: current, name: "Saved controller — disconnected"})
        return rows
    }
    function putResult(id, value) {
        const next = Object.assign({}, setupResults)
        next[id] = value
        setupResults = next
    }
    function prepareController(id) {
        if (!id || !connected(id)) return
        // Detection must never overwrite a user's recorded buttons.
        if (savedReady(id)) { putResult(id, ""); return }
        const review = connected(id).review
        if (review.native_sdl3 && !saved(id).layout)
            putResult(id, settingsModel.use_sdl3_controller_mapping(id))
    }
    function assignPlayer(player, id) {
        if (dirty || !id || !connected(id)) return
        if (playerDevices.some((other, index) => index !== player && other === id)) {
            status = "That controller already belongs to another player."
            return
        }
        const next = playerDevices.slice()
        next[player] = id
        const error = settingsModel.save_controller_player_order(JSON.stringify(next))
        if (error) { status = error; return }
        playerDevices = next
        selectedPlayer = player
        prepareController(id)
        selectDevice(id)
        status = "Player assignments saved."
    }
    function addPlayer() {
        if (playerDevices.length >= playerLimit || playerDevices.some(id => !id)) return
        playerDevices = playerDevices.concat([""])
    }
    function movePlayerUp(player) {
        if (dirty || player < 1 || !playerDevices[player]) return
        const next = playerDevices.slice()
        const previous = next[player - 1]
        next[player - 1] = next[player]; next[player] = previous
        const error = settingsModel.save_controller_player_order(JSON.stringify(next))
        if (error) { status = error; return }
        playerDevices = next; selectedPlayer = player - 1
        selectDevice(next[selectedPlayer]); status = "Player order saved."
    }
    function removeLastPlayer() {
        if (playerDevices.length < 2 || dirty) return
        const next = playerDevices.slice(0, -1)
        const error = settingsModel.save_controller_player_order(JSON.stringify(next))
        if (error) { status = error; return }
        playerDevices = next
        selectedPlayer = Math.min(selectedPlayer, next.length - 1)
        selectDevice(next[selectedPlayer])
        status = "Player assignments saved."
    }
    function restorePlayers() {
        const ids = JSON.parse(settingsModel.controller_player_order_json())
        playerDevices = ids.length ? ids : [""]
        selectedPlayer = Math.min(selectedPlayer, playerDevices.length - 1)
        for (const id of playerDevices) prepareController(id)
        selectDevice(playerDevices[selectedPlayer] || "")
    }
    function selectDevice(id) {
        selectedDevice = id
        calibrationBaseline = id ? settingsModel.controller_calibration_json(id) : "{}"
        calibration = JSON.parse(calibrationBaseline)
        loadMapping()
    }
    function loadMapping() {
        choices = calibration.target_mappings && calibration.target_mappings[selectedProfile]
            ? Object.assign({}, calibration.target_mappings[selectedProfile]) : ({})
        dirty = false; status = ""
        generate()
    }
    function generate() {
        preview = sourceLayout && selectedProfile
            ? JSON.parse(settingsModel.guided_controller_preview(selectedDevice, selectedProfile, JSON.stringify(choices)))
            : ({rows: [], error: ""})
    }
    function chooseTarget() {
        selectedProfile = applicableTargets.length === 1 ? applicableTargets[0].id : ""
        loadMapping()
    }
    function startForGame(title, platform, emulator) {
        gameTitle = title; gamePlatform = platform; gameEmulator = emulator
        stage = 0; chooseTarget(); restorePlayers()
        settingsModel.refresh_controllers()
    }
    function openCalibrationFor(index, layoutId) {
        const id = settingsModel.controller_key_at(index)
        wizard.openFor(id, settingsModel.controller_name_at(index))
        if (!saved(id).layout && layoutId) {
            const choice = wizard.catalog.layouts.findIndex(item => item.id === layoutId)
            if (choice >= 0) wizard.resetLayout(choice)
        }
    }
    function recordController(id) {
        const device = connected(id)
        if (!device) return
        const model = device.review.selected || device.review.detected
        openCalibrationFor(device.index, model ? model.layout || "" : "")
    }
    function chooseModel(id) {
        modelDevice = id; modelSearch.text = ""; modelError.text = ""
        modelDialog.refresh(); modelChoice.currentIndex = -1; modelDialog.open()
    }
    function applyModel(modelId) {
        const error = settingsModel.save_controller_model(modelDevice, modelId)
        if (error) { modelError.text = error; return }
        prepareController(modelDevice)
        modelDialog.close()
        // Foreign OS/driver numbers cannot safely become native input codes.
        // Preselect the physical layout, then guide recording where necessary.
        if (!savedReady(modelDevice)) recordController(modelDevice)
    }
    Component.onCompleted: { initialized = true; restorePlayers() }
    onVisibleChanged: if (visible && initialized && !dirty) { stage = 0; restorePlayers() }
    Connections {
        target: setup.gamepad
        function onInput_revisionChanged() {
            if (!setup.visible) return
            const id = setup.settingsModel.controller_key_for_input(setup.gamepad.last_device_key)
            if (!id) return
            setup.lastPressedDevice = id
            const next = Object.assign({}, setup.activeDevices)
            next[id] = Date.now() + 900; setup.activeDevices = next
        }
    }
    Timer {
        interval: 150; repeat: true; running: setup.visible && Object.keys(setup.activeDevices).length > 0
        onTriggered: {
            const next = {}
            for (const id of Object.keys(setup.activeDevices))
                if (setup.activeDevices[id] > Date.now()) next[id] = setup.activeDevices[id]
            setup.activeDevices = next
        }
    }
    Timer {
        interval: 3000; repeat: true
        running: setup.visible && !wizard.visible && !modelDialog.visible && !renameDialog.visible && !setup.settingsModel.controller_busy
        onTriggered: {
            setup.settingsModel.refresh_controllers()
            for (const id of setup.playerDevices) setup.prepareController(id)
        }
    }

    RowLayout {
        Repeater {
            model: ["1  Players & controllers", "2  Target system", "3  Review mapping"]
            Button {
                required property int index
                required property string modelData
                text: modelData; highlighted: setup.stage === index
                enabled: !setup.dirty && (index === 0 || (setup.playersReady && (index === 1 || (!!setup.profile && setup.playerDevices.length <= setup.playerLimit))))
                onClicked: setup.stage = index
            }
        }
    }
    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: ["Who’s playing?", "What are you playing?", "Review each player’s mapping"][setup.stage]; font.pixelSize: 24; font.bold: true }
    Label {
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: ["Choose a controller for each player. Lunchbox will restore its saved buttons or try automatic setup.",
            "Choose the emulator and system. Lunchbox matches each player’s buttons to the target controller.",
            "The left side is your controller; the right side is the system’s controller. Select a target button to inspect or change its assignment."][setup.stage]
    }
    ColumnLayout {
        visible: setup.stage === 0; Layout.fillWidth: true; spacing: 12
        Label {
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: setup.activeDevices[setup.lastPressedDevice] && setup.connected(setup.lastPressedDevice)
                ? "Button pressed on: " + setup.connected(setup.lastPressedDevice).name
                : "Not sure which controller is which? Press a button to identify it."
            color: setup.activeDevices[setup.lastPressedDevice] ? "#8ad4b7" : palette.text
        }
        Label { visible: setup.connectedControllers.length === 0; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "No controllers connected. Plug one in or pair it over Bluetooth; it will appear here." }
        Repeater {
            model: setup.playerDevices.length
            delegate: Frame {
                id: playerCard
                required property int index
                objectName: "playerCard" + index
                readonly property string deviceId: setup.playerDevices[index] || ""
                readonly property var device: setup.connected(deviceId)
                readonly property var modelReview: device ? device.review : ({})
                readonly property var physicalModel: modelReview.selected || modelReview.detected
                readonly property var saved: { setup.settingsModel.controller_revision; return setup.saved(deviceId) }
                readonly property bool ready: { setup.settingsModel.controller_revision; return !!device && setup.savedReady(deviceId) }
                readonly property bool receivingInput: !!setup.activeDevices[deviceId]
                Layout.fillWidth: true; padding: 16
                background: Rectangle { radius: 8; color: playerCard.receivingInput ? "#174c38" : "#18212b"; border.color: playerCard.receivingInput ? "#55eaa0" : "#39424d"; border.width: playerCard.receivingInput ? 2 : 1 }
                ColumnLayout {
                    anchors.fill: parent; spacing: 10
                    RowLayout {
                        Label { text: "Player " + (playerCard.index + 1); font.bold: true; font.pixelSize: 18 }
                        Label { visible: playerCard.receivingInput; text: "Button detected"; color: "#8ad4b7" }
                        Item { Layout.fillWidth: true }
                        ToolButton { text: "Move up"; visible: playerCard.index > 0 && !!playerCard.deviceId; onClicked: setup.movePlayerUp(playerCard.index); Accessible.name: "Move this controller to Player " + playerCard.index }
                        Label { text: playerCard.ready ? "Ready" : playerCard.deviceId ? "Setup needed" : "Choose a controller"; color: playerCard.ready ? "#8ad4b7" : "#ffb454" }
                    }
                    RowLayout {
                        ComboBox {
                            objectName: "playerController" + playerCard.index
                            Layout.fillWidth: true; model: setup.controllerChoices(playerCard.index); textRole: "name"
                            currentIndex: Math.max(0, model.findIndex(item => item.id === playerCard.deviceId))
                            onActivated: setup.assignPlayer(playerCard.index, model[currentIndex].id)
                            Accessible.name: "Controller for Player " + (playerCard.index + 1)
                        }
                        Button { text: "Use the controller I pressed"; visible: !playerCard.deviceId && !!setup.lastPressedDevice && setup.playerDevices.indexOf(setup.lastPressedDevice) < 0; onClicked: setup.assignPlayer(playerCard.index, setup.lastPressedDevice) }
                        Button {
                            text: "Rename"; visible: !!playerCard.device
                            onClicked: {
                                setup.renameDevice = playerCard.deviceId
                                controllerName.text = setup.settingsModel.controller_alias_at(playerCard.device.index)
                                renameError.text = ""; renameDialog.open()
                            }
                        }
                    }
                    Label {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: !!playerCard.deviceId
                        text: !playerCard.device ? "Reconnect this controller or choose a replacement. Its saved setup has not been removed."
                            : playerCard.ready ? (playerCard.physicalModel ? playerCard.physicalModel.name : (setup.catalog.layouts.find(item => item.id === playerCard.saved.layout) || {}).name)
                                + " · " + (playerCard.modelReview.native_sdl3 ? "Buttons filled automatically" : "Saved buttons restored")
                            : playerCard.physicalModel ? playerCard.physicalModel.name + " · Model " + (playerCard.modelReview.selected ? "chosen by you" : "detected")
                            : "This device doesn’t report an identifiable model. Choose its model or record its buttons."
                    }
                    Label {
                        visible: !!playerCard.device && !playerCard.ready && !!playerCard.physicalModel
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: playerCard.saved.os && playerCard.saved.os !== setup.catalog.host_os
                            ? "These saved buttons belong to another operating system. Record them once on this computer."
                            : "A verified automatic button map isn’t available for this connection. We’ll guide you through recording the buttons."
                    }
                    Label { visible: !!setup.setupResults[playerCard.deviceId]; text: setup.setupResults[playerCard.deviceId] || ""; color: "#ffb454"; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                    RowLayout {
                        visible: !!playerCard.device
                        Button { text: playerCard.ready ? "Review / change buttons" : "Record buttons"; highlighted: !playerCard.ready && !!playerCard.physicalModel; onClicked: setup.recordController(playerCard.deviceId) }
                        Button { text: playerCard.physicalModel ? "Change model" : "Choose controller model"; highlighted: !playerCard.ready && !playerCard.physicalModel; onClicked: setup.chooseModel(playerCard.deviceId) }
                        ToolButton { id: detailsToggle; text: checked ? "Hide device details" : "Device details"; checkable: true }
                    }
                    Label {
                        visible: detailsToggle.checked && !!playerCard.device
                        Layout.fillWidth: true; wrapMode: Text.WrapAnywhere
                        text: "Reported name: " + (playerCard.modelReview.device_name || "")
                            + "\nUnique hardware ID: " + (playerCard.modelReview.hardware_unique_id || "Not reported by this device")
                            + "\nConnection ID: " + playerCard.deviceId
                            + (playerCard.modelReview.steam_virtual ? "\nThis is a Steam Input virtual controller, not a separate physical device." : "")
                    }
                }
            }
        }
        RowLayout {
            Button { objectName: "addPlayer"; text: "Add player"; enabled: setup.playerDevices.length < setup.playerLimit && setup.playerDevices.every(id => !!id); onClicked: setup.addPlayer() }
            Button { text: "Remove last player"; visible: setup.playerDevices.length > 1; onClicked: setup.removeLastPlayer() }
            Item { Layout.fillWidth: true }
            Button { text: "Refresh"; enabled: !setup.settingsModel.controller_busy; onClicked: setup.settingsModel.refresh_controllers() }
        }
        CheckBox { id: showVirtual; text: "Include virtual controllers (Steam Input, etc.)" }
        Label { text: "Player assignments and controller setups are saved as you go and reused across games."; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        Button { objectName: "nextTarget"; text: "Next: target system"; highlighted: true; enabled: setup.playersReady; onClicked: setup.stage = 1 }
    }
    ColumnLayout {
        visible: setup.stage === 1; Layout.fillWidth: true; spacing: 12
        Label { visible: !!setup.gameTitle; text: setup.gameTitle + "\n" + setup.gameEmulator + " · " + setup.gamePlatform; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        ColumnLayout {
            visible: !setup.gameTitle; Layout.fillWidth: true
            Label { text: "Emulator or RetroArch core" }
            ComboBox {
                Layout.fillWidth: true; model: targetFilter.emulators(setup.catalog.emulator_profiles)
                currentIndex: model.indexOf(setup.gameEmulator)
                displayText: currentIndex < 0 ? "Choose an emulator or core" : currentText
                onActivated: {
                    setup.gameEmulator = currentText
                    const systems = targetFilter.systems(setup.catalog.emulator_profiles, currentText)
                    setup.gamePlatform = systems.length === 1 ? systems[0] : ""
                    setup.chooseTarget()
                }
            }
            Label { text: "System" }
            ComboBox {
                Layout.fillWidth: true; model: targetFilter.systems(setup.catalog.emulator_profiles, setup.gameEmulator)
                currentIndex: model.indexOf(setup.gamePlatform)
                displayText: currentIndex < 0 ? "Choose a system" : currentText
                enabled: model.length > 0
                onActivated: { setup.gamePlatform = currentText; setup.chooseTarget() }
            }
        }
        Label { text: "Target controller"; visible: setup.applicableTargets.length > 0 }
        ComboBox {
            Layout.fillWidth: true; visible: setup.applicableTargets.length > 0
            model: setup.applicableTargets; textRole: "name"
            currentIndex: model.findIndex(item => item.id === setup.selectedProfile)
            displayText: currentIndex < 0 ? "Choose the system’s controller" : currentText
            onActivated: { setup.selectedProfile = model[currentIndex].id; setup.loadMapping() }
        }
        Label {
            visible: !!setup.gameEmulator && setup.applicableTargets.length === 0
            Layout.fillWidth: true; wrapMode: Text.WordWrap; color: "#ffb454"
            text: setup.gamePlatform ? "Controller mapping isn’t supported here yet for " + setup.gameEmulator + " · " + setup.gamePlatform + ". Your player assignments and controller setups are still saved."
                : "Choose a system to see its supported controllers."
        }
        Label { visible: !!setup.profile; text: "This target supports up to " + setup.playerLimit + " player" + (setup.playerLimit === 1 ? "." : "s."); Layout.fillWidth: true; wrapMode: Text.WordWrap }
        Label { visible: !!setup.profile && setup.playerDevices.length > setup.playerLimit; text: "You have " + setup.playerDevices.length + " players selected. Go back and remove the extra players for this target."; color: "#ffb454"; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        RowLayout {
            Button { text: "Back: players"; onClicked: setup.stage = 0 }
            Button {
                objectName: "nextReview"; text: "Next: review mapping"; highlighted: true
                enabled: !!setup.profile && setup.playersReady && setup.playerDevices.length <= setup.playerLimit
                onClicked: { setup.selectedPlayer = 0; setup.selectDevice(setup.playerDevices[0]); setup.stage = 2 }
            }
        }
    }
    ColumnLayout {
        visible: setup.stage === 2; Layout.fillWidth: true; spacing: 12
        ComboBox {
            Layout.fillWidth: true
            model: setup.playerDevices.map((id, index) => "Player " + (index + 1) + " · " + (setup.connected(id) ? setup.connected(id).name : "Disconnected"))
            currentIndex: setup.selectedPlayer; enabled: !setup.dirty
            onActivated: { setup.selectedPlayer = currentIndex; setup.selectDevice(setup.playerDevices[currentIndex]) }
        }
        ControllerMappingView { id: mapping; Layout.fillWidth: true; settingsModel: setup.settingsModel; sourceLayout: setup.sourceLayout; destinationLayout: setup.targetLayout; rows: setup.preview.rows || []; simple: true }
        Label {
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: setup.preview.error || (setup.missing ? setup.missing + " required controls need an assignment." : "All required target controls have an assignment.")
            color: setup.preview.error || setup.missing ? "#ffb454" : "#8ad4b7"
        }
        RowLayout {
            visible: !!mapping.selected
            Label { text: mapping.selected ? mapping.selected.target + " ←" : "" }
            ComboBox {
                Layout.fillWidth: true
                readonly property var controls: setup.sourceLayout ? setup.sourceLayout.controls.filter(control => setup.calibration.bindings[control.id]) : []
                model: controls; textRole: "label"
                currentIndex: mapping.selected ? controls.findIndex(control => control.id === mapping.selected.physical_id) : -1
                displayText: currentIndex < 0 ? "Choose a recorded button" : currentText
                onActivated: {
                    if (!mapping.selected) return
                    const next = Object.assign({}, setup.choices)
                    next[mapping.selected.target_id] = controls[currentIndex].id
                    const result = JSON.parse(setup.settingsModel.guided_controller_preview(setup.selectedDevice, setup.selectedProfile, JSON.stringify(next)))
                    if (result.error) { setup.status = result.error; return }
                    const selected = mapping.selectedIndex
                    setup.choices = next; setup.preview = result; mapping.selectedIndex = selected
                    setup.dirty = true; setup.status = "Save or discard these changes before switching players."
                }
            }
        }
        Label {
            text: setup.profile && setup.profile.native_launch
                ? "Saved mappings are applied when you press Play. Lunchbox checks the emulator’s input support before starting the game."
                : "This preview saves button choices. Applying them at launch depends on support for the selected emulator and input backend."
            Layout.fillWidth: true; wrapMode: Text.WordWrap
        }
        RowLayout {
            Button { text: "Back: target"; enabled: !setup.dirty; onClicked: setup.stage = 1 }
            Button { text: "Reset to automatic"; onClicked: { setup.choices = ({}); setup.generate(); setup.dirty = true } }
            Button { text: "Discard changes"; visible: setup.dirty; onClicked: setup.loadMapping() }
            Button {
                text: "Save Player " + (setup.selectedPlayer + 1) + " mapping"; highlighted: true
                enabled: !setup.settingsModel.busy && !setup.preview.error && setup.preview.rows.length > 0 && setup.missing === 0
                onClicked: {
                    const error = setup.settingsModel.save_guided_controller_mapping(setup.selectedDevice, setup.selectedProfile, JSON.stringify(setup.choices), setup.calibrationBaseline)
                    if (error) setup.status = error
                    else { setup.selectDevice(setup.selectedDevice); setup.status = "Player " + (setup.selectedPlayer + 1) + " mapping saved for this target." }
                }
            }
        }
    }
    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: setup.status; visible: text.length > 0 }

    ControllerCalibrationWizard { id: wizard; settingsModel: setup.settingsModel; gamepad: setup.gamepad; guided: true; onClosed: if (deviceId) setup.selectDevice(deviceId) }
    Dialog {
        id: modelDialog
        parent: Overlay.overlay; anchors.centerIn: parent
        width: Math.min(660, parent ? parent.width - 40 : 660)
        padding: 24; modal: true; title: "Which controller are you holding?"
        property var review: ({candidates: []})
        property var models: []
        function refresh() {
            review = JSON.parse(setup.settingsModel.controller_model_review(setup.modelDevice, modelSearch.text))
            // Present names once and prefer a local connection profile.
            const groups = {}
            for (const item of review.candidates || []) {
                const key = item.name.toLowerCase()
                if (!groups[key] || (item.os === setup.catalog.host_os && groups[key].os !== setup.catalog.host_os)) groups[key] = item
            }
            models = Object.values(groups); modelChoice.currentIndex = -1
        }
        contentItem: ColumnLayout {
            spacing: 12
            Label { text: "Look for the model printed on the controller or its packaging. This chooses its shape and, where available, its known buttons."; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            TextField { id: modelSearch; Layout.fillWidth: true; placeholderText: "Search, e.g. Brawler64, 8BitDo, Steam"; onTextChanged: modelDialog.refresh() }
            ComboBox { id: modelChoice; Layout.fillWidth: true; model: modelDialog.models; textRole: "name"; currentIndex: -1; displayText: currentIndex < 0 ? "Select your controller model" : currentText }
            Label { text: modelDialog.models.length ? "If a model lists USB/Bluetooth or an input mode, match the one you’re using." : "No match found. You can choose a layout and record its buttons instead."; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Label { id: modelError; color: "#ffb454"; visible: !!text; Layout.fillWidth: true; wrapMode: Text.WordWrap }
        }
        footer: DialogButtonBox {
            Button { text: "Record manually"; onClicked: { modelDialog.close(); setup.recordController(setup.modelDevice) } }
            Button { text: "Cancel"; onClicked: modelDialog.close() }
            Button { text: "Use this model"; highlighted: true; enabled: modelChoice.currentIndex >= 0; onClicked: setup.applyModel(modelDialog.models[modelChoice.currentIndex].id) }
        }
        onOpened: modelSearch.forceActiveFocus()
    }
    Dialog {
        id: renameDialog
        parent: Overlay.overlay; anchors.centerIn: parent
        width: Math.min(460, parent ? parent.width - 40 : 460)
        padding: 24; title: "Name this controller"; modal: true
        contentItem: ColumnLayout {
            TextField { id: controllerName; Layout.fillWidth: true; maximumLength: 80; placeholderText: "e.g. Blue controller" }
            Label { text: "Use a name that helps you tell your controllers apart."; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            Label { id: renameError; Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: !!text }
        }
        footer: DialogButtonBox {
            Button { text: "Cancel"; onClicked: renameDialog.close() }
            Button { text: "Save name"; enabled: !setup.settingsModel.busy; onClicked: { renameError.text = setup.settingsModel.save_controller_name(setup.renameDevice, controllerName.text); if (!renameError.text) renameDialog.close() } }
        }
        onOpened: controllerName.forceActiveFocus()
    }
}
