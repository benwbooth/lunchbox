import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GameFileIdentityDialog"
    when: windowShown

    Component {
        id: hostComponent
        ApplicationWindow {
            width: 1100
            height: 840
            visible: true

            QtObject {
                id: model
                property string message: "Choose an exact catalog game."
                property string game_uid: "11111111-1111-4111-8111-111111111111"
                property string game_title: "Source Game"
                property string game_platform: "Source Platform"
                property int file_count: 1
                property int selected_file: 0
                property int candidate_count: 2
                property int selected_candidate: -1
                property int history_count: 1
                property int loadCalls: 0
                property int searchCalls: 0
                property int relinkCalls: 0
                property int undoIndex: -1

                function load_game(uid, title, platform) {
                    loadCalls += 1
                    game_uid = uid
                    game_title = title
                    game_platform = platform
                }
                function search_targets(query, platform) { searchCalls += 1 }
                function select_file(index) { selected_file = index }
                function select_candidate(index) { selected_candidate = index }
                function relink_selected() { relinkCalls += 1 }
                function undo_at(index) { undoIndex = index }
                function file_label_at(index) { return "source-game.bin" }
                function file_path_at(index) { return "/roms/source-game.bin" }
                function file_detail_at(index) { return "4.0 GiB · reviewed · present" }
                function candidate_title_at(index) {
                    return index === 0 ? "Target Game" : "Target Game Deluxe"
                }
                function candidate_platform_at(index) { return "Target Platform" }
                function candidate_match_at(index) { return "Canonical title" }
                function candidate_game_uid_at(index) {
                    return index === 0
                            ? "22222222-2222-4222-8222-222222222222"
                            : "33333333-3333-4333-8333-333333333333"
                }
                function history_summary_at(index) { return "Source Game → Target Game" }
                function history_detail_at(index) {
                    return "Source Platform → Target Platform · exact file identity preserved"
                }
                function history_occurred_at(index) { return 1788249600 }
                function history_undoable_at(index) { return index === 0 }
            }

            Lunchbox.GameFileIdentityDialog {
                id: identityDialog
                identityModel: model
            }

            property alias identityDialog: identityDialog
            property alias model: model
        }
    }

    function test_exact_candidate_requires_review_then_explicit_confirmation() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.identityDialog.begin(host.model.game_uid,
                                  host.model.game_title,
                                  host.model.game_platform)
        tryVerify(() => host.identityDialog.visible)
        compare(host.model.loadCalls, 1)
        verify(host.model.searchCalls >= 1)

        const candidates = findChild(host.identityDialog, "identityCandidateList")
        verify(candidates)
        compare(candidates.count, 2)
        tryVerify(() => candidates.itemAtIndex(0) !== null)
        candidates.itemAtIndex(0).clicked()
        tryCompare(host.model, "selected_candidate", 0)

        const confirm = findChild(host.identityDialog, "confirmIdentityRelinkButton")
        verify(confirm)
        verify(confirm.enabled)
        confirm.clicked()
        compare(host.model.relinkCalls, 1)
    }

    function test_history_is_hidden_until_requested_and_undo_is_explicit() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.identityDialog.begin(host.model.game_uid,
                                  host.model.game_title,
                                  host.model.game_platform)
        tryVerify(() => host.identityDialog.visible)
        const history = findChild(host.identityDialog, "identityHistoryList")
        verify(history)
        verify(!history.visible)

        const toggle = findChild(host.identityDialog, "identityHistoryToggle")
        toggle.clicked()
        tryVerify(() => history.visible)
        tryVerify(() => history.itemAtIndex(0) !== null)
        const undo = findChild(history.itemAtIndex(0), "identityUndoButton-0")
        verify(undo)
        mouseClick(undo, undo.width / 2, undo.height / 2)
        compare(host.model.undoIndex, 0)
    }
}
