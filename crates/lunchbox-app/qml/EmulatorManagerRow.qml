import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: row

    required property int index
    required property var emulatorModel
    required property color ink
    required property color muted
    required property color line
    required property color accent
    required property color accentCool

    readonly property int modelRevision: emulatorModel ? emulatorModel.revision : 0
    readonly property bool managerBusy: emulatorModel ? emulatorModel.busy : false
    readonly property int managerBusyIndex: emulatorModel ? emulatorModel.busy_index : -1
    readonly property string emulatorName: {
        modelRevision
        return emulatorModel ? emulatorModel.name_at(index) : ""
    }
    readonly property string emulatorStatus: {
        modelRevision
        return emulatorModel ? emulatorModel.status_at(index) : ""
    }
    readonly property string emulatorSource: {
        modelRevision
        return emulatorModel ? emulatorModel.source_at(index) : ""
    }
    readonly property string emulatorDetail: {
        modelRevision
        return emulatorModel ? emulatorModel.detail_at(index) : ""
    }
    readonly property bool recommended: {
        modelRevision
        return emulatorModel && emulatorModel.recommended_at(index)
    }
    readonly property bool installAvailable: {
        modelRevision
        return emulatorModel && emulatorModel.can_install_at(index)
    }
    readonly property bool uninstallAvailable: {
        modelRevision
        return emulatorModel && emulatorModel.can_uninstall_at(index)
    }
    readonly property bool installActionVisible: !managerBusy && installAvailable
    readonly property bool uninstallActionVisible: !managerBusy && uninstallAvailable

    signal installRequested(int rowIndex)
    signal uninstallRequested(int rowIndex, string emulatorName, string confirmation)

    width: ListView.view ? ListView.view.width : 720
    height: 78
    radius: 10
    color: emulatorHover.hovered ? "#1b2330" : "#151c27"
    border.color: emulatorStatus === "MANAGED" ? "#28584f" : line

    HoverHandler { id: emulatorHover }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 17
        anchors.rightMargin: 14
        spacing: 14

        Rectangle {
            Layout.preferredWidth: 42
            Layout.preferredHeight: 42
            radius: 11
            color: row.emulatorStatus === "MANAGED"
                   || row.emulatorStatus === "INSTALLED"
                   ? "#1b3430" : "#292a30"
            Text {
                anchors.centerIn: parent
                text: row.emulatorName.length > 0
                      ? row.emulatorName.charAt(0).toUpperCase() : "?"
                color: row.emulatorStatus === "MANAGED"
                       || row.emulatorStatus === "INSTALLED"
                       ? row.accentCool : row.accent
                font.pixelSize: 18
                font.weight: Font.Black
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3
            RowLayout {
                Layout.fillWidth: true
                spacing: 9
                Text {
                    text: row.emulatorName
                    color: row.ink
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                }
                Rectangle {
                    visible: row.recommended
                    Layout.preferredWidth: recommendedText.implicitWidth + 14
                    Layout.preferredHeight: 20
                    radius: 6
                    color: "#173a2d"
                    border.color: "#347259"
                    Text {
                        id: recommendedText
                        anchors.centerIn: parent
                        text: "RECOMMENDED"
                        color: row.accentCool
                        font.pixelSize: 8
                        font.weight: Font.Bold
                    }
                }
                Rectangle {
                    Layout.preferredWidth: sourceText.implicitWidth + 14
                    Layout.preferredHeight: 20
                    radius: 6
                    color: "#202938"
                    border.color: row.line
                    Text {
                        id: sourceText
                        anchors.centerIn: parent
                        text: row.emulatorSource
                        color: "#b7c2d2"
                        font.pixelSize: 9
                        font.weight: Font.Bold
                    }
                }
                Item { Layout.fillWidth: true }
            }
            Text {
                Layout.fillWidth: true
                text: row.emulatorDetail
                color: row.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
        }

        Text {
            Layout.preferredWidth: 142
            text: row.emulatorStatus
            color: row.emulatorStatus === "MANAGED"
                   ? row.accentCool
                   : row.emulatorStatus === "INSTALLED"
                     ? "#b7c2d2" : row.accent
            horizontalAlignment: Text.AlignRight
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.6
        }

        BusyIndicator {
            visible: row.managerBusy && row.managerBusyIndex === row.index
            running: visible
            Layout.preferredWidth: 26
            Layout.preferredHeight: 26
        }

        HeaderButton {
            objectName: "emulatorInstallButton"
            visible: row.installActionVisible
            text: "Install"
            active: true
            onClicked: row.installRequested(row.index)
        }

        Button {
            objectName: "emulatorUninstallButton"
            visible: row.uninstallActionVisible
            text: "Uninstall"
            onClicked: row.uninstallRequested(
                           row.index,
                           row.emulatorName,
                           row.emulatorModel.uninstall_confirmation_at(row.index))
        }
    }
}
