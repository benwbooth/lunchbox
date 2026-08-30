pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: view

    required property var auditModel
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color panelRaised: "#1a2230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d6c6"
    signal reviewGame(string gameUid, int databaseId, string title, string platform)

    modal: true
    anchors.centerIn: parent
    width: Math.min(1240, parent ? parent.width - 54 : 1240)
    height: Math.min(850, parent ? parent.height - 54 : 850)
    padding: 18
    closePolicy: Popup.CloseOnEscape

    function statusColor(status) {
        if (status === "ready")
            return accentCool
        if (status === "missing" || status === "manual")
            return "#ff7a88"
        if (status === "runtime" || status === "error")
            return accent
        return "#69b7ff"
    }

    function openAndScan() {
        open()
        if (!auditModel.busy)
            auditModel.start_audit()
    }

    function reviewRow(index) {
        const gameUid = auditModel.row_game_uid_at(index)
        const databaseId = auditModel.row_database_id_at(index)
        const title = auditModel.row_title_at(index)
        const platform = auditModel.row_platform_at(index)
        close()
        reviewGame(gameUid, databaseId, title, platform)
    }

    onClosed: {
        if (auditModel.busy)
            auditModel.cancel()
    }

    background: Rectangle {
        radius: 15
        color: "#111822"
        border.color: view.line
    }

    header: Rectangle {
        width: parent.width
        height: 76
        radius: 15
        color: view.panelRaised

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 22
            anchors.rightMargin: 18
            spacing: 16

            Rectangle {
                Layout.preferredWidth: 40
                Layout.preferredHeight: 40
                radius: 11
                color: Qt.rgba(view.accent.r, view.accent.g, view.accent.b, 0.12)
                border.color: Qt.rgba(view.accent.r, view.accent.g, view.accent.b, 0.5)
                Text {
                    anchors.centerIn: parent
                    text: "◆"
                    color: view.accent
                    font.pixelSize: 18
                    font.weight: Font.Bold
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text {
                    text: "BIOS & FIRMWARE READINESS"
                    color: view.ink
                    font.pixelSize: 20
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
                Text {
                    text: "Verify the exact emulator choice for every local platform and game override"
                    color: view.muted
                    font.pixelSize: 10
                }
            }

            HeaderButton {
                text: "×"
                implicitWidth: 38
                leftPadding: 0
                rightPadding: 0
                onClicked: view.close()
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: 13

        RowLayout {
            Layout.fillWidth: true
            spacing: 9

            AuditMetric {
                Layout.fillWidth: true
                label: "Platforms"
                value: view.auditModel.platform_count
                tone: "#69b7ff"
            }
            AuditMetric {
                Layout.fillWidth: true
                label: "Required"
                value: view.auditModel.missing_count
                tone: "#ff7a88"
            }
            AuditMetric {
                Layout.fillWidth: true
                label: "Manual dumps"
                value: view.auditModel.manual_count
                tone: "#ff7a88"
            }
            AuditMetric {
                Layout.fillWidth: true
                label: "No emulator"
                value: view.auditModel.no_runtime_count
                tone: view.accent
            }
            AuditMetric {
                Layout.fillWidth: true
                label: "Optional"
                value: view.auditModel.optional_count
                tone: "#69b7ff"
            }
            AuditMetric {
                Layout.fillWidth: true
                label: "Ready"
                value: view.auditModel.ready_count
                tone: view.accentCool
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 9

            TextField {
                id: searchField
                Layout.fillWidth: true
                Layout.minimumWidth: 280
                placeholderText: "Search game, platform, emulator, package, or source"
                selectByMouse: true
                onTextEdited: filterDelay.restart()
            }
            ComboBox {
                id: statusFilter
                Layout.preferredWidth: 172
                textRole: "label"
                valueRole: "value"
                model: [
                    { label: "Action needed", value: "issues" },
                    { label: "All packages", value: "all" },
                    { label: "Required", value: "missing" },
                    { label: "Manual dumps", value: "manual" },
                    { label: "No emulator", value: "runtime" },
                    { label: "Review errors", value: "error" },
                    { label: "Optional", value: "optional" },
                    { label: "Ready", value: "ready" }
                ]
                onActivated: filterDelay.restart()
            }
            ComboBox {
                id: sortOrder
                Layout.preferredWidth: 154
                textRole: "label"
                valueRole: "value"
                model: [
                    { label: "Action priority", value: "status" },
                    { label: "Platform", value: "platform" },
                    { label: "Package", value: "package" }
                ]
                onActivated: filterDelay.restart()
            }
            HeaderButton {
                text: view.auditModel.busy ? "Cancel" : "Scan collection"
                active: !view.auditModel.busy
                implicitWidth: 140
                onClicked: {
                    if (view.auditModel.busy)
                        view.auditModel.cancel()
                    else
                        view.auditModel.start_audit()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 11
            color: "#0d141e"
            border.color: view.line

            ListView {
                id: firmwareList
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                focus: true
                model: view.auditModel.entry_count
                boundsBehavior: Flickable.StopAtBounds
                keyNavigationEnabled: true
                highlightMoveDuration: 70
                highlight: Rectangle {
                    radius: 9
                    color: "#202938"
                    border.color: view.line
                }
                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AlwaysOn
                }
                AcceleratedWheelHandler {
                    scroller: firmwareList
                }
                Keys.onPressed: function(event) {
                    if (event.key === Qt.Key_Home) {
                        firmwareList.positionViewAtBeginning()
                        firmwareList.currentIndex = firmwareList.count > 0 ? 0 : -1
                    } else if (event.key === Qt.Key_End) {
                        firmwareList.positionViewAtEnd()
                        firmwareList.currentIndex = firmwareList.count - 1
                    } else if (event.key === Qt.Key_PageDown) {
                        firmwareList.contentY = Math.min(
                                    firmwareList.originY + Math.max(0, firmwareList.contentHeight - firmwareList.height),
                                    firmwareList.contentY + firmwareList.height * 0.86)
                    } else if (event.key === Qt.Key_PageUp) {
                        firmwareList.contentY = Math.max(firmwareList.originY,
                                                        firmwareList.contentY - firmwareList.height * 0.86)
                    } else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter)
                               && firmwareList.currentIndex >= 0) {
                        view.reviewRow(firmwareList.currentIndex)
                    } else {
                        return
                    }
                    event.accepted = true
                }

                delegate: Rectangle {
                    id: auditRow
                    required property int index
                    width: ListView.view.width
                    height: 104
                    color: rowHover.hovered ? "#182230" : "transparent"
                    border.color: "#202a39"
                    property string rowStatus: {
                        view.auditModel.revision
                        return view.auditModel.row_status_at(index)
                    }

                    HoverHandler { id: rowHover }
                    TapHandler {
                        onTapped: firmwareList.currentIndex = auditRow.index
                        onDoubleTapped: view.reviewRow(auditRow.index)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 14
                        anchors.rightMargin: 17
                        spacing: 13

                        Rectangle {
                            Layout.preferredWidth: 116
                            Layout.preferredHeight: 27
                            radius: 13
                            color: Qt.rgba(view.statusColor(auditRow.rowStatus).r,
                                           view.statusColor(auditRow.rowStatus).g,
                                           view.statusColor(auditRow.rowStatus).b, 0.13)
                            border.color: view.statusColor(auditRow.rowStatus)
                            Text {
                                anchors.centerIn: parent
                                text: view.auditModel.row_status_label_at(auditRow.index)
                                color: view.statusColor(auditRow.rowStatus)
                                font.pixelSize: 9
                                font.weight: Font.Bold
                                font.letterSpacing: 0.45
                            }
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 9
                                Text {
                                    Layout.fillWidth: true
                                    text: view.auditModel.row_package_at(auditRow.index).length > 0
                                          ? view.auditModel.row_package_at(auditRow.index)
                                          : view.auditModel.row_title_at(auditRow.index)
                                    color: view.ink
                                    font.pixelSize: 13
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                }
                                Text {
                                    text: view.auditModel.row_platform_at(auditRow.index)
                                    color: view.muted
                                    font.pixelSize: 10
                                    elide: Text.ElideRight
                                    Layout.maximumWidth: 260
                                }
                            }
                            Text {
                                Layout.fillWidth: true
                                text: {
                                    const emulator = view.auditModel.row_emulator_at(auditRow.index)
                                    const source = view.auditModel.row_source_at(auditRow.index)
                                    const game = view.auditModel.row_title_at(auditRow.index)
                                    return [game, emulator, source].filter(function(value) {
                                        return value.length > 0
                                    }).join("  ·  ")
                                }
                                color: "#718096"
                                font.pixelSize: 9
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                text: view.auditModel.row_detail_at(auditRow.index)
                                color: view.statusColor(auditRow.rowStatus)
                                font.pixelSize: 10
                                elide: Text.ElideRight
                            }
                        }

                        HeaderButton {
                            text: "Review game"
                            implicitWidth: 124
                            onClicked: view.reviewRow(auditRow.index)
                        }
                    }
                }
            }

            Column {
                anchors.centerIn: parent
                width: Math.min(560, parent.width - 80)
                spacing: 10
                visible: !view.auditModel.busy && view.auditModel.entry_count === 0
                Text {
                    width: parent.width
                    text: view.auditModel.game_count === 0
                          ? "NO LOCAL GAMES TO INSPECT"
                          : "NO FIRMWARE ITEMS MATCH THIS VIEW"
                    color: view.ink
                    font.pixelSize: 16
                    font.weight: Font.Bold
                    horizontalAlignment: Text.AlignHCenter
                }
                Text {
                    width: parent.width
                    text: view.auditModel.game_count === 0
                          ? "Import a ROM folder or install a downloaded game, then scan My Collection again."
                          : "Your current filter is clear. Choose All packages to review optional and ready firmware."
                    color: view.muted
                    font.pixelSize: 11
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 4
                visible: view.auditModel.busy
                color: "#1a2330"
                Rectangle {
                    height: parent.height
                    width: view.auditModel.progress_total > 0
                           ? parent.width * Math.min(1, view.auditModel.progress_current
                                                       / view.auditModel.progress_total)
                           : parent.width * 0.24
                    color: view.accent
                    Behavior on width { NumberAnimation { duration: 100 } }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            Text {
                Layout.fillWidth: true
                text: view.auditModel.message
                color: view.auditModel.busy ? view.accent : view.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
            Text {
                visible: view.auditModel.busy && view.auditModel.current_platform.length > 0
                text: view.auditModel.current_platform
                color: "#718096"
                font.pixelSize: 9
                elide: Text.ElideRight
                Layout.maximumWidth: 320
            }
            Text {
                visible: view.auditModel.unavailable_game_count > 0
                text: view.auditModel.unavailable_game_count + " local file"
                      + (view.auditModel.unavailable_game_count === 1 ? "" : "s")
                      + " offline"
                color: "#ff7a88"
                font.pixelSize: 9
            }
        }
    }

    footer: Rectangle {
        width: parent.width
        height: 62
        radius: 15
        color: view.panelRaised
        Text {
            anchors.left: parent.left
            anchors.leftMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            text: "Review game opens the existing exact-emulator firmware tools; no BIOS file is downloaded or moved without that workflow."
            color: view.muted
            font.pixelSize: 9
        }
        HeaderButton {
            anchors.right: parent.right
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            text: "Close"
            onClicked: view.close()
        }
    }

    Timer {
        id: filterDelay
        interval: 120
        repeat: false
        onTriggered: view.auditModel.apply_filter(searchField.text,
                                                  statusFilter.currentValue,
                                                  sortOrder.currentValue)
    }
}
