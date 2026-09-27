import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "CouchGameBrowser"
    when: windowShown
    visible: true
    width: 1280; height: 720

    Component {
        id: browserComponent
        Lunchbox.CouchGameBrowser {
            width: 1120; height: 540
            background: "#101620"; panel: "#182230"; panelRaised: "#253244"
            ink: "#ffffff"; muted: "#8899aa"; accent: "#ffb454"; accentCool: "#62d9d0"
            cardRadius: 12
            library: ListModel {
                property int media_revision: 0
                function artwork_url(id, kind) { return "" }
                function request_artwork(id, title, platform, kind) {}
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
