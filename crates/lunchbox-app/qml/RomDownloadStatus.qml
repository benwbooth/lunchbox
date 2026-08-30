import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: status

    property var queue: null
    property bool expanded: false
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color panel: "#131923"
    property color panelRaised: "#1a2230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d6c6"
    property int previousActiveCount: 0
    signal manageRequested()

    readonly property int activeCount: queue ? queue.active_count : 0
    readonly property int finishedCount: queue ? queue.finished_count : 0
    readonly property int failedCount: queue ? queue.failed_count : 0
    readonly property int jobCount: queue ? queue.job_count : 0
    readonly property string aggregateSpeed: queue ? queue.aggregate_speed : "0 B/s"
    readonly property int expandedIdealHeight: 70
                                                + ((queue && queue.message.length > 0) ? 58 : 18)
                                                + Math.min(4, jobCount) * 112
                                                + 72
    readonly property string collapsedSummary: activeCount > 0
                                                ? activeCount + (activeCount === 1 ? " active download" : " active downloads")
                                                  + (aggregateSpeed.length > 0 ? "  ·  " + aggregateSpeed : "")
                                              : failedCount > 0
                                                ? failedCount + (failedCount === 1 ? " download needs attention" : " downloads need attention")
                                              : finishedCount > 0
                                                ? finishedCount + (finishedCount === 1 ? " recent download" : " recent downloads")
                                                : "ROM downloads"

    objectName: "romDownloadStatus"
    visible: queue !== null
    width: expanded ? Math.min(460, parent ? parent.width - 32 : 460)
                    : Math.min(330, Math.max(230, collapsedContent.implicitWidth + 34))
    height: expanded ? Math.min(560,
                                parent ? parent.height - 110 : 560,
                                Math.max(260, expandedIdealHeight)) : 48
    clip: false

    Behavior on width { NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
    Behavior on height { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }

    function stateColor(stateName) {
        if (stateName === "IMPORTED" || stateName === "COMPLETE")
            return accentCool
        if (stateName === "FAILED")
            return "#ff8b9a"
        if (stateName === "CANCELLED")
            return muted
        return accent
    }

    function toggle() {
        expanded = !expanded
    }

    Component.onCompleted: previousActiveCount = activeCount

    Connections {
        target: status.queue
        ignoreUnknownSignals: true
        function onActive_countChanged() {
            if (status.activeCount > status.previousActiveCount)
                status.expanded = true
            status.previousActiveCount = status.activeCount
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: status.expanded ? 15 : 24
        color: status.expanded ? "#101620" : status.panelRaised
        border.color: status.failedCount > 0 ? "#8f3f50"
                     : status.activeCount > 0 ? status.accentCool : status.line
        border.width: 1

        Behavior on radius { NumberAnimation { duration: 130 } }

        RowLayout {
            id: collapsedContent
            objectName: "collapsedContent"
            anchors.fill: parent
            anchors.leftMargin: 15
            anchors.rightMargin: 15
            spacing: 9
            visible: !status.expanded

            Text {
                text: status.activeCount > 0 ? "↓" : "✓"
                color: status.activeCount > 0 ? status.accentCool : status.muted
                font.pixelSize: 17
                font.weight: Font.Bold
            }
            Text {
                objectName: "collapsedSummary"
                Layout.fillWidth: true
                text: status.collapsedSummary
                color: status.ink
                font.pixelSize: 11
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                font.features: { "tnum": 1 }
            }
            Rectangle {
                visible: status.failedCount > 0
                Layout.preferredWidth: 22
                Layout.preferredHeight: 22
                radius: 11
                color: "#5b2632"
                Text {
                    anchors.centerIn: parent
                    text: status.failedCount
                    color: "#ffd2d8"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                }
            }
            Text {
                text: "⌃"
                color: status.muted
                font.pixelSize: 13
            }
        }

        MouseArea {
            anchors.fill: parent
            visible: !status.expanded
            cursorShape: Qt.PointingHandCursor
            onClicked: status.expanded = true
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0
            visible: status.expanded

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 70
                radius: 15
                color: status.panel
                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 15
                    color: parent.color
                }
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 17
                    anchors.rightMargin: 10
                    spacing: 10
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Text {
                            text: "ROM DOWNLOADS"
                            color: status.ink
                            font.pixelSize: 14
                            font.weight: Font.Bold
                            font.letterSpacing: 1
                        }
                        Text {
                            text: status.activeCount + " active  ·  "
                                  + status.jobCount + " total  ·  "
                                  + status.aggregateSpeed
                            color: status.activeCount > 0 ? status.accentCool : status.muted
                            font.pixelSize: 10
                            font.features: { "tnum": 1 }
                        }
                    }
                    RoundButton {
                        text: "↻"
                        flat: true
                        enabled: status.queue && !status.queue.busy
                        onClicked: status.queue.refresh()
                        ToolTip.visible: hovered
                        ToolTip.text: "Refresh downloads"
                    }
                    RoundButton {
                        objectName: "collapseButton"
                        text: "⌄"
                        flat: true
                        onClicked: status.expanded = false
                        ToolTip.visible: hovered
                        ToolTip.text: "Hide download queue"
                    }
                }
            }

            Text {
                Layout.fillWidth: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                Layout.topMargin: 10
                Layout.bottomMargin: 8
                visible: status.queue && status.queue.message.length > 0
                text: status.queue ? status.queue.message : ""
                color: status.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }

            ListView {
                id: jobs
                objectName: "downloadRows"
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                clip: true
                spacing: 8
                model: status.jobCount
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

                delegate: Rectangle {
                    id: downloadRow
                    required property int index
                    property int queueRevision: status.queue ? status.queue.revision : 0
                    readonly property string stateName: {
                        queueRevision
                        return status.queue ? status.queue.job_state_at(index) : ""
                    }
                    width: jobs.width - (jobs.ScrollBar.vertical.visible ? 10 : 0)
                    height: 104
                    radius: 10
                    color: status.panelRaised
                    border.color: downloadRow.stateName === "FAILED" ? "#70404a" : status.line

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 11
                        spacing: 4
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                Layout.fillWidth: true
                                text: {
                                    downloadRow.queueRevision
                                    return status.queue ? status.queue.job_title_at(downloadRow.index) : ""
                                }
                                color: status.ink
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                text: downloadRow.stateName
                                color: status.stateColor(text)
                                font.pixelSize: 8
                                font.weight: Font.Bold
                                font.letterSpacing: 0.5
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: {
                                downloadRow.queueRevision
                                return status.queue ? status.queue.job_platform_at(downloadRow.index) : ""
                            }
                            color: status.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                        InlineProgressBar {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 6
                            value: {
                                downloadRow.queueRevision
                                return status.queue ? status.queue.job_progress_at(downloadRow.index) : 0
                            }
                            fillColor: downloadRow.stateName === "FAILED" ? "#df7182" : status.accentCool
                            trackColor: status.line
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                Layout.fillWidth: true
                                text: {
                                    downloadRow.queueRevision
                                    return status.queue ? status.queue.job_detail_at(downloadRow.index) : ""
                                }
                                color: status.muted
                                font.pixelSize: 9
                                elide: Text.ElideRight
                            }
                            Button {
                                visible: status.queue && status.queue.job_can_pause(downloadRow.index)
                                enabled: status.queue && !status.queue.busy
                                text: "Pause"
                                onClicked: status.queue.pause_job(downloadRow.index)
                            }
                            Button {
                                visible: status.queue && status.queue.job_can_resume(downloadRow.index)
                                enabled: status.queue && !status.queue.busy
                                text: "Resume"
                                onClicked: status.queue.resume_job(downloadRow.index)
                            }
                        }
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: status.jobCount === 0
                    text: "No ROM downloads yet"
                    color: status.muted
                    font.pixelSize: 11
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 52
                color: status.panel
                border.color: status.line
                Button {
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    text: "OPEN DOWNLOAD MANAGER"
                    onClicked: status.manageRequested()
                }
            }
        }
    }
}
