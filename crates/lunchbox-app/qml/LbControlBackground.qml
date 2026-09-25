import QtQuick

Rectangle {
    property bool pressed: false
    property bool hovered: false
    property bool focused: false
    property bool selected: false
    property bool flat: false
    property bool positive: false

    radius: 7
    color: !enabled ? "#151c27"
           : pressed ? positive ? "#1b5937" : "#34445a"
           : selected ? positive ? "#237a4d" : "#2d3440"
           : hovered ? positive ? "#245e40" : "#293648"
           : flat ? "transparent" : "#202a39"
    border.width: flat && !hovered && !focused && !selected ? 0 : 1
    border.color: focused || selected ? positive ? "#5ee391" : "#ffb454"
                  : hovered ? "#53647c" : "#3a495f"
    opacity: enabled ? 1 : 0.5
}
