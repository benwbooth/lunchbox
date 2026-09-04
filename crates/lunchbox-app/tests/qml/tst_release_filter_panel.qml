import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ReleaseFilterPanel"
    when: windowShown
    width: 420
    height: 900

    Component {
        id: panelComponent

        Item {
            width: 360
            height: 860

            QtObject {
                id: libraryState
                property int release_region_count: 4
                property int selected_release_region_count: 0
                property int release_region_revision: 0
                property bool hide_adult: false
                property bool adult_release_filter: false
                property bool hide_non_retail: false
                property bool non_retail_release_filter: false
                property var selectedRegions: [false, false, false, false]
                property var regionNames: ["USA", "Japan", "Europe", "Brazil"]
                property int setCount: 0

                function release_region_name_at(index) {
                    return regionNames[index]
                }
                function release_region_selected_at(index) {
                    return selectedRegions[index]
                }
                function toggle_release_region(index) {
                    const changed = selectedRegions.slice()
                    changed[index] = !changed[index]
                    selectedRegions = changed
                    selected_release_region_count += changed[index] ? 1 : -1
                    release_region_revision += 1
                }
                function clear_release_region_filters() {
                    selectedRegions = [false, false, false, false]
                    selected_release_region_count = 0
                    release_region_revision += 1
                }
                function release_category_mode(category) {
                    if (category === "adult")
                        return adult_release_filter ? "only" : hide_adult ? "exclude" : "any"
                    return non_retail_release_filter ? "only" : hide_non_retail ? "exclude" : "any"
                }
                function set_release_category_mode(category, mode) {
                    if (category === "adult") {
                        hide_adult = mode === "exclude"
                        adult_release_filter = mode === "only"
                    } else {
                        hide_non_retail = mode === "exclude"
                        non_retail_release_filter = mode === "only"
                    }
                    setCount += 1
                }
            }

            Lunchbox.ReleaseFilterPanel {
                id: releaseFilters
                width: parent.width
                libraryModel: libraryState
            }

            SignalSpy {
                id: changedSpy
                target: releaseFilters
                signalName: "filtersChanged"
            }

            property alias library: libraryState
            property alias panel: releaseFilters
            property alias changedSpy: changedSpy
        }
    }

    function test_all_region_choices_are_data_driven_and_accumulate() {
        const host = createTemporaryObject(panelComponent, testCase)
        verify(host)
        const usa = findChild(host.panel, "releaseRegion_0")
        const brazil = findChild(host.panel, "releaseRegion_3")
        verify(usa)
        verify(brazil)

        usa.activate()
        compare(host.library.selected_release_region_count, 1)
        verify(host.library.release_region_selected_at(0))
        brazil.activate()
        compare(host.library.selected_release_region_count, 2)
        verify(host.library.release_region_selected_at(3))
        compare(host.changedSpy.count, 2)
    }

    function test_categories_have_one_unambiguous_three_state_control_each() {
        const host = createTemporaryObject(panelComponent, testCase)
        verify(host)
        const adult = findChild(host.panel, "adultReleaseMode")
        const nonRetail = findChild(host.panel, "nonRetailReleaseMode")
        verify(adult)
        verify(nonRetail)
        compare(findChild(host.panel, "adultReleaseFilter"), null)

        adult.activated(1)
        compare(host.library.hide_adult, true)
        compare(host.library.adult_release_filter, false)
        adult.activated(2)
        compare(host.library.hide_adult, false)
        compare(host.library.adult_release_filter, true)
        nonRetail.activated(2)
        compare(host.library.non_retail_release_filter, true)
        compare(host.library.setCount, 3)
        compare(host.changedSpy.count, 3)
    }
}
