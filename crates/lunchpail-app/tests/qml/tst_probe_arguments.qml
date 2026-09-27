import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "ProbeArguments"
    Lunchpail.ProbeArguments { id: parser }

    function test_probe_switches_data() {
        return [
            { tag: "normal", args: ["lunchpail", "--database", "/tmp/catalog.db"], expected: false },
            { tag: "executable", args: ["/tmp/probe/lunchpail"], expected: false },
            { tag: "database", args: ["lunchpail", "--database", "/tmp/probe.db"], expected: false },
            { tag: "output", args: ["lunchpail", "--screenshot-output", "/tmp/probe.png"], expected: false },
            { tag: "assignment", args: ["lunchpail", "--database=probe.db"], expected: false },
            { tag: "startup", args: ["lunchpail", "--startup-probe"], expected: true },
            { tag: "ui", args: ["lunchpail", "--controller-calibration-ui-probe"], expected: true }
        ]
    }

    function test_probe_switches(data) {
        compare(parser.isProbeRun(data.args), data.expected)
    }
}
