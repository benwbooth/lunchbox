import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

LbDialog {
    id: explorer
    required property var settingsModel
    property var gamepad: null
    readonly property bool calibrationActive: workflow.calibrationActive
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1100, parent ? parent.width - 40 : 1100)
    // The review step includes two diagrams and mapping controls; use the
    // available screen height so its actions are visible without a long scroll.
    height: Math.min(workflow.stage === 2 ? 1400 : 900,
                     parent ? parent.height - 40 : 900)
    padding: 24
    modal: true
    closePolicy: Popup.NoAutoClose
    title: "Controller setup"
    function reviewCalibrationFont() {
        if (settingsModel.controller_count() > 0)
            workflow.openCalibrationFor(0, "")
    }
    function openForGame(name, platform, emulator, gameUid) {
        workflow.startForGame(name, platform, emulator, gameUid)
        open()
    }
    contentItem: MomentumScrollView {
        id: scroll
        objectName: "controllerSetupScroll"
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        // A transient scrollbar changes availableWidth, which changes the
        // diagram height and flips the scrollbar again at the threshold.
        ScrollBar.vertical.policy: ScrollBar.AlwaysOn
        GuidedControllerSetup {
            id: workflow
            objectName: "controllerSetupWorkflow"
            width: scroll.availableWidth
            settingsModel: explorer.settingsModel
            gamepad: explorer.gamepad
        }
    }
    footer: DialogButtonBox {
        background: Rectangle { color: "transparent" }
        LbButton {
            text: workflow.dirty ? "Discard changes and close" : "Close"
            onClicked: { workflow.loadMapping(); explorer.close() }
        }
    }
}
