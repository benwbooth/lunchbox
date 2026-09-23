import QtQuick
import QtQuick.Layouts
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

    RowLayout {
        id: styledControls
        y: 170
        width: 400
        height: 42

        Lunchbox.VideoSeekSlider {
            id: styledSeek
            Layout.fillWidth: true
            mediaPlayer: player
            background: Rectangle { height: 4 }
            handle: Rectangle { width: 12; height: 12 }
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

    function test_keyboard_seek_remains_available() {
        verify(seek.activeFocusOnTab)
        seek.forceActiveFocus()
        keyClick(Qt.Key_Right)
        verify(player.position > 0)
    }

    function test_styled_slider_has_usable_pointer_height() {
        verify(styledSeek.height >= 32,
               "The styled detail timeline must retain a usable hit area; height="
               + styledSeek.height)
        const point = styledSeek.mapToItem(styledControls,
                                           styledSeek.width * 0.75,
                                           styledSeek.height / 2)
        mouseClick(styledControls, point.x, point.y)
        verify(player.position > player.duration * 0.6)
        player.position = 0
        const start = styledSeek.mapToItem(styledControls,
                                           styledSeek.width * 0.1,
                                           styledSeek.height / 2)
        mouseDrag(styledControls, start.x, start.y,
                  styledSeek.width * 0.7, 0)
        verify(player.position > player.duration * 0.6)
    }
}
