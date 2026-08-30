pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: settings

    required property var library
    required property color inkColor
    required property color mutedColor
    required property color accentColor
    required property color lineColor
    spacing: 10

    Timer {
        id: volumeCommit
        interval: 220
        repeat: false
        onTriggered: settings.library.save_couch_audio_settings(
                         settings.library.couch_music_enabled,
                         Math.round(volumeSlider.value))
    }

    Text {
        Layout.fillWidth: true
        text: "BACKGROUND MUSIC"
        color: settings.inkColor
        font.pixelSize: 12
        font.weight: Font.Bold
        font.letterSpacing: 0.7
    }
    Text {
        Layout.fillWidth: true
        text: "Couch Mode plays a cached soundtrack only after the selected game remains stable for a moment. Fast browsing stays silent, launching pauses playback, and no new media is downloaded by this setting."
        color: settings.mutedColor
        font.pixelSize: 11
        wrapMode: Text.WordWrap
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Switch {
            text: "Play cached game music automatically"
            checked: settings.library.couch_music_enabled
            enabled: settings.library.ready && !settings.library.couch_state_saving
            onToggled: settings.library.save_couch_audio_settings(
                           checked, settings.library.couch_music_volume)
            Accessible.name: "Play cached game music automatically in Couch Mode"
        }
        Item { Layout.fillWidth: true }
        Text {
            text: settings.library.couch_state_saving
                  ? "SAVING…" : "SAVED AUTOMATICALLY"
            color: settings.library.couch_state_saving
                   ? settings.accentColor : settings.mutedColor
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 0.7
        }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        Text {
            text: "Music volume"
            color: settings.inkColor
            font.pixelSize: 12
        }
        Slider {
            id: volumeSlider
            Layout.fillWidth: true
            from: 0
            to: 100
            stepSize: 1
            value: settings.library.couch_music_volume
            enabled: settings.library.ready
                     && settings.library.couch_music_enabled
                     && !settings.library.couch_state_saving
            onMoved: volumeCommit.restart()
            Accessible.name: "Couch Mode background music volume"
            Accessible.description: Math.round(value) + " percent"
        }
        Text {
            Layout.preferredWidth: 48
            text: Math.round(volumeSlider.value) + "%"
            color: settings.library.couch_music_enabled
                   ? settings.inkColor : settings.mutedColor
            font.pixelSize: 11
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignRight
        }
    }
    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 1
        color: settings.lineColor
    }
}
