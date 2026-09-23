import QtQuick
import QtQuick.Controls

Slider {
    id: seek

    required property var mediaPlayer

    from: 0
    to: mediaPlayer ? Math.max(1, mediaPlayer.duration) : 1
    // Custom 4px tracks and 12px handles have no implicit size. Without a
    // minimum, RowLayout collapses the styled details slider to zero height.
    implicitHeight: 36
    enabled: mediaPlayer && mediaPlayer.seekable && mediaPlayer.duration > 0
    Accessible.name: "Video position"

    onMoved: {
        if (mediaPlayer && mediaPlayer.seekable)
            mediaPlayer.position = value
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
