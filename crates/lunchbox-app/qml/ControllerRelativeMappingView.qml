import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: view
    property var routes: []
    property bool editingEnabled: false
    signal editRequested(var assignment)
    readonly property var selected: routeChoice.currentIndex >= 0
        && routeChoice.currentIndex < routes.length ? routes[routeChoice.currentIndex] : null
    readonly property bool buttonRoute: selected !== null && selected.kind === "button"
    readonly property int selectedPlayer: selected ? (selected.player !== undefined ? selected.player : selected.assignment.source_player) : 0
    readonly property int selectedOutput: selected ? (selected.output !== undefined ? selected.output
        : buttonRoute ? selected.assignment.output_button : selected.assignment.output_axis) : -1
    function buttonLabel(index) {
        return ["Left", "Right", "Middle", "Side", "Extra", "Forward", "Back", "Task"][index] || "Unknown"
    }
    onRoutesChanged: routeChoice.currentIndex = routes.length ? 0 : -1

    Label {
        Layout.fillWidth: true
        text: "Relative input: source → destination"
        wrapMode: Text.WordWrap
    }
    ComboBox {
        id: routeChoice
        Layout.fillWidth: true
        model: view.routes.map(route => route.title || "Player " + route.assignment.source_player
            + " · " + route.fieldLabel + " · " + route.assignment.field.tag
            + " / mask " + route.assignment.field.mask + " / default " + route.assignment.field.defvalue)
        Accessible.name: "Saved relative-axis or mouse-button route to display"
    }
    Button {
        text: "Edit selected route…"
        visible: view.editingEnabled
        enabled: view.editingEnabled && view.selected !== null
        Accessible.description: "Load this saved assignment into the relative-input editor without changing the draft."
        onClicked: view.editRequested(view.selected.assignment)
    }
    GridLayout {
        Layout.fillWidth: true
        columns: width >= 560 ? 3 : 1
        Frame {
            Layout.fillWidth: true
            Layout.preferredWidth: 240
            ColumnLayout {
                anchors.fill: parent
                Label { text: view.buttonRoute ? "Source: physical mouse button" : "Source: physical relative axes"; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                Item {
                    visible: !view.buttonRoute
                    Layout.fillWidth: true
                    implicitHeight: 130
                    Rectangle {
                        anchors.centerIn: parent
                        width: 100; height: 2
                        color: horizontalAxis.palette.mid
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        width: 2; height: 100
                        color: horizontalAxis.palette.mid
                    }
                    Label {
                        id: horizontalAxis
                        anchors.centerIn: parent
                        text: "← X →"
                        font.pixelSize: 24
                        font.bold: view.selected !== null && view.selected.physical_axis === 0
                        color: font.bold ? palette.highlight : palette.mid
                        Accessible.name: "Physical X axis" + (font.bold ? ", mapped by selected route" : ", not selected")
                    }
                    Label {
                        anchors.horizontalCenter: parent.horizontalCenter
                        anchors.top: parent.top
                        text: "↑ Y ↓"
                        font.pixelSize: 24
                        font.bold: view.selected !== null && view.selected.physical_axis === 1
                        color: font.bold ? palette.highlight : palette.mid
                        Accessible.name: "Physical Y axis" + (font.bold ? ", mapped by selected route" : ", not selected")
                    }
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 6
                    visible: view.buttonRoute
                    Repeater {
                        model: 8
                        delegate: Rectangle {
                            required property int index
                            readonly property bool mapped: view.buttonRoute && view.selected.physical_button === 0x110 + index
                            width: 58; height: 58; radius: 29
                            color: mapped ? sourceButton.palette.highlight : sourceButton.palette.base
                            border.color: sourceButton.palette.mid
                            border.width: mapped ? 3 : 1
                            Label {
                                id: sourceButton
                                anchors.centerIn: parent
                                text: (parent.index + 1) + "\n" + view.buttonLabel(parent.index)
                                font.bold: parent.mapped
                                color: parent.mapped ? palette.highlightedText : palette.text
                                horizontalAlignment: Text.AlignHCenter
                                Accessible.name: "Physical " + view.buttonLabel(parent.index) + (parent.mapped ? ", selected source" : ", not selected")
                            }
                        }
                    }
                }
                Label {
                    visible: view.buttonRoute
                    Layout.fillWidth: true
                    text: view.buttonRoute ? "Physical code 0x" + view.selected.physical_button.toString(16) : ""
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    text: view.selected ? view.selected.event_path + "\n" + view.selected.input_identity : ""
                    textFormat: Text.PlainText
                    wrapMode: Text.WrapAnywhere
                }
            }
        }
        Label {
            Layout.alignment: Qt.AlignCenter
            text: view.selected ? (view.buttonRoute ? "Button remap" : view.selected.sensitivity_percent + "%"
                + (view.selected.inverted ? "\ninverted" : ""))
                + (parent.columns === 3 ? "\n→" : "\n↓") : ""
            horizontalAlignment: Text.AlignHCenter
            Accessible.name: view.buttonRoute ? "Saved physical-to-output button mapping" : "Saved sensitivity and inversion"
        }
        Frame {
            Layout.fillWidth: true
            Layout.preferredWidth: 240
            ColumnLayout {
                anchors.fill: parent
                Label { text: "Destination: game control"; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                Label {
                    Layout.fillWidth: true
                    text: view.selected ? "Player " + view.selectedPlayer
                        + " · output " + (view.buttonRoute ? "button " + view.selectedOutput
                            : (view.selectedOutput === 0 ? "X" : "Y")) : ""
                    font.bold: true
                    wrapMode: Text.WordWrap
                }
                Flow {
                    Layout.fillWidth: true
                    spacing: 6
                    visible: view.buttonRoute
                    Repeater {
                        model: 5
                        delegate: Rectangle {
                            required property int index
                            readonly property bool mapped: view.buttonRoute && view.selectedOutput === index + 1
                            width: 48; height: 48; radius: 24
                            color: mapped ? outputButton.palette.highlight : outputButton.palette.base
                            border.color: outputButton.palette.mid
                            border.width: mapped ? 3 : 1
                            Label {
                                id: outputButton
                                anchors.centerIn: parent
                                text: parent.index + 1
                                font.bold: parent.mapped
                                color: parent.mapped ? palette.highlightedText : palette.text
                                Accessible.name: "Native mouse button " + (parent.index + 1) + (parent.mapped ? ", selected output" : ", not selected")
                            }
                        }
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: view.selected ? view.selected.details || view.selected.fieldLabel + "\n"
                        + view.selected.assignment.field.tag + " / " + view.selected.assignment.field.input_type
                        + "\nMask " + view.selected.assignment.field.mask
                        + " · default " + view.selected.assignment.field.defvalue : ""
                    textFormat: Text.PlainText
                    wrapMode: Text.WrapAnywhere
                }
            }
        }
    }
    Label {
        Layout.fillWidth: true
        text: "Logical control schematic, not a device silhouette or physical button placement. Bold outlined/highlighted controls identify the selected mapping; other controls do not prove device capabilities. Only explicitly assigned axes and buttons are forwarded; unassigned controls and scroll are discarded. Saved mapping only—no live input verification."
        wrapMode: Text.WordWrap
    }
}
