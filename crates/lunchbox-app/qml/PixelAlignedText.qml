import QtQuick

// Native glyph bitmaps must land on the window's physical pixel grid, including
// when a centered dialog or a scrolling parent has a fractional device position.
Item {
    id: root
    property alias text: label.text
    property alias font: label.font
    property alias color: label.color
    property alias elide: label.elide
    property alias wrapMode: label.wrapMode
    property alias horizontalAlignment: label.horizontalAlignment
    property real leftPadding: 0
    property real rightPadding: 0
    readonly property real pixelRatio: Window.window ? Window.window.devicePixelRatio : 1
    implicitWidth: label.implicitWidth + leftPadding + rightPadding
    implicitHeight: label.implicitHeight
    FontMetrics { id: metrics; font: label.font }

    function alignToPixels() {
        const dpr = pixelRatio || 1
        const top = (height - label.implicitHeight) / 2
        const origin = mapToItem(null, leftPadding, top + label.baselineOffset)
        label.x = leftPadding + (Math.round(origin.x * dpr) / dpr - origin.x)
        label.y = top + (Math.round(origin.y * dpr) / dpr - origin.y)
    }
    Text {
        id: label
        objectName: "pixelAlignedGlyphs"
        width: Math.max(0, root.width - root.leftPadding - root.rightPadding)
        textFormat: Text.PlainText
        wrapMode: Text.NoWrap
        // Align every wrapped baseline, not just the paragraph's first line.
        lineHeightMode: wrapMode === Text.NoWrap ? Text.ProportionalHeight : Text.FixedHeight
        lineHeight: wrapMode === Text.NoWrap ? 1 : Math.ceil(metrics.lineSpacing * root.pixelRatio) / root.pixelRatio
        elide: Text.ElideRight
        renderType: Text.NativeRendering
    }
    // Ancestor scrolling/centering can move the item without changing its x/y.
    // Recompute before rendering, without rescaling or re-rasterizing the font.
    FrameAnimation { running: root.visible; onTriggered: root.alignToPixels() }
}
