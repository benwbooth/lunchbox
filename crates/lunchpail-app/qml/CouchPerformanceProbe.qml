import QtQuick

// Opt-in, isolated-state integration probe. Never launches a game.
Item {
    id: probe
    required property var app
    required property var library
    required property var details
    required property var view
    property int stage: 0
    property int tick: 0
    property int style: 0
    property var samples: []
    property string originalDetailsId: ""
    property var styles: ["wheel", "shelf", "wall", "album"]
    property var toolSections: ["display", "mods", "achievements", "files"]
    property int toolIndex: 0
    function fail(message) { console.error("LUNCHPAIL_COUCH_SMOOTHNESS_FAILED " + message); Qt.exit(2) }
    function capture(suffix) {
        if (!app.screenshotOutput) { console.log("LUNCHPAIL_COUCH_CAPTURE_SKIPPED no output path"); return }
        const path = app.screenshotOutput.replace(/\.png$/, "-" + suffix + ".png")
        const started = view.grabToImage(function(result) {
            if (!result.saveToFile(path)) probe.fail("Could not save " + path)
            else console.log("LUNCHPAIL_COUCH_CAPTURED " + path)
        })
        if (!started) fail("Could not capture " + suffix)
    }
    FrameAnimation {
        running: probe.stage === 1
        onTriggered: probe.samples.push(frameTime * 1000)
    }
    Timer {
        interval: 200; repeat: true; running: true
        onTriggered: {
            if (!probe.library.ready || probe.library.filtering || !probe.view.active) return
            if (probe.stage === 0) {
                if (!probe.view.selectedGameId || !probe.view.browsing.description) return
                probe.originalDetailsId = probe.details.game_id
                probe.capture("wheel")
                probe.stage = 10
                probe.tick = 0
            } else if (probe.stage === 10) {
                // Readback for a screenshot is not part of normal browsing.
                if (++probe.tick < 5) return
                probe.stage = 1
                probe.tick = 0
            } else if (probe.stage === 1) {
                if (probe.details.game_id !== probe.originalDetailsId)
                    return probe.fail("Browsing loaded full game details")
                probe.view.moveShelf(probe.tick < 8 ? 1 : -1)
                if (++probe.tick < 16) return
                const sorted = probe.samples.slice().sort((a, b) => a - b)
                console.log("LUNCHPAIL_COUCH_FRAME_TIMES " + probe.styles[probe.style]
                            + " count=" + sorted.length + " p95_ms=" + sorted[Math.floor(sorted.length * 0.95)]
                            + " max_ms=" + sorted[sorted.length - 1])
                probe.samples = []
                probe.tick = 0
                if (++probe.style < probe.styles.length) {
                    probe.library.save_couch_view_style(probe.styles[probe.style])
                } else {
                    probe.stage = 2
                    probe.library.save_couch_view_style("wheel")
                    probe.view.focusGameById("9697a5eb-e0b4-4f24-8d43-672701414ee7")
                }
            } else if (probe.stage === 2) {
                if (++probe.tick < 5) return
                probe.capture("logo-wheel")
                probe.stage = 20
                probe.tick = 0
            } else if (probe.stage === 20) {
                if (++probe.tick < 3) return
                probe.view.requestDetails()
                probe.stage = 3
                probe.tick = 0
            } else if (probe.stage === 3) {
                if (probe.details.loading || !probe.view.detailsCurrent) return
                if (++probe.tick < 5) return
                if (!probe.view.overlayOpen || probe.app.couchWorkspace)
                    return probe.fail("Details did not open in the native couch page")
                probe.capture("details")
                probe.stage = 4
                probe.tick = 0
            } else if (probe.stage === 4) {
                if (probe.tick === 1) probe.view.handleNavigation("page_right")
                if (++probe.tick < 3) return
                probe.capture("tools")
                probe.stage = 5
                probe.tick = 0
            } else if (probe.stage === 5) {
                if (probe.tick === 0) probe.app.openCouchGameTool(probe.toolSections[probe.toolIndex])
                if (++probe.tick < 5) return
                probe.app.closeCouchGameTool()
                probe.tick = 0
                if (++probe.toolIndex < probe.toolSections.length) return
                console.log("LUNCHPAIL_COUCH_SMOOTHNESS_READY previews=" + probe.view.browsing.description.length
                            + " title=" + probe.details.title)
                Qt.quit()
            }
        }
    }
    Timer { interval: 60000; running: true; onTriggered: probe.fail("Timed out at stage " + probe.stage) }
}
