import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "PixelAlignedText"
    when: windowShown
    width: 600; height: 150
    Item {
        id: container
        x: 0.35; y: 0.65
        Lunchbox.PixelAlignedText {
            id: text
            width: 240; height: 37
            text: "Select a physical layout"
            font.pointSize: 10
            leftPadding: 8
        }
    }
    function checkAlignment() {
        text.alignToPixels()
        const glyphs = findChild(text, "pixelAlignedGlyphs")
        verify(glyphs !== null)
        compare(glyphs.renderType, Text.NativeRendering)
        const origin = glyphs.mapToItem(null, 0, glyphs.baselineOffset)
        const dpr = text.pixelRatio
        verify(Math.abs(origin.x * dpr - Math.round(origin.x * dpr)) < 0.001)
        verify(Math.abs(origin.y * dpr - Math.round(origin.y * dpr)) < 0.001)
        compare(glyphs.scale, 1)
    }
    function test_fractional_position_and_parent_motion() {
        checkAlignment()
        container.x = 0.8; container.y = 1.15
        checkAlignment()
    }
    function test_wrapped_lines_have_physical_pixel_baseline_spacing() {
        text.wrapMode = Text.WordWrap
        text.elide = Text.ElideNone
        text.text = "Choose a physical layout before recording. No saved bindings were attached to another layout; existing saved calibration stays unchanged unless you save a replacement calibration."
        text.height = Qt.binding(() => text.implicitHeight)
        const glyphs = findChild(text, "pixelAlignedGlyphs")
        tryVerify(() => glyphs.lineCount > 1)
        checkAlignment()
        const spacing = glyphs.lineHeight * text.pixelRatio
        verify(Math.abs(spacing - Math.round(spacing)) < 0.001)
        verify(text.implicitHeight >= glyphs.contentHeight)
        text.width = 180
        checkAlignment()
    }
}
