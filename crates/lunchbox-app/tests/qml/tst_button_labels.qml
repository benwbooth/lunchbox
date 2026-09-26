import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ButtonLabels"
    when: windowShown
    visible: true
    width: 640
    height: 240

    Component { id: action; Lunchbox.LbButton {} }
    Component { id: tool; Lunchbox.LbToolButton {} }
    Component { id: round; Lunchbox.LbRoundButton {} }
    Component { id: tab; Lunchbox.LbTabButton {} }
    Component { id: pane; Lunchbox.PaneButton {} }
    Component { id: header; Lunchbox.HeaderButton {} }
    Component {
        id: details
        Flickable {
            property bool loading: false
            property real headerHeight: 90
            width: 440; height: 200; contentHeight: column.height; clip: true
            Column {
                id: column
                width: parent.width
                Rectangle { width: parent.width; height: headerHeight; color: "#131923" }
                Column {
                    visible: !loading
                    width: parent.width
                    Lunchbox.LbButton {
                        objectName: "detailsAction"
                        width: parent.width; height: 48; text: "Play"
                    }
                }
                Item { width: 440; height: 800 }
            }
        }
    }

    function test_centered_data() {
        return [
            { tag: "action", component: action },
            { tag: "tool", component: tool },
            { tag: "round", component: round },
            { tag: "tab", component: tab },
            { tag: "pane", component: pane },
            { tag: "header", component: header }
        ]
    }

    function verifyCentered(button) {
        const label = findChild(button, "buttonLabel")
        verify(label)
        const center = label.mapToItem(button, label.width / 2, label.height / 2)
        fuzzyCompare(center.x, button.width / 2, 1, "Horizontal label center")
        fuzzyCompare(center.y, button.height / 2, 1, "Vertical label center")
        compare(label.horizontalAlignment, Text.AlignHCenter)
        compare(label.verticalAlignment, Text.AlignVCenter)
        compare(label.elide, Text.ElideNone)
    }

    function test_centered(data) {
        const button = createTemporaryObject(data.component, testCase, {
            x: 20, y: 20, width: 180, height: 48, text: "Controller setup"
        })
        verify(button)
        const label = findChild(button, "buttonLabel")
        verify(label)
        label.renderType = Text.NativeRendering
        waitForRendering(button)
        verifyCentered(button)
        // Callers can reserve unequal padding. That must not shift action labels.
        button.leftPadding = 22
        button.rightPadding = 2
        button.topPadding = 12
        button.bottomPadding = 2
        waitForRendering(button)
        verifyCentered(button)
        for (let width of [100, 280, 160]) {
            button.width = width
            button.height = width === 100 ? 36 : 48
            button.text = width === 100 ? "A long action label" : "Play"
            button.checked = !button.checked
            button.forceActiveFocus()
            mousePress(button)
            waitForRendering(button)
            verifyCentered(button)
            mouseRelease(button)
            verifyCentered(button)
        }
        for (let pass = 0; pass < 5; ++pass) {
            button.visible = false
            button.y = 170.35
            button.height = 0
            wait(30)
            button.text = pass % 2 === 0 ? "Play" : "View 3 sessions"
            button.y = 20.15
            button.height = 48
            button.visible = true
            waitForRendering(button)
            verifyCentered(button)
        }
    }

    function test_compact_labels_data() {
        return [
            { tag: "icon", width: 28, height: 28, text: "+", fullSize: true },
            { tag: "short action", width: 52, height: 28, text: "Close", fullSize: true },
            { tag: "long action", width: 145, height: 32,
              text: "Save player 1 mapping for system", fullSize: false }
        ]
    }

    function test_compact_labels(data) {
        const button = createTemporaryObject(action, testCase, {
            x: 20, y: 20, width: data.width, height: data.height, text: data.text
        })
        waitForRendering(button)
        const label = findChild(button, "buttonLabel")
        verifyCentered(button)
        verify(label.contentWidth <= label.width,
               "Visible label must fit inside the centered text area")
        verify(label.contentHeight <= label.height)
        verify(label.fontInfo.pixelSize <= 13, "Fitting must never enlarge a label")
        if (data.fullSize)
            compare(label.fontInfo.pixelSize, 13)
    }

    function test_painted_label_center_data() {
        return [
            { tag: "normal", text: "Controller setup", width: 180, height: 48 },
            { tag: "small", text: "Close", width: 52, height: 28 },
            { tag: "fitted", text: "Save player 1 mapping for system", width: 145, height: 32 },
            { tag: "play", text: "Play", width: 280, height: 48 }
        ]
    }

    function test_painted_label_center(data) {
        const button = createTemporaryObject(action, testCase, {
            x: 20, y: 20, width: data.width, height: data.height, text: data.text
        })
        const label = findChild(button, "buttonLabel")
        label.renderType = Text.NativeRendering
        waitForRendering(button)
        verifyPaintedCenter(button)
    }

    function verifyPaintedCenter(button) {
        // QuickTest's image pixel coordinates differ from its reported size
        // at fractional DPR. Check ink at 1x; the transform/position checks
        // above still exercise every scroll/reload at fractional scaling.
        if (testCase.Window.window.devicePixelRatio !== 1)
            return
        // Capture the containing scene: grabImage on a nested Flickable child
        // can read the wrong rectangle after its parent moves.
        const pixels = grabImage(testCase)
        const origin = button.mapToItem(testCase, 0, 0)
        const sx = pixels.width / testCase.width
        const sy = pixels.height / testCase.height
        const x0 = Math.round(origin.x * sx)
        const y0 = Math.round(origin.y * sy)
        const width = Math.round(button.width * sx)
        const height = Math.round(button.height * sy)
        let left = pixels.width, right = -1, top = pixels.height, bottom = -1
        const margin = Math.ceil(7 * Math.max(sx, sy))
        for (let y = y0 + margin; y < y0 + height - margin; ++y) {
            for (let x = x0 + margin; x < x0 + width - margin; ++x) {
                const c = pixels.pixel(x, y)
                if (c.r > 0.75 && c.g > 0.75 && c.b > 0.75) {
                    left = Math.min(left, x); right = Math.max(right, x)
                    top = Math.min(top, y); bottom = Math.max(bottom, y)
                }
            }
        }
        verify(right >= left, "The label must actually be drawn")
        verify(Math.abs((left + right + 1 - width) / 2 - x0) <= 2,
               "Painted horizontal center: " + [left, right, x0, width])
        verify(Math.abs((top + bottom + 1 - height) / 2 - y0) <= 2,
               "Painted vertical center: " + [top, bottom, y0, height])
    }

    function test_release_change_after_scrolling() {
        const pane = createTemporaryObject(details, testCase)
        const button = findChild(pane, "detailsAction")
        for (let pass = 0; pass < 50; ++pass) {
            pane.contentY = 600.35
            waitForRendering(pane)
            pane.loading = true
            pane.headerHeight = pass % 2 ? 90 : 35
            waitForRendering(pane)
            pane.loading = false
            pane.contentY = 0
            waitForRendering(button)
            wait(20) // Allow the column and the next scene-graph sync to settle.
            verifyCentered(button)
            verifyPaintedCenter(button)
        }
    }
}
