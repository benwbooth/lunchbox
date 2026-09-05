import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "ProbeArguments"
    Lunchbox.ProbeArguments { id: parser }

    function test_probe_switches_data() {
        return [
            { tag: "normal", args: ["lunchbox", "--database", "/tmp/catalog.db"], expected: false },
            { tag: "executable", args: ["/tmp/probe/lunchbox"], expected: false },
            { tag: "database", args: ["lunchbox", "--database", "/tmp/probe.db"], expected: false },
            { tag: "output", args: ["lunchbox", "--screenshot-output", "/tmp/probe.png"], expected: false },
            { tag: "assignment", args: ["lunchbox", "--database=probe.db"], expected: false },
            { tag: "startup", args: ["lunchbox", "--startup-probe"], expected: true },
            { tag: "ui", args: ["lunchbox", "--controller-calibration-ui-probe"], expected: true }
        ]
    }

    function test_probe_switches(data) {
        compare(parser.isProbeRun(data.args), data.expected)
    }
}
