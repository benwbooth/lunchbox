import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "CouchDetailsPage"
    when: windowShown
    visible: true
    width: 1280; height: 800
    QtObject {
        id: info
        property int variant_count: 3
        property string description: "A game description long enough to check the dedicated couch layout."
        property string release_date: "1991"
        property string genre: "Platform"
        property string developer: "Nintendo"
        property string players: "2"
        property string rating: "4.5"
        property bool activity_visible: true
        property int play_count: 4
        property string play_time: "2 hours"
        property string last_played: "Yesterday"
        property string completion_state: "in-progress"
        property string notes: "Remember the secret exit."
    }
    Component {
        id: pageComponent
        Lunchpail.CouchDetailsPage {
            width: 1280; height: 800; details: info; ready: true
            gameTitle: "Super Mario World"; platform: "Super Nintendo Entertainment System"
        }
    }
    SignalSpy { id: managed; signalName: "manageRequested" }
    SignalSpy { id: played; signalName: "primaryRequested" }
    SignalSpy { id: closed; signalName: "closeRequested" }

    function test_controller_can_reach_all_tools() {
        const page = createTemporaryObject(pageComponent, this)
        managed.target = page; managed.clear()
        played.target = page; played.clear()
        page.handleNavigation("accept")
        compare(played.count, 1)
        page.handleNavigation("page_right")
        compare(page.tabIndex, 1)
        page.handleNavigation("down")
        page.handleNavigation("accept")
        compare(managed.signalArguments[0][0], "display")
        page.handleNavigation("right")
        page.handleNavigation("down")
        page.handleNavigation("down")
        page.handleNavigation("accept")
        compare(managed.signalArguments[1][0], "advanced")
        page.handleNavigation("page_right")
        page.handleNavigation("down")
        page.handleNavigation("accept")
        compare(managed.signalArguments[2][0], "artwork")
        page.handleNavigation("page_right")
        page.handleNavigation("down")
        page.handleNavigation("accept")
        compare(managed.signalArguments[3][0], "activity")
    }
    function test_tools_fit_and_click_at_720p() {
        const page = createTemporaryObject(pageComponent, this, {height: 720, tabIndex: 1})
        managed.target = page; managed.clear()
        wait(30)
        for (let i = 0; i < page.tools.length; i++) {
            const button = findChild(page, "couchDetailsTool" + i)
            verify(button && button.visible && button.enabled)
            const edge = button.mapToItem(page, button.width, button.height)
            verify(edge.x <= page.width && edge.y < page.height - 45,
                   "Tool must fit above the navigation legend")
            mouseClick(button)
            compare(managed.signalArguments[i][0], page.tools[i].key)
        }
        closed.target = page; closed.clear()
        page.handleNavigation("back")
        compare(closed.count, 1)
    }
    function test_unready_tools_do_not_act() {
        const page = createTemporaryObject(pageComponent, this, {ready: false, tabIndex: 1, navigationArea: 2})
        managed.target = page; managed.clear()
        page.handleNavigation("accept")
        compare(managed.count, 0)
        verify(!findChild(page, "couchDetailsTool0").enabled)
    }
}
