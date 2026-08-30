import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "LaunchCommandPreview"

    QtObject {
        id: fakeModel
        property bool launch_profile_preview_valid: true
        property string launch_profile_preview_runtime: "RetroArch"
        property string launch_profile_preview_message: "Built-in template · 4 direct argv tokens"
        property int launch_profile_preview_argument_count: 4
        property int launch_profile_preview_revision: 1
        property var argv: ["--verbose", "-L", "<retroarch-core>", "<selected-file>"]

        function launch_profile_preview_argument_at(index) {
            return index >= 0 && index < argv.length ? argv[index] : ""
        }
    }

    Lunchbox.LaunchCommandPreview {
        id: preview
        width: 420
        height: implicitHeight
        previewModel: fakeModel
    }

    function init() {
        fakeModel.launch_profile_preview_valid = true
        fakeModel.launch_profile_preview_runtime = "RetroArch"
        fakeModel.launch_profile_preview_message = "Built-in template · 4 direct argv tokens"
        fakeModel.argv = ["--verbose", "-L", "<retroarch-core>", "<selected-file>"]
        fakeModel.launch_profile_preview_argument_count = 4
        fakeModel.launch_profile_preview_revision += 1
    }

    function test_valid_preview_exposes_runtime_and_separate_arguments() {
        compare(preview.previewValid, true)
        compare(preview.runtime, "RetroArch")
        compare(preview.argumentCount, 4)
        compare(preview.argumentAt(2), "<retroarch-core>")
        verify(preview.implicitHeight > 100)
    }

    function test_invalid_preview_replaces_tokens_with_actionable_error() {
        fakeModel.launch_profile_preview_valid = false
        fakeModel.launch_profile_preview_runtime = ""
        fakeModel.launch_profile_preview_message = "Check this command: unterminated quote"
        fakeModel.argv = []
        fakeModel.launch_profile_preview_argument_count = 0
        fakeModel.launch_profile_preview_revision += 1

        tryCompare(preview, "previewValid", false)
        compare(preview.argumentCount, 0)
        verify(preview.message.indexOf("unterminated quote") >= 0)
        verify(preview.implicitHeight > 40)
    }
}
