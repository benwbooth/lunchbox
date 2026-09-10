import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog
    required property var settingsModel
    property var projection: null
    property bool captureActive: false
    property var captureEdges: [null, null, null, null]
    property var capturedDraft: null
    property string captureMessage: ""
    property string sampleMessage: ""
    title: "Absolute device calibration records"
    modal: true
    width: Math.min(820, parent ? parent.width : 820)
    height: Math.min(720, parent ? parent.height : 720)
    closePolicy: Popup.NoAutoClose
    onClosed: stopCapture()
    Component.onDestruction: stopCapture()

    function captureCommand(command) {
        try {
            const result = JSON.parse(settingsModel.absolute_capture_command(JSON.stringify(command)))
            captureActive = result.active === true
            if (result.edges) captureEdges = result.edges
            if (result.error) captureMessage = result.error
            if (result.draft) {
                capturedDraft = result.draft
                captureMessage = "Captured draft ready for review. Add it to the JSON list, then stage and save separately."
            }
            if (command.action === "poll") {
                sampleMessage = result.sample
                    ? "Raw X=" + result.sample.position.raw_x + ", Y=" + result.sample.position.raw_y + "; sample age " + result.sample.age_ms + " ms"
                    : "No new complete position in this poll. Move both axes before recording an edge."
            }
            if (!captureActive) sampleMessage = "Capture stopped."
            return result
        } catch (error) {
            // Do not leave the native session alive if its response is unreadable.
            try { settingsModel.absolute_capture_command('{"action":"cancel"}') } catch (_) {}
            captureActive = false
            captureMessage = error.message
            return {error: error.message}
        }
    }
    function stopCapture() {
        if (captureActive) captureCommand({action: "cancel"})
    }
    Timer {
        interval: 50
        repeat: true
        running: dialog.visible && dialog.captureActive && Qt.application.state === Qt.ApplicationActive
        onTriggered: dialog.captureCommand({action: "poll"})
    }
    Connections {
        target: Qt.application
        function onStateChanged() {
            if (Qt.application.state !== Qt.ApplicationActive) dialog.stopCapture()
        }
    }

    function loadAndOpen() {
        editor.text = settingsModel.absolute_device_settings_json()
        status.text = ""
        projection = null
        open()
    }
    function preview() {
        projection = null
        try {
            const records = JSON.parse(editor.text)
            if (!Array.isArray(records) || recordIndex.value >= records.length)
                throw new Error("Select an existing zero-based device record.")
            const x = Number(rawX.text), y = Number(rawY.text)
            if (!rawX.acceptableInput || !rawY.acceptableInput || !Number.isInteger(x) || !Number.isInteger(y))
                throw new Error("Enter signed 32-bit integer readings for both physical axes.")
            const result = JSON.parse(settingsModel.absolute_calibration_preview_json(
                JSON.stringify(records[recordIndex.value]), x, y))
            if (result.error) throw new Error(result.error)
            projection = result.projection
            status.text = "Calibrated X=" + projection.x + ", Y=" + projection.y
                + (projection.outside_calibrated_area ? "; outside calibrated area (clamped)." : "; inside calibrated area.")
                + "\nLibretro aim X=" + result.libretro_projection.x + ", Y=" + result.libretro_projection.y
                + ". Normal aim only: no offscreen/reload signal is inferred. Emulator routing is not enabled."
        } catch (error) { status.text = error.message }
    }
    contentItem: ScrollView {
        id: scroll
        clip: true
        ColumnLayout {
            width: scroll.availableWidth
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Advanced absolute calibration. Capture is explicit and Linux64-only; emulator routing is not enabled. The coordinate preview below uses manual samples. Runtime behavior remains unverified."
            }
            GroupBox {
                title: "Acquire rectangular edges from a selected device"
                Layout.fillWidth: true
                ColumnLayout {
                    anchors.fill: parent
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: "Enter the exact event node and canonical sysfs device identity, and explicitly choose physical ABS axis codes. Point at each intended calibrated edge and confirm it. This does not identify screen bounds, correct perspective or infer offscreen shots. Capture is non-exclusive and stops on close, focus loss or timeout."
                    }
                    TextField { id: capturePath; Layout.fillWidth: true; enabled: !dialog.captureActive; placeholderText: "/dev/input/eventN"; Accessible.name: "Exact absolute event node" }
                    TextField { id: captureIdentity; Layout.fillWidth: true; enabled: !dialog.captureActive; placeholderText: "Canonical /sys/devices/… input identity"; Accessible.name: "Absolute device sysfs identity" }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8
                        Label { text: "Physical X code" }
                        SpinBox { id: captureX; from: 0; to: 40; enabled: !dialog.captureActive; Accessible.name: "Physical X ABS code" }
                        Label { text: "Physical Y code" }
                        SpinBox { id: captureY; from: 0; to: 40; value: 1; enabled: !dialog.captureActive; Accessible.name: "Physical Y ABS code" }
                        CheckBox { id: captureSwap; text: "Swap X/Y"; enabled: !dialog.captureActive }
                    }
                    Flow {
                        Layout.fillWidth: true
                        spacing: 8
                        Button {
                            text: "Start device capture"
                            enabled: !dialog.captureActive && capturePath.text.length > 0 && captureIdentity.text.length > 0 && captureX.value !== captureY.value
                            onClicked: {
                                dialog.captureEdges = [null, null, null, null]
                                dialog.captureMessage = ""
                                dialog.captureCommand({action: "start", event_path: capturePath.text, input_identity: captureIdentity.text, x_code: captureX.value, y_code: captureY.value, swap_xy: captureSwap.checked})
                            }
                        }
                        Button { text: "Cancel capture"; enabled: dialog.captureActive; onClicked: dialog.stopCapture() }
                        Repeater {
                            model: ["left", "right", "top", "bottom"]
                            Button {
                                required property string modelData
                                required property int index
                                text: "Record " + modelData + (dialog.captureEdges[index] !== null ? " (" + dialog.captureEdges[index] + ")" : "")
                                enabled: dialog.captureActive
                                onClicked: {
                                    dialog.captureMessage = ""
                                    dialog.captureCommand({action: "record", edge: modelData})
                                }
                            }
                        }
                        Button {
                            text: "Finish to draft"
                            enabled: dialog.captureActive && dialog.captureEdges.every(value => value !== null)
                            onClicked: dialog.captureCommand({action: "finish"})
                        }
                    }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: dialog.sampleMessage }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: dialog.captureMessage }
                    TextArea {
                        Layout.fillWidth: true
                        visible: dialog.capturedDraft !== null
                        readOnly: true
                        selectByMouse: true
                        wrapMode: TextEdit.Wrap
                        text: dialog.capturedDraft ? JSON.stringify(dialog.capturedDraft, null, 2) : ""
                        Accessible.name: "Captured calibration draft"
                    }
                    Button {
                        text: "Add captured draft to JSON list"
                        enabled: !dialog.captureActive && dialog.capturedDraft !== null
                        onClicked: {
                            try {
                                const records = JSON.parse(editor.text)
                                if (!Array.isArray(records) || records.length >= 16) throw new Error("Expected a list with fewer than 16 devices.")
                                if (records.some(record => record.event_path === dialog.capturedDraft.event_path || record.input_identity === dialog.capturedDraft.input_identity))
                                    throw new Error("This device already exists in the list. Review and remove its old record explicitly before adding the replacement.")
                                records.push(dialog.capturedDraft)
                                editor.text = JSON.stringify(records, null, 2)
                                dialog.capturedDraft = null
                                dialog.captureMessage = "Added to JSON draft only. Stage and save separately."
                            } catch (error) { dialog.captureMessage = error.message }
                        }
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "JSON array entries require event_path, input_identity, x_axis and y_axis ({code, minimum, maximum}), and calibration ({left, right, top, bottom, swap_xy}). Edges are measured after swapping axes; reversed edges invert that direction. Use [] to remove all records."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 210
                TextArea {
                    id: editor
                    selectByMouse: true
                    font.family: "monospace"
                    wrapMode: TextEdit.NoWrap
                    Accessible.name: "Absolute device records JSON"
                    onTextChanged: { dialog.projection = null; status.text = "" }
                }
            }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Label { text: "Record index" }
                SpinBox {
                    id: recordIndex
                    from: 0; to: 15
                    Accessible.name: "Zero-based record index"
                    onValueChanged: { dialog.projection = null; status.text = "" }
                }
                TextField {
                    id: rawX
                    width: 155
                    placeholderText: "Physical X reading"
                    Accessible.name: placeholderText
                    validator: IntValidator { bottom: -2147483648; top: 2147483647 }
                    onTextChanged: { dialog.projection = null; status.text = "" }
                }
                TextField {
                    id: rawY
                    width: 155
                    placeholderText: "Physical Y reading"
                    Accessible.name: placeholderText
                    validator: IntValidator { bottom: -2147483648; top: 2147483647 }
                    onTextChanged: { dialog.projection = null; status.text = "" }
                }
                Button { text: "Preview sample"; onClicked: dialog.preview() }
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 160
                color: "#18232d"
                border.color: "#8198aa"
                Accessible.name: "Calibrated coordinate rectangle; X increases right, Y increases down"
                Rectangle {
                    visible: dialog.projection !== null
                    width: 12; height: 12; radius: 6
                    color: dialog.projection && dialog.projection.outside_calibrated_area ? "#ffb454" : "#77d8b0"
                    x: dialog.projection ? dialog.projection.x / 65535 * (parent.width - width) : 0
                    y: dialog.projection ? dialog.projection.y / 65535 * (parent.height - height) : 0
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Rectangle: unsigned 0–65535 on each axis, X right / Y down. Not game viewport coordinates. Outside this rectangle does not establish hardware offscreen or reload semantics."
            }
            Label { id: status; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Button {
                    text: "Stage settings"
                    onClicked: {
                        const error = dialog.settingsModel.stage_absolute_device_settings(editor.text)
                        status.text = error || "Staged only. Save on the main settings page to persist. No device was opened."
                    }
                }
                Button { text: "Close"; onClicked: dialog.close() }
            }
        }
    }
}
