import QtQuick

QtObject {

// Presentation coordinates only. Calibration IDs and solver geometry stay intact.
readonly property var controls: [
    {id:"l", x:188, y:125, label:"L", kind:"shoulder"},
    {id:"r", x:490, y:125, label:"R", kind:"shoulder"},
    {id:"stick_up", x:190, y:160, label:"", kind:"direction"},
    {id:"stick_down", x:190, y:240, label:"", kind:"direction"},
    {id:"stick_left", x:150, y:200, label:"", kind:"direction"},
    {id:"stick_right", x:230, y:200, label:"", kind:"direction"},
    {id:"up", x:275, y:270, label:"", kind:"direction"},
    {id:"down", x:275, y:310, label:"", kind:"direction"},
    {id:"left", x:255, y:290, label:"", kind:"direction"},
    {id:"right", x:295, y:290, label:"", kind:"direction"},
    {id:"b", x:458, y:250, label:"B", kind:"face"},
    {id:"a", x:502, y:291, label:"A", kind:"face"},
    {id:"c_left", x:500, y:193, label:"←", kind:"c"},
    {id:"c_up", x:542, y:159, label:"↑", kind:"c"},
    {id:"c_down", x:542, y:227, label:"↓", kind:"c"},
    {id:"c_right", x:584, y:193, label:"→", kind:"c"},
    {id:"start", x:353, y:215, label:"START", kind:"menu"},
    {id:"select", x:313, y:162, label:"−", kind:"menu"},
    {id:"home", x:393, y:162, label:"⌂", kind:"menu"},
    {id:"z", x:736, y:224, label:"Z LEFT", kind:"rear"},
    {id:"z_right", x:822, y:224, label:"Z RIGHT", kind:"rear"}
]
function point(id) { return controls.find(control => control.id === id) || null }

}
