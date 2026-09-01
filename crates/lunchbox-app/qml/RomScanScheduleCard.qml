import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card

    property var scanModel: null
    property color ink: "#eef4fb"
    property color muted: "#8e9aaa"
    property color accent: "#ffad42"
    property color accentCool: "#62e0cf"
    property color panelRaised: "#121c29"
    property color line: "#263344"
    signal reviewRequested()

    readonly property bool hasReview: scanModel
                                      && scanModel.schedule_review_profile_count > 0
    readonly property bool running: scanModel && scanModel.batch_scanning
    readonly property bool controlsBusy: !scanModel || scanModel.busy
                                         || scanModel.profile_busy
                                         || scanModel.schedule_busy
    readonly property string badgeText: running ? "CHECKING"
                                               : hasReview
                                                 ? scanModel.schedule_review_profile_count + " TO REVIEW"
                                                 : scanModel && scanModel.schedule_cadence === "manual"
                                                   ? "MANUAL"
                                                   : "UP TO DATE"

    function cadenceIndex() {
        if (!scanModel)
            return 0
        const cadence = String(scanModel.schedule_cadence)
        for (let index = 0; index < cadenceModel.count; ++index) {
            if (cadenceModel.get(index).key === cadence)
                return index
        }
        return 0
    }

    implicitHeight: Math.max(124, scheduleLayout.implicitHeight + 26)
    radius: 10
    color: panelRaised
    border.width: 1
    border.color: hasReview ? "#775428" : running ? "#275f58" : line

    ListModel {
        id: cadenceModel
        ListElement { label: "Manual only"; key: "manual" }
        ListElement { label: "At startup"; key: "startup" }
        ListElement { label: "Daily"; key: "daily" }
        ListElement { label: "Weekly"; key: "weekly" }
    }

    RowLayout {
        id: scheduleLayout
        anchors.fill: parent
        anchors.margins: 13
        spacing: 12

        Rectangle {
            Layout.preferredWidth: 38
            Layout.preferredHeight: 38
            Layout.alignment: Qt.AlignTop
            radius: 10
            color: hasReview ? "#3a2b19" : running ? "#18342f" : "#172637"

            Text {
                anchors.centerIn: parent
                text: running ? "↻" : hasReview ? "!" : "✓"
                color: hasReview ? card.accent : card.accentCool
                font.pixelSize: running ? 20 : 17
                font.weight: Font.Bold

                RotationAnimation on rotation {
                    running: card.running
                    from: 0
                    to: 360
                    duration: 950
                    loops: Animation.Infinite
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 4

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    text: "AUTOMATIC COLLECTION CHECKS"
                    color: card.ink
                    font.pixelSize: 11
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }

                Rectangle {
                    objectName: "romScanScheduleBadge"
                    implicitWidth: scheduleBadge.implicitWidth + 14
                    implicitHeight: 22
                    radius: 11
                    color: card.hasReview ? "#3a2b19"
                                          : card.running ? "#18342f" : "#182333"
                    border.width: 1
                    border.color: card.hasReview ? "#775428"
                                                : card.running ? "#275f58" : card.line

                    Text {
                        id: scheduleBadge
                        anchors.centerIn: parent
                        text: card.badgeText
                        color: card.hasReview ? card.accent : card.accentCool
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.6
                    }
                }

                Item { Layout.fillWidth: true }
            }

            Text {
                Layout.fillWidth: true
                text: card.scanModel ? card.scanModel.schedule_status : "Loading scan schedule…"
                color: card.hasReview ? "#e2bd84" : card.muted
                font.pixelSize: 10
                wrapMode: Text.WordWrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 7

                Text {
                    Layout.fillWidth: true
                    text: card.scanModel ? card.scanModel.schedule_last_run : ""
                    color: card.muted
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignRight
                    text: card.scanModel ? card.scanModel.schedule_next_run : ""
                    color: card.accentCool
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 170
            Layout.fillHeight: true
            spacing: 7

            ComboBox {
                id: cadencePicker
                objectName: "romScanCadencePicker"
                Layout.fillWidth: true
                model: cadenceModel
                textRole: "label"
                valueRole: "key"
                currentIndex: card.cadenceIndex()
                enabled: !card.controlsBusy && !card.running
                Accessible.name: "Automatic ROM collection scan frequency"
                onActivated: {
                    if (card.scanModel && currentValue !== card.scanModel.schedule_cadence)
                        card.scanModel.set_scan_schedule(currentValue)
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 6

                Button {
                    objectName: "romScanRunNowButton"
                    Layout.fillWidth: true
                    text: "RUN NOW"
                    enabled: !card.controlsBusy && !card.running
                             && card.scanModel.profile_count > 0
                    onClicked: card.scanModel.run_scheduled_scan_now()
                    ToolTip.visible: hovered
                    ToolTip.text: "Check every saved collection without importing anything"
                }

                BusyIndicator {
                    visible: card.scanModel && card.scanModel.schedule_busy
                    running: visible
                    Layout.preferredWidth: 24
                    Layout.preferredHeight: 24
                }
            }

            Button {
                objectName: "romScanReviewButton"
                Layout.fillWidth: true
                visible: card.hasReview
                text: "REVIEW CHANGES"
                enabled: !card.controlsBusy && !card.running
                onClicked: card.reviewRequested()
                ToolTip.visible: hovered
                ToolTip.text: card.scanModel
                              ? "Rescan “" + card.scanModel.scheduled_review_profile_name_at(0)
                                + "” and review every result"
                              : "Review the next changed collection"
            }
        }
    }
}
