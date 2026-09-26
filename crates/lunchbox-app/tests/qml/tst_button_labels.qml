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
        fuzzyCompare(center.x, button.width / 2, 0.5, "Horizontal label center")
        fuzzyCompare(center.y, button.height / 2, 0.5, "Vertical label center")
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
}
