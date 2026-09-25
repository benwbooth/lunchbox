import QtQuick
import QtQuick.Controls as Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ThemedControls"
    when: windowShown
    visible: true
    width: 420
    height: 280

    property int buttonClicks: 0
    property int comboActivations: 0
    property int dialogAccepts: 0

    Component {
        id: positiveButtonComponent
        Lunchbox.LbButton { text: "PLAY"; highlighted: true; positive: true }
    }
    Component {
        id: longButtonComponent
        Lunchbox.LbButton { text: "SAVE PLAYER 1 MAPPING FOR THIS SYSTEM" }
    }

    Component {
        id: positiveRoundButtonComponent
        Lunchbox.LbRoundButton { text: "▶"; highlighted: true; positive: true }
    }
    Component {
        id: textFieldComponent
        Lunchbox.LbTextField { text: "Player one" }
    }
    Component {
        id: textAreaComponent
        Lunchbox.LbTextArea { text: "Notes" }
    }
    Component {
        id: spinBoxComponent
        Lunchbox.LbSpinBox { from: 1; to: 8; value: 2 }
    }

    Lunchbox.LbDialog {
        id: dialog
        standardButtons: Controls.Dialog.Ok | Controls.Dialog.Cancel
        onAccepted: testCase.dialogAccepts += 1
    }

    Column {
        x: 20
        y: 20
        spacing: 12

        Lunchbox.LbButton {
            id: button
            text: "Launch"
            onClicked: testCase.buttonClicks += 1
        }
        Lunchbox.LbButton {
            id: fixedHeightButton
            width: 180
            height: 48
            text: "THIS PLATFORM"
            font.pixelSize: 8
        }
        Lunchbox.LbButton {
            id: narrowButton
            width: 145
            height: 34
            text: "THIS PLATFORM DEFAULT"
        }
        Lunchbox.LbComboBox {
            id: combo
            width: 240
            model: ["System default", "RetroTube TV", "None"]
            onActivated: testCase.comboActivations += 1
        }
        Lunchbox.LbComboBox {
            id: customCombo
            width: 240
            model: ["First", "Second"]
            delegate: Lunchbox.LbItemDelegate {
                required property int index
                width: customCombo.popup.width - customCombo.popup.leftPadding
                       - customCombo.popup.rightPadding
                text: customCombo.textAt(index)
            }
        }
        Lunchbox.LbRoundButton {
            id: roundButton
            text: "?"
        }
        Lunchbox.LbToolButton {
            id: toolButton
            text: "More"
        }
        Controls.TabBar {
            id: tabs
            width: 240
            Lunchbox.LbTabButton { text: "Overview" }
            Lunchbox.LbTabButton { text: "Details" }
        }
    }

    function test_buttons_share_surface_and_remain_interactive() {
        compare(button.background.color, "#202a39")
        compare(roundButton.background.color, "#202a39")
        mouseClick(button)
        compare(buttonClicks, 1)
        verify(toolButton.background !== null)
    }

    function test_fixed_height_button_honors_requested_label_size() {
        compare(fixedHeightButton.contentItem.font.pixelSize, 8)
        compare(fixedHeightButton.contentItem.fontInfo.pixelSize, 8)
        verify(Math.abs(fixedHeightButton.contentItem.y
                        + fixedHeightButton.contentItem.height / 2
                        - fixedHeightButton.height / 2) <= 1)
    }

    function test_button_press_does_not_depress_surface() {
        mouseMove(fixedHeightButton)
        const resting = fixedHeightButton.background.color.toString()
        mousePress(fixedHeightButton)
        compare(fixedHeightButton.background.color.toString(), resting)
        mouseRelease(fixedHeightButton)
    }

    function test_button_label_shrinks_to_fit_without_ellipsis() {
        compare(narrowButton.contentItem.elide, Text.ElideNone)
        verify(narrowButton.contentItem.fontInfo.pixelSize < 12)
        verify(narrowButton.contentItem.contentWidth <= narrowButton.contentItem.width)
        const longButton = createTemporaryObject(longButtonComponent, testCase)
        verify(longButton)
        verify(longButton.implicitWidth <= longButton.maximumImplicitWidth)
        verify(longButton.contentItem.contentWidth <= longButton.contentItem.width)
    }

    function test_settings_inputs_match_dropdown_surface() {
        const field = createTemporaryObject(textFieldComponent, testCase)
        const area = createTemporaryObject(textAreaComponent, testCase)
        const spin = createTemporaryObject(spinBoxComponent, testCase)
        verify(field && area && spin)
        compare(field.background.color, combo.background.color)
        compare(area.background.color, combo.background.color)
        compare(spin.background.color, combo.background.color)
        compare(field.background.radius, combo.background.radius)
        compare(spin.value, 2)
        verify(spin.up.indicator !== null)
    }

    function test_play_buttons_are_green_without_recoloring_other_buttons() {
        const play = createTemporaryObject(positiveButtonComponent, testCase)
        const playBadge = createTemporaryObject(positiveRoundButtonComponent, testCase)
        verify(play)
        verify(playBadge)
        compare(play.background.color, "#237a4d")
        compare(play.background.border.color, "#5ee391")
        compare(playBadge.background.color, "#237a4d")
        compare(playBadge.background.border.color, "#5ee391")
        compare(button.background.color, "#202a39")
    }

    function test_dropdown_is_themed_and_keeps_selection_semantics() {
        compare(combo.background.color, "#202a39")
        compare(combo.displayText, "System default")
        mouseClick(combo, combo.width - 12, combo.height / 2)
        tryVerify(function() { return combo.popup.visible })
        verify(combo.popup.background !== null)
        keyClick(Qt.Key_Down)
        keyClick(Qt.Key_Return)
        tryCompare(combo, "currentIndex", 1)
        compare(combo.displayText, "RetroTube TV")
        compare(comboActivations, 1)
    }

    function test_custom_dropdown_delegate_keeps_selection_semantics() {
        mouseClick(customCombo, customCombo.width - 12, customCombo.height / 2)
        tryVerify(function() { return customCombo.popup.visible })
        tryVerify(function() {
            return customCombo.popup.contentItem.itemAtIndex(1) !== null
        })
        const second = customCombo.popup.contentItem.itemAtIndex(1)
        mouseClick(second, second.width / 2, second.height / 2)
        tryCompare(customCombo, "currentIndex", 1)
        compare(customCombo.displayText, "Second")
    }

    function test_tabs_share_surface_and_keep_selection_semantics() {
        compare(tabs.itemAt(0).background.color, "#2d3440")
        mouseClick(tabs.itemAt(1))
        compare(tabs.currentIndex, 1)
        compare(tabs.itemAt(1).background.color, "#2d3440")
    }

    function test_dialog_standard_buttons_use_the_same_surface() {
        dialog.open()
        tryVerify(function() { return dialog.visible })
        let ok = null
        for (let index = 0; index < dialog.footer.count; ++index) {
            const candidate = dialog.footer.itemAt(index)
            if (candidate && candidate.text.toUpperCase() === "OK") {
                ok = candidate
                break
            }
        }
        verify(ok !== null)
        compare(ok.background.color, "#202a39")
        mouseClick(ok)
        compare(dialogAccepts, 1)
    }
}
