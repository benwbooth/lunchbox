import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "AlphabetRail"

    QtObject {
        id: fakeLibrary
        property int alphabet_revision: 1
        property bool alphabet_navigation_available: true

        function alphabet_target_row(label) {
            if (!alphabet_navigation_available)
                return -1
            if (label === "#")
                return 0
            if (label === "A")
                return 4
            if (label === "M")
                return 42
            if (label === "Z")
                return 99
            return -1
        }

        function alphabet_label_for_row(row) {
            if (!alphabet_navigation_available)
                return ""
            return row >= 42 && row < 99 ? "M" : row >= 99 ? "Z" : "A"
        }
    }

    Lunchbox.AlphabetRail {
        id: rail
        width: 34
        height: 540
        libraryModel: fakeLibrary
        currentRow: 42
    }

    SignalSpy {
        id: jumpSpy
        target: rail
        signalName: "jumpRequested"
    }

    function init() {
        fakeLibrary.alphabet_navigation_available = true
        fakeLibrary.alphabet_revision += 1
        rail.currentRow = 42
        jumpSpy.clear()
    }

    function test_exposes_all_legacy_title_buckets() {
        compare(rail.labels.length, 27)
        compare(rail.labels[0], "#")
        compare(rail.labels[1], "A")
        compare(rail.labels[26], "Z")
        compare(rail.activeLabel, "M")
    }

    function test_activation_emits_the_preindexed_row() {
        verify(rail.activateLabel("Z"))
        compare(jumpSpy.count, 1)
        compare(jumpSpy.signalArguments[0][0], 99)

        verify(!rail.activateLabel("Q"))
        compare(jumpSpy.count, 1)
    }

    function test_non_title_sort_disables_navigation_without_hiding_the_rail() {
        fakeLibrary.alphabet_navigation_available = false
        fakeLibrary.alphabet_revision += 1
        tryCompare(rail, "navigationAvailable", false)
        compare(rail.activeLabel, "")
        verify(!rail.activateLabel("A"))
        compare(jumpSpy.count, 0)
    }
}
