import QtQuick

Item {
    id: marquee
    property alias text: label.text
    property alias color: label.color
    property alias font: label.font
    property bool hovered: false
    property int pauseDuration: 650
    property real pixelsPerSecond: 35
    readonly property real overflow: Math.max(0, label.implicitWidth - width)
    readonly property bool scrolling: hovered && visible && overflow > 1
    property real progress: 0
    implicitHeight: label.implicitHeight
    clip: true

    Text {
        id: label
        x: marquee.scrolling ? -marquee.overflow * marquee.progress : 0
        width: marquee.scrolling ? implicitWidth : marquee.width
        elide: marquee.scrolling ? Text.ElideNone : Text.ElideRight
        font.kerning: true
        font.hintingPreference: Font.PreferVerticalHinting
        renderType: Text.NativeRendering
        onTextChanged: marquee.restart()
    }

    function restart() {
        progress = 0
        if (motion.running) motion.restart()
    }
    onOverflowChanged: restart()
    onScrollingChanged: progress = 0

    SequentialAnimation {
        id: motion
        running: marquee.scrolling
        loops: Animation.Infinite
        PauseAnimation { duration: marquee.pauseDuration }
        NumberAnimation {
            target: marquee; property: "progress"; from: 0; to: 1
            duration: Math.max(1, marquee.overflow / Math.max(1, marquee.pixelsPerSecond) * 1000)
            easing.type: Easing.Linear
        }
        PauseAnimation { duration: marquee.pauseDuration }
        NumberAnimation {
            target: marquee; property: "progress"; from: 1; to: 0
            duration: Math.max(1, marquee.overflow / Math.max(1, marquee.pixelsPerSecond) * 1000)
            easing.type: Easing.Linear
        }
    }
}
