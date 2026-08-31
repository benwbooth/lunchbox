import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var libraryModel
    property int currentRow: -1
    property color ink: "#f4f7fb"
    property color muted: "#8e99aa"
    property color accent: "#ffb84d"
    property color accentCool: "#58d6c7"
    property color panel: "#171f2b"
    property color line: "#2a3546"

    signal jumpRequested(int row)

    readonly property var labels: [
        "#", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L",
        "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"
    ]
    readonly property int revision: libraryModel ? libraryModel.alphabet_revision : 0
    readonly property bool navigationAvailable: {
        revision
        return libraryModel && libraryModel.alphabet_navigation_available
    }
    readonly property string activeLabel: {
        revision
        return libraryModel && currentRow >= 0
                ? libraryModel.alphabet_label_for_row(currentRow) : ""
    }

    readonly property int preferredButtonHeight: 18
    implicitWidth: 34
    implicitHeight: 12 + labels.length * preferredButtonHeight
    Accessible.role: Accessible.List
    Accessible.name: "Game title alphabet"

    function targetRow(label) {
        revision
        return libraryModel ? libraryModel.alphabet_target_row(label) : -1
    }

    function activateLabel(label) {
        const row = targetRow(label)
        if (navigationAvailable && row >= 0) {
            jumpRequested(row)
            return true
        }
        return false
    }

    function focusAdjacent(fromIndex, direction) {
        let index = fromIndex
        for (let checked = 0; checked < labels.length; ++checked) {
            index = (index + direction + labels.length) % labels.length
            const button = buttons.itemAt(index)
            if (button && button.actionable) {
                button.forceActiveFocus()
                return
            }
        }
    }

    Rectangle {
        anchors.fill: parent
        radius: 12
        color: root.panel
        opacity: 0.96
        border.color: root.navigationAvailable ? root.line : Qt.darker(root.line, 1.18)
    }

    HoverHandler { id: railHover }
    ToolTip.visible: railHover.hovered && !root.navigationAvailable
    ToolTip.delay: 450
    ToolTip.text: "Sort by Title to use alphabet navigation"

    Column {
        anchors.fill: parent
        anchors.topMargin: 6
        anchors.bottomMargin: 6

        Repeater {
            id: buttons
            model: root.labels

            delegate: Button {
                id: letterButton
                required property int index
                required property string modelData
                readonly property int targetRow: root.targetRow(modelData)
                readonly property bool actionable: root.navigationAvailable && targetRow >= 0
                readonly property bool activeLetter: modelData === root.activeLabel

                width: root.width
                height: (root.height - 12) / root.labels.length
                padding: 0
                text: modelData
                flat: true
                opacity: actionable ? 1 : 0.3
                focusPolicy: Qt.StrongFocus
                Accessible.name: actionable ? "Jump to " + modelData
                                            : "No games beginning with " + modelData
                Accessible.description: !root.navigationAvailable
                                        ? "Sort the library by title to enable this control"
                                        : ""

                onClicked: root.activateLabel(modelData)

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Up) {
                        root.focusAdjacent(index, -1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Down) {
                        root.focusAdjacent(index, 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Home) {
                        root.focusAdjacent(root.labels.length, 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_End) {
                        root.focusAdjacent(0, -1)
                        event.accepted = true
                    }
                }

                ToolTip.visible: hovered && root.navigationAvailable
                ToolTip.delay: 450
                ToolTip.text: actionable ? "Jump to " + modelData
                                         : "No matching titles"

                contentItem: Text {
                    text: letterButton.text
                    color: letterButton.activeLetter ? "#11161d"
                           : letterButton.actionable ? root.ink : root.muted
                    font.pixelSize: Math.max(8, Math.min(11, letterButton.height - 2))
                    font.weight: letterButton.activeLetter ? Font.Black
                                                          : Font.DemiBold
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }

                background: Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width - 6
                    height: Math.max(10, parent.height - 1)
                    radius: Math.min(6, height / 2)
                    color: letterButton.activeLetter ? root.accent
                           : letterButton.activeFocus ? "#304052"
                           : letterButton.hovered && letterButton.actionable ? "#273445"
                           : "transparent"
                    border.width: letterButton.activeFocus ? 1 : 0
                    border.color: root.accentCool
                }
            }
        }
    }
}
