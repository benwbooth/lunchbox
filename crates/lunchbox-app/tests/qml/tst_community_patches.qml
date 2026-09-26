import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "CommunityPatches"
    when: windowShown
    visible: true
    width: 570; height: 1900
    Rectangle { anchors.fill: parent; z: -1; color: "#101720" }
    QtObject {
        id: mockCatalog
        property string results_json: "[]"
        property string details_json: "null"
        property string package_json: "null"
        property string message: ""
        property bool busy: false
        property string selected: ""
        property var lastSearch: []
        property var lastVariant: []
        function select_game(game, platform) { selected = game }
        function search(source, title, kind) { lastSearch = [source, title, kind] }
        function use_variant(index, path, enable) { lastVariant = [index, path, enable] }
    }
    Component {
        id: component
        Lunchbox.CommunityPatchesPane {
            width: 550; backend: mockCatalog
            gameId: "ff2"; gameTitle: "Final Fantasy II (Japan)"
            platform: "Nintendo Entertainment System"; romPath: "/example/ff2.nes"
        }
    }
    readonly property var backend: mockCatalog
    function init() {
        backend.results_json = "[]"
        backend.details_json = "null"
        backend.package_json = "null"
        backend.lastSearch = []
        backend.lastVariant = []
    }
    function test_search_is_explicit_and_prefilled() {
        const pane = createTemporaryObject(component, testCase)
        verify(pane !== null)
        compare(pane.expanded, false)
        compare(backend.lastSearch.length, 0)
        compare(findChild(pane, "patchQuery").text, "Final Fantasy II")
        mouseClick(findChild(pane, "communityPatchesAccordion"))
        wait(50)
        compare(pane.expanded, true)
        mouseClick(findChild(pane, "searchCommunityPatches"))
        compare(backend.lastSearch.join("|"), "archive|Final Fantasy II|translations")
        pane.gameId = "next"
        compare(pane.expanded, false)
        compare(backend.selected, "next")
    }
    function test_variant_selection_and_layout() {
        backend.results_json = JSON.stringify([{title:"Final Fantasy II — English translation #139"}])
        backend.details_json = JSON.stringify({title:"Final Fantasy II — English translation #139", platform:"Nintendo Entertainment System", language:"English", version:"1.0", description:"An English translation. Use the Japanese original with the matching checksum.", source_url:"https://example.test/credits", bases:[{name:"Final Fantasy II (J).nes",crc32:"b7327510",sha1:"",md5:""}], files:[]})
        backend.package_json = JSON.stringify({variants:[{name:"English.ips",format:"IPS"},{name:"Alternate names.ips",format:"IPS"}],readme:"Author's installation notes. Choose one variant only."})
        const pane = createTemporaryObject(component, testCase)
        pane.expanded = true
        wait(80)
        compare(backend.lastVariant.length, 0)
        const apply = findChild(pane, "applyPatchVariant1")
        verify(apply !== null)
        mouseClick(apply)
        compare(backend.lastVariant.join("|"), "1|/example/ff2.nes|true")
        pane.locked = true
        compare(apply.enabled, false)
        pane.locked = false
        pane.romPath = ""
        compare(apply.enabled, false)
        pane.romPath = "/example/ff2.nes"
        verify(pane.height > 500 && pane.height < testCase.height)
        grabImage(testCase).save("/tmp/lunchbox-community-patches.png")
    }
    function test_github_source_accepts_author_repository() {
        const pane = createTemporaryObject(component, testCase)
        pane.expanded = true
        findChild(pane, "patchSource").currentIndex = 2
        const query = findChild(pane, "patchQuery")
        verify(query.placeholderText.indexOf("owner/repository") >= 0)
        query.text = "Dimedime-d/kptranslation"
        wait(50)
        mouseClick(findChild(pane, "searchCommunityPatches"))
        compare(backend.lastSearch.join("|"), "github|Dimedime-d/kptranslation|translations")
    }
}
