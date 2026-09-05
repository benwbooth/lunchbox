import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: feedback
    required property var gamepad
    required property string controllerName
    property bool matchesInput: false
    property string lastControl: ""
    readonly property bool flashing: flash.running
    padding: 10
    background: Rectangle {
        radius: 8
        color: feedback.flashing ? "#243c3e" : "#17212b"
        border.color: feedback.flashing ? "#62d6c6" : "#354252"
        border.width: feedback.flashing ? 2 : 1
    }
    function receiveInput() {
        if (!visible || !matchesInput) return
        lastControl = gamepad.last_control
        flash.restart()
    }
    Connections {
        target: feedback.gamepad
        function onInput_revisionChanged() { feedback.receiveInput() }
    }
    Timer { id: flash; interval: 1200 }
    contentItem: ColumnLayout {
        spacing: 6
        Label {
            Layout.fillWidth: true
            text: feedback.controllerName
            color: "#eef3f7"
            wrapMode: Text.Wrap
            font.bold: true
        }
        Label {
            Layout.fillWidth: true
            text: feedback.lastControl.length ? "Reported input: " + feedback.lastControl
                                               : "Press a button on this controller to identify it."
            color: feedback.flashing ? "#62d6c6" : "#a9b7c7"
            wrapMode: Text.Wrap
            Accessible.role: Accessible.StaticText
        }
        Flow {
            Layout.fillWidth: true
            spacing: 5
            Repeater {
                model: ["South", "East", "West", "North", "DPadUp", "DPadDown",
                        "DPadLeft", "DPadRight", "LeftBumper", "RightBumper", "Start", "Select"]
                delegate: Rectangle {
                    required property string modelData
                    readonly property bool lit: feedback.flashing && feedback.lastControl === modelData
                    width: chip.implicitWidth + 14
                    height: 25
                    radius: 5
                    color: lit ? "#62d6c6" : "#253140"
                    Label {
                        id: chip
                        anchors.centerIn: parent
                        text: parent.modelData
                        color: parent.lit ? "#102426" : "#b9c5d3"
                        font.pixelSize: 11
                    }
                }
            }
        }
    }
}
