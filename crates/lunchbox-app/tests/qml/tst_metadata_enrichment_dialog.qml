import QtQuick
import QtQuick.Controls
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "MetadataEnrichmentDialog"
    when: windowShown

    Component {
        id: hostComponent

        ApplicationWindow {
            visible: true
            width: 1100
            height: 820

            QtObject {
                id: igdbState
                property bool initialized: true
                property bool busy: false
                property bool credentials_saved: true
                property string message: "Found two possible games."
                property string selected_game_name: ""
                property int database_id: 0
                property int game_count: 2
                property int revision: 0
                property int chosenIndex: -1
                function initialize() {}
                function begin_selection(databaseId, title, platform, artworkType) {
                    database_id = databaseId
                    selected_game_name = ""
                    game_count = 2
                    message = "Found two possible games."
                    ++revision
                }
                function search_games(query) {
                    selected_game_name = ""
                    game_count = 2
                    ++revision
                }
                function choose_game(index) {
                    chosenIndex = index
                    selected_game_name = "Super Mario Bros."
                    game_count = 0
                    ++revision
                }
                function game_name_at(index) {
                    return index === 0 ? "Super Mario Bros." : "Mario Bros."
                }
                function game_detail_at(index) {
                    return index === 0 ? "IGDB ID 7346 · NES" : "IGDB ID 123 · Arcade"
                }
                function selected_metadata_value(field) {
                    const values = {
                        title: "Super Mario Bros.",
                        description: "Rescue the Mushroom Kingdom.",
                        release_date: "1985-09-13",
                        developer: "Nintendo R&D4",
                        publisher: "Nintendo",
                        genre: "Platform",
                        players: "",
                        rating: "4.3"
                    }
                    return values[field] || ""
                }
            }

            QtObject {
                id: screenScraperState
                property bool initialized: true
                property bool busy: false
                property bool credentials_saved: false
                property string message: "ScreenScraper is not configured."
                property string selected_game_name: ""
                property int game_count: 0
                property int revision: 0
                function initialize() {}
                function begin_selection(databaseId, title, platform, artworkType) {}
                function search_games(query) {}
                function choose_game(index) {}
                function game_name_at(index) { return "" }
                function game_detail_at(index) { return "" }
                function selected_metadata_value(field) { return "" }
            }

            QtObject {
                id: detailsState
                property int database_id: 140
                property string title: "Super Mario Bros."
                property string platform: "Nintendo Entertainment System"
                property string metadata_title: "Super Mario Bros."
                property string metadata_description: ""
                property string metadata_release_date: ""
                property string metadata_developer: "Existing Studio"
                property string metadata_publisher: ""
                property string metadata_genre: ""
                property string metadata_players: "1-2"
                property string metadata_rating: ""
            }

            Lunchbox.MetadataEnrichmentDialog {
                id: review
                igdbModel: igdbState
                screenScraperModel: screenScraperState
                gameDetailsModel: detailsState
                canonicalTitle: "Super Mario Bros."
            }

            property alias review: review
            property alias igdbState: igdbState
            property alias detailsState: detailsState
        }
    }

    function test_exact_record_and_missing_only_review() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.review.openReview()
        tryVerify(() => host.review.opened)
        compare(host.igdbState.game_count, 2)

        host.igdbState.choose_game(0)
        tryCompare(host.review, "fieldCount", 7)
        compare(host.igdbState.chosenIndex, 0)
        compare(host.review.selectedFieldCount, 5)

        host.review.applySelectedFields()
        compare(host.detailsState.metadata_description,
                "Rescue the Mushroom Kingdom.")
        compare(host.detailsState.metadata_release_date, "1985-09-13")
        compare(host.detailsState.metadata_publisher, "Nintendo")
        compare(host.detailsState.metadata_genre, "Platform")
        compare(host.detailsState.metadata_rating, "4.3")
        compare(host.detailsState.metadata_developer, "Existing Studio")
        compare(host.detailsState.metadata_players, "1-2")
    }

    function test_conflicts_require_explicit_all_changes_action() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.review.openReview()
        host.igdbState.choose_game(0)
        tryCompare(host.review, "fieldCount", 7)
        host.review.selectChanges(true)
        compare(host.review.selectedFieldCount, 6)
        host.review.applySelectedFields()
        compare(host.detailsState.metadata_developer, "Nintendo R&D4")
        compare(host.detailsState.metadata_players, "1-2")
    }
}
