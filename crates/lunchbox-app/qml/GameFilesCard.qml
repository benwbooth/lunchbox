pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Rectangle {
    id: card

    required property var detailsModel
    required property color ink
    required property color muted
    required property color line
    required property color accent
    required property color accentCool

    signal manageIdentityRequested()
    signal removeInstallationRequested()

    implicitHeight: contents.implicitHeight + 28
    radius: 11
    color: "#171f2b"
    border.color: detailsModel.local_file_preference_stale
                  ? accent : detailsModel.managed_install_present
                    ? "#4d4650" : line

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 9

        Row {
            width: parent.width
            spacing: 8

            Column {
                width: parent.width - installationState.width - 8
                spacing: 2

                Text {
                    width: parent.width
                    text: card.detailsModel.local_file_count > 1
                          ? "GAME VERSIONS" : "GAME FILE"
                    color: card.ink
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1.1
                }

                Text {
                    width: parent.width
                    visible: card.detailsModel.local_file_count > 1
                    text: card.detailsModel.local_file_count + " available · click to use for the next launch"
                    color: card.muted
                    font.pixelSize: 9
                }
            }

            Rectangle {
                id: installationState
                width: installationStateText.implicitWidth + 14
                height: 22
                radius: 7
                color: card.detailsModel.managed_install_present
                       ? "#342a31" : "#202b3a"
                border.color: card.detailsModel.managed_install_present
                              ? "#745868" : card.line

                Text {
                    id: installationStateText
                    anchors.centerIn: parent
                    text: card.detailsModel.managed_install_present
                          ? "MANAGED" : "IMPORTED"
                    color: card.detailsModel.managed_install_present
                           ? "#e8b7cf" : card.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
            }
        }

        ListView {
            id: versions
            objectName: "gameVersionList"
            property bool expanded: false
            readonly property int visibleCount: expanded ? count : Math.min(3, count)
            width: parent.width
            height: visibleCount * 62 + Math.max(0, visibleCount - 1) * spacing
            model: Math.max(0, card.detailsModel.local_file_count)
            spacing: 6
            clip: true
            interactive: false

            delegate: Rectangle {
                id: versionRow
                required property int index
                readonly property int fileRevision: card.detailsModel.local_file_revision
                readonly property bool preferred: {
                    const revision = fileRevision
                    return card.detailsModel.local_file_is_preferred_at(index)
                }
                readonly property bool selected: card.detailsModel.selected_local_file === index

                objectName: "gameVersionRow" + index
                width: ListView.view.width
                height: 62
                radius: 8
                color: selected ? "#233345" : "#121923"
                border.width: selected ? 2 : 1
                border.color: selected ? card.accentCool : "#2b3849"
                Accessible.role: Accessible.ListItem
                Accessible.name: card.detailsModel.local_file_name_at(index)
                Accessible.description: preferred ? "Default game version" : "Game version"

                Row {
                    anchors.fill: parent
                    anchors.margins: 9
                    spacing: 9

                    Rectangle {
                        width: 24
                        height: 24
                        anchors.verticalCenter: parent.verticalCenter
                        radius: 12
                        color: versionRow.selected ? card.accentCool : "#293647"

                        Text {
                            anchors.centerIn: parent
                            text: versionRow.selected ? "✓" : (versionRow.index + 1)
                            color: versionRow.selected ? "#10211e" : card.muted
                            font.pixelSize: 10
                            font.weight: Font.Bold
                        }
                    }

                    Column {
                        width: parent.width - 33 - defaultPill.width
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3

                        Text {
                            width: parent.width
                            text: card.detailsModel.local_file_name_at(versionRow.index)
                            color: versionRow.selected ? card.ink : "#c3ccd8"
                            font.pixelSize: 10
                            font.weight: versionRow.selected ? Font.DemiBold : Font.Normal
                            elide: Text.ElideMiddle
                        }

                        Text {
                            width: parent.width
                            text: card.detailsModel.local_file_path_at(versionRow.index)
                            color: card.muted
                            font.pixelSize: 8
                            elide: Text.ElideMiddle
                        }
                    }

                    Rectangle {
                        id: defaultPill
                        width: versionRow.preferred ? defaultPillText.implicitWidth + 12 : 0
                        height: 20
                        anchors.verticalCenter: parent.verticalCenter
                        visible: versionRow.preferred
                        radius: 6
                        color: "#173c35"
                        border.color: card.accentCool

                        Text {
                            id: defaultPillText
                            anchors.centerIn: parent
                            text: "DEFAULT"
                            color: card.accentCool
                            font.pixelSize: 7
                            font.weight: Font.Bold
                            font.letterSpacing: 0.6
                        }
                    }
                }

                HoverHandler { id: versionHover }
                ToolTip.visible: versionHover.hovered
                ToolTip.text: card.detailsModel.local_file_path_at(versionRow.index)

                TapHandler {
                    enabled: !card.detailsModel.launch_busy
                             && !card.detailsModel.game_running
                    onTapped: card.detailsModel.select_local_file(versionRow.index)
                }
            }
        }

        Button {
            objectName: "showAllGameVersionsButton"
            width: parent.width
            height: 30
            visible: versions.count > 3
            text: versions.expanded ? "SHOW FEWER VERSIONS"
                  : "SHOW " + (versions.count - 3) + " MORE"
            font.pixelSize: 8
            font.weight: Font.Bold
            onClicked: versions.expanded = !versions.expanded
            background: Rectangle {
                radius: 7
                color: parent.down ? "#293748" : "#202b3a"
                border.color: card.line
            }
            contentItem: Text {
                text: parent.text
                color: card.muted
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Text {
            width: parent.width
            visible: card.detailsModel.local_file_preference_message.length > 0
            text: card.detailsModel.local_file_preference_message
            color: card.detailsModel.local_file_preference_stale
                   || text.indexOf("Could not") === 0 ? card.accent : card.muted
            font.pixelSize: 9
            lineHeight: 1.25
            wrapMode: Text.WordWrap
        }

        Row {
            width: parent.width
            spacing: 8
            visible: card.detailsModel.local_file_count > 1
                     || card.detailsModel.local_file_preference_configured

            Button {
                id: makeDefaultVersionButton
                objectName: "makeDefaultVersionButton"
                width: card.detailsModel.local_file_preference_configured
                       ? (parent.width - 8) / 2 : parent.width
                height: 34
                visible: !card.detailsModel.selected_local_file_is_preferred
                         || !card.detailsModel.local_file_preference_configured
                text: "MAKE SELECTED DEFAULT"
                enabled: card.detailsModel.selected_local_file >= 0
                         && !card.detailsModel.launch_busy
                         && !card.detailsModel.game_running
                font.pixelSize: 8
                font.weight: Font.Bold
                Accessible.name: "Make selected game version the default"
                onClicked: card.detailsModel.set_selected_local_file_preferred()
                background: Rectangle {
                    radius: 7
                    color: parent.down ? "#24433f" : "#1d3433"
                    border.color: parent.enabled ? "#3d756b" : card.line
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#94e2d4" : card.muted
                    font: parent.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }

            Button {
                objectName: "automaticVersionButton"
                width: makeDefaultVersionButton.visible
                       ? (parent.width - 8) / 2 : parent.width
                height: 34
                visible: card.detailsModel.local_file_preference_configured
                text: "USE AUTOMATIC"
                enabled: !card.detailsModel.launch_busy
                         && !card.detailsModel.game_running
                font.pixelSize: 8
                font.weight: Font.Bold
                Accessible.name: "Clear the default game version"
                onClicked: card.detailsModel.clear_local_file_preference()
                background: Rectangle {
                    radius: 7
                    color: parent.down ? "#293748" : "#202b3a"
                    border.color: parent.enabled ? "#53647a" : card.line
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#c7d1df" : card.muted
                    font: parent.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Text {
            width: parent.width
            visible: card.detailsModel.install_management_message.length > 0
            text: card.detailsModel.install_management_message
            color: text.indexOf("Could not") === 0 ? "#ef9b92" : card.muted
            font.pixelSize: 9
            lineHeight: 1.25
            wrapMode: Text.WordWrap
        }

        Row {
            width: parent.width
            spacing: 8

            Button {
                width: (parent.width - 8) / 2
                height: 34
                text: "OPEN FOLDER"
                enabled: card.detailsModel.selected_local_directory_url.toString().length > 0
                         && !card.detailsModel.install_management_busy
                font.pixelSize: 9
                font.weight: Font.Bold
                onClicked: Qt.openUrlExternally(card.detailsModel.selected_local_directory_url)
                background: Rectangle {
                    radius: 7
                    color: parent.down ? "#293748" : "#202b3a"
                    border.color: parent.enabled ? "#53647a" : card.line
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#c7d1df" : card.muted
                    font: parent.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Button {
                width: (parent.width - 8) / 2
                height: 34
                visible: !card.detailsModel.managed_install_present
                text: "MANAGE IDENTITY"
                enabled: !card.detailsModel.install_management_busy
                         && card.detailsModel.game_id.length > 0
                font.pixelSize: 8
                font.weight: Font.Bold
                onClicked: card.manageIdentityRequested()
                background: Rectangle {
                    radius: 7
                    color: parent.down ? "#24433f" : "#1d3433"
                    border.color: parent.enabled ? "#3d756b" : card.line
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#94e2d4" : card.muted
                    font: parent.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Button {
                width: (parent.width - 8) / 2
                height: 34
                visible: card.detailsModel.managed_install_present
                text: card.detailsModel.install_management_busy ? "VERIFYING…"
                      : card.detailsModel.managed_install_can_delete
                        ? "UNINSTALL" : "REMOVE FROM LIBRARY"
                enabled: !card.detailsModel.install_management_busy
                         && !card.detailsModel.launch_busy
                         && !card.detailsModel.game_running
                font.pixelSize: 8
                font.weight: Font.Bold
                onClicked: card.removeInstallationRequested()
                background: Rectangle {
                    radius: 7
                    color: parent.down ? "#55313a" : "#39252d"
                    border.color: parent.enabled ? "#8b5668" : card.line
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#f1bdc9" : card.muted
                    font: parent.font
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }
        }
    }
}
