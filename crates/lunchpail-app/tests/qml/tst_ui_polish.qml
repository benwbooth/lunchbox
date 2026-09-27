import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import QtTest
import "../../qml" as Lunchpail

TestCase {
    id: testCase
    name: "UiPolish"
    when: windowShown
    width: 720
    height: 600
    visible: true

    Rectangle { anchors.fill: parent; color: "#131923" }
    ColumnLayout {
        id: gallery
        x: 24; y: 24; width: 470; spacing: 14
        Text { text: "Lunchpail controls"; color: "#f4f7fb"; font.pixelSize: 22 }
        RowLayout {
            Lunchpail.LbButton { text: "Play"; highlighted: true; positive: true }
            Lunchpail.LbButton { text: "Controller setup" }
            Lunchpail.LbButton { text: "Unavailable"; enabled: false }
        }
        Lunchpail.LbComboBox { id: combo; Layout.fillWidth: true; model: ["System default", "RetroTube TV", "No shader"] }
        Lunchpail.LbTextField { Layout.fillWidth: true; placeholderText: "Search games" }
        Lunchpail.LbCheckBox { id: checkbox; text: "Include virtual controllers" }
        Lunchpail.LbSwitch { id: toggle; text: "Resume the last save state" }
        RowLayout {
            Controls.ButtonGroup { id: group }
            Lunchpail.LbRadioButton { id: first; text: "This game"; checked: true; Controls.ButtonGroup.group: group }
            Lunchpail.LbRadioButton { id: second; text: "This system"; Controls.ButtonGroup.group: group }
        }
        Lunchpail.LbSlider { id: slider; Layout.fillWidth: true; value: 0.5 }
        Lunchpail.LbCheckBox { id: wrapped; Layout.preferredWidth: 200; text: "Keep all controller options available when reviewing this mapping" }
    }
    Lunchpail.LbComboBox {
        id: edgeCombo
        x: 490; y: 550; width: 210
        model: Array.from({length: 25}, (_, i) => "Connected controller " + (i + 1))
    }
    Lunchpail.LbDialog {
        id: dialog
        title: "Controller setup"
        width: 400
        standardButtons: Controls.Dialog.Close
        contentItem: Text { text: "Your existing mappings are preserved."; color: "#f4f7fb" }
    }
    SignalSpy { id: rejected; target: dialog; signalName: "rejected" }

    Component {
        id: fieldsComponent
        RowLayout {
            width: 600
            Lunchpail.LbTextField { objectName: "username"; Layout.fillWidth: true; placeholderText: "Username" }
            Lunchpail.SecretField { Layout.fillWidth: true; placeholderText: "Saved password (leave blank to keep)" }
        }
    }
    function test_empty_fields_share_layout_space_with_password_fields() {
        const row = createTemporaryObject(fieldsComponent, testCase)
        verify(row)
        waitForRendering(row)
        const username = findChild(row, "username")
        verify(username.width >= 240)
        verify(username.width - username.leftPadding - username.rightPadding > 200)
    }

    Component {
        id: frameComponent
        Lunchpail.LbFrame {
            contentItem: ColumnLayout {
                Text { text: "Player 1"; font.pixelSize: 18 }
                Lunchpail.LbComboBox { model: ["N30", "Steam Controller 2"]; implicitWidth: 260 }
                Lunchpail.LbButton { text: "Review / change buttons" }
            }
        }
    }
    function test_frames_size_to_all_of_their_controls() {
        const frame = createTemporaryObject(frameComponent, testCase)
        verify(frame)
        verify(frame.implicitHeight >= frame.contentItem.implicitHeight + 32)
        verify(frame.implicitWidth >= 292)
        verify(frame.contentItem.height >= frame.contentItem.implicitHeight)
    }

    function test_options_preserve_mouse_keyboard_and_group_behavior() {
        mouseClick(checkbox)
        verify(checkbox.checked)
        checkbox.forceActiveFocus()
        keyClick(Qt.Key_Space)
        verify(!checkbox.checked)
        checkbox.tristate = true
        checkbox.checkState = Qt.PartiallyChecked
        compare(checkbox.checkState, Qt.PartiallyChecked)
        mouseClick(toggle)
        verify(toggle.checked)
        toggle.forceActiveFocus()
        keyClick(Qt.Key_Space)
        verify(!toggle.checked)
        mouseClick(second)
        verify(second.checked)
        verify(!first.checked)
        mouseClick(first)
        verify(first.checked)
        verify(!second.checked)
    }
    function test_options_wrap_without_clipping() {
        verify(wrapped.implicitHeight > checkbox.implicitHeight)
        verify(wrapped.contentItem.contentWidth <= wrapped.contentItem.width)
        compare(wrapped.contentItem.textFormat, Text.PlainText)
        compare(toggle.font.pixelSize, checkbox.font.pixelSize)
        compare(first.font.pixelSize, checkbox.font.pixelSize)
    }
    function test_slider_keyboard_and_pointer_still_work() {
        slider.forceActiveFocus()
        const before = slider.value
        keyClick(Qt.Key_Right)
        verify(slider.value > before)
        mouseClick(slider, slider.width * 0.25, slider.height / 2)
        verify(slider.value < before)
        verify(slider.providesFocusIndicator)
    }
    function test_dropdown_stays_on_screen_and_can_scroll() {
        mouseClick(edgeCombo)
        tryVerify(() => edgeCombo.popup.visible)
        const menu = edgeCombo.popup
        verify(menu.x >= 0)
        verify(menu.y >= 0)
        verify(menu.y + menu.height <= height)
        verify(menu.x + menu.width <= width)
        verify(menu.contentItem.contentHeight > menu.contentItem.height)
        keyClick(Qt.Key_End)
        keyClick(Qt.Key_Return)
        tryVerify(() => !menu.visible)
        compare(edgeCombo.currentIndex, 24)
    }
    function test_dialog_is_themed_and_close_remains_a_rejection() {
        dialog.open()
        tryVerify(() => dialog.visible)
        compare(dialog.background.color, "#1a2230")
        compare(dialog.font.pixelSize, 13)
        rejected.clear()
        dialog.reject()
        compare(rejected.count, 1)
    }
    function test_visual_fixture() {
        checkbox.checkState = Qt.Checked
        toggle.checked = true
        slider.value = 0.5
        mouseMove(testCase, 710, 10)
        wait(120) // Let the switch thumb finish its brief state transition.
        waitForRendering(gallery)
        grabImage(testCase).save("/tmp/lunchpail-ui-polish-controls.png")
    }
}
