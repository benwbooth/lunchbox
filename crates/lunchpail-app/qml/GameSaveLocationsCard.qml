pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Rectangle {
    id: card

    property var target: ({})
    property var backup: ({})
    required property color ink
    required property color muted
    required property color panel
    required property color line

    signal openFolderRequested(url folder)

    implicitHeight: contents.implicitHeight + 28
    radius: 11
    color: panel
    border.color: line

    component LocationRow: Column {
        id: location
        required property string label
        required property var entry
        property string note: ""
        spacing: 6

        Row {
            width: parent.width
            spacing: 8

            Text {
                width: parent.width - openFolder.width - parent.spacing
                anchors.verticalCenter: parent.verticalCenter
                text: location.label
                color: card.ink
                font.pixelSize: 13
                font.weight: Font.DemiBold
                wrapMode: Text.Wrap
            }

            LbButton {
                id: openFolder
                objectName: location.objectName + "OpenFolder"
                visible: !!location.entry.url
                width: visible ? implicitWidth : 0
                text: "Open folder"
                enabled: !!location.entry.exists
                onClicked: card.openFolderRequested(location.entry.url)
            }
        }

        TextEdit {
            objectName: location.objectName + "Path"
            width: parent.width
            text: location.entry.path || ""
            color: card.ink
            font.pixelSize: 13
            textFormat: TextEdit.PlainText
            wrapMode: TextEdit.Wrap
            readOnly: true
            selectByMouse: true
            activeFocusOnTab: true
            Accessible.name: location.label
            selectedTextColor: "#ffffff"
            selectionColor: "#355b73"
        }

        Text {
            width: parent.width
            visible: text.length > 0
            text: location.note
            color: card.muted
            font.pixelSize: 12
            wrapMode: Text.Wrap
        }
    }

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 12

        Text {
            text: "Save locations"
            color: card.ink
            font.pixelSize: 14
            font.weight: Font.DemiBold
        }

        Text {
            width: parent.width
            text: "Folders for the selected emulator. Other games may share these folders. Select a path to copy it."
            color: card.muted
            font.pixelSize: 12
            wrapMode: Text.Wrap
        }

        Repeater {
            model: card.target.state_locations || []

            LocationRow {
                required property var modelData
                required property int index
                objectName: "saveStateLocation" + index
                width: contents.width
                label: index === 0 ? "Save states" : "Save states (" + (index + 1) + ")"
                entry: modelData
                note: modelData.exists ? "" : "This folder has not been created yet."
            }
        }

        Text {
            objectName: "saveLocationUnavailable"
            width: parent.width
            visible: !(card.target.state_locations || []).length
            text: "Save-state location is not available for the selected emulator."
            color: card.muted
            font.pixelSize: 12
            wrapMode: Text.Wrap
        }

        Rectangle {
            width: parent.width
            height: 1
            color: card.line
        }

        LocationRow {
            objectName: "saveBackupLocation"
            width: parent.width
            visible: !!card.backup.path
            label: "Save backup" + (card.backup.local ? "" : " · " + (card.backup.provider || ""))
            entry: card.backup
            note: (card.backup.automatic ? "Automatic backup enabled." : "Automatic backup is off.")
                  + (card.backup.local
                     ? (card.backup.exists ? " Contains save states and saved RAM." : " This folder is created by the first backup.")
                     : " Stored with your cloud provider, not on this computer.")
        }

        Text {
            objectName: "saveBackupUnavailable"
            width: parent.width
            visible: !card.backup.path
            text: card.backup.loading ? "Loading backup location…"
                  : card.backup.unavailable ? "Backup settings could not be read. Check Save & state synchronization in Settings."
                  : !card.backup.configured ? "Save backup is not configured. Set it up in Settings → Save & state synchronization."
                  : "Backup location is not available for the selected emulator."
            color: card.muted
            font.pixelSize: 12
            wrapMode: Text.Wrap
        }
    }
}
