import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "CouchGameBrowser"
    when: windowShown
    visible: true
    width: 1280; height: 720

    Component {
        id: browserComponent
        Lunchpail.CouchGameBrowser {
            width: 1120; height: 540
            background: "#101620"; panel: "#182230"; panelRaised: "#253244"
            ink: "#ffffff"; muted: "#8899aa"; accent: "#ffb454"; accentCool: "#62d9d0"
            cardRadius: 12
            library: ListModel {
                property int media_revision: 0
                property int favorite_revision: 0
                property int favorite_pending_count: 0
                property var favorites: ({})
                function is_favorite(id) { return favorites[id] === true }
                function favorite_pending(id) { return false }
                function set_favorite(id, favorite) {
                    favorites[id] = favorite
                    favorite_revision++
                }
                function artwork_url(id, kind) { return "" }
                function exact_artwork_url(id, kind) { return "" }
                function request_artwork(id, title, platform, kind) {}
                function request_artwork_for_game(id, kind) {}
                property var visibleArtwork: ({})
                function set_visible_artwork_games(view, ids, kind) {
                    visibleArtwork[view] = JSON.parse(ids)
                }
                Component.onCompleted: {
                    for (let i = 0; i < 36; ++i)
                        append({ gameId: "game-" + i, gameTitle: "Game " + i,
                            gameCanonicalTitle: "Game " + i, gamePlatform: "Arcade",
                            gameLocal: true, gameDownloadable: false,
                            gameDatabaseId: i + 1, gameMediaId: i + 1,
                            gameStatus: "Installed" })
                }
            }
        }
    }
    function test_all_views_preserve_identity() {
        const browser = createTemporaryObject(browserComponent, this)
        verify(browser)
        tryCompare(browser, "count", 36)
        browser.currentIndex = 17
        for (const style of ["wheel", "shelf", "wall", "album", "wheel"]) {
            browser.viewStyle = style
            browser.positionViewAtIndex(17, ListView.Center)
            tryVerify(function() { return browser.currentItem !== null }, 1500,
                      style + " has no current item at " + browser.currentIndex)
            tryCompare(browser, "currentIndex", 17)
            tryVerify(function() { return browser.currentItem.gameId === "game-17" },
                      1500, style + " selected " + browser.currentItem.gameId)
        }
    }
    function test_wall_rows_and_final_item() {
        const browser = createTemporaryObject(browserComponent, this, { viewStyle: "wall" })
        tryCompare(browser, "count", 36)
        verify(browser.columns >= 3)
        browser.currentIndex = browser.columns
        browser.positionViewAtIndex(browser.currentIndex, ListView.Contain)
        tryVerify(function() { return browser.currentItem && browser.currentItem.gameId === "game-" + browser.columns })
        browser.currentIndex = 35
        browser.positionViewAtEnd()
        tryVerify(function() { return browser.currentItem && browser.currentItem.gameId === "game-35" })
    }
    function test_all_views_report_visible_media_and_clear_old_view() {
        const browser = createTemporaryObject(browserComponent, this)
        tryCompare(browser, "count", 36)
        browser.currentIndex = 17
        for (const style of ["wheel", "shelf", "wall", "album"]) {
            browser.viewStyle = style
            browser.positionViewAtIndex(17, ListView.Center)
            const key = style === "shelf" ? "couch-shelf"
                        : style === "wall" ? "couch-wall" : "couch-path"
            tryVerify(() => (browser.library.visibleArtwork[key] || []).includes("game-17"),
                      1500, style + " should prioritize its displayed selection")
            for (const other of ["couch-shelf", "couch-wall", "couch-path"]) {
                if (other !== key)
                    compare((browser.library.visibleArtwork[other] || []).length, 0,
                            "Destroyed " + other + " must release its priority")
            }
        }
    }
    SignalSpy { id: activation; signalName: "cardActivated" }
    function test_wall_wheel_keeps_gliding_and_reserves_slim_scrollbar() {
        const browser = createTemporaryObject(browserComponent, this, { viewStyle: "wall" })
        tryCompare(browser, "count", 36)
        const grid = findChild(browser, "couchWallGrid")
        verify(grid)
        verify(grid.defaultWheelMomentum)
        const momentum = findChild(grid, "momentumWheelHandler")
        verify(momentum && momentum.enabled)
        verify(grid.verticalScrollBarGutter <= 14)
        verify(grid.cellWidth * grid.columnCount <= grid.width - grid.verticalScrollBarGutter + 0.01)
        mouseWheel(grid, grid.width / 2, grid.height / 2, 0, -120)
        const immediate = grid.contentY
        verify(immediate > 0, "Wheel input should move immediately")
        verify(momentum.momentumRunning, "Wheel must start momentum; position=" + immediate)
        wait(120)
        verify(grid.contentY > immediate + 10, "Cover wall should keep gliding after the wheel stops; immediate=" + immediate + ", now=" + grid.contentY + ", running=" + momentum.momentumRunning)
    }
    function test_wall_star_toggles_only_favorite() {
        const browser = createTemporaryObject(browserComponent, this, { viewStyle: "wall" })
        tryCompare(browser, "count", 36)
        browser.currentIndex = 1
        tryVerify(function() { return browser.currentItem !== null })
        activation.target = browser
        activation.clear()
        const star = findChild(browser.currentItem, "coverFavoriteButton")
        verify(star)
        verify(star.visible)
        compare(star.favorite, false)
        mouseClick(star)
        tryCompare(star, "favorite", true)
        compare(browser.library.is_favorite("game-1"), true)
        compare(activation.count, 0, "Starring a cover must not open the game")
        mouseClick(star)
        tryCompare(star, "favorite", false)
        compare(activation.count, 0)
        activation.target = null
    }
    function test_small_model_and_empty_library() {
        const browser = createTemporaryObject(browserComponent, this)
        tryCompare(browser, "count", 36)
        browser.library.clear()
        tryCompare(browser, "count", 0)
        for (const style of ["wall", "album", "shelf", "wheel"]) {
            browser.viewStyle = style
            wait(30)
            compare(browser.count, 0)
        }
        browser.library.append({ gameId: "only", gameTitle: "Only game",
            gameCanonicalTitle: "Only game", gamePlatform: "NES", gameLocal: true,
            gameDownloadable: false, gameDatabaseId: 1, gameMediaId: 1, gameStatus: "Installed" })
        for (const style of ["wheel", "album", "wall", "shelf"]) {
            browser.viewStyle = style
            tryCompare(browser, "count", 1)
            tryVerify(function() { return browser.currentItem && browser.currentItem.gameId === "only" })
        }
    }
}
