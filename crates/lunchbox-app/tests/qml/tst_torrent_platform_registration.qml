import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "TorrentPlatformRegistration"
    when: windowShown

    Component {
        id: hostComponent

        Item {
            width: 700
            height: 140

            QtObject {
                id: torrentState
                property bool collection_mode: false
                property int batch_source_count: 2
                property bool register_for_platform: false
                property bool busy: false
                property string game_platform: "Nintendo Switch"
                property int registrationCalls: 0
                function set_platform_registration(enabled) {
                    register_for_platform = enabled
                    ++registrationCalls
                }
            }

            Lunchbox.TorrentPlatformRegistration {
                id: registration
                width: 640
                height: 64
                torrent: torrentState
            }

            property alias torrentState: torrentState
            property alias registration: registration
        }
    }

    function test_batch_sources_are_explicitly_platform_scoped() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        const checkBox = findChild(host.registration,
                                   "torrentPlatformRegistrationCheckBox")
        const title = findChild(host.registration,
                                "torrentPlatformRegistrationTitle")
        verify(checkBox)
        verify(title)
        compare(checkBox.checked, true)
        compare(checkBox.enabled, false)
        compare(title.text, "USE THESE TORRENTS FOR NINTENDO SWITCH")
    }

    function test_single_source_keeps_optional_toggle() {
        const host = createTemporaryObject(hostComponent, testCase)
        verify(host)
        host.torrentState.batch_source_count = 0
        const checkBox = findChild(host.registration,
                                   "torrentPlatformRegistrationCheckBox")
        tryCompare(checkBox, "enabled", true)
        compare(checkBox.checked, false)
        checkBox.click()
        compare(host.torrentState.register_for_platform, true)
        compare(host.torrentState.registrationCalls, 1)
    }
}
