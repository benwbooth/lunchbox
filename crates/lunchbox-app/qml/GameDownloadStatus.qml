pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: card

    required property var queue
    required property string gameId
    required property bool gameLocal
    required property bool gameLoading
    required property color ink
    required property color muted
    required property color panel
    required property color line
    required property color accent
    required property color accentCool
    readonly property color playGreen: "#5ee391"
    property bool alternativesAvailable: false
    property bool alternativesExpanded: false

    signal manageRequested()
    signal alternativesRequested()
    signal playRequested()

    readonly property int queueRevision: queue.revision
    readonly property int jobIndex: {
        queueRevision
        return gameId.length > 0 ? queue.job_index_for_game(gameId) : -1
    }
    readonly property string jobState: jobIndex >= 0
                                               ? queue.job_state_at(jobIndex) : ""
    readonly property string badge: jobIndex >= 0
                                            ? queue.job_badge_at(jobIndex) : ""

    // An installed game's final state is the hero's Play button; the download
    // card only exists while something is actually in flight or failed. It
    // stays hidden while the details load so no intermediate state flashes.
    readonly property bool cardVisible: jobIndex >= 0
             && !gameLoading
             && !(gameLocal && jobState === "IMPORTED")
    visible: cardVisible
    height: visible ? contents.implicitHeight + 24 : 0
    radius: 11
    color: panel
    border.color: jobState === "FAILED" ? "#8f3f50"
                  : jobState === "IMPORTED" ? playGreen : accent

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
                text: card.jobState === "IMPORTED" ? "READY TO PLAY"
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
                objectName: "statePill"
                width: stateLabel.implicitWidth + 16
                height: 24
                radius: 7
                color: card.jobState === "FAILED" ? "#2a1a22"
                       : card.jobState === "IMPORTED" ? "#17342f" : "#30291d"
                border.color: card.jobState === "FAILED" ? "#8f3f50"
                              : card.jobState === "IMPORTED" ? card.playGreen : card.accent
                Text {
                    id: stateLabel
                    anchors.centerIn: parent
                    text: card.badge
                    color: card.jobState === "FAILED" ? "#ff8b9a"
                           : card.jobState === "IMPORTED" ? card.playGreen : card.accent
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
                objectName: "downloadPlayAction"
                visible: card.jobState === "IMPORTED"
                text: "Play"
                active: card.jobState === "IMPORTED"
                positive: true
                implicitHeight: 34
                onClicked: card.playRequested()
            }
            HeaderButton {
                text: "View download"
                implicitHeight: 34
                onClicked: card.manageRequested()
            }
            HeaderButton {
                visible: card.alternativesAvailable
                text: card.alternativesExpanded ? "Hide other sources"
                                                : "Show other sources"
                implicitHeight: 34
                onClicked: card.alternativesRequested()
            }
        }
    }
}
