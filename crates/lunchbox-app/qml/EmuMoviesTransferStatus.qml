import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: status

    required property var providerModel
    required property color ink
    required property color muted
    required property color accent
    required property color line

    readonly property bool transferActive: providerModel.busy
                                           && providerModel.transfer_progress >= 0

    implicitHeight: content.implicitHeight + 20
    radius: 8
    color: "#111923"
    border.color: transferActive ? accent : line

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: 10
        spacing: 6

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                Layout.fillWidth: true
                text: status.transferActive
                      ? "EMUMOVIES · "
                        + (providerModel.last_media_kind.length > 0
                           ? providerModel.last_media_kind.toUpperCase()
                           : "MEDIA")
                      : "EMUMOVIES FTP"
                color: status.transferActive ? status.accent : status.ink
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.9
                elide: Text.ElideRight
            }

            Text {
                visible: status.transferActive
                text: providerModel.cancel_requested
                      ? "CANCELLING"
                      : providerModel.transfer_progress + "%"
                color: providerModel.cancel_requested
                       ? status.muted : status.accent
                font.pixelSize: 9
                font.weight: Font.Bold
            }
        }

        Text {
            Layout.fillWidth: true
            text: providerModel.message
            color: status.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            visible: status.transferActive
            spacing: 8

            InlineProgressBar {
                Layout.fillWidth: true
                from: 0
                to: 100
                indeterminate: providerModel.transfer_progress <= 0
                               && !providerModel.cancel_requested
                value: Math.max(0, providerModel.transfer_progress)
                fillColor: status.accent
                trackColor: status.line
            }

            Button {
                Layout.preferredWidth: 88
                text: providerModel.cancel_requested ? "Cancelling…" : "Cancel"
                enabled: !providerModel.cancel_requested
                onClicked: providerModel.cancel()
            }
        }
    }
}
