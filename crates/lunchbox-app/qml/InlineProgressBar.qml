import QtQuick

Item {
    id: control

    property real from: 0
    property real to: 1
    property real value: 0
    property bool indeterminate: false
    property color trackColor: "#303b4d"
    property color fillColor: "#66c6b4"
    readonly property real normalizedValue: {
        const span = control.to - control.from
        if (span <= 0)
            return 0
        return Math.max(0, Math.min(1, (control.value - control.from) / span))
    }

    implicitWidth: 120
    implicitHeight: 7
    Accessible.role: Accessible.ProgressBar
    Accessible.name: control.indeterminate
                     ? "Progress in progress"
                     : Math.round(control.normalizedValue * 100) + "% complete"

    Rectangle {
        id: track
        anchors.fill: parent
        radius: Math.min(width, height) / 2
        color: control.trackColor
        clip: true

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: control.indeterminate ? 0 : parent.width * control.normalizedValue
            radius: parent.radius
            color: control.fillColor

            Behavior on width {
                enabled: !control.indeterminate
                NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
            }
        }

        Rectangle {
            id: busySegment
            visible: control.indeterminate
            width: Math.max(24, track.width * 0.28)
            height: track.height
            radius: track.radius
            color: control.fillColor

            SequentialAnimation on x {
                running: control.indeterminate && control.visible
                loops: Animation.Infinite
                NumberAnimation {
                    from: -busySegment.width
                    to: track.width
                    duration: 850
                    easing.type: Easing.InOutQuad
                }
                PauseAnimation { duration: 80 }
            }
        }
    }
}
