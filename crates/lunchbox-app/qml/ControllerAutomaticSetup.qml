import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: setup
    required property var settingsModel
    required property var gamepad
    property bool testInput: false
    readonly property bool calibrationActive: calibration.visible
    readonly property var calibrationContentItem: calibration.contentItem
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

    Switch {
        text: "Automatic selection for existing launch profiles"
        checked: setup.settingsModel.controller_automatic
        onToggled: setup.settingsModel.set_controller_automatic_enabled(checked)
    }
    Label {
        Layout.fillWidth: true
        text: "Calibrated launch supports Linux RetroArch (native or Flatpak) with FCEUmm, Gambatte, Genesis Plus GX, Mupen64Plus-Next and Beetle PCE Fast (two-button games). RetroArch automatic overrides/remaps (and N64/PCE per-game core options) are suspended for that session; your saved files are unchanged. Other cores/emulators still need adapters. Disable Apply saved calibrations to keep an emulator’s native setup."
        color: "#ffb454"
        wrapMode: Text.WordWrap
    }
    Label {
        Layout.fillWidth: true
        visible: !setup.settingsModel.controller_remapping_available
        text: "Controller navigation works independently. Game remapping needs a supported, managed InputPlumber device on Linux; standalone Windows/macOS adapters are not available yet. You can save preferences now, but they are not applied until the backend is ready."
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
        text: "Apply saved calibrations at emulator launch"
        checked: setup.settingsModel.controller_calibrated_launch
        onToggled: setup.settingsModel.set_controller_calibrated_launch_enabled(checked)
    }
    Label {
        Layout.fillWidth: true
        text: "Press a button: its controller row flashes and shows the reported control. South/East/West/North are the reported positions, not necessarily the labels printed on your pad. Test mode prevents presses from activating settings."
        wrapMode: Text.WordWrap
        color: "#95a2b6"
    }
    Repeater {
        model: { setup.revision; return setup.settingsModel.controller_count() }
        delegate: ColumnLayout {
            id: device
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
