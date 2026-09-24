import QtQuick
import QtQuick.Controls as Controls

Controls.Dialog {
    id: control

    footer: Controls.DialogButtonBox {
        visible: count > 0
        standardButtons: control.standardButtons
        delegate: LbButton {}
        background: Rectangle { color: "transparent" }
    }
}
