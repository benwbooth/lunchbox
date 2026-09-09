import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: explorer
    required property var settingsModel
    property var gamepad: null
    readonly property bool calibrationActive: workflow.calibrationActive
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1100, parent ? parent.width - 40 : 1100)
    height: Math.min(900, parent ? parent.height - 40 : 900)
    padding: 24
    modal: true
    closePolicy: Popup.NoAutoClose
    title: "Controller setup"
    function reviewCalibrationFont() {
        if (settingsModel.controller_count() > 0)
            workflow.openCalibrationFor(0, "")
    }
    function openForGame(name, platform, emulator) {
        workflow.startForGame(name, platform, emulator)
        open()
    }
    contentItem: ScrollView {
        id: scroll
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        GuidedControllerSetup {
            id: workflow
            width: scroll.availableWidth
            settingsModel: explorer.settingsModel
            gamepad: explorer.gamepad
        }
    }
    footer: DialogButtonBox {
        Button {
            text: workflow.dirty ? "Discard changes and close" : "Close"
            onClicked: { workflow.loadMapping(); explorer.close() }
        }
    }
}
