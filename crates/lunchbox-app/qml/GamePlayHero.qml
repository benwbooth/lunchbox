pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Rectangle {
    id: hero

    required property bool local
    required property bool loading
    required property bool canLaunch
    required property bool discoveryBusy
    required property bool launchBusy
    required property bool gameRunning
    required property string emulatorName
    required property string platform
    required property string launchStatus
    required property string preferenceScope
    required property int emulatorOptionCount
    required property int selectedEmulatorOption
    required property int firmwareMissingCount
    required property var emulatorLabelAt
    required property color ink
    required property color muted
    required property color line
    required property color accentCool

    signal playRequested()
    signal cancelLaunchRequested()
    signal setupRequested()
    signal firmwareSetupRequested()
    signal manageEmulatorsRequested()
    signal emulatorSelected(int index)
    signal saveGameDefaultRequested()
    signal savePlatformDefaultRequested()
    signal clearDefaultRequested()

    readonly property bool emulatorMissing: !discoveryBusy && emulatorOptionCount === 0
    readonly property bool firmwareSetupNeeded: !discoveryBusy
                                                && firmwareMissingCount > 0

    visible: local && !loading
    implicitHeight: contents.implicitHeight + 28
    height: visible ? implicitHeight : 0
    radius: 12
    color: "#112d24"
    border.width: 2
    border.color: canLaunch ? "#43c981" : "#347259"

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 10

        Row {
            width: parent.width
            spacing: 9
            Rectangle {
                width: 30
                height: 30
                radius: 15
                color: hero.canLaunch ? "#2cad6d" : "#255d47"
                Text {
                    anchors.centerIn: parent
                    text: "▶"
                    color: "white"
                    font.pixelSize: 13
                    font.weight: Font.Bold
                }
            }
            Column {
                width: parent.width - 39
                spacing: 2
                Text {
                    width: parent.width
                    text: hero.gameRunning ? "NOW PLAYING"
                          : hero.canLaunch ? "READY TO PLAY"
                          : hero.emulatorMissing ? "EMULATOR NEEDED" : "SET UP PLAY"
                    color: hero.canLaunch ? "#83e3ad" : hero.accentCool
                    font.pixelSize: 12
                    font.weight: Font.Bold
                    font.letterSpacing: 0.9
                }
                Text {
                    width: parent.width
                    text: hero.discoveryBusy ? "Detecting installed emulators…"
                          : hero.emulatorName.length > 0 ? hero.emulatorName
                          : "Choose or install a compatible emulator"
                    color: hero.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }
        }

        Text {
            width: parent.width
            visible: hero.launchStatus.length > 0
            text: hero.launchStatus
            color: hero.muted
            font.pixelSize: 9
            lineHeight: 1.2
            wrapMode: Text.WordWrap
            maximumLineCount: 3
            elide: Text.ElideRight
        }

        Column {
            width: parent.width
            visible: hero.emulatorOptionCount > 0
            spacing: 5
            Text {
                text: "PLAY WITH"
                color: "#83e3ad"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            ComboBox {
                id: emulatorPicker
                width: parent.width
                height: 40
                model: hero.emulatorOptionCount
                currentIndex: hero.selectedEmulatorOption
                displayText: currentIndex >= 0
                             ? hero.emulatorLabelAt(currentIndex)
                             : "Choose an emulator"
                onActivated: function(index) { hero.emulatorSelected(index) }
                delegate: ItemDelegate {
                    required property int index
                    width: emulatorPicker.width
                    text: hero.emulatorLabelAt(index)
                    font.pixelSize: 10
                    highlighted: emulatorPicker.highlightedIndex === index
                }
                contentItem: Text {
                    leftPadding: 11
                    rightPadding: 30
                    text: emulatorPicker.displayText
                    color: hero.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
                background: Rectangle {
                    radius: 8
                    color: "#0d211a"
                    border.width: 2
                    border.color: "#43a876"
                }
                Accessible.name: "Select emulator"
            }
            Text {
                width: parent.width
                text: hero.preferenceScope === "game"
                      ? "Game default · choose another here for a one-off launch"
                      : hero.preferenceScope === "platform"
                        ? hero.platform + " default · choose another here for a one-off launch"
                        : "This choice is for the next launch only unless you save a default"
                color: hero.muted
                font.pixelSize: 9
                wrapMode: Text.WordWrap
            }
            Row {
                width: parent.width
                spacing: 6
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "GAME DEFAULT"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.saveGameDefaultRequested()
                    background: Rectangle {
                        radius: 7
                        color: hero.preferenceScope === "game" ? "#245b45" : "#173a2d"
                        border.color: hero.preferenceScope === "game" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text { text: parent.text; color: hero.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "PLATFORM DEFAULT"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.savePlatformDefaultRequested()
                    background: Rectangle {
                        radius: 7
                        color: hero.preferenceScope === "platform" ? "#245b45" : "#173a2d"
                        border.color: hero.preferenceScope === "platform" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text { text: parent.text; color: hero.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    visible: hero.preferenceScope.length > 0
                    text: "RESET DEFAULT"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.clearDefaultRequested()
                    background: Rectangle { radius: 7; color: "#173a2d"; border.color: "#347259" }
                    contentItem: Text { text: parent.text; color: hero.muted; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
            }
        }

        Button {
            id: launchAction
            objectName: "launchAction"
            width: parent.width
            height: 48
            text: hero.launchBusy ? "CANCEL PREPARATION"
                  : hero.gameRunning ? "GAME IS RUNNING"
                  : hero.canLaunch ? "▶  PLAY"
                  : hero.firmwareSetupNeeded ? "SET UP FIRMWARE"
                  : hero.emulatorMissing ? "INSTALL AN EMULATOR" : "RECHECK PLAY SETUP"
            enabled: hero.launchBusy
                     || (!hero.gameRunning && !hero.discoveryBusy)
            font.pixelSize: 12
            font.weight: Font.Bold
            onClicked: {
                if (hero.launchBusy)
                    hero.cancelLaunchRequested()
                else if (hero.canLaunch)
                    hero.playRequested()
                else if (hero.firmwareSetupNeeded)
                    hero.firmwareSetupRequested()
                else
                    hero.setupRequested()
            }
            background: Rectangle {
                radius: 9
                color: parent.enabled
                       ? (hero.launchBusy
                          ? (parent.down ? "#8f5228" : "#ba6c32")
                          : (parent.down ? "#238153" : "#2cad6d"))
                       : "#244337"
                border.color: parent.enabled
                              ? (hero.launchBusy ? "#f0ac65" : "#75e2a5")
                              : hero.line
            }
            contentItem: Text {
                text: parent.text
                color: parent.enabled ? "white" : hero.muted
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Button {
            width: parent.width
            height: 32
            visible: !hero.gameRunning
            text: hero.platform.length > 0
                  ? "MANAGE " + hero.platform.toUpperCase() + " EMULATORS"
                  : "MANAGE EMULATORS"
            font.pixelSize: 9
            font.weight: Font.Bold
            onClicked: hero.manageEmulatorsRequested()
            background: Rectangle {
                radius: 8
                color: parent.down ? "#214636" : "#173a2d"
                border.color: "#347259"
            }
            contentItem: Text {
                text: parent.text
                color: hero.accentCool
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
