import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "CouchBackgroundMusic"
    when: windowShown

    Component {
        id: hostComponent
        Item {
            width: 640
            height: 360
            property alias deck: musicDeck
            property alias libraryMock: libraryState
            property alias detailsMock: detailsState

            QtObject {
                id: libraryState
                property bool couch_music_enabled: true
                property int couch_music_volume: 35
            }
            QtObject {
                id: detailsState
                property string game_id: "game-a"
                property string title: "Super Mario Bros."
                property bool soundtrack_available: true
                property url soundtrack_url: "file:///tmp/lunchbox-couch-music-probe.mp3"
                property string soundtrack_title: "Overworld Theme"
                property bool game_running: false
            }
            Lunchbox.CouchBackgroundMusic {
                id: musicDeck
                library: libraryState
                details: detailsState
                selectedGameId: "game-a"
                active: true
                panelColor: "#111823"
                inkColor: "#f4f7fb"
                mutedColor: "#8d99aa"
                accentColor: "#ffb454"
            }
        }
    }

    function test_selection_must_settle_before_audio_is_committed() {
        const host = createTemporaryObject(hostComponent, this)
        verify(host !== null)
        verify(host.deck.presentationAvailable)
        compare(host.deck.committedSource.toString(), "")
        wait(300)
        compare(host.deck.committedSource.toString(), "")
        wait(420)
        compare(host.deck.committedSource.toString(),
                "file:///tmp/lunchbox-couch-music-probe.mp3")
    }

    function test_fast_selection_change_cancels_pending_audio() {
        const host = createTemporaryObject(hostComponent, this)
        verify(host !== null)
        wait(260)
        host.deck.selectedGameId = "game-b"
        compare(host.deck.requestedSource.toString(), "")
        wait(500)
        compare(host.deck.committedSource.toString(), "")
        verify(!host.deck.presentationAvailable)
    }

    function test_disable_and_launch_block_audio_without_losing_identity() {
        const host = createTemporaryObject(hostComponent, this)
        verify(host !== null)
        compare(host.deck.detailsCurrent, true)

        host.libraryMock.couch_music_enabled = false
        compare(host.deck.requestedSource.toString(), "")
        verify(!host.deck.presentationAvailable)

        host.libraryMock.couch_music_enabled = true
        host.detailsMock.game_running = true
        compare(host.deck.requestedSource.toString(), "")
        compare(host.deck.detailsCurrent, true)
    }
}
