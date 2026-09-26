import QtQuick

// Qt owns the content item's geometry. Keep the text in a child so that
// control padding/layout updates cannot overwrite its centered geometry.
Item {
    id: content
    required property var control
    property color color: "#f4f7fb"
    property int pixelSize: 13
    property int fontWeight: Font.Medium

    implicitWidth: metrics.advanceWidth(label.text)
    implicitHeight: metrics.height

    FontMetrics { id: metrics; font: label.font }

    Text {
        id: label
        objectName: "buttonLabel"
        // Reserve equal space on both sides, but do not let ordinary action
        // padding consume nearly all of a compact icon button's surface.
        width: Math.max(0, content.control.width - 2 * Math.min(
                            Math.max(content.control.leftPadding, content.control.rightPadding),
                            content.control.width * 0.15))
        height: Math.max(0, content.control.height - 2 * Math.min(
                             Math.max(content.control.topPadding, content.control.bottomPadding),
                             content.control.height * 0.15))
        x: (content.control.width - width) / 2 - content.x
        y: (content.control.height - height) / 2 - content.y
        text: content.control.text
        textFormat: Text.PlainText
        font.family: content.control.font.family
        font.pixelSize: content.pixelSize
        font.weight: content.fontWeight
        font.italic: content.control.font.italic
        font.letterSpacing: 0
        color: content.color
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        fontSizeMode: Text.Fit
        minimumPixelSize: 6
        elide: Text.ElideNone
    }
}
