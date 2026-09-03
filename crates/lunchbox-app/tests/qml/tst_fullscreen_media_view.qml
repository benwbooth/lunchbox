import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "FullscreenMediaView"
    when: windowShown
    width: 640
    height: 480

    Component {
        id: viewerComponent
        Lunchbox.FullscreenMediaView {
            mediaUrl: "file:///nonexistent-media.png"
            mediaTitle: "Quik Majik Adventure"
            mediaSource: "eXoDOS"
            canRotate: true
        }
    }

    function test_shows_title_source_and_rotation_controls() {
        const viewer = createTemporaryObject(viewerComponent, testCase)
        verify(viewer)
        viewer.open()
        verify(viewer.opened)
        compare(viewer.mediaTitle, "Quik Majik Adventure")
        compare(viewer.mediaSource, "eXoDOS")
        verify(viewer.canRotate)
    }

    function test_rotation_controls_emit_signals() {
        const viewer = createTemporaryObject(viewerComponent, testCase)
        viewer.open()
        let previous = 0
        let next = 0
        viewer.previousRequested.connect(function() { previous++ })
        viewer.nextRequested.connect(function() { next++ })
        const previousButton = findChild(viewer, "previousButton")
        const nextButton = findChild(viewer, "nextButton")
        verify(previousButton)
        verify(nextButton)
        compare(previousButton.Accessible.name, "Previous media item")
        previousButton.clicked()
        nextButton.clicked()
        compare(previous, 1)
        compare(next, 1)
    }

    function test_close_button_dismisses() {
        const viewer = createTemporaryObject(viewerComponent, testCase)
        viewer.open()
        verify(viewer.opened)
        const close = findChild(viewer, "closeButton")
        verify(close)
        close.clicked()
        verify(!viewer.opened)
    }

    function test_hides_rotation_controls_for_single_item() {
        const viewer = createTemporaryObject(viewerComponent, testCase, {
            canRotate: false
        })
        viewer.open()
        const previousButton = findChild(viewer, "previousButton")
        verify(previousButton)
        verify(!previousButton.visible)
    }
}
