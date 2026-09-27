import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property var previewModel
    property color ink: "#f4f7fb"
    property color muted: "#8e99aa"
    property color accent: "#ffb84d"
    property color accentCool: "#58d6c7"
    property color panel: "#0b121b"
    property color line: "#2a3546"
    property bool compact: false

    readonly property int revision: previewModel
                                            ? previewModel.launch_profile_preview_revision : 0
    readonly property bool previewValid: {
        revision
        return previewModel && previewModel.launch_profile_preview_valid
    }
    readonly property int argumentCount: {
        revision
        return previewModel ? previewModel.launch_profile_preview_argument_count : 0
    }
    readonly property string runtime: {
        revision
        return previewModel ? previewModel.launch_profile_preview_runtime : ""
    }
    readonly property string message: {
        revision
        return previewModel ? previewModel.launch_profile_preview_message : ""
    }

    implicitHeight: previewColumn.implicitHeight + 22
    Accessible.role: Accessible.Grouping
    Accessible.name: "Direct process command preview"
    Accessible.description: message

    function argumentAt(index) {
        revision
        return previewModel
                ? previewModel.launch_profile_preview_argument_at(index) : ""
    }

    Rectangle {
        anchors.fill: parent
        radius: 9
        color: root.panel
        border.color: root.previewValid ? "#28584f" : "#70404d"
    }

    Column {
        id: previewColumn
        x: 11
        y: 11
        width: parent.width - 22
        spacing: root.compact ? 6 : 8

        Row {
            width: parent.width
            spacing: 8

            Text {
                width: parent.width - previewBadge.width - 8
                text: "DIRECT PROCESS PREVIEW"
                color: root.ink
                font.pixelSize: root.compact ? 8 : 9
                font.weight: Font.Bold
                font.letterSpacing: 0.45
                elide: Text.ElideRight
            }

            Rectangle {
                id: previewBadge
                width: previewBadgeText.implicitWidth + 14
                height: 20
                radius: 10
                color: root.previewValid ? "#17352f" : "#3a2029"
                border.color: root.previewValid ? "#347b6d" : "#70404d"

                Text {
                    id: previewBadgeText
                    anchors.centerIn: parent
                    text: root.previewValid ? "VALID ARGV" : "CHECK"
                    color: root.previewValid ? root.accentCool : "#ff9a9a"
                    font.pixelSize: 7
                    font.weight: Font.Bold
                }
            }
        }

        Text {
            width: parent.width
            text: root.message
            color: root.previewValid ? root.muted : "#ff9a9a"
            font.pixelSize: root.compact ? 8 : 9
            wrapMode: Text.WordWrap
        }

        Rectangle {
            width: parent.width
            height: programText.implicitHeight + 14
            radius: 6
            color: "#0e1722"
            border.color: root.line
            visible: root.previewValid

            Text {
                id: programText
                x: 8
                y: 7
                width: parent.width - 16
                text: "RUNTIME   " + root.runtime
                color: root.accent
                font.family: "monospace"
                font.pixelSize: root.compact ? 8 : 9
                font.weight: Font.DemiBold
                wrapMode: Text.WrapAnywhere
            }
        }

        Flow {
            id: argumentFlow
            width: parent.width
            height: childrenRect.height
            spacing: 5
            visible: root.previewValid && root.argumentCount > 0

            Repeater {
                model: root.argumentCount

                delegate: Rectangle {
                    id: argumentChip
                    required property int index
                    readonly property string argument: root.argumentAt(index)
                    width: Math.min(argumentFlow.width,
                                    argumentText.implicitWidth + argumentIndex.implicitWidth + 25)
                    height: Math.max(argumentIndex.implicitHeight,
                                     argumentText.implicitHeight) + 12
                    radius: 6
                    color: "#141f2c"
                    border.color: root.line
                    Accessible.role: Accessible.StaticText
                    Accessible.name: "Argument " + index + ": " + argument

                    Text {
                        id: argumentIndex
                        x: 7
                        y: 6
                        text: "[" + argumentChip.index + "]"
                        color: root.accentCool
                        font.family: "monospace"
                        font.pixelSize: root.compact ? 7 : 8
                        font.weight: Font.Bold
                    }

                    Text {
                        id: argumentText
                        x: argumentIndex.x + argumentIndex.implicitWidth + 6
                        y: 6
                        width: argumentChip.width - x - 7
                        text: argumentChip.argument.length > 0
                              ? argumentChip.argument : "<empty>"
                        color: root.ink
                        font.family: "monospace"
                        font.pixelSize: root.compact ? 8 : 9
                        wrapMode: Text.WrapAnywhere
                    }
                }
            }
        }

        Text {
            width: parent.width
            visible: root.previewValid
            text: "Each numbered value is passed as one argument. No shell parses this command; the executable and host wrapper resolve when Play starts."
            color: "#68768a"
            font.pixelSize: root.compact ? 7 : 8
            lineHeight: 1.15
            wrapMode: Text.WordWrap
        }
    }
}
