pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: view
    objectName: "libraryAuditHistory"

    required property var auditModel
    property color ink: "#edf3f8"
    property color muted: "#8e9bad"
    property color panel: "#111822"
    property color panelRaised: "#182230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d7c6"

    modal: true
    anchors.centerIn: parent
    width: Math.min(760, parent ? parent.width - 64 : 760)
    height: Math.min(620, parent ? parent.height - 64 : 620)
    padding: 18
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        radius: 15
        color: view.panel
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
            spacing: 14

            Rectangle {
                Layout.preferredWidth: 40
                Layout.preferredHeight: 40
                radius: 11
                color: Qt.rgba(view.accent.r, view.accent.g, view.accent.b, 0.12)
                border.color: Qt.rgba(view.accent.r, view.accent.g, view.accent.b, 0.5)
                Text {
                    anchors.centerIn: parent
                    text: "↕"
                    color: view.accent
                    font.pixelSize: 17
                    font.weight: Font.Bold
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1
                Text {
                    text: "AUDIT HISTORY"
                    color: view.ink
                    font.pixelSize: 19
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
                Text {
                    text: "Up to 100 complete audit summaries · cancelled runs are never baselines"
                    color: view.muted
                    font.pixelSize: 10
                }
            }

            HeaderButton {
                text: "×"
                implicitWidth: 38
                leftPadding: 0
                rightPadding: 0
                Accessible.name: "Close audit history"
                onClicked: view.close()
            }
        }
    }

    contentItem: Rectangle {
        radius: 11
        color: "#0d141e"
        border.color: view.line

        ListView {
            id: historyList
            objectName: "libraryAuditHistoryList"
            anchors.fill: parent
            anchors.margins: 1
            clip: true
            focus: true
            model: view.auditModel.history_count
            boundsBehavior: Flickable.StopAtBounds
            keyNavigationEnabled: true
            highlightMoveDuration: 80
            currentIndex: count > 0 ? 0 : -1
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AlwaysOn }
            AcceleratedWheelHandler { scroller: historyList }

            function handleNavigation(key) {
                if (key === Qt.Key_Home) {
                    historyList.positionViewAtBeginning()
                    historyList.currentIndex = historyList.count > 0 ? 0 : -1
                    return true
                }
                if (key === Qt.Key_End) {
                    historyList.positionViewAtEnd()
                    historyList.currentIndex = historyList.count - 1
                    return true
                }
                if (key === Qt.Key_PageDown) {
                    historyList.contentY = Math.min(
                                historyList.originY + Math.max(
                                    0, historyList.contentHeight - historyList.height),
                                historyList.contentY + historyList.height * 0.86)
                    return true
                }
                if (key === Qt.Key_PageUp) {
                    historyList.contentY = Math.max(
                                historyList.originY,
                                historyList.contentY - historyList.height * 0.86)
                    return true
                }
                return false
            }

            Keys.priority: Keys.BeforeItem
            Keys.onPressed: function(event) {
                if (historyList.handleNavigation(event.key))
                    event.accepted = true
            }

            delegate: Rectangle {
                id: historyRow
                required property int index
                width: ListView.view.width
                height: 94
                color: historyRow.index === historyList.currentIndex
                       ? "#1a2533" : rowHover.hovered ? "#151f2c" : "transparent"
                border.color: "#202a39"

                HoverHandler { id: rowHover }
                TapHandler { onTapped: historyList.currentIndex = historyRow.index }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 18
                    spacing: 14

                    Rectangle {
                        Layout.preferredWidth: 76
                        Layout.preferredHeight: 48
                        radius: 10
                        color: historyRow.index === 0 ? "#1a2d2b" : "#151d29"
                        border.color: historyRow.index === 0 ? "#315c54" : view.line
                        Column {
                            anchors.centerIn: parent
                            spacing: 1
                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: view.auditModel.history_issue_count_at(historyRow.index)
                                color: historyRow.index === 0 ? view.accentCool : view.ink
                                font.pixelSize: 17
                                font.weight: Font.Bold
                            }
                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: "ISSUES"
                                color: view.muted
                                font.pixelSize: 7
                                font.weight: Font.Bold
                                font.letterSpacing: 0.7
                            }
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 3
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                objectName: historyRow.index === 0
                                            ? "libraryAuditLatestLabel" : ""
                                Layout.fillWidth: true
                                text: new Date(view.auditModel.history_finished_at(
                                                   historyRow.index) * 1000).toLocaleString()
                                color: view.ink
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                visible: historyRow.index === 0
                                text: "LATEST"
                                color: view.accentCool
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 0.8
                            }
                            Text {
                                text: view.auditModel.history_total_count_at(historyRow.index)
                                      + " RECORDS"
                                color: view.muted
                                font.pixelSize: 8
                                font.weight: Font.Bold
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: view.auditModel.history_summary_at(historyRow.index)
                            color: view.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: view.auditModel.history_comparison_at(historyRow.index)
                            color: historyRow.index === 0 ? view.accentCool : "#718096"
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(460, parent.width - 64)
            spacing: 8
            visible: view.auditModel.history_count === 0
            Text {
                width: parent.width
                text: "NO COMPLETE AUDITS YET"
                color: view.ink
                font.pixelSize: 15
                font.weight: Font.Bold
                horizontalAlignment: Text.AlignHCenter
            }
            Text {
                width: parent.width
                text: "Finish a Library Audit to establish the first trusted comparison baseline."
                color: view.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }
        }
    }

    footer: Rectangle {
        width: parent.width
        height: 62
        radius: 15
        color: view.panelRaised
        HeaderButton {
            anchors.right: parent.right
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            text: "Close"
            onClicked: view.close()
        }
    }
}
