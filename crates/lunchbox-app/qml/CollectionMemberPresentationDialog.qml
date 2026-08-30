import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: control

    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color panel: "#131923"
    property color panelRaised: "#1a2230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d6c6"
    property string collectionName: ""
    property string libraryTitle: ""
    property string platform: ""
    property string gameUid: ""
    property bool busy: false
    property string statusMessage: ""
    readonly property alias playlistTitle: titleField.text
    readonly property alias entryNotes: notesField.text
    readonly property bool valid: notesField.text.length <= 4000

    signal saveRequested(string playlistTitle, string entryNotes)

    function prepare(collectionName, libraryTitle, platform, gameUid,
                     playlistTitle, entryNotes) {
        control.collectionName = collectionName
        control.libraryTitle = libraryTitle
        control.platform = platform
        control.gameUid = gameUid
        titleField.text = playlistTitle
        notesField.text = entryNotes
    }

    modal: true
    width: Math.min(660, parent ? parent.width - 48 : 660)
    height: Math.min(610, parent ? parent.height - 48 : 610)
    padding: 22
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        color: control.panel
        radius: 14
        border.color: control.line
    }

    header: Rectangle {
        width: parent.width
        height: 68
        color: control.panelRaised
        radius: 14
        Column {
            anchors.left: parent.left
            anchors.leftMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                text: "COLLECTION PRESENTATION"
                color: control.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            Text {
                text: control.collectionName
                color: control.accent
                font.pixelSize: 10
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
        }
        RoundButton {
            anchors.right: parent.right
            anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            text: "×"
            flat: true
            font.pixelSize: 20
            enabled: !control.busy
            onClicked: control.close()
        }
    }

    contentItem: ColumnLayout {
        spacing: 13

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 74
            radius: 10
            color: "#101720"
            border.color: control.line
            Column {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                Text {
                    width: parent.width
                    text: control.libraryTitle
                    color: control.ink
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: control.platform + "  ·  exact game identity stays unchanged"
                    color: control.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }

        Text {
            text: "PLAYLIST TITLE"
            color: control.accent
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.2
        }
        TextField {
            id: titleField
            Layout.fillWidth: true
            maximumLength: 500
            placeholderText: control.libraryTitle
            selectByMouse: true
            Accessible.name: "Collection-specific title"
        }
        Text {
            Layout.fillWidth: true
            text: "Optional. This title appears only while browsing ‘"
                  + control.collectionName + "’."
            color: control.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            Text {
                text: "PLAYLIST NOTES"
                color: control.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            Item { Layout.fillWidth: true }
            Text {
                text: notesField.text.length + " / 4000"
                color: control.valid ? control.muted : "#ff7c75"
                font.pixelSize: 9
            }
        }
        NativeTextArea {
            id: notesField
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 135
            placeholderText: "Optional notes for this shelf: play order, controller setup, shared save, or event instructions"
            wrapMode: TextEdit.Wrap
            selectByMouse: true
            color: control.ink
            backgroundColor: "#101721"
            line: control.valid ? control.line : "#ff7c75"
            accent: control.accent
        }

        Text {
            Layout.fillWidth: true
            text: control.busy ? control.statusMessage
                  : "Matching, metadata, downloads, and launch behavior continue to use the exact library record."
            color: control.busy ? control.accentCool : control.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }
    }

    footer: Rectangle {
        width: parent.width
        height: 66
        color: control.panelRaised
        radius: 14
        Row {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            Button {
                text: "Restore library presentation"
                visible: titleField.text.trim().length > 0
                         || notesField.text.trim().length > 0
                enabled: !control.busy
                onClicked: control.saveRequested("", "")
            }
        }
        Row {
            anchors.right: parent.right
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            Button {
                text: "Cancel"
                enabled: !control.busy
                onClicked: control.close()
            }
            HeaderButton {
                text: "Save"
                active: true
                enabled: control.valid && !control.busy
                onClicked: control.saveRequested(titleField.text, notesField.text)
            }
        }
    }
}
