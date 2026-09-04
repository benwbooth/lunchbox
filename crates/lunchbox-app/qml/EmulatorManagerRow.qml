import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: row

    required property int rowIndex
    required property var emulatorModel
    required property color ink
    required property color muted
    required property color line
    required property color accent
    required property color accentCool
    property bool defaultActionsAvailable: false
    property bool gameDefaultActionsAvailable: false
    property bool defaultTargetAvailable: false
    property bool gameDefault: false
    property bool platformDefault: false
    property string platformName: ""

    readonly property int modelRevision: emulatorModel ? emulatorModel.revision : 0
    readonly property bool managerBusy: emulatorModel ? emulatorModel.busy : false
    readonly property int managerBusyIndex: emulatorModel ? emulatorModel.busy_index : -1
    readonly property string emulatorName: {
        modelRevision
        return emulatorModel ? emulatorModel.name_at(rowIndex) : ""
    }
    readonly property string emulatorStatus: {
        modelRevision
        return emulatorModel ? emulatorModel.status_at(rowIndex) : ""
    }
    readonly property string emulatorSource: {
        modelRevision
        return emulatorModel ? emulatorModel.source_at(rowIndex) : ""
    }
    readonly property string emulatorDetail: {
        modelRevision
        return emulatorModel ? emulatorModel.detail_at(rowIndex) : ""
    }
    readonly property bool recommended: {
        modelRevision
        return emulatorModel && emulatorModel.recommended_at(rowIndex)
    }
    readonly property bool installAvailable: {
        modelRevision
        return emulatorModel && emulatorModel.can_install_at(rowIndex)
    }
    readonly property bool uninstallAvailable: {
        modelRevision
        return emulatorModel && emulatorModel.can_uninstall_at(rowIndex)
    }
    readonly property bool installActionVisible: !managerBusy && installAvailable
    readonly property bool uninstallActionVisible: !managerBusy && uninstallAvailable
    readonly property bool gameDefaultActionVisible: defaultActionsAvailable
                                                     && gameDefaultActionsAvailable
                                                     && defaultTargetAvailable
    readonly property bool platformDefaultActionVisible: defaultActionsAvailable
                                                         && defaultTargetAvailable

    signal installRequested(int rowIndex)
    signal uninstallRequested(int rowIndex, string emulatorName, string confirmation)
    signal gameDefaultRequested(int rowIndex)
    signal platformDefaultRequested(int rowIndex)

    width: ListView.view ? ListView.view.width : 720
    height: defaultActionsAvailable ? 116 : 78
    radius: 10
    color: emulatorHover.hovered ? "#1b2330" : "#151c27"
    border.color: emulatorStatus === "MANAGED" ? "#28584f" : line

    HoverHandler { id: emulatorHover }

    RowLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 78
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
            visible: row.managerBusy && row.managerBusyIndex === row.rowIndex
            running: visible
            Layout.preferredWidth: 26
            Layout.preferredHeight: 26
        }

        HeaderButton {
            objectName: "emulatorInstallButton"
            visible: row.installActionVisible
            text: "Install"
            active: true
            onClicked: row.installRequested(row.rowIndex)
        }

        Button {
            objectName: "emulatorUninstallButton"
            visible: row.uninstallActionVisible
            text: "Uninstall"
            onClicked: row.uninstallRequested(
                           row.rowIndex,
                           row.emulatorName,
                           row.emulatorModel.uninstall_confirmation_at(row.rowIndex))
        }
    }

    RowLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: 73
        anchors.rightMargin: 14
        height: 38
        spacing: 8
        visible: row.defaultActionsAvailable

        Text {
            Layout.fillWidth: true
            text: row.defaultTargetAvailable
                  ? "Choose where this installed emulator is the default"
                  : "Install this emulator to use it as a default"
            color: row.muted
            font.pixelSize: 9
            elide: Text.ElideRight
        }

        Button {
            id: emulatorGameDefaultButton
            objectName: "emulatorGameDefaultButton"
            visible: row.gameDefaultActionVisible
            enabled: !row.managerBusy
            Layout.preferredWidth: 132
            Layout.preferredHeight: 29
            text: row.gameDefault ? "✓  GAME DEFAULT" : "USE FOR GAME"
            font.pixelSize: 8
            font.weight: Font.Bold
            onClicked: row.gameDefaultRequested(row.rowIndex)
            ToolTip.visible: hovered
            ToolTip.text: "Use " + row.emulatorName + " by default for this game"
        }

        Button {
            id: emulatorPlatformDefaultButton
            objectName: "emulatorPlatformDefaultButton"
            visible: row.platformDefaultActionVisible
            enabled: !row.managerBusy
            Layout.preferredWidth: 142
            Layout.preferredHeight: 29
            text: row.platformDefault ? "✓  SYSTEM DEFAULT" : "USE FOR SYSTEM"
            font.pixelSize: 8
            font.weight: Font.Bold
            onClicked: row.platformDefaultRequested(row.rowIndex)
            ToolTip.visible: hovered
            ToolTip.text: "Use " + row.emulatorName + " by default for " + row.platformName
        }
    }
}
