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
