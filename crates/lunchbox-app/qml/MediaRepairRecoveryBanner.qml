import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: banner

    required property var auditModel
    required property color ink
    required property color muted
    required property color accent

    Layout.fillWidth: true
    Layout.preferredHeight: 88
    visible: auditModel.recovery_available
    radius: 11
    color: "#1b2532"
    border.color: accent

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 16
        anchors.rightMargin: 12
        spacing: 14

        Rectangle {
            Layout.preferredWidth: 36
            Layout.preferredHeight: 36
            radius: 18
            color: Qt.rgba(banner.accent.r, banner.accent.g,
                           banner.accent.b, 0.14)
            Text {
                anchors.centerIn: parent
                text: "↻"
                color: banner.accent
                font.pixelSize: 20
                font.weight: Font.Bold
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3
            Text {
                text: "INTERRUPTED ARTWORK REPAIR"
                color: banner.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                font.letterSpacing: 0.5
            }
            Text {
                Layout.fillWidth: true
                text: banner.auditModel.recovery_summary
                color: banner.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }
        }

        HeaderButton {
            text: "Discard"
            enabled: banner.auditModel.initialized && !banner.auditModel.busy
            Accessible.name: "Discard interrupted artwork repair"
            onClicked: banner.auditModel.discard_recovery()
        }
        HeaderButton {
            text: "Resume " + banner.auditModel.recovery_count
            active: true
            enabled: banner.auditModel.initialized && !banner.auditModel.busy
            Accessible.name: "Resume interrupted artwork repair"
            onClicked: banner.auditModel.resume_repair()
        }
    }
}
