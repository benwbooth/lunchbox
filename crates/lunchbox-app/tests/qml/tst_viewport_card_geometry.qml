import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ViewportCardGeometry"
    when: windowShown
    visible: true
    width: 1024
    height: 800
    property bool staticHoverActive: false
    readonly property bool dynamicHover: dynamicHandler.hovered
                                         && dynamicGeometry.hoverContainsViewportPoint(
                                             dynamicHandler.point.scenePosition.x,
                                             dynamicHandler.point.scenePosition.y)

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

    Item {
        id: hoverCapture
        visible: testCase.staticHoverActive
        x: geometry.hoverViewportX
        y: geometry.hoverViewportY
        width: geometry.hoverWidth
        height: geometry.hoverHeight
        HoverHandler { id: hoverHandler }
    }
    readonly property bool effectiveHover: hoverHandler.hovered
                                           && geometry.hoverContainsViewportPoint(
                                               hoverHandler.point.scenePosition.x,
                                               hoverHandler.point.scenePosition.y)

    Lunchbox.ViewportCardGeometry {
        id: dynamicGeometry
        expanded: testCase.dynamicHover
        baseWidth: 200
        baseHeight: 280
        tileWidth: 216
        tileHeight: 296
        tileViewportX: 900
        tileViewportY: 700
        viewportWidth: 1000
        viewportHeight: 800
        trailingInset: 20
    }
    Item {
        visible: !testCase.staticHoverActive
        x: dynamicGeometry.tileViewportX
        y: dynamicGeometry.tileViewportY
        Item {
            id: dynamicCapture
            x: dynamicGeometry.expanded ? dynamicGeometry.hoverLocalX
                                        : dynamicGeometry.localX
            y: dynamicGeometry.expanded ? dynamicGeometry.hoverLocalY
                                        : dynamicGeometry.localY
            width: dynamicGeometry.expanded ? dynamicGeometry.hoverWidth
                                            : dynamicGeometry.baseWidth
            height: dynamicGeometry.expanded ? dynamicGeometry.hoverHeight
                                             : dynamicGeometry.baseHeight
            HoverHandler { id: dynamicHandler }
        }
    }

    function fuzzyCompare(actual, expected, message) {
        verify(Math.abs(actual - expected) < 0.01,
               message + ": expected " + expected + ", got " + actual)
    }

    function init() {
        staticHoverActive = false
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

    function test_bottom_edge_hover_region_covers_resting_and_expanded_cards() {
        geometry.tileViewportX = 400
        geometry.tileViewportY = 700

        verify(geometry.hoverViewportY <= geometry.maximumViewportY,
               "hover region must start above the inward-shifted expanded card")
        verify(geometry.hoverViewportBottom
               >= geometry.restingViewportY + geometry.baseHeight,
               "hover region must retain the resting card's bottom edge")
        verify(geometry.hoverViewportY <= geometry.viewportHeight - geometry.margin)
        verify(geometry.hoverViewportBottom >= geometry.viewportHeight,
               "a pointer at the clipped bottom row remains inside the stable union")
    }

    function test_diagonal_edge_hover_excludes_empty_bounding_box_corners() {
        geometry.tileViewportX = 900
        geometry.tileViewportY = 700

        verify(geometry.hoverContainsViewportPoint(930, 740),
               "the original card still accepts hover during repositioning")
        verify(geometry.hoverContainsViewportPoint(600, 260),
               "the expanded card accepts hover after repositioning")
        verify(!geometry.hoverContainsViewportPoint(600, 900),
               "empty space below the expanded card must release hover")
        verify(!geometry.hoverContainsViewportPoint(1000, 260),
               "empty space beside the expanded card must not steal hover")
    }

    function test_hover_handler_releases_pointer_in_empty_corner() {
        staticHoverActive = true
        geometry.tileViewportX = 900
        geometry.tileViewportY = 700

        mouseMove(testCase, 600, 260)
        tryCompare(testCase, "effectiveHover", true)
        mouseMove(testCase, 990, 260)
        tryCompare(testCase, "effectiveHover", false)
        mouseMove(testCase, 920, 740)
        tryCompare(testCase, "effectiveHover", true)
    }

    function test_hover_expands_then_collapses_without_rearming_in_empty_space() {
        mouseMove(testCase, 500, 100)
        tryCompare(testCase, "dynamicHover", false)
        mouseMove(testCase, 930, 740)
        tryCompare(testCase, "dynamicHover", true)
        tryCompare(dynamicGeometry, "expansion", 2)
        mouseMove(testCase, 990, 260)
        tryCompare(testCase, "dynamicHover", false)
        tryCompare(dynamicGeometry, "expansion", 1)
        wait(200)
        compare(testCase.dynamicHover, false)
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
