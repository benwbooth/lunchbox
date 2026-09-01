pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: page

    required property var detailsModel
    required property color ink
    required property color muted
    required property color panel
    required property color panelRaised
    required property color line
    required property color accent
    required property color accentCool

    signal closeRequested()
    signal primaryActionRequested()
    signal choosePackageRequested(string packageName)
    signal refreshRequested()
    signal openFolderRequested()
    signal manageEmulatorsRequested()
    signal playRequested()

    readonly property bool switchSetup:
        detailsModel.platform.toLowerCase().indexOf("nintendo switch") >= 0
    readonly property bool ready: detailsModel.firmware_missing_count === 0
                                  && detailsModel.can_launch
    readonly property string nextPackage: detailsModel.firmware_next_package
    readonly property string nextPackageLower: nextPackage.toLowerCase()

    color: "#0c1119"

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 82
            color: page.panelRaised
            border.color: page.line

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 28
                anchors.rightMargin: 28
                spacing: 18

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        Layout.fillWidth: true
                        text: page.switchSetup ? "NINTENDO SWITCH PLAY SETUP"
                                               : "EMULATOR PLAY SETUP"
                        color: page.ink
                        font.pixelSize: 18
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: page.detailsModel.title + "  ·  "
                              + page.detailsModel.emulator_name
                        color: page.muted
                        font.pixelSize: 11
                        elide: Text.ElideRight
                    }
                }

                Rectangle {
                    Layout.preferredWidth: setupStateLabel.implicitWidth + 22
                    Layout.preferredHeight: 30
                    radius: 9
                    color: page.ready ? "#17342f" : "#38291d"
                    border.color: page.ready ? page.accentCool : page.accent
                    Text {
                        id: setupStateLabel
                        anchors.centerIn: parent
                        text: page.detailsModel.firmware_busy ? "WORKING"
                              : page.ready ? "READY TO PLAY" : "SETUP REQUIRED"
                        color: page.ready ? page.accentCool : page.accent
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                }

                Button {
                    objectName: "closeFirmwareSetup"
                    text: "CLOSE"
                    onClicked: page.closeRequested()
                }
            }
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            contentWidth: availableWidth

            Item {
                width: parent.width
                implicitHeight: setupContents.implicitHeight + 72

                Column {
                    id: setupContents
                    width: Math.min(860, parent.width - 64)
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: 34
                    spacing: 18

                    Text {
                        width: parent.width
                        text: page.ready
                              ? "Your emulator now has the prerequisites required for this game."
                              : page.detailsModel.firmware_summary
                        color: page.ink
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        lineHeight: 1.25
                        wrapMode: Text.WordWrap
                    }

                    Rectangle {
                        width: parent.width
                        height: safetyText.implicitHeight + 28
                        radius: 10
                        color: "#151d29"
                        border.color: page.line
                        Text {
                            id: safetyText
                            anchors.fill: parent
                            anchors.margins: 14
                            text: page.switchSetup
                                  ? "Use keys and firmware dumped from a Nintendo Switch you own. The keys selector accepts prod.keys directly or a ZIP containing prod.keys and optional title.keys. Lunchbox checks the emulator's active profile before asking for either package."
                                  : "Use firmware acquired lawfully for hardware you own. Lunchbox validates managed packages before storing or syncing them."
                            color: page.muted
                            font.pixelSize: 11
                            lineHeight: 1.3
                            wrapMode: Text.WordWrap
                        }
                    }

                    Column {
                        width: parent.width
                        visible: page.switchSetup
                        spacing: 10

                        Repeater {
                            model: [
                                {
                                    name: "KEYS PACKAGE",
                                    packageName: "switch-keys.zip",
                                    description: "Choose prod.keys directly or a ZIP containing prod.keys and optional title.keys",
                                    current: page.nextPackageLower === "switch-keys.zip",
                                    complete: page.detailsModel.switch_prod_keys_ready,
                                    required: true
                                },
                                {
                                    name: "SWITCH FIRMWARE",
                                    packageName: "switch-firmware.zip",
                                    description: "System firmware · choose a ZIP containing the dumped NCA files",
                                    current: page.nextPackageLower === "switch-firmware.zip",
                                    complete: page.detailsModel.switch_firmware_ready,
                                    required: true
                                }
                            ]

                            delegate: Rectangle {
                                id: requirement
                                required property var modelData
                                width: setupContents.width
                                height: 88
                                radius: 11
                                color: modelData.current ? "#2b241b" : "#141d29"
                                border.width: modelData.current ? 2 : 1
                                border.color: modelData.current ? page.accent
                                              : modelData.complete ? page.accentCool : page.line

                                RowLayout {
                                    anchors.fill: parent
                                    anchors.margins: 14
                                    spacing: 14
                                    Rectangle {
                                        Layout.preferredWidth: 34
                                        Layout.preferredHeight: 34
                                        radius: 17
                                        color: requirement.modelData.complete ? "#236247"
                                               : requirement.modelData.current ? "#805523"
                                                                               : "#26313e"
                                        Text {
                                            anchors.centerIn: parent
                                            text: requirement.modelData.complete ? "✓"
                                                  : requirement.modelData.current ? "→" : "·"
                                            color: "white"
                                            font.pixelSize: 15
                                            font.weight: Font.Bold
                                        }
                                    }
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 3
                                        Text {
                                            Layout.fillWidth: true
                                            text: requirement.modelData.name
                                            color: page.ink
                                            font.pixelSize: 11
                                            font.weight: Font.Bold
                                        }
                                        Text {
                                            Layout.fillWidth: true
                                            text: requirement.modelData.description
                                            color: page.muted
                                            font.pixelSize: 10
                                            wrapMode: Text.WordWrap
                                        }
                                    }
                                    Text {
                                        text: requirement.modelData.complete ? "READY"
                                              : requirement.modelData.current ? "NEXT"
                                              : requirement.modelData.required ? "MISSING"
                                                                               : "OPTIONAL"
                                        color: requirement.modelData.complete ? page.accentCool
                                               : requirement.modelData.current ? page.accent : page.muted
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                    }
                                    Button {
                                        objectName: "firmwarePackageAction-"
                                                    + requirement.modelData.packageName
                                        readonly property bool selectorAvailable:
                                            !requirement.modelData.complete
                                        visible: selectorAvailable
                                        Layout.preferredWidth: visible ? 150 : 0
                                        Layout.preferredHeight: 36
                                        text: requirement.modelData.current
                                              && page.detailsModel.firmware_setup_action === "sync"
                                              ? "SYNC NOW"
                                              : "CHOOSE FILE"
                                        enabled: !page.detailsModel.firmware_busy
                                        font.pixelSize: 9
                                        font.weight: Font.Bold
                                        onClicked: {
                                            if (requirement.modelData.current
                                                    && page.detailsModel.firmware_setup_action
                                                       === "sync")
                                                page.primaryActionRequested()
                                            else
                                                page.choosePackageRequested(
                                                    requirement.modelData.packageName)
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Rectangle {
                        visible: !page.switchSetup || page.ready
                        width: parent.width
                        height: visible ? currentActionColumn.implicitHeight + 30 : 0
                        radius: 12
                        color: page.panelRaised
                        border.width: 2
                        border.color: page.ready ? page.accentCool : page.accent

                        Column {
                            id: currentActionColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 15
                            spacing: 10

                            Text {
                                width: parent.width
                                text: page.ready ? "SETUP COMPLETE" : "NEXT STEP"
                                color: page.ready ? page.accentCool : page.accent
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 1
                            }
                            Text {
                                objectName: "firmwareNextPackage"
                                width: parent.width
                                text: page.ready ? "Launch " + page.detailsModel.title
                                      : page.nextPackage.length > 0 ? page.nextPackage
                                                                    : "Review firmware requirements"
                                color: page.ink
                                font.pixelSize: 16
                                font.weight: Font.Bold
                                wrapMode: Text.WrapAnywhere
                            }
                            Text {
                                width: parent.width
                                visible: !page.ready
                                text: page.detailsModel.launch_status
                                color: page.muted
                                font.pixelSize: 10
                                lineHeight: 1.25
                                wrapMode: Text.WordWrap
                            }
                            Button {
                                objectName: "firmwarePrimaryAction"
                                width: parent.width
                                height: 48
                                text: page.ready ? "▶  PLAY"
                                      : page.detailsModel.firmware_busy ? "WORKING…"
                                      : page.detailsModel.firmware_setup_label
                                enabled: page.ready || !page.detailsModel.firmware_busy
                                font.pixelSize: 11
                                font.weight: Font.Bold
                                onClicked: {
                                    if (page.ready)
                                        page.playRequested()
                                    else
                                        page.primaryActionRequested()
                                }
                                background: Rectangle {
                                    radius: 9
                                    color: parent.enabled
                                           ? (parent.down ? "#d68d36" : page.accent)
                                           : "#26313e"
                                    border.color: parent.enabled ? "#ffc579" : page.line
                                }
                                contentItem: Text {
                                    text: parent.text
                                    color: parent.enabled ? "#1b140c" : page.muted
                                    font: parent.font
                                    horizontalAlignment: Text.AlignHCenter
                                    verticalAlignment: Text.AlignVCenter
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }

                    RowLayout {
                        width: parent.width
                        spacing: 10
                        Button {
                            text: "RECHECK SETUP"
                            enabled: !page.detailsModel.firmware_busy
                            onClicked: page.refreshRequested()
                        }
                        Button {
                            text: "OPEN MANAGED FOLDER"
                            enabled: !page.detailsModel.firmware_busy
                            onClicked: page.openFolderRequested()
                        }
                        Button {
                            text: "MANAGE EMULATOR"
                            enabled: !page.detailsModel.firmware_busy
                            onClicked: page.manageEmulatorsRequested()
                        }
                        Item { Layout.fillWidth: true }
                    }
                }
            }
        }
    }
}
