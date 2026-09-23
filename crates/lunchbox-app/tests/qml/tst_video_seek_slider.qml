import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "VideoSeekSlider"
    when: windowShown
    visible: true
    width: 480
    height: 240

    QtObject {
        id: player
        property int position: 0
        property int duration: 36000
        property bool seekable: true
    }

    Flickable {
        id: detailsScroll
        width: 400
        height: 150
        contentWidth: width
        contentHeight: 500

        Lunchbox.VideoSeekSlider {
            id: seek
            x: 20
            y: 20
            width: 300
            height: 42
            mediaPlayer: player
        }
    }

    function init() {
        player.seekable = true
        player.position = 0
        detailsScroll.contentY = 0
        tryCompare(seek, "value", 0)
    }

    function test_track_click_seeks_in_flickable() {
        mouseClick(seek, seek.width * 0.75, seek.height / 2)
        verify(player.position > player.duration * 0.6)
        compare(detailsScroll.contentY, 0)
    }

    function test_drag_seeks_without_scrolling_details() {
        mouseDrag(seek, seek.width * 0.1, seek.height / 2,
                  seek.width * 0.7, 0)
        verify(player.position > player.duration * 0.6)
        compare(detailsScroll.contentY, 0)
    }

    function test_playback_updates_timeline() {
        player.position = 9000
        tryCompare(seek, "value", 9000)
    }

    function test_unseekable_media_disables_timeline() {
        player.seekable = false
        verify(!seek.enabled)
    }
}
