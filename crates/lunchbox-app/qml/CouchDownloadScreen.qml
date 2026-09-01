import QtQuick
import QtQuick.Controls

FocusScope {
    id: screen

    required property var details
    property bool active: false
    property string gameTitle: ""
    property string platformName: ""
    property url coverUrl: ""
    property url heroUrl: ""
    property color backgroundColor: "#0b1119"
    property color panelColor: "#121b27"
    property color panelRaisedColor: "#1a2635"
    property color inkColor: "#f3f6fa"
    property color mutedColor: "#98a7ba"
    property color accentColor: "#f2a43a"
    property color accentCoolColor: "#56dbc8"
    property color dangerColor: "#ff6b6b"
    property int cardRadius: 18
    property string inputHint: "ENTER  SELECT   ·   ESC  RETURN"

    property string phase: "candidates"
    property int selectedCandidate: 0
    property int reviewIndex: -1
    property bool queuePending: false

    readonly property int candidateCount: {
        details.detail_revision
        return details.download_candidate_count()
    }
    readonly property bool needsSetup: details.download_preflight_action
                                                === "configure_qbittorrent"
    readonly property string actionLabel: phase === "candidates"
            ? (candidateCount > 0 ? "REVIEW DOWNLOAD" : "WAITING FOR MATCHES")
            : phase === "queue"
              ? "ADDING TO DOWNLOADS…"
              : phase === "result"
                ? "RETURN TO BROWSING"
                : details.download_preflight_busy
                  ? "CHECKING STORAGE…"
                  : needsSetup
                    ? "OPEN DOWNLOAD SETTINGS"
                    : details.download_preflight_ready
                      ? "DOWNLOAD"
                      : "TRY AGAIN"

    signal closeRequested()
    signal configureRequested()
    signal importTorrentRequested()

    visible: active
    focus: active

    function withAlpha(color, opacity) {
        return Qt.rgba(color.r, color.g, color.b, opacity)
    }

    function resetForGame() {
        phase = "candidates"
        selectedCandidate = candidateCount > 0 ? 0 : -1
        reviewIndex = -1
        queuePending = false
        Qt.callLater(function() {
            if (candidateList.count > 0 && selectedCandidate >= 0) {
                candidateList.currentIndex = selectedCandidate
                candidateList.positionViewAtIndex(selectedCandidate,
                                                  ListView.Contain)
            }
            forceActiveFocus()
        })
    }

    function moveSelection(delta) {
        if (candidateCount <= 0)
            return
        selectedCandidate = Math.max(0, Math.min(candidateCount - 1,
                                                 selectedCandidate + delta))
        candidateList.currentIndex = selectedCandidate
        candidateList.positionViewAtIndex(selectedCandidate, ListView.Contain)
    }

    function beginReview() {
        if (selectedCandidate < 0 || selectedCandidate >= candidateCount)
            return
        const selected = details.select_download_candidate(selectedCandidate)
        if (selected < 0)
            return
        reviewIndex = selected
        phase = "review"
        details.inspect_download(reviewIndex)
    }

    function activateReview() {
        if (phase === "queue")
            return
        if (phase === "result") {
            closeRequested()
            return
        }
        if (needsSetup) {
            configureRequested()
            return
        }
        if (!details.download_preflight_ready) {
            if (!details.download_preflight_busy && reviewIndex >= 0)
                details.inspect_download(reviewIndex)
            return
        }
        queuePending = true
        phase = "queue"
        details.queue_file(reviewIndex)
        Qt.callLater(function() {
            if (queuePending && !details.download_busy) {
                queuePending = false
                phase = "result"
            }
        })
    }

    function handleNavigation(action) {
        if (!active)
            return false
        forceActiveFocus()
        if (phase === "candidates") {
            if (action === "back" || action === "menu") {
                closeRequested()
            } else if (action === "up" || action === "left") {
                moveSelection(-1)
            } else if (action === "down" || action === "right") {
                moveSelection(1)
            } else if (action === "page_left") {
                moveSelection(-5)
            } else if (action === "page_right") {
                moveSelection(5)
            } else if (action === "home") {
                selectedCandidate = candidateCount > 0 ? 0 : -1
                candidateList.currentIndex = selectedCandidate
                if (selectedCandidate >= 0)
                    candidateList.positionViewAtBeginning()
            } else if (action === "accept") {
                if (candidateCount > 0)
                    beginReview()
                else if (!details.torrent_loading)
                    importTorrentRequested()
            } else {
                return false
            }
            return true
        }
        if (action === "back" || action === "menu") {
            if (phase === "review") {
                phase = "candidates"
                Qt.callLater(function() { candidateList.forceActiveFocus() })
            } else if (phase !== "queue") {
                closeRequested()
            }
        } else if (action === "accept") {
            activateReview()
        } else {
            return false
        }
        return true
    }

    onActiveChanged: {
        if (active)
            resetForGame()
    }

    onCandidateCountChanged: {
        if (candidateCount <= 0) {
            selectedCandidate = -1
        } else if (selectedCandidate < 0) {
            selectedCandidate = 0
        } else if (selectedCandidate >= candidateCount) {
            selectedCandidate = candidateCount - 1
        }
    }

    Connections {
        target: screen.details
        ignoreUnknownSignals: true

        function onDownload_busyChanged() {
            if (screen.queuePending && !screen.details.download_busy) {
                screen.queuePending = false
                screen.phase = "result"
                screen.forceActiveFocus()
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        color: screen.backgroundColor
    }

    Image {
        anchors.fill: parent
        source: screen.heroUrl
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: status === Image.Ready ? 0.23 : 0
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0; color: "#55000000" }
            GradientStop { position: 0.48; color: "#bb0b1119" }
            GradientStop { position: 1; color: screen.backgroundColor }
        }
    }

    Row {
        id: heading
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Math.max(44, parent.width * 0.045)
        anchors.rightMargin: Math.max(44, parent.width * 0.045)
        anchors.topMargin: Math.max(34, parent.height * 0.045)
        height: 88
        spacing: 22

        ArtworkMat {
            width: 58
            height: 82
            radius: 8
            artworkPresent: coverImage.status === Image.Ready
            fallbackColor: screen.panelRaisedColor
            neutralColor: screen.backgroundColor
            clip: true

            Image {
                id: coverImage
                anchors.fill: parent
                source: screen.coverUrl
                fillMode: Image.PreserveAspectFit
                asynchronous: true
            }
            Text {
                anchors.centerIn: parent
                visible: parent.fallbackVisible
                text: screen.gameTitle.length > 0
                      ? screen.gameTitle.charAt(0).toUpperCase() : "?"
                color: screen.inkColor
                font.pixelSize: 24
                font.weight: Font.Black
            }
        }

        Column {
            width: parent.width - 80
            anchors.verticalCenter: parent.verticalCenter
            spacing: 5

            Text {
                width: parent.width
                text: screen.phase === "candidates" ? "CHOOSE A DOWNLOAD"
                                                    : "REVIEW DOWNLOAD"
                color: screen.accentColor
                font.pixelSize: 12
                font.weight: Font.Bold
                font.letterSpacing: 1.5
            }
            Text {
                width: parent.width
                text: screen.gameTitle
                color: screen.inkColor
                font.pixelSize: Math.max(26, Math.min(40, screen.width * 0.027))
                font.weight: Font.Black
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: screen.platformName
                color: screen.mutedColor
                font.pixelSize: 12
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
        }
    }

    Item {
        anchors.left: heading.left
        anchors.right: heading.right
        anchors.top: heading.bottom
        anchors.bottom: footer.top
        anchors.topMargin: 24
        anchors.bottomMargin: 22

        Column {
            anchors.fill: parent
            spacing: 12
            visible: screen.phase === "candidates"

            Row {
                width: parent.width
                spacing: 12

                Text {
                    text: screen.candidateCount > 0
                          ? screen.candidateCount + " exact candidate"
                            + (screen.candidateCount === 1 ? "" : "s") + " ready"
                          : "Finding exact candidates"
                    color: screen.inkColor
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                }
                Text {
                    visible: screen.details.torrent_loading
                    text: "·  CHECKING MORE SOURCES…"
                    color: screen.accentCoolColor
                    font.pixelSize: 11
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                }
            }

            Text {
                width: parent.width
                text: screen.candidateCount > 0
                      ? "The strongest title, region, and version match is first. Nothing is downloaded until you review storage and confirm."
                      : screen.details.message
                color: screen.mutedColor
                font.pixelSize: 12
                wrapMode: Text.WordWrap
            }

            ListView {
                id: candidateList
                width: parent.width
                height: parent.height - y
                clip: true
                model: screen.candidateCount
                currentIndex: screen.selectedCandidate
                spacing: 8
                boundsBehavior: Flickable.StopAtBounds
                keyNavigationEnabled: false

                ScrollBar.vertical: ScrollBar {
                    policy: ScrollBar.AsNeeded
                    width: 10
                }

                delegate: Rectangle {
                    id: candidateRow
                    required property int index
                    width: candidateList.width - 16
                    height: 92
                    radius: Math.max(9, screen.cardRadius - 6)
                    color: candidateRow.index === screen.selectedCandidate
                           ? screen.withAlpha(screen.accentColor, 0.18)
                           : rowHover.hovered
                             ? screen.withAlpha(screen.panelRaisedColor, 0.94)
                             : screen.withAlpha(screen.panelColor, 0.88)
                    border.color: candidateRow.index === screen.selectedCandidate
                                  ? screen.accentColor
                                  : screen.withAlpha(screen.mutedColor, 0.3)
                    border.width: candidateRow.index === screen.selectedCandidate ? 2 : 1

                    Column {
                        anchors.left: parent.left
                        anchors.right: candidateRank.left
                        anchors.leftMargin: 18
                        anchors.rightMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 5

                        Text {
                            width: parent.width
                            text: screen.details.download_candidate_source_at(candidateRow.index)
                            color: candidateRow.index === screen.selectedCandidate
                                   ? screen.accentColor : screen.accentCoolColor
                            font.pixelSize: 10
                            font.weight: Font.Bold
                            font.letterSpacing: 0.7
                            elide: Text.ElideRight
                        }
                        Text {
                            width: parent.width
                            text: screen.details.download_candidate_name_at(candidateRow.index)
                            color: screen.inkColor
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                            elide: Text.ElideMiddle
                        }
                        Text {
                            width: parent.width
                            text: screen.details.download_candidate_detail_at(candidateRow.index)
                            color: screen.mutedColor
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                    }

                    Text {
                        id: candidateRank
                        anchors.right: parent.right
                        anchors.rightMargin: 18
                        anchors.verticalCenter: parent.verticalCenter
                        text: candidateRow.index === 0 ? "BEST\nMATCH"
                                                      : (candidateRow.index + 1).toString()
                        color: candidateRow.index === 0 ? screen.accentColor
                                                       : screen.mutedColor
                        font.pixelSize: candidateRow.index === 0 ? 10 : 16
                        font.weight: Font.Black
                        horizontalAlignment: Text.AlignHCenter
                    }

                    HoverHandler {
                        id: rowHover
                        onHoveredChanged: {
                            if (hovered) {
                                screen.selectedCandidate = candidateRow.index
                                candidateList.currentIndex = candidateRow.index
                            }
                        }
                    }
                    TapHandler {
                        onTapped: {
                            screen.selectedCandidate = candidateRow.index
                            candidateList.currentIndex = candidateRow.index
                            screen.beginReview()
                        }
                    }
                }

                Column {
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 80, 620)
                    visible: screen.candidateCount === 0
                    spacing: 18

                    Text {
                        width: parent.width
                        text: screen.details.torrent_loading
                              ? "Checking the available preservation sets…"
                              : "No matching download is available for this exact release. Import a torrent to review and select only this game's file."
                        color: screen.mutedColor
                        font.pixelSize: 16
                        font.weight: Font.Medium
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: Math.min(390, parent.width)
                        height: 58
                        visible: !screen.details.torrent_loading
                        radius: Math.max(10, screen.cardRadius - 5)
                        color: importHover.hovered
                               ? Qt.lighter(screen.accentColor, 1.08)
                               : screen.accentColor
                        border.color: screen.inkColor
                        border.width: 2

                        Text {
                            anchors.centerIn: parent
                            text: "IMPORT TORRENT"
                            color: "#1b140c"
                            font.pixelSize: 13
                            font.weight: Font.Black
                            font.letterSpacing: 0.9
                        }
                        HoverHandler { id: importHover }
                        TapHandler {
                            onTapped: screen.importTorrentRequested()
                        }
                    }
                }
            }
        }

        Column {
            anchors.fill: parent
            spacing: 16
            visible: screen.phase !== "candidates"

            Rectangle {
                width: parent.width
                height: 92
                radius: screen.cardRadius
                color: screen.withAlpha(screen.panelRaisedColor, 0.9)
                border.color: screen.withAlpha(screen.mutedColor, 0.34)

                Column {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 6
                    Text {
                        width: parent.width
                        text: screen.selectedCandidate >= 0
                              ? screen.details.download_candidate_source_at(screen.selectedCandidate)
                              : "SELECTED SOURCE"
                        color: screen.accentCoolColor
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: screen.selectedCandidate >= 0
                              ? screen.details.download_candidate_name_at(screen.selectedCandidate)
                              : ""
                        color: screen.inkColor
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        elide: Text.ElideMiddle
                    }
                    Text {
                        width: parent.width
                        text: screen.selectedCandidate >= 0
                              ? screen.details.download_candidate_detail_at(screen.selectedCandidate)
                              : ""
                        color: screen.mutedColor
                        font.pixelSize: 10
                        elide: Text.ElideRight
                    }
                }
            }

            Rectangle {
                width: parent.width
                height: parent.height - 108
                radius: screen.cardRadius
                color: screen.withAlpha(screen.panelColor, 0.94)
                border.color: screen.needsSetup ? screen.accentColor
                              : screen.details.download_preflight_ready
                                ? screen.accentCoolColor
                                : screen.withAlpha(screen.mutedColor, 0.38)

                Column {
                    anchors.fill: parent
                    anchors.margins: Math.max(22, parent.width * 0.025)
                    spacing: 16

                    Text {
                        width: parent.width
                        text: screen.phase === "result" ? "DOWNLOAD REQUEST"
                              : screen.phase === "queue" ? "ADDING DOWNLOAD"
                              : screen.details.download_preflight_busy
                                ? "CHECKING YOUR STORAGE"
                                : screen.needsSetup ? "ONE-TIME SETUP"
                                  : screen.details.download_preflight_ready
                                    ? "READY TO DOWNLOAD" : "CHECK REQUIRED"
                        color: screen.needsSetup ? screen.accentColor
                              : screen.details.download_preflight_ready
                                ? screen.accentCoolColor : screen.mutedColor
                        font.pixelSize: 13
                        font.weight: Font.Bold
                        font.letterSpacing: 1.2
                    }

                    Text {
                        width: parent.width
                        text: screen.phase === "result" ? screen.details.message
                              : screen.phase === "queue"
                                ? "Lunchbox is adding only this reviewed selection to qBittorrent."
                                : screen.details.download_preflight_status
                        color: screen.inkColor
                        font.pixelSize: 17
                        font.weight: Font.Medium
                        lineHeight: 1.25
                        wrapMode: Text.WordWrap
                    }

                    Repeater {
                        model: screen.phase === "review" ? [
                            { label: "DOWNLOAD", value: screen.details.download_preflight_mode },
                            { label: "STORAGE", value: screen.details.download_preflight_storage },
                            { label: "GAME FILE", value: screen.details.download_preflight_destination }
                        ] : []

                        delegate: Row {
                            required property var modelData
                            width: parent.width
                            spacing: 16
                            visible: modelData.value.length > 0

                            Text {
                                width: 104
                                text: modelData.label
                                color: screen.mutedColor
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                font.letterSpacing: 0.8
                            }
                            Text {
                                width: parent.width - 120
                                text: modelData.value
                                color: screen.inkColor
                                font.pixelSize: 12
                                wrapMode: Text.WrapAnywhere
                            }
                        }
                    }

                    Item { width: 1; height: 1 }

                    Rectangle {
                        width: Math.min(430, parent.width)
                        height: 58
                        radius: Math.max(10, screen.cardRadius - 5)
                        color: screen.phase === "queue"
                               ? screen.withAlpha(screen.mutedColor, 0.28)
                               : screen.withAlpha(screen.accentColor, 0.92)
                        border.color: screen.inkColor
                        border.width: screen.phase === "queue" ? 0 : 2
                        opacity: screen.phase === "review"
                                 && screen.details.download_preflight_busy ? 0.72 : 1

                        Text {
                            anchors.centerIn: parent
                            text: screen.actionLabel
                            color: screen.inkColor
                            font.pixelSize: 13
                            font.weight: Font.Black
                            font.letterSpacing: 0.9
                        }
                        TapHandler {
                            enabled: screen.phase !== "queue"
                            onTapped: screen.activateReview()
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        id: footer
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 54
        color: screen.withAlpha(screen.panelColor, 0.96)
        border.color: screen.withAlpha(screen.mutedColor, 0.25)

        Text {
            anchors.left: parent.left
            anchors.leftMargin: Math.max(44, parent.width * 0.045)
            anchors.verticalCenter: parent.verticalCenter
            text: screen.inputHint
            color: screen.mutedColor
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
        Text {
            anchors.right: parent.right
            anchors.rightMargin: Math.max(44, parent.width * 0.045)
            anchors.verticalCenter: parent.verticalCenter
            text: screen.phase === "candidates" && screen.candidateCount > 0
                  ? (screen.selectedCandidate + 1) + " / " + screen.candidateCount
                  : "EXACT FILE · VERIFIED STORAGE"
            color: screen.accentCoolColor
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.8
        }
    }
}
