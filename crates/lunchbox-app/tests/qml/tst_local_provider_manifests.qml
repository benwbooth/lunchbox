import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "LocalProviderManifests"
    when: windowShown

    Component {
        id: hostComponent

        Window {
            visible: true
            width: 900
            height: 560

            QtObject {
                id: providerState
                property bool busy: false
                property int entry_count: 1
                property int revision: 1
                property string message: "1 local provider manifest is installed."
                property int importCount: 0
                property int resyncIndex: -1
                property int removeIndex: -1
                function choose_and_import() { ++importCount }
                function import_path(source) { }
                function provider_id_at(index) { return "my-switch-library" }
                function name_at(index) { return "My Switch Library" }
                function version_at(index) { return "2026.09" }
                function terms_url_at(index) { return "https://example.invalid/terms" }
                function path_at(index) { return "/tmp/provider/catalog.json" }
                function offer_count_at(index) { return 2 }
                function file_available_at(index) { return true }
                function resync_at(index) { resyncIndex = index }
                function remove_at(index) { removeIndex = index }
            }

            Lunchbox.LocalProviderManifests {
                id: providers
                anchors.fill: parent
                manifestModel: providerState
            }

            property alias providers: providers
            property alias providerState: providerState
        }
    }

    function test_import_and_resync_route_to_the_model() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const importButton = findChild(host.providers, "localProviderImport")
        verify(importButton)
        mouseClick(importButton, importButton.width / 2, importButton.height / 2)
        compare(host.providerState.importCount, 1)

        const list = findChild(host.providers, "localProviderList")
        tryVerify(() => list.itemAtIndex(0) !== null)
        const resync = findChild(host.providers, "localProviderResync-0")
        verify(resync)
        mouseClick(resync, resync.width / 2, resync.height / 2)
        compare(host.providerState.resyncIndex, 0)
    }

    function test_remove_requires_confirmation() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const list = findChild(host.providers, "localProviderList")
        tryVerify(() => list.itemAtIndex(0) !== null)
        const remove = findChild(host.providers, "localProviderRemove-0")
        verify(remove)
        mouseClick(remove, remove.width / 2, remove.height / 2)

        const dialog = findChild(host.providers, "localProviderRemoveDialog")
        verify(dialog)
        tryVerify(() => dialog.opened)
        compare(host.providerState.removeIndex, -1)
        dialog.accept()
        tryCompare(host.providerState, "removeIndex", 0)
    }
}
