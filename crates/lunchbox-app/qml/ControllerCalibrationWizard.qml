import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: wizard
    required property var settingsModel
    required property var gamepad
    readonly property var catalog: JSON.parse(settingsModel.controller_catalog_json())
    property string deviceId: ""
    property string deviceName: ""
    property int layoutIndex: 0
    property int step: 0
    property var bindings: ({})
    property string status: ""
    property bool waitingForRelease: false
    property bool showPreview: false
    property int profileIndex: 0
    readonly property var layout: catalog.layouts[layoutIndex]
    readonly property var currentControl: layout && step < layout.controls.length ? layout.controls[step] : null
    readonly property var preview: showPreview ? JSON.parse(settingsModel.controller_mapping_preview(
        layout.id, JSON.stringify(bindings), catalog.emulator_profiles[profileIndex].id)) : ({ rows: [], warnings: [] })
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(900, parent ? parent.width - 32 : 900)
    height: Math.min(850, parent ? parent.height - 32 : 850)
    modal: true
    closePolicy: Popup.CloseOnEscape
    title: "Calibrate " + deviceName

    function openFor(id, name) {
        deviceId = id; deviceName = name
        const saved = JSON.parse(settingsModel.controller_calibration_json(id))
        layoutIndex = Math.max(0, catalog.layouts.findIndex(item => item.id === saved.layout))
        bindings = saved.os === catalog.host_os ? (saved.bindings || {}) : ({})
        step = 0; waitingForRelease = false; showPreview = false
        status = "Choose the closest physical layout, then press the highlighted control."
        open()
    }
    function resetLayout(index) {
        layoutIndex = index; bindings = ({}); step = 0; waitingForRelease = false
        status = "Press the highlighted control. Release it before the next prompt."
    }
    function receiveInput() {
        if (!visible || !currentControl || waitingForRelease || showPreview) return
        if (settingsModel.controller_key_for_input(gamepad.last_device_key) !== deviceId) {
            status = "That input came from another controller. Use " + deviceName + "."
            return
        }
        let input
        try { input = JSON.parse(gamepad.last_binding) } catch (error) { return }
        if (currentControl.analog && input.kind !== "axis") {
            status = "Move the analog stick in the highlighted direction."
            return
        }
        const duplicate = Object.keys(bindings).find(id => id !== currentControl.id
            && bindings[id].code === input.code && bindings[id].kind === input.kind
            && bindings[id].direction === input.direction)
        if (duplicate) {
            const label = layout.controls.find(control => control.id === duplicate).label
            status = "This input is already assigned to " + label + ". Go back to correct it, or skip a duplicate hardware button."
            return
        }
        let next = Object.assign({}, bindings)
        next[currentControl.id] = input
        bindings = next
        waitingForRelease = true
        status = "Recorded " + currentControl.label + " → " + input.logical + " (code " + input.code + "). Release all controls to continue."
    }
    function receiveNeutral() {
        if (!visible || !waitingForRelease) return
        if (settingsModel.controller_key_for_input(gamepad.neutral_device_key) !== deviceId) return
        waitingForRelease = false
        step++
        status = currentControl ? "Press " + currentControl.label + "." : "Calibration complete. Review the mapping or use this calibration, then save settings."
    }
    function skip() {
        if (!currentControl) return
        let next = Object.assign({}, bindings)
        delete next[currentControl.id]
        bindings = next
        step++; waitingForRelease = false
        status = "Skipped controls remain unavailable; no input is invented for them."
    }
    Connections {
        target: wizard.gamepad
        function onInput_revisionChanged() { wizard.receiveInput() }
        function onNeutral_revisionChanged() { wizard.receiveNeutral() }
    }

    contentItem: ScrollView {
        id: scroll
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scroll.availableWidth
            spacing: 12
            ComboBox {
                Layout.fillWidth: true
                model: wizard.catalog.layouts
                textRole: "name"
                currentIndex: wizard.layoutIndex
                onActivated: wizard.resetLayout(currentIndex)
                Accessible.name: "Physical controller layout"
            }
            Label {
                Layout.fillWidth: true
                visible: wizard.layout.notes.length > 0
                text: wizard.layout.notes
                color: "#a6b6c7"
                wrapMode: Text.WordWrap
            }
            Image {
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(330, wizard.height * 0.40)
                source: wizard.settingsModel.controller_diagram(wizard.layout.id,
                    wizard.currentControl ? wizard.currentControl.id : "")
                sourceSize.width: Math.ceil(width * Screen.devicePixelRatio)
                sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
                fillMode: Image.PreserveAspectFit
                Accessible.name: "Controller diagram: " + (wizard.currentControl ? "press " + wizard.currentControl.label : "complete")
            }
            Label {
                Layout.fillWidth: true
                text: wizard.currentControl ? "Press " + wizard.currentControl.label
                    + (wizard.currentControl.optional ? " (optional)" : "") : "Controls recorded"
                font.pixelSize: 24
                font.bold: true
                color: "#ffb454"
                wrapMode: Text.WordWrap
            }
            ProgressBar {
                Layout.fillWidth: true
                from: 0; to: wizard.layout.controls.length; value: wizard.step
            }
            Label {
                Layout.fillWidth: true
                text: wizard.status
                wrapMode: Text.WordWrap
                color: "#bfcddb"
            }
            RowLayout {
                Button {
                    text: "Back"
                    enabled: wizard.step > 0
                    onClicked: { wizard.step--; wizard.waitingForRelease = false; wizard.status = "Press the highlighted control to replace its binding." }
                }
                Button { text: "Skip / not present"; enabled: !!wizard.currentControl; onClicked: wizard.skip() }
                Button { text: "Start over"; onClicked: wizard.resetLayout(wizard.layoutIndex) }
            }
            CheckBox {
                text: "Preview system / emulator mapping"
                checked: wizard.showPreview
                onToggled: wizard.showPreview = checked
            }
            ColumnLayout {
                visible: wizard.showPreview
                Layout.fillWidth: true
                ComboBox {
                    Layout.fillWidth: true
                    model: wizard.catalog.emulator_profiles
                    textRole: "name"
                    currentIndex: wizard.profileIndex
                    onActivated: wizard.profileIndex = currentIndex
                    Accessible.name: "Emulator input contract"
                }
                Label {
                    Layout.fillWidth: true
                    text: "Documented mapping preview — not yet applied automatically"
                    color: "#ffb454"
                    wrapMode: Text.WordWrap
                }
                Repeater {
                    model: wizard.preview.rows
                    delegate: Label {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.target + " ← " + modelData.physical + " → " + modelData.output
                            + (modelData.input ? "" : " · MISSING")
                        wrapMode: Text.WordWrap
                        color: modelData.input ? "#dbe5ed" : "#ffb454"
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: wizard.preview.warnings.join("\n")
                    color: "#a6b6c7"
                    wrapMode: Text.WordWrap
                }
            }
        }
    }
    footer: DialogButtonBox {
        Button { text: "Cancel"; DialogButtonBox.buttonRole: DialogButtonBox.RejectRole; onClicked: wizard.close() }
        Button {
            text: "Use calibration"
            enabled: Object.keys(wizard.bindings).length > 0
            DialogButtonBox.buttonRole: DialogButtonBox.ActionRole
            onClicked: {
                const error = wizard.settingsModel.save_controller_calibration(wizard.deviceId, wizard.layout.id, JSON.stringify(wizard.bindings))
                if (error.length) wizard.status = error
                else wizard.close()
            }
        }
    }
}
