import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: coverage
    required property var settingsModel
    property var recordSlugs: ["blastem","bizhawk","duckstation","desmume","dolphin","flycast","gearcoleco","gopher64","hatari","jgenesis","mame","mednafen","melonds","mgba","nestopia-ue","openmsx","pcsx2","ppsspp","punes","retroarch","rmg","rpcs3","scummvm","simple64","stella","vice","xemu"]
    property var report: ({})
    property string selectionStatus: ""
    signal nativeRuntimeRequested()
    signal perGameSetupRequested(string core)
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1040, parent ? parent.width - 32 : 1040)
    height: Math.min(780, parent ? parent.height - 32 : 780)
    modal: true
    title: "Controller mapping coverage — implemented and remaining"
    standardButtons: Dialog.Close
    onOpened: reload()

    function reload() {
        report = JSON.parse(settingsModel.controller_coverage_json())
    }
    function statusName(status) {
        if (status === "per_game") return "Per-game adapter code — not universal or runtime verified"
        if (status === "limited") return "Launch adapter: listed modes only"
        if (status === "explicit") return "Launch modes implemented — explicit selection required"
        if (status === "preview") return "Mapping exists; no matching platform adapter"
        if (status === "in_progress") return "Native adapter in progress — not counted yet"
        return "Missing launch support"
    }
    readonly property var filteredRows: {
        const query = search.text.toLowerCase().trim()
        return (report.rows || []).filter(row => {
            const implemented = row.status === "limited" || row.status === "explicit"
            if (filter.currentIndex === 1 && implemented) return false
            if (filter.currentIndex === 2 && !implemented && row.status !== "per_game") return false
            if (filter.currentIndex === 3 && row.kind !== "RetroArch") return false
            if (filter.currentIndex === 4 && row.kind === "RetroArch") return false
            return [row.name, row.platform, row.kind, row.detail || "",
                (row.implemented || []).join(" "), (row.remaining || []).join(" ")]
                .join(" ").toLowerCase().includes(query)
        })
    }

    contentItem: ColumnLayout {
        spacing: 10
        Label {
            Layout.fillWidth: true
            visible: !coverage.report.error
            text: "RetroArch static-profile coverage: " + (coverage.report.cores_with_contracts || 0) + "/" + (coverage.report.core_count || 0)
                + " cores (" + (coverage.report.core_entry_percent || 0).toFixed(1)
                + "%; not overall completion). Per-game adapters tracked separately: "
                + (coverage.report.dynamic_core_adapters || []).join(", ")
                + ". These adapters are not claims of complete game/peripheral support."
            font.bold: true
            wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true
            visible: !coverage.report.error
            text: (coverage.report.automatic_core_count || 0) + " cores have automatic default modes; "
                + (coverage.report.explicit_only_core_count || 0) + " require an explicit target mode. " + coverage.selectionStatus
            wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true
            visible: !coverage.report.error
            text: "RetroArch core/platform pairs: " + (coverage.report.core_platforms_with_contracts || 0)
                + "/" + (coverage.report.core_platform_count || 0) + " have a matching launch mode ("
                + (coverage.report.core_platform_percent || 0).toFixed(1) + "%); "
                + (coverage.report.core_platforms_without_contracts || 0) + " have none. "
                + "A contract for a different platform does not count here. Matching is not all-mode or runtime verification."
            wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true
            visible: !coverage.report.error
            text: "Standalone source adapters: " + (coverage.report.standalone_with_dispatch || 0)
                + "/" + (coverage.report.standalone_count || 0) + " catalog candidates ("
                + (coverage.report.standalone_entry_percent || 0).toFixed(1) + "%). "
                + "Combined core/native source coverage: " + (coverage.report.source_entries_with_dispatch || 0)
                + "/" + (coverage.report.source_entry_count || 0) + " (" + (coverage.report.source_entry_percent || 0).toFixed(1)
                + "%). These count partial implementations, not finished or tested support."
            wrapMode: Text.WordWrap
        }
        Label {
            Layout.fillWidth: true
            text: coverage.report.error || "Coverage is implementation status, not an installed-core or runtime result. Automatic adapters currently require Linux RetroArch. An input profile describes target controls, RetroPad bindings and mode options; it is not a separate service. A supported core does not imply every peripheral, game or mode is covered."
            color: "#ffb454"
            wrapMode: Text.WordWrap
        }
        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: "All cores / emulators" }
            TabButton { text: "Core input profiles / layout capabilities" }
        }
        RowLayout {
            visible: tabs.currentIndex === 0
            Layout.fillWidth: true
            TextField {
                id: search
                Layout.fillWidth: true
                placeholderText: "Find a core, emulator or platform…"
                Accessible.name: "Search controller coverage"
            }
            ComboBox {
                id: filter
                model: ["All coverage", "Incomplete / preview", "Some launch support", "RetroArch", "Standalone"]
                Accessible.name: "Controller coverage filter"
            }
            Button { text: "Refresh"; onClicked: coverage.reload() }
        }
        Label {
            visible: tabs.currentIndex === 0
            text: coverage.filteredRows.length + " platform-specific entries. Counts above deduplicate core identities and emulator definitions."
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }
        ListView {
            visible: tabs.currentIndex === 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 12
            model: coverage.filteredRows
            ScrollBar.vertical: ScrollBar { }
            delegate: Column {
                id: delegateRoot
                required property var modelData
                property bool expandedLocations: false
                property string locationText: ""
                width: ListView.view.width - 20
                spacing: 4
                Label {
                    width: parent.width
                    text: modelData.name + " · " + modelData.platform
                    font.bold: true
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    text: modelData.kind + " — " + coverage.statusName(modelData.status)
                    color: modelData.status === "limited" ? "#62d6c6" : "#ffb454"
                    wrapMode: Text.WordWrap
                }
                Label { width: parent.width; text: modelData.detail; wrapMode: Text.WordWrap }
                Label {
                    width: parent.width
                    visible: !!modelData.implemented
                    text: modelData.implemented ? "Implemented code (unverified):\n• " + modelData.implemented.join("\n• ") : ""
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    visible: !!modelData.remaining
                    text: modelData.remaining ? "Remaining:\n• " + modelData.remaining.join("\n• ") : ""
                    wrapMode: Text.WordWrap
                }
                Button {
                    visible: coverage.recordSlugs.indexOf(modelData.name.toLowerCase()) >= 0
                              || coverage.recordSlugs.indexOf(modelData.name.toLowerCase().replace(/\s+/g, "-")) >= 0
                    text: expandedLocations ? "Hide captured locations" : "Show captured locations"
                    onClicked: {
                        expandedLocations = !expandedLocations
                        if (expandedLocations && locationText.length === 0) {
                            const slug = coverage.recordSlugs.find(s => s === modelData.name.toLowerCase())
                                         || modelData.name.toLowerCase().replace(/\s+/g, "-")
                            locationText = settingsModel.emulator_platform_locations_json(slug)
                        }
                    }
                }
                Label {
                    width: parent.width
                    visible: expandedLocations
                    text: {
                        if (!expandedLocations || locationText.length === 0) return ""
                        try {
                            const parsed = JSON.parse(locationText)
                            if (parsed.error) return parsed.error
                            return parsed.locations.map(function(loc) {
                                const resolved = loc.resolved ? "→ " + loc.resolved : "(documented only)"
                                return loc.platform + " / " + loc.purpose + ": " + loc.documented + " " + resolved +
                                       (loc.naming ? "\n    naming: " + loc.naming : "")
                            }).join("\n")
                        } catch (e) { return locationText }
                    }
                    wrapMode: Text.WordWrap
                    font.family: "monospace"
                }
                Button {
                    visible: modelData.native_setup === true && Qt.platform.os === "linux"
                    text: "Configure native runtime and players…"
                    onClicked: { coverage.close(); coverage.nativeRuntimeRequested() }
                }
                Button {
                    visible: !!modelData.per_game_setup && Qt.platform.os === "linux"
                    text: "Open " + modelData.name + " per-game setup…"
                    onClicked: { coverage.close(); coverage.perGameSetupRequested(modelData.per_game_setup) }
                }
                Label {
                    width: parent.width
                    visible: modelData.profiles.length > 0
                    text: "Modes: " + modelData.profiles.join(", ")
                    wrapMode: Text.WordWrap
                }
                Label {
                    visible: modelData.choices.length > 1
                    width: parent.width
                    text: "Default target mode for this core/platform (not a physical-controller mapping):"
                    wrapMode: Text.WordWrap
                }
                ComboBox {
                    visible: modelData.choices.length > 1
                    width: parent.width
                    model: modelData.choices
                    textRole: "name"
                    currentIndex: Math.max(0, modelData.choices.findIndex(choice => choice.id === (modelData.selected || "")))
                    onActivated: {
                        coverage.selectionStatus = coverage.settingsModel.choose_controller_launch_mode(
                            modelData.name, modelData.platform, modelData.choices[currentIndex].id)
                        coverage.reload()
                    }
                    Accessible.name: "Target input mode for " + modelData.name + " " + modelData.platform
                }
            }
        }
        Label {
            visible: tabs.currentIndex === 1
            Layout.fillWidth: true
            text: "One source layout → shared capability rules → target layout → core input profile. Below assumes every declared source control is calibrated; your saved calibration may contain fewer. Missing hardware is not a missing emulator adapter. No controller/core pair-specific backend is needed."
            wrapMode: Text.WordWrap
        }
        ComboBox {
            id: sourceLayout
            visible: tabs.currentIndex === 1
            Layout.fillWidth: true
            model: coverage.report.profiles && coverage.report.profiles.length ? coverage.report.profiles[0].layouts : []
            textRole: "name"
            Accessible.name: "Source layout for capability coverage"
        }
        ListView {
            visible: tabs.currentIndex === 1
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 16
            model: coverage.report.profiles || []
            ScrollBar.vertical: ScrollBar { }
            delegate: Column {
                required property var modelData
                readonly property var capability: modelData.layouts[sourceLayout.currentIndex]
                property bool showMapping: false
                width: ListView.view.width - 20
                spacing: 4
                Label { width: parent.width; text: modelData.name; font.bold: true; wrapMode: Text.WordWrap }
                Label {
                    width: parent.width
                    text: modelData.launch ? "Linux RetroArch adapter · device " + modelData.launch.device
                        + " · up to " + modelData.launch.max_players + " players / " + modelData.frontend_ports + " frontend ports"
                        + (modelData.explicit_selection ? " · explicit mode selection required" : "")
                        : modelData.guided_native && modelData.native_launch
                            ? "Guided native mapping · up to " + modelData.native_launch.max_players
                                + " players · runtime/backend requirements apply; compatibility unverified"
                        : "Preview only — automatic launch adapter not enabled"
                    color: modelData.launch ? "#62d6c6" : "#ffb454"
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    text: "Target: " + modelData.target + " · transport: " + modelData.transport
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    text: !parent.capability ? "Choose a source layout" : parent.capability.missing.length
                        ? "Missing required capabilities: " + parent.capability.missing.join("; ")
                        : "Layout can supply required controls under shared rules; actual calibration and launch readiness still apply."
                    color: parent.capability && parent.capability.missing.length ? "#ffb454" : "#95a2b6"
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    text: "Conditions: " + modelData.conditions.join(" · ")
                    wrapMode: Text.WordWrap
                }
                Button {
                    text: parent.showMapping ? "Hide composed mapping" : "Show composed mapping"
                    onClicked: parent.showMapping = !parent.showMapping
                }
                Label {
                    width: parent.width
                    visible: parent.showMapping
                    text: "Physical layout control → target control → output binding\n"
                        + (parent.capability ? parent.capability.mapping.join("\n") : "")
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label {
                    width: parent.width
                    text: "Profile source: " + modelData.source
                    textFormat: Text.PlainText
                    wrapMode: Text.WrapAnywhere
                }
            }
        }
    }
}
