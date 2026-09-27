import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "CouchSharedTools"
    when: windowShown
    Component {
        id: appComponent
        ApplicationWindow {
            id: app
            width: 600; height: 400
            visible: true
            property alias router: navigation
            property alias dialog: toolsDialog
            property alias button: dialogButton
            QtObject {
                // Model-like objects coexist with visual children in Main.
                function data() { return null }
            }
            QtObject {
                id: gamepad
                property int navigation_revision: 0
                property string navigation_action: ""
                property string active_device: ""
                property int connected_count: 1
            }
            Lunchbox.DesktopGamepadNavigation {
                id: navigation
                applicationWindow: app
                gamepad: gamepad
                overlayItem: Overlay.overlay
            }
            Lunchbox.LbDialog {
                id: toolsDialog
                parent: Overlay.overlay
                width: 400; height: 250
                modal: true
                title: "Couch tools"
                contentItem: Lunchbox.LbButton { id: dialogButton; text: "Settings" }
                onOpened: dialogButton.forceActiveFocus()
            }
        }
    }
    function test_shared_dialog_has_focus_scope_and_controller_back() {
        const app = createTemporaryObject(appComponent, this)
        verify(app)
        app.dialog.open()
        tryVerify(function() { return app.router.popupScope !== null })
        verify(app.router.within(app.button, app.router.currentScope()))
        verify(app.router.closeFocusedPopup())
        tryCompare(app.dialog, "visible", false)
    }
    Component {
        id: scaledControl
        Item {
            x: 20; y: 30; width: 100; height: 50
            scale: 2
            transformOrigin: Item.TopLeft
        }
    }
    function test_scaled_couch_controls_use_window_coordinates() {
        const app = createTemporaryObject(appComponent, this)
        const item = createTemporaryObject(scaledControl, app.contentItem)
        const rect = app.router.visibleRect(item)
        verify(rect)
        compare(rect.right - rect.x, 200)
        compare(rect.bottom - rect.y, 100)
    }
}
