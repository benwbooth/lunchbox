pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: hero

    required property string gameId
    required property bool local
    required property bool loading
    required property bool canLaunch
    required property bool discoveryBusy
    required property bool launchBusy
    required property bool gameRunning
    required property bool sessionStopping
    required property string sessionTitle
    required property bool preparable
    required property bool prepareBusy
    required property string emulatorName
    required property string platform
    required property string launchStatus
    required property string preferenceScope
    required property int emulatorOptionCount
    required property int selectedEmulatorOption
    required property bool translationOptedIn
    required property bool translationFeatureEnabled
    required property int firmwareMissingCount
    required property string firmwareSetupLabel
    required property var emulatorLabelAt
    required property var emulatorOptionKindAt
    required property var emulatorOptionStarredAt
    readonly property var standaloneIndices: {
        const out = []
        for (let i = 0; i < emulatorOptionCount; ++i)
            if (emulatorOptionKindAt(i) === "standalone") out.push(i)
        return out
    }
    readonly property var retroarchIndices: {
        const out = []
        for (let i = 0; i < emulatorOptionCount; ++i)
            if (emulatorOptionKindAt(i) === "retroarch") out.push(i)
        return out
    }
    readonly property bool hasStarredOption: {
        for (let i = 0; i < emulatorOptionCount; ++i)
            if (emulatorOptionStarredAt(i)) return true
        return false
    }
    required property color ink
    required property color muted
    required property color line
    required property color accentCool
    required property string displayScope
    required property string displayFullscreen
    required property string displayShader
    required property string displayBezel
    required property string displaySaveStates
    required property string displayInheritedFullscreenLabel
    required property string displayInheritedShaderLabel
    required property string displayInheritedBezelLabel
    required property string displayInheritedSaveStatesLabel
    required property string displayEffectiveSummary
    required property int displayRevision
    required property bool displayFullscreenSupported
    required property bool displayShaderSupported
    required property bool displayBezelSupported
    required property bool displaySaveStatesSupported
    required property var displayShaderPresetCount
    required property var displayShaderPresetIdAt
    required property var displayShaderPresetLabelAt
    required property var displayBezelChoiceCount
    required property var displayBezelChoiceIdAt
    required property var displayBezelChoiceLabelAt
    required property var displayScopeSelected
    required property var displaySettingSaved
    required property var translationOptInSelected

    signal playRequested()
    signal controllerMappingRequested()
    signal cancelLaunchRequested()
    signal stopEmulatorRequested()
    signal setupRequested()
    signal prepareRequested()
    signal firmwareSetupRequested()
    signal manageEmulatorsRequested()
    signal emulatorSelected(int index)
    signal saveGameDefaultRequested()
    signal savePlatformDefaultRequested()
    signal clearDefaultRequested()

    readonly property bool prepareNeeded: !discoveryBusy && !prepareBusy && preparable
                                          && emulatorOptionCount === 0
    readonly property bool emulatorMissing: !discoveryBusy && !preparable
                                            && emulatorOptionCount === 0
    readonly property bool firmwareSetupNeeded: !discoveryBusy
                                                && firmwareMissingCount > 0
    readonly property bool displaySectionAvailable: emulatorOptionCount > 0
                                                     && (displayFullscreenSupported
                                                         || displayShaderSupported
                                                         || displayBezelSupported
                                                         || displaySaveStatesSupported)
    property bool settingsExpanded: false
    property bool displayExpanded: false
    property var modsBackend: null
    property var achievementsBackend: null
    signal achievementsSetupRequested()
    property var pickPatchFile: function() { return "" }
    property var pickCheatFile: function() { return "" }
    property var pickCheatExport: function() { return "" }
    onGameIdChanged: {
        settingsExpanded = false
        displayExpanded = false
    }

    visible: local && !loading
    implicitHeight: contents.implicitHeight + 28
    height: visible ? implicitHeight : 0
    radius: 12
    color: "#112d24"
    border.width: 2
    border.color: canLaunch ? "#43c981" : "#347259"

    Column {
        id: contents
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 10

        Row {
            width: parent.width
            spacing: 9
            Rectangle {
                width: 30
                height: 30
                radius: 15
                color: hero.canLaunch ? "#2cad6d" : "#255d47"
                SemanticIcon {
                    anchors.centerIn: parent
                    width: 19
                    height: 19
                    name: "play"
                    filled: true
                    color: "white"
                }
            }
            Column {
                width: parent.width - 39
                spacing: 2
                Text {
                    width: parent.width
                    text: hero.gameRunning ? "NOW PLAYING"
                          : hero.canLaunch ? "READY TO PLAY"
                          : hero.prepareNeeded ? "PREPARE TO PLAY"
                          : hero.emulatorMissing ? "EMULATOR NEEDED" : "SET UP PLAY"
                    color: hero.canLaunch ? "#83e3ad" : hero.accentCool
                    font.pixelSize: 12
                    font.weight: Font.Bold
                    font.letterSpacing: 0.9
                }
                Text {
                    width: parent.width
                    text: hero.discoveryBusy ? "Detecting installed emulators…"
                          : hero.prepareNeeded ? "Prepare this archived PC game to detect installed emulators"
                          : hero.emulatorName.length > 0 ? hero.emulatorName
                          : "Choose or install a compatible emulator"
                    color: hero.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
            }
        }

        Text {
            width: parent.width
            visible: hero.gameRunning && hero.sessionTitle.length > 0
            text: "Now playing: " + hero.sessionTitle
            color: hero.accentCool
            font.pixelSize: 10
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            visible: hero.launchStatus.length > 0
            text: hero.launchStatus
            color: hero.muted
            font.pixelSize: 9
            lineHeight: 1.2
            wrapMode: Text.WordWrap
            maximumLineCount: 6
            elide: Text.ElideRight
        }

        LbButton {
            id: launchAction
            objectName: "launchAction"
            width: parent.width
            height: 48
            text: hero.launchBusy ? "Cancel preparation"
                  : hero.gameRunning ? hero.sessionStopping ? "Stopping emulator…" : "■  Stop emulator"
                  : hero.canLaunch ? "▶  Play"
                  : hero.prepareBusy ? "Preparing install…"
                  : hero.prepareNeeded ? "Prepare install"
                  : hero.firmwareSetupNeeded ? hero.firmwareSetupLabel
                  : hero.emulatorMissing ? "Install an emulator" : "Recheck play setup"
            enabled: !hero.sessionStopping && (hero.launchBusy || hero.gameRunning
                     || (!hero.gameRunning && !hero.discoveryBusy && !hero.prepareBusy)
                     )
            font.pixelSize: 12
            font.weight: Font.Bold
            highlighted: hero.canLaunch && !hero.launchBusy && !hero.gameRunning
            positive: highlighted
            onClicked: {
                if (hero.launchBusy)
                    hero.cancelLaunchRequested()
                else if (hero.gameRunning)
                    hero.stopEmulatorRequested()
                else if (hero.canLaunch)
                    hero.playRequested()
                else if (hero.prepareNeeded)
                    hero.prepareRequested()
                else if (hero.firmwareSetupNeeded)
                    hero.firmwareSetupRequested()
                else
                    hero.setupRequested()
            }
        }

        LbButton {
            objectName: "settingsAccordionButton"
            width: parent.width
            text: (hero.settingsExpanded ? "▾  " : "▸  ") + "Settings & mappings"
            flat: true
            font.pixelSize: 11
            font.weight: Font.Bold
            onClicked: hero.settingsExpanded = !hero.settingsExpanded
            Accessible.name: "Settings and mappings"
            Accessible.description: hero.settingsExpanded
                                    ? "Collapse game settings and controller mapping"
                                    : "Expand game settings and controller mapping"
        }

        LbButton {
            objectName: "translationOptInButton"
            width: parent.width
            visible: hero.settingsExpanded && hero.selectedEmulatorOption >= 0
                     && hero.emulatorOptionKindAt(hero.selectedEmulatorOption) === "retroarch"
            text: !hero.translationFeatureEnabled
                  ? "Enable game translation in Settings"
                  : hero.translationOptedIn
                  ? "Translate this game: On"
                  : "Translate this game: Off"
            enabled: hero.translationFeatureEnabled && !hero.launchBusy && !hero.gameRunning
            highlighted: hero.translationOptedIn
            onClicked: hero.translationOptInSelected(!hero.translationOptedIn)
            Accessible.description: "Remembered for this game. Other games launch without translation."
        }

        RetroAchievementsPane {
            width: parent.width
            visible: hero.settingsExpanded && hero.achievementsBackend !== null
            backend: hero.achievementsBackend
            gameId: hero.gameId
            locked: hero.launchBusy || hero.gameRunning
            retroarch: hero.selectedEmulatorOption >= 0 && hero.emulatorOptionKindAt(hero.selectedEmulatorOption) === "retroarch"
            onSetupRequested: hero.achievementsSetupRequested()
        }

        GameModsPane {
            width: parent.width
            visible: hero.settingsExpanded && hero.modsBackend !== null
            backend: hero.modsBackend
            gameId: hero.gameId
            locked: hero.launchBusy || hero.gameRunning
            retroarch: hero.selectedEmulatorOption >= 0 && hero.emulatorOptionKindAt(hero.selectedEmulatorOption) === "retroarch"
            pickPatchFile: hero.pickPatchFile
            pickCheatFile: hero.pickCheatFile
            pickCheatExport: hero.pickCheatExport
        }

        Column {
            objectName: "displaySection"
            width: parent.width
            visible: hero.settingsExpanded && hero.displaySectionAvailable
            spacing: 5
            LbButton {
                objectName: "displayAccordionButton"
                width: parent.width
                text: (hero.displayExpanded ? "▾  " : "▸  ") + "Display settings"
                flat: true
                font.pixelSize: 11
                font.weight: Font.Bold
                onClicked: hero.displayExpanded = !hero.displayExpanded
                Accessible.name: "Display settings"
                Accessible.description: hero.displayExpanded ? "Collapse display settings" : "Expand display settings"
            }
            RowLayout {
                width: parent.width
                spacing: 6
                visible: hero.displayExpanded
                LbButton {
                    Layout.fillWidth: true
                    text: "This game"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    flat: true
                    leftPadding: 8
                    rightPadding: 8
                    topPadding: 3
                    bottomPadding: 3
                    highlighted: hero.displayScope === "game"
                    onClicked: hero.displayScopeSelected("game")
                }
                LbButton {
                    Layout.fillWidth: true
                    text: "This platform"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    flat: true
                    leftPadding: 8
                    rightPadding: 8
                    topPadding: 3
                    bottomPadding: 3
                    highlighted: hero.displayScope === "platform"
                    onClicked: hero.displayScopeSelected("platform")
                }
            }
            Column {
                width: parent.width
                spacing: 2
                visible: hero.displayExpanded && hero.displayFullscreenSupported
                Text {
                    text: "FULLSCREEN"
                    color: hero.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
                LbComboBox {
                    id: displayFullscreenCombo
                    objectName: "displayFullscreenCombo"
                    width: parent.width
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { value: "", label: hero.displayInheritedFullscreenLabel },
                        { value: "true", label: "On" },
                        { value: "false", label: "Off" }
                    ]
                    onModelChanged: syncValue()
                    Component.onCompleted: syncValue()
                    function syncValue() {
                        currentIndex = ["", "true", "false"].indexOf(hero.displayFullscreen)
                        if (currentIndex < 0)
                            currentIndex = 0
                    }
                    onActivated: hero.displaySettingSaved("fullscreen", currentValue)
                    Connections {
                        target: hero
                        function onDisplayRevisionChanged() { displayFullscreenCombo.syncValue() }
                    }
                }
                Text {
                    width: parent.width
                    visible: hero.displayFullscreen === "" || text.length > 28
                    text: displayFullscreenCombo.displayText
                    color: hero.muted
                    font.pixelSize: 8
                    wrapMode: Text.WordWrap
                }
            }
            Column {
                objectName: "displayOptions"
                width: parent.width
                spacing: 6
                visible: hero.displayExpanded
                Column {
                    width: parent.width
                    spacing: 2
                    visible: hero.displayShaderSupported
                    Text {
                        text: "CRT"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    LbComboBox {
                        id: displayShaderCombo
                        objectName: "displayShaderCombo"
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: {
                            const revision = hero.displayRevision
                            const items = [{ value: "", label: hero.displayInheritedShaderLabel }]
                            const count = hero.displayShaderPresetCount()
                            for (let i = 0; i < count; ++i)
                                items.push({
                                    value: hero.displayShaderPresetIdAt(i),
                                    label: hero.displayShaderPresetLabelAt(i)
                                })
                            return items
                        }
                        onModelChanged: displayShaderCombo.syncValue()
                        Component.onCompleted: displayShaderCombo.syncValue()
                        function syncValue() {
                            currentIndex = indexOfValue(hero.displayShader)
                            if (currentIndex < 0)
                                currentIndex = 0
                        }
                        onActivated: function(index) {
                            hero.displaySettingSaved("shader", currentValue)
                        }
                        Connections {
                            target: hero
                            function onDisplayRevisionChanged() { displayShaderCombo.syncValue() }
                        }
                        delegate: LbItemDelegate {
                            required property int index
                            width: displayShaderCombo.width
                            implicitHeight: Math.max(32, shaderChoiceText.implicitHeight + topPadding + bottomPadding)
                            highlighted: displayShaderCombo.highlightedIndex === index
                            contentItem: Text {
                                id: shaderChoiceText
                                text: displayShaderCombo.model[index].label
                                color: "#f4f7fb"
                                font.pixelSize: 10
                                wrapMode: Text.WordWrap
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                    }
                    Text {
                        objectName: "displayShaderFullValue"
                        width: parent.width
                        visible: hero.displayShader === "" || text.length > 28
                        text: displayShaderCombo.displayText
                        color: hero.muted
                        font.pixelSize: 8
                        wrapMode: Text.WordWrap
                    }
                }
                Column {
                    width: parent.width
                    spacing: 2
                    visible: hero.displayBezelSupported
                    Text {
                        text: "BEZEL"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    LbComboBox {
                        id: displayBezelCombo
                        objectName: "displayBezelCombo"
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: {
                            hero.displayRevision
                            const items = [
                                { value: "", label: hero.displayInheritedBezelLabel },
                                { value: "off", label: "Off" }
                            ]
                            for (let i = 0; i < hero.displayBezelChoiceCount(); ++i)
                                items.push({
                                    value: hero.displayBezelChoiceIdAt(i),
                                    label: hero.displayBezelChoiceLabelAt(i)
                                })
                            return items
                        }
                        onModelChanged: displayBezelCombo.syncValue()
                        Component.onCompleted: displayBezelCombo.syncValue()
                        function syncValue() {
                            currentIndex = indexOfValue(hero.displayBezel)
                            if (currentIndex < 0)
                                currentIndex = 0
                        }
                        onActivated: function(index) {
                            hero.displaySettingSaved("bezel", currentValue)
                        }
                        Connections {
                            target: hero
                            function onDisplayRevisionChanged() { displayBezelCombo.syncValue() }
                        }
                        delegate: LbItemDelegate {
                            required property int index
                            width: displayBezelCombo.width
                            implicitHeight: Math.max(32, bezelChoiceText.implicitHeight + topPadding + bottomPadding)
                            highlighted: displayBezelCombo.highlightedIndex === index
                            contentItem: Text {
                                id: bezelChoiceText
                                text: displayBezelCombo.model[index].label
                                color: "#f4f7fb"
                                font.pixelSize: 10
                                wrapMode: Text.WordWrap
                                verticalAlignment: Text.AlignVCenter
                            }
                        }
                    }
                    Text {
                        width: parent.width
                        visible: hero.displayBezel === "" || text.length > 28
                        text: displayBezelCombo.displayText
                        color: hero.muted
                        font.pixelSize: 8
                        wrapMode: Text.WordWrap
                    }
                    Text {
                        width: parent.width
                        visible: (hero.displayBezel === "ultrawide"
                                  || hero.displayBezel === "ultrawide-night")
                                 && hero.displayFullscreen === "false"
                        text: "21:9 art requires fullscreen"
                        color: hero.muted
                        font.pixelSize: 8
                        wrapMode: Text.WordWrap
                    }
                }
                Column {
                    width: parent.width
                    spacing: 2
                    visible: hero.displaySaveStatesSupported
                    Text {
                        text: "SAVE STATES"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    LbComboBox {
                        id: displaySaveStatesCombo
                        objectName: "displaySaveStatesCombo"
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { value: "", label: hero.displayInheritedSaveStatesLabel },
                            { value: "off", label: "Off" },
                            { value: "on", label: "Save + resume" }
                        ]
                        onModelChanged: displaySaveStatesCombo.syncValue()
                        Component.onCompleted: displaySaveStatesCombo.syncValue()
                        function syncValue() {
                            currentIndex = ["", "off", "on"].indexOf(hero.displaySaveStates)
                            if (currentIndex < 0)
                                currentIndex = 0
                        }
                        onActivated: function(index) {
                            hero.displaySettingSaved("save_states", currentValue)
                        }
                        Connections {
                            target: hero
                            function onDisplayRevisionChanged() { displaySaveStatesCombo.syncValue() }
                        }
                        delegate: LbItemDelegate {
                            required property int index
                            width: displaySaveStatesCombo.width
                            text: displaySaveStatesCombo.model[index].label
                            font.pixelSize: 10
                            highlighted: displaySaveStatesCombo.highlightedIndex === index
                        }
                    }
                    Text {
                        width: parent.width
                        visible: hero.displaySaveStates === "" || text.length > 28
                        text: displaySaveStatesCombo.displayText
                        color: hero.muted
                        font.pixelSize: 8
                        wrapMode: Text.WordWrap
                    }
                }
            }
            Text {
                width: parent.width
                visible: hero.displayExpanded
                text: hero.displayScope === "game"
                      ? "Display choices saved for this game; empty values inherit the platform profile."
                      : "Display choices saved for " + hero.platform + "; empty values inherit the global profile."
                color: hero.muted
                font.pixelSize: 8
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                visible: hero.displayExpanded && hero.displayBezelSupported
                text: "System pack uses matching per-game artwork when available, otherwise a system bezel."
                color: hero.muted
                font.pixelSize: 8
                wrapMode: Text.WordWrap
            }
            Text {
                objectName: "displayEffectiveSummary"
                width: parent.width
                visible: hero.displayExpanded && hero.displayEffectiveSummary.length > 0
                text: hero.displayEffectiveSummary
                color: hero.muted
                font.pixelSize: 8
                font.italic: true
                wrapMode: Text.WordWrap
            }
        }

        Column {
            objectName: "emulatorSection"
            width: parent.width
            // The emulator choice must stay reachable even when the selected
            // emulator has no display features: displaySection above hides in
            // that case, and nesting the picker inside it left no way to pick
            // a different emulator (e.g. a standalone auto-pick on SNES).
            visible: hero.settingsExpanded && hero.emulatorOptionCount > 0
            spacing: 5
            Text {
                text: "PLAY WITH"
                color: "#83e3ad"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            LbComboBox {
                id: emulatorPicker
                objectName: "emulatorPicker"
                width: parent.width
                height: 40
                textRole: "label"
                valueRole: "index"
                model: {
                    // Two explicit passes keep each section contiguous for
                    // the popup's section headers: standalone, then cores.
                    const items = []
                    for (let pass = 0; pass < 2; ++pass) {
                        const wanted = pass === 0 ? "standalone" : "retroarch"
                        for (let i = 0; i < hero.emulatorOptionCount; ++i) {
                            if (hero.emulatorOptionKindAt(i) !== wanted)
                                continue
                            items.push({
                                index: i,
                                kind: wanted === "retroarch"
                                      ? "RetroArch cores" : "Standalone",
                                label: hero.emulatorLabelAt(i)
                            })
                        }
                    }
                    return items
                }
                displayText: currentIndex >= 0 && currentIndex < model.length
                             ? model[currentIndex].label : "Choose an emulator"
                onActivated: function(activatedIndex) {
                    hero.emulatorSelected(model[activatedIndex].index)
                }
                function syncSelection() {
                    for (let i = 0; i < model.length; ++i) {
                        if (model[i].index === hero.selectedEmulatorOption) {
                            currentIndex = i
                            return
                        }
                    }
                    currentIndex = -1
                }
                onModelChanged: emulatorPicker.syncSelection()
                Component.onCompleted: emulatorPicker.syncSelection()
                Connections {
                    target: hero
                    function onSelectedEmulatorOptionChanged() { emulatorPicker.syncSelection() }
                }
                delegate: LbItemDelegate {
                    required property int index
                    width: ListView.view ? (ListView.view.verticalContentWidth || ListView.view.width) : emulatorPicker.width
                    height: 34
                    text: emulatorPicker.model[index].label
                    font.pixelSize: 10
                    highlighted: emulatorPicker.highlightedIndex === index
                }
                popup: Popup {
                    y: emulatorPicker.height - 1
                    width: emulatorPicker.width
                    height: Math.min(contentItem.implicitHeight + 2, 480)
                    padding: 1
                    contentItem: MomentumListView {
                        clip: true
                        implicitHeight: contentHeight
                        model: emulatorPicker.popup.visible
                               ? emulatorPicker.model : null
                        currentIndex: emulatorPicker.highlightedIndex
                        section.property: "kind"
                        section.delegate: Text {
                            required property string section
                            text: section
                            topPadding: 6
                            leftPadding: 10
                            bottomPadding: 2
                            color: "#ffb454"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                        delegate: LbItemDelegate {
                            required property int index
                            width: ListView.view ? (ListView.view.verticalContentWidth || ListView.view.width) : 0
                            height: 32
                            text: emulatorPicker.model[index].label
                            font.pixelSize: 10
                            highlighted: emulatorPicker.highlightedIndex === index
                                             || emulatorPicker.currentIndex === index
                            onClicked: {
                                emulatorPicker.currentIndex = index
                                emulatorPicker.popup.close()
                                emulatorPicker.activated(index)
                            }
                        }
                    }
                    background: Rectangle {
                        color: "#151d29"
                        border.color: "#53647c"
                        radius: 8
                    }
                }
                Accessible.name: "Select emulator"
            }
            Text {
                visible: hero.hasStarredOption
                width: parent.width
                text: "★ Recommended · emulator order: Emulation General Wiki (CC BY-SA)"
                color: hero.muted
                font.pixelSize: 8
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                text: hero.preferenceScope === "game"
                      ? "Game default · choose another here for a one-off launch"
                      : hero.preferenceScope === "platform"
                        ? hero.platform + " default · choose another here for a one-off launch"
                        : "This choice is for the next launch only unless you save a default"
                color: hero.muted
                font.pixelSize: 9
                wrapMode: Text.WordWrap
            }
            Text {
                text: "EMULATOR DEFAULT"
                color: "#83e3ad"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            Row {
                width: parent.width
                spacing: 6
                LbButton {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "This game"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    highlighted: hero.preferenceScope === "game"
                    onClicked: hero.saveGameDefaultRequested()
                }
                LbButton {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "This platform"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    highlighted: hero.preferenceScope === "platform"
                    onClicked: hero.savePlatformDefaultRequested()
                }
                LbButton {
                    width: (parent.width - 12) / 3
                    height: 32
                    visible: hero.preferenceScope.length > 0
                    text: "Reset"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.clearDefaultRequested()
                }
            }
        }

        LbButton {
            objectName: "controllerMappingButton"
            width: parent.width
            visible: hero.settingsExpanded
            text: "Controller mapping…"
            onClicked: hero.controllerMappingRequested()
        }

        LbButton {
            objectName: "manageEmulatorsButton"
            width: parent.width
            height: 32
            visible: hero.settingsExpanded && !hero.gameRunning
            text: hero.platform.length > 0
                  ? "Manage " + hero.platform + " emulators"
                  : "Manage emulators"
            font.pixelSize: 9
            font.weight: Font.Bold
            onClicked: hero.manageEmulatorsRequested()
        }
    }
}
