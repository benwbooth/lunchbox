import QtQuick
import QtQuick.Controls as C
import QtQuick.Templates as T

T.ComboBox {
    id: control

    leftPadding: 11
    rightPadding: 30
    topPadding: 6
    bottomPadding: 6
    implicitWidth: Math.max(82, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(34, implicitContentHeight + topPadding + bottomPadding)
    font.family: Qt.application.font.family
    font.pixelSize: 13

    background: LbControlBackground {
        pressed: control.down
        hovered: control.hovered
        focused: control.visualFocus
        selected: control.popup.visible
        enabled: control.enabled
    }

    contentItem: Text {
        anchors.fill: parent
        anchors.leftMargin: control.leftPadding
        anchors.rightMargin: control.rightPadding
        anchors.topMargin: control.topPadding
        anchors.bottomMargin: control.bottomPadding
        text: control.displayText
        font.family: control.font.family
        font.weight: Font.Medium
        font.italic: control.font.italic
        font.letterSpacing: 0
        font.pixelSize: 13
        color: control.enabled ? "#f4f7fb" : "#8d99aa"
        verticalAlignment: Text.AlignVCenter
        fontSizeMode: Text.HorizontalFit
        minimumPixelSize: 8
        elide: Text.ElideNone
    }

    indicator: Canvas {
        x: control.width - width - 11
        y: (control.height - height) / 2
        width: 12
        height: 8
        opacity: control.enabled ? 1 : 0.5
        onPaint: {
            const context = getContext("2d")
            context.clearRect(0, 0, width, height)
            context.strokeStyle = "#c0c8d4"
            context.lineWidth = 1.7
            context.lineCap = "round"
            context.lineJoin = "round"
            context.beginPath()
            context.moveTo(1, 2)
            context.lineTo(width / 2, height - 2)
            context.lineTo(width - 1, 2)
            context.stroke()
        }
    }

    delegate: LbItemDelegate {
        required property int index
        width: control.popup.width - control.popup.leftPadding - control.popup.rightPadding
        text: control.textAt(index)
        highlighted: control.highlightedIndex === index
    }

    popup: T.Popup {
        // A ComboBox inside a modal Dialog otherwise leaves its popup beneath
        // the dialog surface in the live window, even while popup.visible is true.
        parent: C.Overlay.overlay
        // KDE's modal Dialog sits at z=300. The menu must be above that
        // overlay, not merely above the ComboBox inside the dialog.
        z: 10000
        x: 0
        y: 0
        // mapToItem() does not bind to movement of every ancestor of the
        // ComboBox. Calculate the overlay position when the menu opens, after
        // its containing dialog has settled but before the menu is painted.
        onAboutToShow: {
            const origin = control.mapToItem(parent, 0, control.height + 2)
            x = origin.x
            y = origin.y
        }
        width: control.width
        implicitHeight: Math.min(360, contentItem.implicitHeight + topPadding + bottomPadding)
        padding: 4
        background: Rectangle {
            radius: 8
            color: "#151d29"
            border.color: "#53647c"
        }
        contentItem: ListView {
            id: popupList
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
            MouseArea {
                anchors.fill: parent
                z: 10
                preventStealing: false
                onWheel: function(wheel) { wheel.accepted = false }
                onClicked: function(mouse) {
                    const index = popupList.indexAt(mouse.x,
                                                    mouse.y + popupList.contentY)
                    if (index < 0)
                        return
                    control.currentIndex = index
                    control.activated(index)
                    control.popup.close()
                }
            }
        }
    }
}
