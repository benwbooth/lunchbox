import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "BulkMetadataEditor"
    when: windowShown

    Component {
        id: hostComponent

        Window {
            visible: true
            width: 900
            height: 700

            QtObject {
                id: metadataState
                property bool bulk_metadata_busy: false
                property string bulk_metadata_message: "Ready for an exact-ID bulk edit."
                property int callCount: 0
                property string field: ""
                property string value: ""
                property bool restore: false
                function apply_bulk_metadata(nextField, nextValue, nextRestore) {
                    ++callCount
                    field = nextField
                    value = nextValue
                    restore = nextRestore
                    bulk_metadata_message = "Saved."
                }
            }

            Lunchbox.BulkMetadataEditor {
                id: editor
                anchors.centerIn: parent
                metadataModel: metadataState
                visibleGameCount: 17
                scopeLabel: "Nintendo Switch · Available"
            }

            property alias editor: editor
            property alias metadataState: metadataState
        }
    }

    function openEditor(host) {
        host.editor.openForScope()
        tryVerify(() => host.editor.opened)
    }

    function test_set_value_requires_review_and_sends_one_exact_operation() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        openEditor(host)

        const field = findChild(host.editor, "bulkMetadataField")
        const value = findChild(host.editor, "bulkMetadataValue")
        const review = findChild(host.editor, "bulkMetadataReview")
        verify(field)
        verify(value)
        verify(review)
        field.currentIndex = 2
        value.text = "Action adventure"
        tryVerify(() => review.enabled)
        mouseClick(review, review.width / 2, review.height / 2)
        compare(host.editor.reviewing, true)
        compare(host.metadataState.callCount, 0)

        const apply = findChild(host.editor, "bulkMetadataApply")
        verify(apply)
        tryVerify(() => apply.visible && apply.enabled)
        wait(30)
        mouseClick(apply, apply.width / 2, apply.height / 2)
        compare(host.metadataState.callCount, 1)
        compare(host.metadataState.field, "genre")
        compare(host.metadataState.value, "Action adventure")
        compare(host.metadataState.restore, false)
    }

    function test_restore_needs_no_value_and_preserves_explicit_mode() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        openEditor(host)

        const field = findChild(host.editor, "bulkMetadataField")
        const action = findChild(host.editor, "bulkMetadataAction")
        const value = findChild(host.editor, "bulkMetadataValue")
        const review = findChild(host.editor, "bulkMetadataReview")
        field.currentIndex = 6
        action.currentIndex = 1
        tryCompare(value, "visible", false)
        tryVerify(() => review.enabled)
        mouseClick(review, review.width / 2, review.height / 2)

        const apply = findChild(host.editor, "bulkMetadataApply")
        tryVerify(() => apply.visible && apply.enabled)
        wait(30)
        mouseClick(apply, apply.width / 2, apply.height / 2)
        compare(host.metadataState.callCount, 1)
        compare(host.metadataState.field, "release_type")
        compare(host.metadataState.restore, true)
    }

    function test_empty_scope_cannot_advance() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.editor.visibleGameCount = 0
        openEditor(host)
        const value = findChild(host.editor, "bulkMetadataValue")
        const review = findChild(host.editor, "bulkMetadataReview")
        value.text = "Nintendo"
        tryCompare(review, "enabled", false)
        compare(host.metadataState.callCount, 0)
    }
}
