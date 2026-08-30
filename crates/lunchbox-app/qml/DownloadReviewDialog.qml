import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog

    required property var detailsModel
    required property var settingsModel
    required property color ink
    required property color muted
    required property color panel
    required property color panelRaised
    required property color line
    required property color accent
    required property color accentCool

    property int reviewIndex: -1
    readonly property bool hasPlan: reviewIndex >= 0
                                    && detailsModel.file_has_download_plan(reviewIndex)
    readonly property string planKind: hasPlan
                                               ? detailsModel.file_plan_kind_at(reviewIndex) : ""
    readonly property string reviewTitle: planKind === "optical_multidisc"
                                                   ? "REVIEW MULTI-DISC DOWNLOAD"
                                                   : planKind.indexOf("arcade_") === 0
                                                     ? "REVIEW ARCADE DOWNLOAD"
                                                     : planKind.length > 0
                                                       ? "REVIEW RELATED DOWNLOAD"
                                                       : "REVIEW DOWNLOAD"
    readonly property string readinessLabel: detailsModel.download_preflight_busy
                                                     ? "CHECKING"
                                                     : detailsModel.download_preflight_ready
                                                       ? "READY"
                                                       : "NEEDS ATTENTION"

    signal queueRequested(int index)

    function openFor(index) {
        if (index < 0 || index >= detailsModel.file_count)
            return
        reviewIndex = index
        open()
        detailsModel.inspect_download(index)
    }

    modal: true
    width: Math.min(820, parent ? parent.width - 48 : 820)
    height: Math.min(hasPlan ? 720 : 610, parent ? parent.height - 48 : 720)
    leftPadding: 22
    rightPadding: 22
    topPadding: 18
    bottomPadding: 18
    closePolicy: Popup.CloseOnEscape
    onClosed: reviewIndex = -1

    background: Rectangle {
        color: dialog.panel
        radius: 16
        border.width: 1
        border.color: detailsModel.download_preflight_ready
                      ? dialog.accentCool : dialog.line
    }

    header: Rectangle {
        width: parent.width
        height: 86
        color: "#17242d"
        radius: 16

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 24
            anchors.right: readinessBadge.left
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            spacing: 5

            Text {
                width: parent.width
                text: dialog.reviewTitle
                color: dialog.ink
                font.pixelSize: 18
                font.weight: Font.Black
                font.letterSpacing: 1
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: dialog.reviewIndex >= 0
                      ? detailsModel.file_name_at(dialog.reviewIndex) : ""
                color: dialog.accentCool
                font.pixelSize: 11
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
            }
        }

        Rectangle {
            id: readinessBadge
            anchors.right: parent.right
            anchors.rightMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            width: readinessText.implicitWidth + 22
            height: 30
            radius: 9
            color: detailsModel.download_preflight_ready ? "#17352f"
                                                         : detailsModel.download_preflight_busy
                                                           ? "#172e3b" : "#38251f"
            border.color: detailsModel.download_preflight_ready
                          ? dialog.accentCool
                          : detailsModel.download_preflight_busy
                            ? "#5b90b7" : dialog.accent
            Text {
                id: readinessText
                anchors.centerIn: parent
                text: dialog.readinessLabel
                color: detailsModel.download_preflight_ready
                       ? dialog.accentCool : dialog.accent
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: 12

        Text {
            Layout.fillWidth: true
            text: dialog.reviewIndex >= 0
                  ? detailsModel.file_detail_at(dialog.reviewIndex) : ""
            color: dialog.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: statusColumn.implicitHeight + 22
            radius: 10
            color: dialog.panelRaised
            border.color: detailsModel.download_preflight_ready
                          ? dialog.accentCool
                          : detailsModel.download_preflight_busy
                            ? dialog.line : dialog.accent

            Column {
                id: statusColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 11
                spacing: 7

                RowLayout {
                    width: parent.width
                    spacing: 10
                    BusyIndicator {
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                        running: detailsModel.download_preflight_busy
                        visible: running
                    }
                    Text {
                        Layout.fillWidth: true
                        text: detailsModel.download_preflight_status
                        color: detailsModel.download_preflight_ready
                               ? dialog.ink : dialog.accent
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                        wrapMode: Text.WordWrap
                    }
                }
                Text {
                    width: parent.width
                    visible: detailsModel.download_preflight_mode.length > 0
                    text: detailsModel.download_preflight_mode
                    color: dialog.accentCool
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    wrapMode: Text.WordWrap
                }
                Text {
                    width: parent.width
                    visible: detailsModel.download_preflight_storage.length > 0
                    text: detailsModel.download_preflight_storage
                    color: dialog.muted
                    font.pixelSize: 9
                    wrapMode: Text.WrapAnywhere
                }
                Text {
                    width: parent.width
                    visible: detailsModel.download_preflight_destination.length > 0
                    text: detailsModel.download_preflight_destination
                    color: dialog.muted
                    font.pixelSize: 9
                    wrapMode: Text.WrapAnywhere
                }
            }
        }

        Text {
            Layout.fillWidth: true
            text: settingsModel.download_entire_torrent
                  ? "Whole-torrent mode is enabled. The storage check covers the complete source; disable it in Settings to select only the reviewed member set."
                  : dialog.planKind === "optical_multidisc"
                    ? "Every required disc and companion file is one durable queue item. Relative layout is preserved and the playlist is published last."
                    : dialog.planKind.indexOf("arcade_") === 0
                      ? "Only the reviewed ROM, machine data, framefile, laserdisc, video, and audio members needed by this machine are selected."
                      : dialog.planKind.length > 0
                        ? "The reviewed game, metadata, game-data, and utility members are retained as one recoverable download."
                        : "Only this exact reviewed torrent member is selected."
            color: settingsModel.download_entire_torrent ? dialog.accent : dialog.muted
            font.pixelSize: 10
            lineHeight: 1.25
            wrapMode: Text.WordWrap
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: dialog.line }

        Text {
            text: dialog.hasPlan ? "EXACT TORRENT MEMBERS" : "EXACT TORRENT MEMBER"
            color: dialog.accent
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1.1
        }

        ScrollView {
            id: memberScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            Text {
                width: memberScroll.availableWidth
                text: dialog.reviewIndex < 0 ? ""
                      : dialog.hasPlan
                        ? detailsModel.file_plan_members_at(dialog.reviewIndex)
                        : detailsModel.file_name_at(dialog.reviewIndex)
                color: dialog.ink
                font.family: "monospace"
                font.pixelSize: 10
                lineHeight: 1.35
                wrapMode: Text.WrapAnywhere
            }
        }

        Text {
            Layout.fillWidth: true
            text: "Filename similarity never establishes identity. Lunchbox will send only the reviewed selection above, and the same storage and duplicate checks run again immediately before qBittorrent is changed."
            color: dialog.muted
            font.pixelSize: 9
            wrapMode: Text.WordWrap
        }
    }

    footer: Rectangle {
        width: parent.width
        height: 70
        color: dialog.panelRaised
        radius: 16

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9

            HeaderButton {
                text: "Cancel"
                enabled: !detailsModel.download_busy
                onClicked: dialog.close()
            }
            HeaderButton {
                text: detailsModel.download_preflight_busy ? "Checking…" : "Recheck"
                enabled: dialog.reviewIndex >= 0
                         && !detailsModel.download_preflight_busy
                         && !detailsModel.download_busy
                onClicked: detailsModel.inspect_download(dialog.reviewIndex)
            }
            Button {
                width: 158
                height: 40
                text: settingsModel.download_entire_torrent ? "QUEUE TORRENT"
                                                             : "QUEUE DOWNLOAD"
                enabled: dialog.reviewIndex >= 0
                         && detailsModel.download_preflight_ready
                         && !detailsModel.download_preflight_busy
                         && !detailsModel.download_busy
                onClicked: dialog.queueRequested(dialog.reviewIndex)
                background: Rectangle {
                    radius: 9
                    color: parent.enabled
                           ? (parent.down ? "#d89444" : dialog.accent) : "#39414e"
                }
                contentItem: Text {
                    text: parent.text
                    color: parent.enabled ? "#17110a" : dialog.muted
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 0.5
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
            }
        }
    }
}
