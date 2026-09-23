import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "NotificationHistory"

    Lunchbox.NotificationHistory { id: history }

    function init() {
        history.clear()
    }

    function test_newest_first_and_round_trip() {
        history.append("Faxanadu save state saved", true, "2026-09-23T01:00:00Z")
        history.append("Cloud sync complete", true, "2026-09-23T01:01:00Z")
        compare(history.count, 2)
        compare(history.entries[0].message, "Cloud sync complete")
        const saved = history.serialized()
        history.clear()
        history.initialize(saved)
        compare(history.count, 2)
        compare(history.entries[1].message, "Faxanadu save state saved")
        history.clear()
        compare(history.serialized(), "[]")
    }

    function test_bad_data_and_bounded_history() {
        history.initialize("{broken")
        compare(history.count, 0)
        history.initialize('[{"message":"valid","when":"now","good":true},'
                           + '{"message":42,"when":"now","good":true}]')
        compare(history.count, 1)
        for (let i = 0; i < 105; ++i)
            history.append("notice " + i, false, "now")
        compare(history.count, 100)
        compare(history.entries[0].message, "notice 104")
    }
}
