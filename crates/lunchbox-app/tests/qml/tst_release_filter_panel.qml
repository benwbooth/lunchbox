import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "ReleaseFilterPanel"
    when: windowShown
    width: 420
    height: 760

    Component {
        id: panelComponent

        Item {
            width: 360
            height: 700

            QtObject {
                id: libraryState
                property bool usa_release_filter: false
                property bool japan_release_filter: false
                property bool adult_release_filter: false
                property bool non_retail_release_filter: false
                property int setCount: 0

                function set_release_filters(usa, japan, adult, nonRetail) {
                    usa_release_filter = usa
                    japan_release_filter = japan
                    adult_release_filter = adult
                    non_retail_release_filter = nonRetail
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

    function test_region_choices_accumulate_with_or_semantics() {
        const host = createTemporaryObject(panelComponent, testCase)
        verify(host)
        const usa = findChild(host.panel, "usaReleaseFilter")
        const japan = findChild(host.panel, "japanReleaseFilter")
        verify(usa)
        verify(japan)

        usa.toggled()
        compare(host.library.usa_release_filter, true)
        compare(host.library.japan_release_filter, false)
        japan.toggled()
        compare(host.library.usa_release_filter, true)
        compare(host.library.japan_release_filter, true)
        compare(host.changedSpy.count, 2)
    }

    function test_category_choices_are_independently_selectable() {
        const host = createTemporaryObject(panelComponent, testCase)
        verify(host)
        const adult = findChild(host.panel, "adultReleaseFilter")
        const nonRetail = findChild(host.panel, "nonRetailReleaseFilter")
        verify(adult)
        verify(nonRetail)

        adult.toggled()
        nonRetail.toggled()
        compare(host.library.adult_release_filter, true)
        compare(host.library.non_retail_release_filter, true)
        compare(host.library.setCount, 2)
        compare(host.changedSpy.count, 2)
    }
}
