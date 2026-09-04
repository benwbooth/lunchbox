import QtQuick
import QtQuick.Controls

Column {
    id: panel

    required property var libraryModel
    property color ink: "#f4f7fb"
    property color muted: "#687488"
    property color line: "#2a3545"
    property color accentCool: "#62d6c6"
    signal filtersChanged()

    width: parent ? parent.width : 320
    spacing: 5

    Row {
        width: parent.width
        height: 28

        Text {
            width: parent.width - clearRegions.width
            anchors.verticalCenter: parent.verticalCenter
            text: "RELEASE REGIONS"
                  + (panel.libraryModel.selected_release_region_count > 0
                     ? "  ·  " + panel.libraryModel.selected_release_region_count + " selected"
                     : "  ·  Any")
            color: panel.muted
            font.pixelSize: 9
            font.weight: Font.Bold
            font.letterSpacing: 1
        }
        Button {
            id: clearRegions
            objectName: "clearReleaseRegions"
            width: 50
            height: 28
            text: "Clear"
            flat: true
            visible: panel.libraryModel.selected_release_region_count > 0
            onClicked: {
                panel.libraryModel.clear_release_region_filters()
                panel.filtersChanged()
            }
        }
    }

    Flow {
        id: regionFlow
        width: parent.width
        height: implicitHeight
        spacing: 4

        Repeater {
            model: panel.libraryModel.release_region_count

            delegate: Rectangle {
                required property int index
                readonly property bool selected: {
                    panel.libraryModel.release_region_revision
                    return panel.libraryModel.release_region_selected_at(index)
                }

                objectName: "releaseRegion_" + index
                width: (regionFlow.width - regionFlow.spacing) / 2
                height: 34
                radius: 7
                color: selected ? "#183530" : (regionHover.hovered ? "#1b2432" : "transparent")
                border.color: selected ? panel.accentCool : panel.line

                function activate() {
                    panel.libraryModel.toggle_release_region(index)
                    panel.filtersChanged()
                }

                HoverHandler { id: regionHover }
                MouseArea {
                    anchors.fill: parent
                    onClicked: parent.activate()
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    width: 15
                    height: 15
                    radius: 4
                    color: parent.selected ? panel.accentCool : "#101721"
                    border.color: parent.selected ? panel.accentCool : "#455166"
                    Text {
                        anchors.centerIn: parent
                        text: parent.parent.selected ? "✓" : ""
                        color: "#0c1716"
                        font.pixelSize: 11
                        font.weight: Font.Black
                    }
                }
                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 29
                    anchors.right: parent.right
                    anchors.rightMargin: 7
                    anchors.verticalCenter: parent.verticalCenter
                    text: panel.libraryModel.release_region_name_at(index)
                    color: panel.ink
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
                ToolTip.visible: regionHover.hovered
                ToolTip.text: panel.libraryModel.release_region_name_at(index)
            }
        }
    }

    Rectangle {
        width: parent.width
        height: 1
        color: panel.line
    }

    Text {
        width: parent.width
        topPadding: 5
        text: "RELEASE CATEGORIES"
        color: panel.muted
        font.pixelSize: 9
        font.weight: Font.Bold
        font.letterSpacing: 1
    }

    Row {
        width: parent.width
        height: 40
        spacing: 8

        Text {
            width: parent.width - adultMode.width - parent.spacing
            anchors.verticalCenter: parent.verticalCenter
            text: "Adult releases"
            color: panel.ink
            font.pixelSize: 11
            font.weight: Font.DemiBold
        }
        ComboBox {
            id: adultMode
            objectName: "adultReleaseMode"
            width: 116
            height: 36
            model: ["Any", "Exclude", "Only"]
            currentIndex: {
                panel.libraryModel.hide_adult
                panel.libraryModel.adult_release_filter
                const mode = panel.libraryModel.release_category_mode("adult")
                return mode === "exclude" ? 1 : mode === "only" ? 2 : 0
            }
            onActivated: function(index) {
                panel.libraryModel.set_release_category_mode(
                            "adult", index === 1 ? "exclude" : index === 2 ? "only" : "any")
                panel.filtersChanged()
            }
        }
    }

    Row {
        width: parent.width
        height: 40
        spacing: 8

        Text {
            width: parent.width - nonRetailMode.width - parent.spacing
            anchors.verticalCenter: parent.verticalCenter
            text: "Homebrew / pirate releases"
            color: panel.ink
            font.pixelSize: 11
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }
        ComboBox {
            id: nonRetailMode
            objectName: "nonRetailReleaseMode"
            width: 116
            height: 36
            model: ["Any", "Exclude", "Only"]
            currentIndex: {
                panel.libraryModel.hide_non_retail
                panel.libraryModel.non_retail_release_filter
                const mode = panel.libraryModel.release_category_mode("non-retail")
                return mode === "exclude" ? 1 : mode === "only" ? 2 : 0
            }
            onActivated: function(index) {
                panel.libraryModel.set_release_category_mode(
                            "non-retail", index === 1 ? "exclude" : index === 2 ? "only" : "any")
                panel.filtersChanged()
            }
        }
    }
}
