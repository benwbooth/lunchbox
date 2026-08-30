pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: screen

    required property var details
    required property string gameTitle
    required property string platformName
    required property url coverUrl
    required property url heroUrl
    required property url themeBackgroundUrl
    required property color backgroundColor
    required property color panelColor
    required property color panelRaisedColor
    required property color inkColor
    required property color mutedColor
    required property color accentColor
    required property color accentCoolColor
    required property color dangerColor
    required property string inputHint
    property int cardRadius: 18
    property bool active: false
    property double runningSince: 0
    property int runningSeconds: 0

    readonly property bool launchFailed:
        !details.launch_busy && !details.game_running
        && details.launch_status.indexOf("Could not launch") === 0
    readonly property string phase: details.launch_busy ? "starting"
                                          : details.game_running ? "running"
                                          : launchFailed ? "failed" : "complete"
    readonly property string phaseEyebrow: phase === "starting" ? "NOW LAUNCHING"
                                                 : phase === "running" ? "NOW PLAYING"
                                                 : phase === "failed" ? "LAUNCH FAILED"
                                                                      : "WELCOME BACK"
    readonly property string phaseHeadline: phase === "starting"
        ? "Preparing your game"
        : phase === "running"
          ? "The emulator is running"
          : phase === "failed"
            ? "The game could not be started"
            : details.activity_busy
              ? "Saving your play session"
              : "Session complete"
    readonly property color phaseColor: phase === "failed" ? dangerColor
                                              : phase === "running" ? accentCoolColor
                                                                    : accentColor
    readonly property int sessionRevision: details.session_history_revision
    readonly property bool sessionAvailable: {
        sessionRevision
        return details.session_count > 0
    }
    readonly property string latestSessionDuration: {
        sessionRevision
        return sessionAvailable ? details.session_duration_at(0) : "Not recorded"
    }
    readonly property string latestSessionOutcome: {
        sessionRevision
        return sessionAvailable ? details.session_outcome_label_at(0) : "Session ended"
    }
    readonly property string latestSessionEmulator: {
        sessionRevision
        return sessionAvailable ? details.session_emulator_at(0)
                                : details.emulator_name
    }

    signal closeRequested()
    signal detailsRequested()
    signal menuRequested()

    visible: active
    opacity: active ? 1 : 0
    Accessible.role: Accessible.Pane
    Accessible.name: phaseEyebrow + ", " + gameTitle
    Behavior on opacity { NumberAnimation { duration: 150 } }

    function withAlpha(value, alpha) {
        return Qt.rgba(value.r, value.g, value.b, alpha)
    }

    function formatElapsed(seconds) {
        const safe = Math.max(0, Math.floor(seconds))
        const hours = Math.floor(safe / 3600)
        const minutes = Math.floor((safe % 3600) / 60)
        const remainder = safe % 60
        if (hours > 0)
            return hours + ":" + String(minutes).padStart(2, "0")
                   + ":" + String(remainder).padStart(2, "0")
        return String(minutes).padStart(2, "0") + ":"
               + String(remainder).padStart(2, "0")
    }

    function syncRunningClock() {
        if (phase === "running") {
            if (runningSince <= 0)
                runningSince = Date.now()
            runningSeconds = Math.max(0, Math.floor((Date.now() - runningSince) / 1000))
        } else {
            runningSince = 0
            runningSeconds = 0
        }
    }

    function capture(path, callback) {
        screen.grabToImage(function(result) {
            callback(result.saveToFile(path))
        })
    }

    onPhaseChanged: syncRunningClock()
    Component.onCompleted: syncRunningClock()

    Timer {
        interval: 1000
        repeat: true
        running: screen.phase === "running"
        onTriggered: screen.syncRunningClock()
    }

    component CrispText: Text {
        font.kerning: true
        font.hintingPreference: Font.PreferVerticalHinting
        renderType: Text.NativeRendering
    }

    Rectangle {
        anchors.fill: parent
        color: screen.withAlpha(screen.backgroundColor, 0.97)
    }

    Image {
        anchors.fill: parent
        source: screen.themeBackgroundUrl
        asynchronous: true
        cache: true
        autoTransform: true
        fillMode: Image.PreserveAspectCrop
        opacity: status === Image.Ready ? 0.31 : 0
    }

    Image {
        anchors.fill: parent
        source: screen.heroUrl
        asynchronous: true
        cache: true
        autoTransform: true
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: Math.max(1, Math.round(width * 1.25))
        sourceSize.height: Math.max(1, Math.round(height * 1.25))
        opacity: status === Image.Ready ? 0.46 : 0
    }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: screen.withAlpha(screen.backgroundColor, 0.99) }
            GradientStop { position: 0.52; color: screen.withAlpha(screen.backgroundColor, 0.91) }
            GradientStop { position: 1; color: screen.withAlpha(screen.backgroundColor, 0.72) }
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Math.max(5, parent.height * 0.007)
        color: screen.withAlpha(screen.panelRaisedColor, 0.64)
        clip: true

        Rectangle {
            id: launchSweep
            width: parent.width * 0.34
            height: parent.height
            radius: height / 2
            color: screen.phaseColor
            visible: screen.phase === "starting"
            x: -width
            SequentialAnimation on x {
                running: screen.phase === "starting" && screen.active
                loops: Animation.Infinite
                NumberAnimation {
                    from: -launchSweep.width
                    to: launchSweep.parent.width
                    duration: 1150
                    easing.type: Easing.InOutCubic
                }
                PauseAnimation { duration: 90 }
            }
        }

        Rectangle {
            anchors.fill: parent
            color: screen.phaseColor
            opacity: screen.phase === "starting" ? 0 : 0.9
        }
    }

    Row {
        anchors.centerIn: parent
        width: Math.min(1240, parent.width - 150)
        height: Math.min(700, parent.height - 150)
        spacing: 64

        Item {
            width: 340
            height: parent.height

            Rectangle {
                id: coverMat
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: 16
                width: 284
                height: 400
                radius: Math.max(12, screen.cardRadius)
                color: screen.withAlpha(screen.panelColor, 0.96)
                border.color: screen.withAlpha(screen.inkColor, 0.55)
                border.width: 1
                clip: true

                Image {
                    id: coverImage
                    anchors.fill: parent
                    anchors.margins: 3
                    source: screen.coverUrl
                    asynchronous: true
                    cache: true
                    mipmap: true
                    autoTransform: true
                    fillMode: Image.PreserveAspectFit
                    sourceSize.width: 568
                    sourceSize.height: 800
                    opacity: status === Image.Ready ? 1 : 0
                }

                CrispText {
                    anchors.centerIn: parent
                    text: screen.gameTitle.length > 0
                          ? screen.gameTitle.charAt(0).toUpperCase() : "L"
                    color: screen.withAlpha(screen.inkColor, 0.72)
                    font.pixelSize: 92
                    font.weight: Font.Black
                    visible: coverImage.status !== Image.Ready
                }
            }

            Column {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: coverMat.bottom
                anchors.topMargin: 26
                spacing: 8

                CrispText {
                    width: parent.width
                    text: screen.gameTitle
                    color: screen.inkColor
                    font.pixelSize: 25
                    font.weight: Font.Black
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    horizontalAlignment: Text.AlignHCenter
                }
                CrispText {
                    width: parent.width
                    text: screen.platformName.toUpperCase()
                    color: screen.mutedColor
                    font.pixelSize: 10
                    font.weight: Font.Bold
                    font.letterSpacing: 1
                    elide: Text.ElideRight
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }

        Column {
            width: parent.width - 404
            anchors.verticalCenter: parent.verticalCenter
            spacing: 19

            CrispText {
                width: parent.width
                text: screen.phaseEyebrow
                color: screen.phaseColor
                font.pixelSize: 13
                font.weight: Font.Bold
                font.letterSpacing: 2.2
            }

            CrispText {
                width: parent.width
                text: screen.phaseHeadline
                color: screen.inkColor
                font.pixelSize: 42
                font.weight: Font.Black
                lineHeight: 0.96
                wrapMode: Text.WordWrap
            }

            CrispText {
                width: parent.width
                text: screen.phase === "starting"
                      ? "Lunchbox is validating firmware, resolving the exact emulator command, and preparing writable runtime files."
                      : screen.phase === "running"
                        ? "Your emulator is in the foreground. This screen keeps the exact game and session context ready when you return."
                        : screen.phase === "failed"
                          ? "Review the launch status below, then open Desktop Details to fix the exact emulator, firmware, or command profile."
                          : screen.details.activity_busy
                            ? "The emulator has closed. Play time and the final session result are being committed now."
                            : "Your play activity is saved. Continue browsing, inspect the session, or choose another release."
                color: screen.mutedColor
                font.pixelSize: 15
                lineHeight: 1.16
                wrapMode: Text.WordWrap
            }

            Rectangle {
                width: parent.width
                height: screen.phase === "complete" ? 124 : 106
                radius: Math.max(12, screen.cardRadius - 2)
                color: screen.withAlpha(screen.panelRaisedColor, 0.63)
                border.color: screen.withAlpha(screen.phaseColor, 0.46)

                Column {
                    anchors.fill: parent
                    anchors.margins: 18
                    spacing: 9

                    CrispText {
                        text: screen.phase === "complete" ? "SESSION RESULT" : "LAUNCH STATUS"
                        color: screen.mutedColor
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 1.2
                    }

                    CrispText {
                        width: parent.width
                        text: screen.details.launch_status.length > 0
                              ? screen.details.launch_status
                              : "Waiting for the emulator supervisor…"
                        color: screen.phase === "failed" ? screen.dangerColor
                              : screen.phase === "running" ? screen.accentCoolColor
                                                           : screen.inkColor
                        font.pixelSize: 13
                        font.weight: Font.DemiBold
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                    }

                    CrispText {
                        visible: screen.phase === "complete"
                        width: parent.width
                        text: screen.latestSessionOutcome + "  ·  "
                              + screen.latestSessionDuration + "  ·  "
                              + screen.details.play_count + " total launch"
                              + (screen.details.play_count === 1 ? "" : "es")
                        color: screen.accentCoolColor
                        font.pixelSize: 11
                        font.weight: Font.Bold
                        elide: Text.ElideRight
                    }
                }
            }

            Row {
                spacing: 10

                Rectangle {
                    width: emulatorText.implicitWidth + 26
                    height: 34
                    radius: Math.max(8, screen.cardRadius - 7)
                    color: screen.withAlpha(screen.accentCoolColor, 0.15)
                    border.color: screen.withAlpha(screen.accentCoolColor, 0.58)
                    CrispText {
                        id: emulatorText
                        anchors.centerIn: parent
                        text: (screen.phase === "complete" && screen.latestSessionEmulator.length > 0
                               ? screen.latestSessionEmulator : screen.details.emulator_name).length > 0
                              ? (screen.phase === "complete" && screen.latestSessionEmulator.length > 0
                                 ? screen.latestSessionEmulator : screen.details.emulator_name)
                              : "EMULATOR SUPERVISOR"
                        color: screen.accentCoolColor
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.65
                    }
                }

                Rectangle {
                    width: phaseMetricText.implicitWidth + 26
                    height: 34
                    radius: Math.max(8, screen.cardRadius - 7)
                    color: screen.withAlpha(screen.phaseColor, 0.15)
                    border.color: screen.withAlpha(screen.phaseColor, 0.58)
                    CrispText {
                        id: phaseMetricText
                        anchors.centerIn: parent
                        text: screen.phase === "running"
                              ? "ACTIVE  " + screen.formatElapsed(screen.runningSeconds)
                              : screen.phase === "complete" ? screen.details.play_time.toUpperCase()
                              : screen.phase === "failed" ? "ACTION REQUIRED" : "PREPARING"
                        color: screen.phaseColor
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                }
            }

            Item { width: 1; height: 2 }

            Row {
                spacing: 11

                Rectangle {
                    width: 154
                    height: 50
                    radius: Math.max(9, screen.cardRadius - 6)
                    color: gameMenuHover.hovered
                           ? screen.withAlpha(screen.panelRaisedColor, 0.92)
                           : screen.withAlpha(screen.panelRaisedColor, 0.68)
                    border.color: gameMenuHover.hovered ? screen.accentColor
                                                        : screen.withAlpha(screen.mutedColor, 0.56)
                    Accessible.role: Accessible.Button
                    Accessible.name: "Game Menu"
                    CrispText {
                        anchors.centerIn: parent
                        text: "GAME MENU"
                        color: screen.inkColor
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.65
                    }
                    HoverHandler { id: gameMenuHover }
                    TapHandler { onTapped: screen.menuRequested() }
                }

                Rectangle {
                    width: 178
                    height: 50
                    radius: Math.max(9, screen.cardRadius - 6)
                    color: detailsHover.hovered
                           ? screen.withAlpha(screen.panelRaisedColor, 0.92)
                           : screen.withAlpha(screen.panelRaisedColor, 0.68)
                    border.color: detailsHover.hovered ? screen.accentColor
                                                       : screen.withAlpha(screen.mutedColor, 0.56)
                    Accessible.role: Accessible.Button
                    Accessible.name: "Desktop Details"
                    CrispText {
                        anchors.centerIn: parent
                        text: "DESKTOP DETAILS"
                        color: screen.inkColor
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.65
                    }
                    HoverHandler { id: detailsHover }
                    TapHandler { onTapped: screen.detailsRequested() }
                }

                Rectangle {
                    width: 196
                    height: 50
                    radius: Math.max(9, screen.cardRadius - 6)
                    color: returnHover.hovered
                           ? screen.withAlpha(screen.accentColor, 0.94)
                           : screen.withAlpha(screen.accentColor, 0.79)
                    border.color: screen.accentColor
                    Accessible.role: Accessible.Button
                    Accessible.name: "Return to Browsing"
                    CrispText {
                        anchors.centerIn: parent
                        text: "RETURN TO BROWSING"
                        color: screen.backgroundColor
                        font.pixelSize: 10
                        font.weight: Font.Bold
                        font.letterSpacing: 0.65
                    }
                    HoverHandler { id: returnHover }
                    TapHandler { onTapped: screen.closeRequested() }
                }
            }
        }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: 30
        anchors.rightMargin: 34
        width: 46
        height: 46
        radius: Math.max(9, screen.cardRadius - 5)
        color: closeHover.hovered
               ? screen.withAlpha(screen.panelRaisedColor, 0.92)
               : screen.withAlpha(screen.panelColor, 0.72)
        border.color: closeHover.hovered ? screen.accentColor
                                         : screen.withAlpha(screen.mutedColor, 0.52)
        Accessible.role: Accessible.Button
        Accessible.name: "Close launch screen"
        CrispText {
            anchors.centerIn: parent
            text: "×"
            color: screen.inkColor
            font.pixelSize: 23
        }
        HoverHandler { id: closeHover }
        TapHandler { onTapped: screen.closeRequested() }
    }

    CrispText {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 30
        text: screen.inputHint
        color: screen.mutedColor
        font.pixelSize: 9
        font.weight: Font.Bold
        font.letterSpacing: 0.75
    }
}
