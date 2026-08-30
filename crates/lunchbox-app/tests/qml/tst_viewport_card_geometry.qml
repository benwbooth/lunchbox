import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "ViewportCardGeometry"

    Lunchbox.ViewportCardGeometry {
        id: geometry
        expanded: true
        baseWidth: 200
        baseHeight: 280
        tileWidth: 216
        tileHeight: 296
        tileViewportX: 400
        tileViewportY: 250
        viewportWidth: 1000
        viewportHeight: 800
        trailingInset: 20
        animated: false
    }

    function fuzzyCompare(actual, expected, message) {
        verify(Math.abs(actual - expected) < 0.01,
               message + ": expected " + expected + ", got " + actual)
    }

    function init() {
        geometry.expanded = true
        geometry.baseWidth = 200
        geometry.baseHeight = 280
        geometry.tileWidth = 216
        geometry.tileHeight = 296
        geometry.tileViewportX = 400
        geometry.tileViewportY = 250
        geometry.viewportWidth = 1000
        geometry.viewportHeight = 800
        geometry.trailingInset = 20
        geometry.margin = 8
    }

    function test_centered_card_keeps_its_natural_position() {
        fuzzyCompare(geometry.expansion, 2,
                     "a roomy viewport should preserve the full hover zoom")
        fuzzyCompare(geometry.viewportX, 308,
                     "centered card should remain centered over its tile")
        fuzzyCompare(geometry.viewportY, 118,
                     "centered card should remain centered over its tile")
    }

    function test_card_moves_inside_every_viewport_edge() {
        geometry.tileViewportX = 0
        geometry.tileViewportY = 0
        fuzzyCompare(geometry.viewportX, 8, "left edge")
        fuzzyCompare(geometry.viewportY, 8, "top edge")

        geometry.tileViewportX = 900
        geometry.tileViewportY = 700
        fuzzyCompare(geometry.viewportX,
                     geometry.viewportWidth - geometry.trailingInset
                     - geometry.margin - geometry.cardWidth,
                     "right edge and scrollbar inset")
        fuzzyCompare(geometry.viewportY,
                     geometry.viewportHeight - geometry.margin
                     - geometry.cardHeight,
                     "bottom edge")
    }

    function test_resting_bottom_rows_stay_inside_their_own_delegates() {
        geometry.expanded = false

        geometry.tileViewportX = 900
        geometry.tileViewportY = 420
        fuzzyCompare(geometry.expansion, 1, "ordinary card expansion")
        fuzzyCompare(geometry.localX, 8,
                     "second-to-bottom card horizontal inset")
        fuzzyCompare(geometry.localY, 8,
                     "second-to-bottom card vertical inset")

        geometry.tileViewportY = 700
        fuzzyCompare(geometry.localX, 8,
                     "bottom card horizontal inset")
        fuzzyCompare(geometry.localY, 8,
                     "bottom card must not move into the row above")
        verify(geometry.viewportY + geometry.cardHeight > geometry.viewportHeight,
               "a partially visible ordinary row should be clipped by the grid, not repositioned")
    }

    function test_two_times_expansion_when_the_viewport_has_room() {
        geometry.baseHeight = 200
        geometry.tileHeight = 216
        fuzzyCompare(geometry.expansion, 2,
                     "a roomy viewport should preserve the full hover zoom")
    }

    function test_small_viewport_uses_largest_fitting_expansion() {
        geometry.viewportWidth = 340
        geometry.viewportHeight = 360
        geometry.trailingInset = 20
        fuzzyCompare(geometry.expansion, 1.22857142857,
                     "the card should shrink only enough to remain wholly visible")
        verify(geometry.viewportX >= geometry.margin)
        verify(geometry.viewportX + geometry.cardWidth
               <= geometry.viewportWidth - geometry.trailingInset - geometry.margin)
        verify(geometry.viewportY >= geometry.margin)
        verify(geometry.viewportY + geometry.cardHeight
               <= geometry.viewportHeight - geometry.margin)
    }
}
