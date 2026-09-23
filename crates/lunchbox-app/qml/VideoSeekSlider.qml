import QtQuick
import QtQuick.Templates as T

T.Slider {
    id: seek

    required property var mediaPlayer
    property color trackColor: "#465166"
    property color progressColor: palette.highlight
    property color handleColor: palette.buttonText

    from: 0
    to: mediaPlayer ? Math.max(1, mediaPlayer.duration) : 1
    // Custom 4px tracks and 12px handles have no implicit size. Without a
    // minimum, RowLayout collapses the styled details slider to zero height.
    implicitHeight: 36
    enabled: mediaPlayer && mediaPlayer.seekable && mediaPlayer.duration > 0
    focusPolicy: Qt.StrongFocus
    Accessible.name: "Video position"

    // Draw only the track and thumb. The platform Slider style adds a focus
    // rectangle around the full 36px hit area, obscuring the video controls.
    background: Rectangle {
        x: seek.leftPadding
        y: seek.topPadding + seek.availableHeight / 2 - height / 2
        width: seek.availableWidth
        height: 4
        radius: 2
        color: seek.trackColor
        Rectangle {
            width: seek.visualPosition * parent.width
            height: parent.height
            radius: 2
            color: seek.progressColor
        }
    }

    handle: Rectangle {
        x: seek.leftPadding + seek.visualPosition * (seek.availableWidth - width)
        y: seek.topPadding + seek.availableHeight / 2 - height / 2
        width: 12
        height: 12
        radius: 6
        color: seek.handleColor
    }

    onMoved: {
        if (mediaPlayer && mediaPlayer.seekable)
            mediaPlayer.position = value
    }

    Keys.onPressed: function(event) {
        if (!seek.enabled)
            return
        const step = Math.max(1000, Math.round(mediaPlayer.duration / 20))
        let next = null
        if (event.key === Qt.Key_Left)
            next = mediaPlayer.position - step
        else if (event.key === Qt.Key_Right)
            next = mediaPlayer.position + step
        else if (event.key === Qt.Key_Home)
            next = 0
        else if (event.key === Qt.Key_End)
            next = mediaPlayer.duration
        if (next !== null) {
            mediaPlayer.position = Math.max(0, Math.min(mediaPlayer.duration, next))
            event.accepted = true
        }
    }

    Binding on value {
        when: !seek.pressed
        value: seek.mediaPlayer ? seek.mediaPlayer.position : 0
        // Restoring the pre-binding value on press makes a track click snap
        // back before Slider can commit the seek. Keep the current position
        // until the pointer updates it, then rebind after release.
        restoreMode: Binding.RestoreNone
    }
}
