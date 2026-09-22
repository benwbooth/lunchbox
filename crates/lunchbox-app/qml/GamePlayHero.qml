pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: hero

    required property bool local
    required property bool loading
    required property bool canLaunch
    required property bool discoveryBusy
    required property bool launchBusy
    required property bool gameRunning
    required property bool preparable
    required property bool prepareBusy
    required property string emulatorName
    required property string platform
    required property string launchStatus
    required property string preferenceScope
    required property int emulatorOptionCount
    required property int selectedEmulatorOption
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
    required property string displayEffectiveSummary
    required property int displayRevision
    required property bool displayFullscreenSupported
    required property bool displayShaderSupported
    required property bool displayBezelSupported
    required property bool displaySaveStatesSupported
    required property var displayShaderPresetCount
    required property var displayShaderPresetIdAt
    required property var displayShaderPresetLabelAt
    required property var displayScopeSelected
    required property var displaySettingSaved

    signal playRequested()
    signal controllerMappingRequested()
    signal cancelLaunchRequested()
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
            visible: hero.launchStatus.length > 0
            text: hero.launchStatus
            color: hero.muted
            font.pixelSize: 9
            lineHeight: 1.2
            wrapMode: Text.WordWrap
            maximumLineCount: 6
            elide: Text.ElideRight
        }

        Column {
            objectName: "displaySection"
            width: parent.width
            visible: hero.displaySectionAvailable
            spacing: 5
            RowLayout {
                width: parent.width
                spacing: 6
                Text {
                    text: "DISPLAY SETTINGS"
                    color: "#83e3ad"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                    Layout.fillWidth: true
                }
                Button {
                    text: "THIS GAME"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    flat: true
                    leftPadding: 8
                    rightPadding: 8
                    topPadding: 3
                    bottomPadding: 3
                    onClicked: hero.displayScopeSelected("game")
                    background: Rectangle {
                        radius: 6
                        color: hero.displayScope === "game" ? "#245b45" : "#12241c"
                        border.color: hero.displayScope === "game" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text {
                        text: parent.text
                        color: hero.displayScope === "game" ? "#83e3ad" : hero.muted
                        font: parent.font
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
                Button {
                    text: "THIS PLATFORM"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    flat: true
                    leftPadding: 8
                    rightPadding: 8
                    topPadding: 3
                    bottomPadding: 3
                    onClicked: hero.displayScopeSelected("platform")
                    background: Rectangle {
                        radius: 6
                        color: hero.displayScope === "platform" ? "#245b45" : "#12241c"
                        border.color: hero.displayScope === "platform" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text {
                        text: parent.text
                        color: hero.displayScope === "platform" ? "#83e3ad" : hero.muted
                        font: parent.font
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
            Column {
                width: (parent.width - 12) / 3
                spacing: 2
                visible: hero.displayFullscreenSupported
                Text {
                    text: "FULLSCREEN"
                    color: hero.muted
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
                ComboBox {
                    id: displayFullscreenCombo
                    objectName: "displayFullscreenCombo"
                    width: parent.width
                    textRole: "label"
                    valueRole: "value"
                    model: [
                        { value: "", label: "Inherit" },
                        { value: "true", label: "On" },
                        { value: "false", label: "Off" }
                    ]
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
                    background: Rectangle {
                        radius: 8
                        color: "#0d211a"
                        border.width: 2
                        border.color: "#43a876"
                    }
                    contentItem: Text {
                        leftPadding: 11
                        rightPadding: 30
                        text: displayFullscreenCombo.displayText
                        color: hero.ink
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                }
            }
            Row {
                width: parent.width
                spacing: 6
                Column {
                    width: (parent.width - 12) / 3
                    spacing: 2
                    visible: hero.displayShaderSupported
                    Text {
                        text: "CRT"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    ComboBox {
                        id: displayShaderCombo
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: {
                            const revision = hero.displayRevision
                            const items = [{ value: "", label: "Inherit" }]
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
background: Rectangle {
                            radius: 8
                            color: "#0d211a"
                            border.width: 2
                            border.color: "#43a876"
                        }
                        contentItem: Text {
                            leftPadding: 11
                            rightPadding: 30
                            text: displayShaderCombo.displayText
                            color: hero.ink
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        delegate: ItemDelegate {
                            required property int index
                            width: displayShaderCombo.width
                            text: displayShaderCombo.model[index].label
                            font.pixelSize: 10
                            highlighted: displayShaderCombo.highlightedIndex === index
                        }
                    }
                }
                Column {
                    width: (parent.width - 12) / 3
                    spacing: 2
                    visible: hero.displayBezelSupported
                    Text {
                        text: "BEZEL"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    ComboBox {
                        id: displayBezelCombo
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { value: "", label: "Inherit" },
                            { value: "off", label: "Off" },
                            { value: "system", label: "System pack" }
                        ]
                        onModelChanged: displayBezelCombo.syncValue()
                        Component.onCompleted: displayBezelCombo.syncValue()
                        function syncValue() {
                            currentIndex = ["", "off", "system"].indexOf(hero.displayBezel)
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
background: Rectangle {
                            radius: 8
                            color: "#0d211a"
                            border.width: 2
                            border.color: "#43a876"
                        }
                        contentItem: Text {
                            leftPadding: 11
                            rightPadding: 30
                            text: displayBezelCombo.displayText
                            color: hero.ink
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        delegate: ItemDelegate {
                            required property int index
                            width: displayBezelCombo.width
                            text: displayBezelCombo.model[index].label
                            font.pixelSize: 10
                            highlighted: displayBezelCombo.highlightedIndex === index
                        }
                    }
                }
                Column {
                    width: (parent.width - 12) / 3
                    spacing: 2
                    visible: hero.displaySaveStatesSupported
                    Text {
                        text: "SAVE STATES"
                        color: hero.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.7
                    }
                    ComboBox {
                        id: displaySaveStatesCombo
                        width: parent.width
                        textRole: "label"
                        valueRole: "value"
                        model: [
                            { value: "", label: "Inherit" },
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
background: Rectangle {
                            radius: 8
                            color: "#0d211a"
                            border.width: 2
                            border.color: "#43a876"
                        }
                        contentItem: Text {
                            leftPadding: 11
                            rightPadding: 30
                            text: displaySaveStatesCombo.displayText
                            color: hero.ink
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        delegate: ItemDelegate {
                            required property int index
                            width: displaySaveStatesCombo.width
                            text: displaySaveStatesCombo.model[index].label
                            font.pixelSize: 10
                            highlighted: displaySaveStatesCombo.highlightedIndex === index
                        }
                    }
                }
            }
            Text {
                width: parent.width
                text: hero.displayScope === "game"
                      ? "Display choices saved for this game; empty values inherit the platform profile."
                      : "Display choices saved for " + hero.platform + "; empty values inherit the global profile."
                color: hero.muted
                font.pixelSize: 8
                wrapMode: Text.WordWrap
            }
            Text {
                width: parent.width
                visible: hero.displayBezelSupported
                text: "System pack uses matching per-game artwork when available, otherwise a system bezel."
                color: hero.muted
                font.pixelSize: 8
                wrapMode: Text.WordWrap
            }
            Text {
                objectName: "displayEffectiveSummary"
                width: parent.width
                visible: hero.displayEffectiveSummary.length > 0
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
            visible: hero.emulatorOptionCount > 0
            spacing: 5
            Text {
                text: "PLAY WITH"
                color: "#83e3ad"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            ComboBox {
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
                delegate: ItemDelegate {
                    required property int index
                    width: ListView.view ? ListView.view.width : emulatorPicker.width
                    height: 34
                    text: emulatorPicker.model[index].label
                    font.pixelSize: 10
                    highlighted: emulatorPicker.highlightedIndex === index
                }
                contentItem: Text {
                    leftPadding: 11
                    rightPadding: 30
                    text: emulatorPicker.displayText
                    color: hero.ink
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    verticalAlignment: Text.AlignVCenter
                    elide: Text.ElideRight
                }
                background: Rectangle {
                    radius: 8
                    color: "#0d211a"
                    border.width: 2
                    border.color: "#43a876"
                }
                popup: Popup {
                    y: emulatorPicker.height - 1
                    width: emulatorPicker.width
                    height: Math.min(contentItem.implicitHeight + 2, 480)
                    padding: 1
                    contentItem: ListView {
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
                            color: "#83e3ad"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                        delegate: ItemDelegate {
                            required property int index
                            width: ListView.view ? ListView.view.width : 0
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
                        color: "#0d211a"
                        border.color: "#43a876"
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
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "THIS GAME"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.saveGameDefaultRequested()
                    background: Rectangle {
                        radius: 7
                        color: hero.preferenceScope === "game" ? "#245b45" : "#173a2d"
                        border.color: hero.preferenceScope === "game" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text { text: parent.text; color: hero.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "THIS PLATFORM"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.savePlatformDefaultRequested()
                    background: Rectangle {
                        radius: 7
                        color: hero.preferenceScope === "platform" ? "#245b45" : "#173a2d"
                        border.color: hero.preferenceScope === "platform" ? "#75e2a5" : "#347259"
                    }
                    contentItem: Text { text: parent.text; color: hero.ink; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    visible: hero.preferenceScope.length > 0
                    text: "RESET"
                    font.pixelSize: 8
                    font.weight: Font.Bold
                    onClicked: hero.clearDefaultRequested()
                    background: Rectangle { radius: 7; color: "#173a2d"; border.color: "#347259" }
                    contentItem: Text { text: parent.text; color: hero.muted; font: parent.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
                }
            }
        }

        Button {
            width: parent.width
            text: "Controller mapping…"
            onClicked: hero.controllerMappingRequested()
        }

        Button {
            id: launchAction
            objectName: "launchAction"
            width: parent.width
            height: 48
            text: hero.launchBusy ? "CANCEL PREPARATION"
                  : hero.gameRunning ? "GAME IS RUNNING"
                  : hero.canLaunch ? "▶  PLAY"
                  : hero.prepareBusy ? "PREPARING INSTALL…"
                  : hero.prepareNeeded ? "PREPARE INSTALL"
                  : hero.firmwareSetupNeeded ? hero.firmwareSetupLabel
                  : hero.emulatorMissing ? "INSTALL AN EMULATOR" : "RECHECK PLAY SETUP"
            enabled: hero.launchBusy
                     || (!hero.gameRunning && !hero.discoveryBusy && !hero.prepareBusy)
            font.pixelSize: 12
            font.weight: Font.Bold
            onClicked: {
                if (hero.launchBusy)
                    hero.cancelLaunchRequested()
                else if (hero.canLaunch)
                    hero.playRequested()
                else if (hero.prepareNeeded)
                    hero.prepareRequested()
                else if (hero.firmwareSetupNeeded)
                    hero.firmwareSetupRequested()
                else
                    hero.setupRequested()
            }
            background: Rectangle {
                radius: 9
                color: parent.enabled
                       ? (hero.launchBusy
                          ? (parent.down ? "#8f5228" : "#ba6c32")
                          : (parent.down ? "#238153" : "#2cad6d"))
                       : "#244337"
                border.color: parent.enabled
                              ? (hero.launchBusy ? "#f0ac65" : "#75e2a5")
                              : hero.line
            }
            contentItem: Text {
                text: parent.text
                color: parent.enabled ? "white" : hero.muted
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }

        Button {
            width: parent.width
            height: 32
            visible: !hero.gameRunning
            text: hero.platform.length > 0
                  ? "MANAGE " + hero.platform.toUpperCase() + " EMULATORS"
                  : "MANAGE EMULATORS"
            font.pixelSize: 9
            font.weight: Font.Bold
            onClicked: hero.manageEmulatorsRequested()
            background: Rectangle {
                radius: 8
                color: parent.down ? "#214636" : "#173a2d"
                border.color: "#347259"
            }
            contentItem: Text {
                text: parent.text
                color: hero.accentCool
                font: parent.font
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
        }
    }
}
