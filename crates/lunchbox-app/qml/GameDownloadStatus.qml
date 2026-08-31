pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: card

    required property var queue
    required property string gameId
    required property color ink
    required property color muted
    required property color panel
    required property color line
    required property color accent
    required property color accentCool
    property bool alternativesAvailable: false
    property bool alternativesExpanded: false

    signal manageRequested()
    signal alternativesRequested()

    readonly property int queueRevision: queue.revision
    readonly property int jobIndex: {
        queueRevision
        return gameId.length > 0 ? queue.job_index_for_game(gameId) : -1
    }
    readonly property string jobState: jobIndex >= 0
                                               ? queue.job_state_at(jobIndex) : ""
    readonly property string badge: jobIndex >= 0
                                            ? queue.job_badge_at(jobIndex) : ""

    visible: jobIndex >= 0
    height: visible ? contents.implicitHeight + 24 : 0
    radius: 11
    color: panel
    border.color: jobState === "FAILED" ? "#8f3f50"
                  : jobState === "IMPORTED" ? accentCool : accent

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 12
        spacing: 9

        Row {
            width: parent.width
            spacing: 8
            Text {
                width: parent.width - statePill.width - 8
                text: card.jobState === "IMPORTED" ? "DOWNLOAD COMPLETE"
                      : card.jobState === "COMPLETE" ? "FINISHING INSTALL"
                      : "ROM DOWNLOAD"
                color: card.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                font.letterSpacing: 0.8
                verticalAlignment: Text.AlignVCenter
            }
            Rectangle {
                id: statePill
                width: stateLabel.implicitWidth + 16
                height: 24
                radius: 7
                color: card.jobState === "FAILED" ? "#2a1a22"
                       : card.jobState === "IMPORTED" ? "#17342f" : "#30291d"
                border.color: card.jobState === "FAILED" ? "#8f3f50"
                              : card.jobState === "IMPORTED" ? card.accentCool : card.accent
                Text {
                    id: stateLabel
                    anchors.centerIn: parent
                    text: card.badge
                    color: card.jobState === "FAILED" ? "#ff8b9a"
                           : card.jobState === "IMPORTED" ? card.accentCool : card.accent
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.6
                }
            }
        }

        InlineProgressBar {
            width: parent.width
            height: 7
            value: card.jobIndex >= 0 ? card.queue.job_progress_at(card.jobIndex) : 0
            fillColor: card.jobState === "FAILED" ? "#d85d70" : card.accentCool
            trackColor: card.line
        }

        Text {
            width: parent.width
            text: card.jobIndex >= 0 ? card.queue.job_detail_at(card.jobIndex) : ""
            color: card.muted
            font.pixelSize: 10
            lineHeight: 1.25
            wrapMode: Text.WordWrap
        }

        Row {
            width: parent.width
            spacing: 8
            HeaderButton {
                text: "VIEW DOWNLOAD"
                implicitHeight: 34
                onClicked: card.manageRequested()
            }
            HeaderButton {
                visible: card.alternativesAvailable
                text: card.alternativesExpanded ? "HIDE OTHER SOURCES"
                                                : "SHOW OTHER SOURCES"
                implicitHeight: 34
                onClicked: card.alternativesRequested()
            }
        }
    }
}
