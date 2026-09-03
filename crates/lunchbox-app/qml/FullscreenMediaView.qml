pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

// Fullscreen overlay for inspecting one media item at native scale.
// Click the backdrop, the close button, or press Escape to dismiss.
Popup {
    id: viewer

    property url mediaUrl: ""
    property string mediaTitle: ""
    property string mediaSource: ""
    property bool canRotate: false

    signal previousRequested()
    signal nextRequested()

    anchors.centerIn: Overlay.overlay
    width: Overlay.overlay ? Overlay.overlay.width : 0
    height: Overlay.overlay ? Overlay.overlay.height : 0
    modal: true
    closePolicy: Popup.CloseOnEscape
    padding: 0
    parent: Overlay.overlay

    background: Rectangle {
        color: "#e8060a0f"

        MouseArea {
            anchors.fill: parent
            onClicked: viewer.close()
        }
    }

    contentItem: Item {
        implicitWidth: viewer.width
        implicitHeight: viewer.height

        Image {
            id: fullImage
            anchors.fill: parent
            anchors.margins: 56
            source: viewer.mediaUrl
            asynchronous: true
            cache: true
            mipmap: true
            autoTransform: true
            fillMode: Image.PreserveAspectFit
            opacity: status === Image.Ready ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 150 } }

            BusyIndicator {
                anchors.centerIn: parent
                running: fullImage.status === Image.Loading
                visible: running
            }
        }

        Column {
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.topMargin: 16
            spacing: 4

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: viewer.mediaTitle.length > 0
                text: viewer.mediaTitle
                color: "#f4f7fb"
                font.pixelSize: 15
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                width: Math.min(viewer.width - 160, implicitWidth)
                horizontalAlignment: Text.AlignHCenter
            }

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: viewer.mediaSource.length > 0
                text: viewer.mediaSource
                color: "#94a0b3"
                font.pixelSize: 10
            }
        }

        RoundButton {
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 14
            width: 38
            height: 38
            objectName: "closeButton"
            text: "✕"
            font.pixelSize: 15
            Accessible.name: "Close fullscreen media"
            onClicked: viewer.close()
            background: Rectangle {
                radius: 19
                color: parent.down ? "#2a3547" : "#1a2331"
                border.color: "#3b4a61"
            }
            contentItem: Text {
                text: parent.text
                color: "#f4f7fb"
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        RoundButton {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 10
            width: 44
            height: 44
            objectName: "previousButton"
            visible: viewer.canRotate
            text: "‹"
            font.pixelSize: 24
            Accessible.name: "Previous media item"
            onClicked: viewer.previousRequested()
            background: Rectangle {
                radius: 22
                color: parent.down ? "#2a3547" : "#1a2331"
                border.color: "#3b4a61"
            }
            contentItem: Text {
                text: parent.text
                color: "#f4f7fb"
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        RoundButton {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.rightMargin: 10
            width: 44
            height: 44
            objectName: "nextButton"
            visible: viewer.canRotate
            text: "›"
            font.pixelSize: 24
            Accessible.name: "Next media item"
            onClicked: viewer.nextRequested()
            background: Rectangle {
                radius: 22
                color: parent.down ? "#2a3547" : "#1a2331"
                border.color: "#3b4a61"
            }
            contentItem: Text {
                text: parent.text
                color: "#f4f7fb"
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
