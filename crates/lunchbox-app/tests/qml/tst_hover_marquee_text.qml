import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "HoverMarqueeText"
    when: windowShown
    visible: true
    width: 500; height: 100
    Lunchbox.HoverMarqueeText {
        id: title
        width: 120
        font.pixelSize: 16
        pauseDuration: 20
        pixelsPerSecond: 500
    }
    function init() {
        title.hovered = false
        title.visible = true
        title.width = 120
        title.text = "A very long game title with a regional edition suffix"
    }
    function test_short_title_does_not_move() {
        title.text = "Short"
        title.hovered = true
        verify(!title.scrolling)
        compare(title.progress, 0)
    }
    function test_hover_scrolls_long_title_and_exit_resets() {
        verify(title.overflow > 0)
        verify(!title.scrolling)
        title.hovered = true
        verify(title.scrolling)
        tryVerify(() => title.progress > 0, 1000)
        title.hovered = false
        compare(title.progress, 0)
        verify(!title.scrolling)
    }
    function test_reused_title_and_resize_reset_motion() {
        title.hovered = true
        tryVerify(() => title.progress > 0, 1000)
        title.text = "Short"
        verify(!title.scrolling)
        compare(title.progress, 0)
        title.text = "Another very long title that needs more room"
        tryVerify(() => title.progress > 0, 1000)
        title.width = 1000
        verify(!title.scrolling)
        compare(title.progress, 0)
    }
    function test_hidden_title_stops_animation() {
        title.hovered = true
        tryVerify(() => title.progress > 0, 1000)
        title.visible = false
        verify(!title.scrolling)
        compare(title.progress, 0)
    }
}
