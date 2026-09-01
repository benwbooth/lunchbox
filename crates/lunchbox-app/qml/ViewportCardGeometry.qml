import QtQuick

QtObject {
    id: geometry

    required property bool expanded
    required property real baseWidth
    required property real baseHeight
    required property real tileWidth
    required property real tileHeight
    required property real tileViewportX
    required property real tileViewportY
    required property real viewportWidth
    required property real viewportHeight

    property real margin: 8
    property real trailingInset: 0
    property bool animated: true
    readonly property real availableWidth: Math.max(
                                                0,
                                                viewportWidth - trailingInset
                                                - 2 * margin)
    readonly property real availableHeight: Math.max(0,
                                                     viewportHeight - 2 * margin)
    readonly property real maximumExpansion: Math.max(
                                                 1,
                                                 Math.min(2,
                                                          availableWidth / Math.max(1, baseWidth),
                                                          availableHeight / Math.max(1, baseHeight)))
    readonly property real restingViewportX: tileViewportX
                                                + (tileWidth - baseWidth) / 2
    readonly property real restingViewportY: tileViewportY
                                                + (tileHeight - baseHeight) / 2
    readonly property real maximumCardWidth: baseWidth * maximumExpansion
    readonly property real maximumCardHeight: baseHeight * maximumExpansion
    readonly property real maximumCenteredViewportX: tileViewportX
                                                       + (tileWidth - maximumCardWidth) / 2
    readonly property real maximumCenteredViewportY: tileViewportY
                                                       + (tileHeight - maximumCardHeight) / 2
    readonly property real maximumViewportX: clampToViewport(
                                                  maximumCenteredViewportX,
                                                  maximumCardWidth,
                                                  viewportWidth,
                                                  trailingInset)
    readonly property real maximumViewportY: clampToViewport(
                                                  maximumCenteredViewportY,
                                                  maximumCardHeight,
                                                  viewportHeight, 0)
    // The hover target must not follow the animated card. At a viewport edge,
    // inward repositioning can otherwise move the card out from under a
    // stationary pointer; collapse moves it back and produces an arm/disarm
    // loop. This stable region is the union of the resting and fully expanded
    // card rectangles.
    readonly property real hoverViewportX: Math.min(restingViewportX,
                                                     maximumViewportX)
    readonly property real hoverViewportY: Math.min(restingViewportY,
                                                     maximumViewportY)
    readonly property real hoverViewportRight: Math.max(
                                                   restingViewportX + baseWidth,
                                                   maximumViewportX + maximumCardWidth)
    readonly property real hoverViewportBottom: Math.max(
                                                    restingViewportY + baseHeight,
                                                    maximumViewportY + maximumCardHeight)
    readonly property real hoverWidth: hoverViewportRight - hoverViewportX
    readonly property real hoverHeight: hoverViewportBottom - hoverViewportY
    readonly property real hoverLocalX: hoverViewportX - tileViewportX
    readonly property real hoverLocalY: hoverViewportY - tileViewportY
    property real expansion: expanded ? maximumExpansion : 1
    readonly property real cardWidth: baseWidth * expansion
    readonly property real cardHeight: baseHeight * expansion
    readonly property real centeredViewportX: tileViewportX
                                                + (tileWidth - cardWidth) / 2
    readonly property real centeredViewportY: tileViewportY
                                                + (tileHeight - cardHeight) / 2
    // A resting card belongs to its delegate and must never be shifted into a
    // neighboring row merely because that row is partly outside the viewport.
    // Keep clamping active through the collapse animation, then return to the
    // delegate-local inset once the card reaches its ordinary size.
    readonly property bool needsViewportClamp: expansion > 1.001
    readonly property real viewportX: needsViewportClamp
                                      ? clampToViewport(centeredViewportX,
                                                        cardWidth,
                                                        viewportWidth,
                                                        trailingInset)
                                      : centeredViewportX
    readonly property real viewportY: needsViewportClamp
                                      ? clampToViewport(centeredViewportY,
                                                        cardHeight,
                                                        viewportHeight, 0)
                                      : centeredViewportY
    readonly property real localX: viewportX - tileViewportX
    readonly property real localY: viewportY - tileViewportY

    function clampToViewport(centeredPosition, extent, viewportExtent,
                             endInset) {
        const minimum = margin
        const maximum = viewportExtent - endInset - margin - extent
        if (maximum < minimum)
            return minimum
        return Math.max(minimum, Math.min(maximum, centeredPosition))
    }

    Behavior on expansion {
        enabled: geometry.animated
        NumberAnimation { duration: 150; easing.type: Easing.OutCubic }
    }
}
