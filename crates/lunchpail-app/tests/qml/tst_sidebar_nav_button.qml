import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "SidebarNavButton"
    when: windowShown
    visible: true
    width: 420
    height: 360

    Item {
        id: narrowHost
        x: 20
        y: 20
        width: 154 // Minimum 180px sidebar minus the platform list's 26px margins.
        height: 260
        Lunchpail.SidebarNavButton {
            id: narrowPlatform
            label: "Super Nintendo Entertainment System"
            glyph: ""
            count: "303561"
        }
    }

    Item {
        id: normalHost
        x: 190
        y: 20
        width: 228 // Default 254px sidebar minus the same margins.
        height: 260
        Lunchpail.SidebarNavButton {
            id: normalPlatform
            label: "Super Nintendo Entertainment System"
            glyph: ""
            count: "303561"
        }
    }

    function verifyFullLabel(button) {
        const label = findChild(button, "sidebarNavLabel")
        verify(label !== null)
        compare(label.elide, Text.ElideNone)
        compare(label.fontSizeMode, Text.FixedSize)
        compare(label.font.pixelSize, 13)
        verify(!label.truncated)
        verify(label.contentWidth <= label.width + 1)
        verify(label.y >= 0)
        verify(label.y + label.contentHeight <= button.height + 1)
    }

    function test_platform_names_are_fully_visible_at_supported_sidebar_widths() {
        verifyFullLabel(narrowPlatform)
        verifyFullLabel(normalPlatform)
        verify(narrowPlatform.height > 43)
    }

    function test_short_and_long_names_keep_same_font_when_resized() {
        const originalLabel = normalPlatform.label
        const originalWidth = normalHost.width
        try {
            const names = ["NES", "Nintendo Game Boy Advance", "Super Nintendo Entertainment System"]
            for (const width of [154, 228, 360]) {
                normalHost.width = width
                for (const name of names) {
                    normalPlatform.label = name
                    wait(0)
                    verifyFullLabel(normalPlatform)
                }
            }
        } finally {
            normalPlatform.label = originalLabel
            normalHost.width = originalWidth
        }
        wait(0)
        grabImage(narrowHost).save("/tmp/lunchpail-platform-labels-narrow.png")
        grabImage(normalHost).save("/tmp/lunchpail-platform-labels-normal.png")
    }
}
