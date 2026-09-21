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
    required property string displayShader
    required property string displayBezel
    required property string displaySaveStates
    required property int displayRevision
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
            maximumLineCount: 3
            elide: Text.ElideRight
        }

        Column {
            width: parent.width
            visible: hero.emulatorOptionCount > 0
                     && (hero.displayShaderSupported
                         || hero.displayBezelSupported
                         || hero.displaySaveStatesSupported)
            spacing: 5
            RowLayout {
                width: parent.width
                spacing: 6
                Text {
                    text: "DISPLAY"
                    color: "#83e3ad"
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    font.letterSpacing: 0.8
                    Layout.fillWidth: true
                }
                Button {
                    text: "GAME"
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
                    text: "PLATFORM"
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
                        font.pixelSize: 9
                        textRole: "label"
                        valueRole: "value"
                        property bool syncing: false
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
                            syncing = true
                            currentIndex = indexOfValue(hero.displayShader)
                            if (currentIndex < 0)
                                currentIndex = 0
                            syncing = false
                        }
                        onActivated: function(index) {
                            hero.displaySettingSaved("shader", currentValue)
                        }
                        Connections {
                            target: hero
                            function onDisplayRevisionChanged() { displayShaderCombo.syncValue() }
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
                        font.pixelSize: 9
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
                        font.pixelSize: 9
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
                text: "PLAY WITH"
                color: "#83e3ad"
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            Text {
                visible: standaloneIndices.length > 0 && retroarchIndices.length > 0
                text: "STANDALONE"
                color: hero.muted
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            ComboBox {
                id: emulatorPicker
                width: parent.width
                height: 40
                visible: standaloneIndices.length > 0
                model: standaloneIndices.length
                currentIndex: standaloneIndices.indexOf(hero.selectedEmulatorOption)
                displayText: currentIndex >= 0
                             ? hero.emulatorLabelAt(standaloneIndices[currentIndex])
                             : "Choose an emulator"
                onActivated: function(index) { hero.emulatorSelected(standaloneIndices[index]) }
                delegate: ItemDelegate {
                    required property int index
                    width: emulatorPicker.width
                    text: hero.emulatorLabelAt(standaloneIndices[index])
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
                Accessible.name: "Select standalone emulator"
            }
            Text {
                visible: retroarchIndices.length > 0
                text: "RETROARCH CORES"
                color: hero.muted
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            ComboBox {
                id: corePicker
                width: parent.width
                height: 40
                visible: retroarchIndices.length > 0
                model: retroarchIndices.length
                currentIndex: retroarchIndices.indexOf(hero.selectedEmulatorOption)
                displayText: currentIndex >= 0
                             ? hero.emulatorLabelAt(retroarchIndices[currentIndex])
                             : "Choose a RetroArch core"
                onActivated: function(index) { hero.emulatorSelected(retroarchIndices[index]) }
                delegate: ItemDelegate {
                    required property int index
                    width: corePicker.width
                    text: hero.emulatorLabelAt(retroarchIndices[index])
                    font.pixelSize: 10
                    highlighted: corePicker.highlightedIndex === index
                }
                contentItem: Text {
                    leftPadding: 11
                    rightPadding: 30
                    text: corePicker.displayText
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
                Accessible.name: "Select RetroArch core"
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
            Row {
                width: parent.width
                spacing: 6
                Button {
                    width: (parent.width - 12) / 3
                    height: 32
                    text: "GAME DEFAULT"
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
                    text: "PLATFORM DEFAULT"
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
                    text: "RESET DEFAULT"
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
