import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: form
    signal deviceAdded(var device)
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(720, parent ? parent.width - 32 : 720)
    height: Math.min(760, parent ? parent.height - 32 : 760)
    modal: true
    title: "Add relative device to draft"
    standardButtons: Dialog.Cancel
    contentItem: ScrollView {
        id: scroll
        clip: true
        ColumnLayout {
            width: scroll.availableWidth
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Enter the exact event path and physical input identity. This form does not discover or open hardware. Select only controls the device provides." }
            Label { text: "Event path" }
            TextField { id: eventPath; Layout.fillWidth: true; placeholderText: "/dev/input/event…"; Accessible.name: "Relative device event path" }
            Label { text: "Physical input identity path" }
            TextField { id: identity; Layout.fillWidth: true; placeholderText: "Exact absolute physical input identity"; Accessible.name: "Relative device physical input identity" }
            RowLayout {
                CheckBox { id: axisX; text: "X"; checked: true }
                CheckBox { id: axisY; text: "Y"; checked: true }
                CheckBox { id: wheelH; text: "Horizontal wheel" }
                CheckBox { id: wheelV; text: "Vertical wheel" }
            }
            Label { text: "Output motion sensitivity (%)" }
            CheckBox { id: exclusive; text: "Exclusively capture the physical source during a session" }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Exclusive capture blocks other applications from the entire physical event node, including unselected controls. Keep another input device available to stop play. This does not prevent the virtual mouse from affecting the desktop. Adding or staging this draft does not capture anything."
            }
            RowLayout {
                Label { text: "X" }
                SpinBox { id: gainX; from: 1; to: 1000; value: 100; editable: true; Accessible.name: "Output X sensitivity percent" }
                Label { text: "Y" }
                SpinBox { id: gainY; from: 1; to: 1000; value: 100; editable: true; Accessible.name: "Output Y sensitivity percent" }
            }
            RowLayout {
                CheckBox { id: swap; text: "Swap X/Y first" }
                CheckBox { id: invertX; text: "Invert output X" }
                CheckBox { id: invertY; text: "Invert output Y" }
            }
            Label { text: "Physical button → output button" }
            Repeater {
                id: buttons
                model: ["Left", "Right", "Middle", "Side", "Extra", "Forward", "Back", "Task"]
                delegate: RowLayout {
                    required property int index
                    required property string modelData
                    property int outputCode: output.currentIndex === 0 ? -1 : 271 + output.currentIndex
                    Label { Layout.preferredWidth: 110; text: modelData + " (" + (272 + index) + ")" }
                    ComboBox {
                        id: output
                        Layout.fillWidth: true
                        model: ["Not mapped", "Left", "Right", "Middle", "Side", "Extra", "Forward", "Back", "Task"]
                        Accessible.name: "Output for " + parent.modelData + " button"
                    }
                }
            }
            Label { id: error; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Button {
                text: "Add to draft"
                onClicked: {
                    if (!eventPath.text.startsWith("/") || !identity.text.startsWith("/")) {
                        error.text = "Both paths must be absolute."
                        return
                    }
                    const axes = []
                    if (axisX.checked) axes.push(0)
                    if (axisY.checked) axes.push(1)
                    if (wheelH.checked) axes.push(6)
                    if (wheelV.checked) axes.push(8)
                    const pairs = []
                    const outputs = []
                    for (let i = 0; i < buttons.count; ++i) {
                        const code = buttons.itemAt(i).outputCode
                        if (code < 0) continue
                        if (outputs.indexOf(code) >= 0) { error.text = "Each output button can be assigned only once."; return }
                        outputs.push(code)
                        pairs.push([272 + i, code])
                    }
                    if (!axes.length && !pairs.length) { error.text = "Select at least one axis or button."; return }
                    form.deviceAdded({event_path: eventPath.text, input_identity: identity.text,
                        axes: axes, buttons: pairs, exclusive_source: exclusive.checked, motion: {x_percent: gainX.value, y_percent: gainY.value,
                            invert_x: invertX.checked, invert_y: invertY.checked, swap_xy: swap.checked}})
                    form.close()
                }
            }
        }
    }
}
