import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "StartupPresentation"
    when: windowShown

    Component {
        id: hiddenGridComponent
        ApplicationWindow {
            id: gridHost
            visible: false
            width: 800
            height: 600
            property int preparedCount: 0
            property alias view: grid
            ColumnLayout {
                id: layout
                anchors.fill: parent
                Label { text: "Library" }
                GridView {
                    id: grid
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    cellWidth: 100
                    cellHeight: 100
                    model: 40
                    delegate: Rectangle { width: 100; height: 100; color: "red" }
                }
            }
            Lunchbox.StartupPresentation {
                ready: true
                prepare: function() {
                    gridHost.contentItem.ensurePolished()
                    layout.ensurePolished()
                    grid.forceLayout()
                    gridHost.preparedCount = grid.contentItem.children.length
                    return grid.width > 0 && grid.height > 0 && gridHost.preparedCount > 0
                }
                onPresent: gridHost.showNormal()
            }
        }
    }

    function test_hidden_window_prepares_real_grid_before_presentation() {
        const host = createTemporaryObject(hiddenGridComponent, this)
        tryCompare(host, "visible", true)
        verify(host.preparedCount > 0)
        verify(host.view.width > 0)
        verify(host.view.height > 0)
        host.close()
    }

    Component {
        id: hostComponent
        Window {
            id: host
            visible: false
            width: 400
            height: 300
            property bool dataReady: false
            property bool layoutReady: false
            property bool invalidateDuringPrepare: false
            property bool invalidateDuringCommit: false
            property int presentations: 0
            property int preparations: 0
            property alias gate: presentation
            Lunchbox.StartupPresentation {
                id: presentation
                ready: host.dataReady
                prepare: function() {
                    ++host.preparations
                    if (host.invalidateDuringPrepare
                            || (host.invalidateDuringCommit && host.preparations === 2))
                        host.dataReady = false
                    return host.layoutReady
                }
                onPresent: {
                    ++host.presentations
                    host.showNormal()
                }
            }
        }
    }

    function test_waits_for_both_data_and_final_layout() {
        const host = createTemporaryObject(hostComponent, this)
        wait(40)
        compare(host.visible, false)
        host.dataReady = true
        wait(40)
        compare(host.visible, false)
        verify(host.preparations > 0)
        host.layoutReady = true
        tryCompare(host, "presentations", 1)
        compare(host.visible, true)
        host.dataReady = false // Later reloads must not hide an existing window.
        wait(40)
        compare(host.visible, true)
        compare(host.presentations, 1)
        host.close()
    }

    function test_queued_presentation_rechecks_readiness() {
        const host = createTemporaryObject(hostComponent, this)
        host.layoutReady = true
        host.invalidateDuringPrepare = true
        host.dataReady = true
        tryCompare(host, "dataReady", false)
        wait(40)
        compare(host.visible, false)
        compare(host.presentations, 0)
        host.invalidateDuringPrepare = false
        host.dataReady = true
        tryCompare(host, "presentations", 1)
        host.close()
    }

    function test_disabled_probe_gate_never_takes_window_ownership() {
        const host = createTemporaryObject(hostComponent, this)
        host.gate.enabled = false
        host.dataReady = true
        host.layoutReady = true
        wait(40)
        compare(host.presentations, 0)
        compare(host.visible, false)
        host.close()
    }

    function test_commit_preparation_can_invalidate_readiness() {
        const host = createTemporaryObject(hostComponent, this)
        host.layoutReady = true
        host.invalidateDuringCommit = true
        host.dataReady = true
        tryCompare(host, "preparations", 2)
        compare(host.dataReady, false)
        compare(host.visible, false)
        compare(host.presentations, 0)
        host.invalidateDuringCommit = false
        host.dataReady = true
        tryCompare(host, "presentations", 1)
        host.close()
    }
}
