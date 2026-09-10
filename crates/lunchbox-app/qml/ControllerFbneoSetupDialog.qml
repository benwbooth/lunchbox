import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog
    required property var settingsModel
    property var setups: []
    property string selectedKey: ""
    property string loadedText: ""
    property string statusText: ""
    property var assignmentRows: []
    property var assignmentErrorPorts: []
    property var staleAssignments: []
    onStaleAssignmentsChanged: staleChoice.currentIndex = -1
    property string assignmentError: ""
    readonly property int mappingRevision: settingsModel.controller_revision
    onMappingRevisionChanged: {
        absoluteSelection.close()
        if (assignmentRows.length > 0) {
            assignmentRows = []
            staleAssignments = []
            assignmentErrorPorts = []
            assignmentError = "Controller settings or inventory changed. Reopen assignment review to resolve current saved calibrations."
            assignmentEditor.close()
            mappingPreview.close()
            mouseDestination.close()
            statusText = assignmentError + " Your draft is unchanged."
        }
        if (reportImport.visible) reportImport.refreshSourceControllers()
    }
    readonly property bool draftChanged: editor.text !== loadedText
    readonly property bool keyboardPassthroughActive: {
        try { return JSON.parse(editor.text).keyboard_passthrough_port !== undefined
            && JSON.parse(editor.text).keyboard_passthrough_port !== null }
        catch (error) { return false }
    }
    function setKeyboardPassthrough(enabled) {
        try {
            const draft = JSON.parse(editor.text)
            if (enabled) {
                const review = JSON.parse(dialog.settingsModel.fbneo_assignment_review_json(editor.text))
                if (review.error) throw new Error(review.error)
                const canPass = address => (review.targets || []).some(row => sameAddress(row.target.address, address)
                    && row.keyboard_passthrough_supported === true)
                const targets = (draft.expected_queries || []).concat((draft.expected_descriptors || []).map(item => item.address))
                const keys = targets.filter(address => address.device === 3)
                if (!keys.length || keys.some(address => address.port !== keys[0].port))
                    throw new Error("Passthrough requires one inspected native keyboard port.")
                const port = keys[0].port
                const player = draft.players.find(player => player.port === port && player.device === 3)
                if (!player) throw new Error("The native keyboard device must already be selected.")
                draft.keyboard_passthrough_port = port
                draft.keyboard_bindings = (draft.keyboard_bindings || []).filter(key => !canPass(key.target))
                player.assignments = player.assignments.filter(assignment => !canPass(assignment.target))
                if (targets.filter(address => address.port === port).every(canPass))
                    player.controller_id = ""
            } else {
                draft.keyboard_passthrough_port = null
            }
            editor.text = JSON.stringify(draft, null, 2)
            statusText = enabled ? "Frontend keyboard selected in draft. Keys absent from its key table still require explicit gamepad mappings. Stage and save to keep it."
                : "Keyboard passthrough disabled in draft. Select a physical controller and map all required keys before staging."
        } catch (error) { assignmentError = "Cannot change keyboard input: " + error }
    }
    readonly property var keyboardChannels: ["B", "Y", "Select", "Start", "Up", "Down", "Left", "Right", "A", "X", "L", "R", "L2", "R2", "L3", "R3"]
    function assignKeyboardChannel(address, channel) {
        try {
            const draft = JSON.parse(editor.text)
            const keys = (draft.keyboard_bindings || []).filter(key => !sameAddress(key.target, address))
            if (channel >= 0) {
                if (keys.some(key => key.channel === channel)) throw new Error("That frontend channel already belongs to another key.")
                keys.push({target: address, channel: channel})
            }
            draft.keyboard_bindings = keys
            editor.text = JSON.stringify(draft, null, 2)
            statusText = "Keyboard channel changed in the draft. Assign its physical source, then stage and save. Every inspected key remains required."
        } catch (error) { assignmentError = "Cannot change keyboard channel: " + error }
    }
    title: "FBNeo per-game setups — advanced"
    width: Math.min(900, parent ? parent.width - 40 : 900)
    height: Math.min(760, parent ? parent.height - 40 : 760)
    modal: true
    closePolicy: Popup.NoAutoClose
    onClosed: { relativeSelection.close(); replaceController.close(); assignmentEditor.close(); removal.close(); reportImport.close(); mouseDestination.close(); keyboardConsent.close(); absoluteSelection.close() }

    function refresh() {
        try {
            setups = JSON.parse(settingsModel.fbneo_controller_setups_json())
            choices.currentIndex = setups.findIndex(item => item.key === selectedKey)
        } catch (error) {
            statusText = "Cannot read saved setups: " + error
        }
    }
    function loadAndOpen() {
        // Keep an un-staged draft if settings navigation closed this dialog.
        if (!draftChanged) refresh()
        open()
    }
    function loadSelected() {
        const text = settingsModel.fbneo_controller_setup_json(selectedKey)
        if (!text) {
            statusText = "That setup no longer exists. Refresh the list."
            return
        }
        loadedText = text
        editor.text = text
        statusText = "Editing a staged setup. Changes are revalidated at launch."
    }
    function refreshAssignments() {
        try {
            const review = JSON.parse(settingsModel.fbneo_assignment_review_json(editor.text))
            assignmentRows = review.targets || []
            staleAssignments = review.stale_assignments || []
            assignmentErrorPorts = (review.calibration_reviews || []).filter(item => !!item.error).map(item => item.port)
            assignmentError = review.error || (review.calibration_reviews || []).map(item => {
                const prefix = "Saved mapping — player " + (item.port + 1) + ": "
                if (item.error) return prefix + item.error
                if (item.relative_only) return prefix + "relative source only; no gamepad required. " + item.notice
                const missing = item.missing || []
                const external = item.external_targets || []
                const coverage = item.required_parts === undefined ? "" : item.mapped_parts + " / " + item.required_parts + " supported native parts mapped" + (item.mapped_percent === null ? " (no supported-part denominator)" : " (" + item.mapped_percent.toFixed(1) + "%)") + "; "
                return prefix + coverage + missing.length + " unassigned parts; " + external.length + " additional targets need external adapters. Not runtime verification."
            }).filter(message => message.length > 0).join("\n")
            if (!review.error) {
                const relative = (review.relative_reviews || []).map(item => {
                    const source = (review.relative_sources || []).find(source => source.port === item.port)
                    return "Mouse capability review, port " + item.port + ": " + item.required.length
                    + " targets; " + item.unsupported.length + " need additional conversion; "
                    + item.candidate_devices.length + " saved devices cover the entire mouse target set. "
                    + "Selected source: " + (source ? source.device.event_path : "none")
                    + ". No live route is assigned; runtime behavior is unverified."
                }).join("\n")
                if (relative) assignmentError += (assignmentError ? "\n" : "") + relative
                if (review.relative_source_error) assignmentError += "\nRelative source: " + review.relative_source_error
                if ((review.relative_sources || []).length) assignmentError += "\n" + review.relative_launch_requirement
                if (review.relative_launch_pending) assignmentError += "\nRelative source or platform requirements remain unmet."
            }
        } catch (error) {
            assignmentRows = []
            staleAssignments = []
            assignmentErrorPorts = []
            assignmentError = "Cannot read assignment draft: " + error
        }
    }
    function sameAddress(left, right) {
        return left.port === right.port && left.device === right.device
            && left.index === right.index && left.id === right.id
    }
    function assignmentNeedsAttention(row) {
        // A player-level failure may concern aliases or paired axes rather
        // than one part. Keep all of that player's rows available for repair.
        return !!row.calibration_error || row.requires_external_adapter || row.relative_launch_pending === true
            || assignmentErrorPorts.indexOf(row.target.address.port) >= 0
            || (row.parts || []).some(part => !part.effective_source || !!part.source_error
                || (part.duplicate_count || 0) > 0 || (part.alias_assignment_count || 0) > 1)
    }
    function removeStaleAssignment(entry) {
        try {
            if (!entry) throw new Error("Select a stale assignment first.")
            const current = JSON.parse(settingsModel.fbneo_assignment_review_json(editor.text))
            const match = (current.stale_assignments || []).find(item => item.player_port === entry.player_port
                && item.index === entry.index && sameAddress(item.assignment.target, entry.assignment.target)
                && item.assignment.part === entry.assignment.part && item.assignment.source === entry.assignment.source)
            if (!match) throw new Error("The stale assignment changed; reopen review before removing it.")
            const draft = JSON.parse(editor.text)
            const player = draft.players.find(player => player.port === entry.player_port)
            player.assignments.splice(entry.index, 1)
            editor.text = JSON.stringify(draft, null, 2)
            statusText = "Removed the selected stale assignment from the draft only. Other assignments were preserved."
            refreshAssignments()
        } catch (error) { assignmentError = "Cannot remove stale assignment: " + error }
    }
    function assignSource(address, part, source) {
        try {
            const current = JSON.parse(settingsModel.fbneo_assignment_review_json(editor.text))
            const row = (current.targets || []).find(row => sameAddress(row.target.address, address))
            const inputPart = row ? (row.parts || []).find(entry => entry.part === part) : null
            if (!inputPart) throw new Error(current.error || "The selected native input part is no longer in the current draft.")
            if (source && !(inputPart.choices || []).some(choice => choice.id === source))
                throw new Error("The selected source is no longer eligible under the current saved calibration. Reopen assignment review.")
            const scrollPosition = targetList.contentY
            const draft = JSON.parse(editor.text)
            const player = draft.players.find(item => item.port === address.port)
            if (!player) throw new Error("Selected input port has no player")
            player.assignments = player.assignments.filter(item =>
                !(sameAddress(item.target, address) && item.part === part))
            if (source) player.assignments.push({ target: address, part: part, source: source })
            editor.text = JSON.stringify(draft, null, 2)
            statusText = "Assignment edited in text draft. Stage setup, then Save settings to keep it."
            refreshAssignments()
            Qt.callLater(() => targetList.contentY = Math.max(0,
                Math.min(scrollPosition, targetList.contentHeight - targetList.height)))
        } catch (error) {
            assignmentError = "Cannot change assignment: " + error
        }
    }
    function assignAxisPair(address, pair) {
        try {
            const current = JSON.parse(settingsModel.fbneo_assignment_review_json(editor.text))
            const row = (current.targets || []).find(row => sameAddress(row.target.address, address))
            if (!row || !(row.axis_pairs || []).some(choice => choice.negative_source === pair.negative_source && choice.positive_source === pair.positive_source))
                throw new Error("Axis pair is no longer an eligible saved calibration.")
            const draft = JSON.parse(editor.text)
            const player = draft.players.find(item => item.port === address.port)
            if (!player) throw new Error("Selected input port has no player")
            player.assignments = player.assignments.filter(item => !(sameAddress(item.target, address) && (item.part === "negative" || item.part === "positive")))
            player.assignments.push({target: address, part: "negative", source: pair.negative_source})
            player.assignments.push({target: address, part: "positive", source: pair.positive_source})
            editor.text = JSON.stringify(draft, null, 2)
            statusText = "Both axis halves edited together in the draft. Other native aliases are unchanged; review any shared-channel errors before staging."
            refreshAssignments()
        } catch (error) { assignmentError = "Cannot assign axis pair: " + error }
    }
    contentItem: ColumnLayout {
        spacing: 10
        Label {
            Layout.fillWidth: true
            text: "Paste a reviewed FBNeo setup or edit an existing one. Assignment editing does not run a core. Use the separate inspection action to create a draft from an explicitly trusted core; dependency and topology inputs must be supplied."
            wrapMode: Text.WordWrap
        }
        RowLayout {
            Layout.fillWidth: true
            ComboBox {
                id: choices
                Layout.fillWidth: true
                enabled: !dialog.draftChanged
                model: dialog.setups.map(item => item.content + " · " + item.players + " player(s)")
                onActivated: {
                    dialog.selectedKey = dialog.setups[currentIndex].key
                    dialog.loadSelected()
                }
            }
            Button {
                text: "Refresh"
                enabled: !dialog.draftChanged
                onClicked: dialog.refresh()
            }
            Button {
                text: "New / paste"
                enabled: !dialog.draftChanged
                onClicked: {
                    dialog.selectedKey = ""
                    dialog.loadedText = ""
                    editor.text = ""
                    choices.currentIndex = -1
                    dialog.statusText = "Paste the complete reviewed setup JSON below."
                }
            }
            Button {
                text: "Import inspection…"
                onClicked: reportImport.open()
            }
        }
        Label {
            visible: dialog.selectedKey.length > 0
            Layout.fillWidth: true
            text: "Exact identity: " + dialog.selectedKey
            textFormat: Text.PlainText
            wrapMode: Text.WrapAnywhere
        }
        Button {
            text: "Choose relative sources…"
            enabled: editor.text.trim().length > 0
            onClicked: relativeSelection.loadAndOpen()
        }
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            TextArea {
                id: editor
                textFormat: TextEdit.PlainText
                wrapMode: TextEdit.NoWrap
                selectByMouse: true
                font.family: "monospace"
                Accessible.name: "Reviewed FBNeo setup JSON"
                placeholderText: "Paste reviewed setup JSON"
            }
        }
        Label {
            Layout.fillWidth: true
            text: dialog.statusText
            textFormat: Text.PlainText
            wrapMode: Text.WrapAnywhere
        }
        Label {
            Layout.fillWidth: true
            text: "Stage setup changes the settings draft only. Use Save settings in the main settings page to persist it. Changing the emulator/core/content identity creates a separate setup. Inspected launches disable RetroArch gamepad hotkeys and turbo; keyboard-mapped setups also disable keyboard hotkeys in their private session."
            wrapMode: Text.WordWrap
        }
        RowLayout {
            Button {
                text: "Edit assignments…"
                enabled: editor.text.trim().length > 0
                onClicked: {
                    dialog.refreshAssignments()
                    assignmentEditor.open()
                }
            }
            Button {
                text: "Replace controller…"
                enabled: editor.text.trim().length > 0
                onClicked: replaceController.loadAndOpen()
            }
            Button {
                text: "Stage setup"
                enabled: editor.text.trim().length > 0
                onClicked: {
                    const error = dialog.settingsModel.save_fbneo_controller_setup(editor.text)
                    if (error) { dialog.statusText = error; return }
                    const saved = JSON.parse(editor.text)
                    dialog.loadedText = editor.text
                    dialog.refresh()
                    const row = dialog.setups.find(item => item.emulator_id === saved.emulator_id
                        && item.core === saved.core && item.content === saved.content)
                    dialog.selectedKey = row ? row.key : ""
                    dialog.refresh()
                    dialog.statusText = "Setup staged. Save settings to keep it. No core was started."
                }
            }
            Button {
                text: "Discard text edits"
                enabled: dialog.draftChanged
                onClicked: editor.text = dialog.loadedText
            }
            Button {
                text: "Remove setup…"
                enabled: dialog.selectedKey.length > 0 && !dialog.draftChanged
                onClicked: {
                    removal.keyToRemove = dialog.selectedKey
                    removal.open()
                }
            }
            Item { Layout.fillWidth: true }
            Button {
                text: dialog.draftChanged ? "Close (keep text draft)" : "Close"
                onClicked: dialog.close()
            }
        }
    }
    Dialog {
        id: relativeSelection
        property string baseText: ""
        property int baseRevision: -1
        property var ports: []
        property var selectedSources: []
        property var relativeOnlyPorts: []
        property var playerSelections: []
        property string errorText: ""
        readonly property bool stale: editor.text !== baseText || dialog.mappingRevision !== baseRevision
        readonly property var selectedPort: ports[relativePortChoice.currentIndex] || null
        readonly property var candidates: selectedPort ? selectedPort.prepared_candidates || [] : []
        readonly property var selectedPlayer: selectedPort
            ? playerSelections.find(player => player.port === selectedPort.port) || null : null
        readonly property bool canOmitGamepad: !stale && selectedPlayer !== null
            && relativeOnlyPorts.indexOf(selectedPlayer.port) >= 0
            && !!selectedPlayer.controller_id && (selectedPlayer.assignments || []).length === 0
        onCandidatesChanged: relativeDeviceChoice.currentIndex = -1
        title: "FBNeo relative-source draft"
        width: Math.min(680, dialog.width - 30)
        height: Math.min(560, dialog.height - 30)
        modal: true
        standardButtons: Dialog.Close
        function loadAndOpen() {
            try {
                baseText = editor.text
                baseRevision = dialog.mappingRevision
                const review = JSON.parse(dialog.settingsModel.fbneo_assignment_review_json(baseText))
                if (review.error) throw new Error(review.error)
                selectedSources = review.relative_sources || []
                relativeOnlyPorts = review.relative_only_ports || []
                playerSelections = JSON.parse(baseText).players || []
                const reviewedPorts = review.relative_reviews || []
                // Retain orphaned selections so stale source choices can be removed.
                ports = reviewedPorts.concat(selectedSources.filter(source => !reviewedPorts.some(port => port.port === source.port))
                    .map(source => ({port: source.port, required: [], unsupported: [], prepared_candidates: []})))
                relativePortChoice.currentIndex = ports.length ? 0 : -1
                relativeDeviceChoice.currentIndex = -1
                errorText = review.relative_source_error || ""
                open()
            } catch (error) { dialog.statusText = "Cannot review relative sources: " + error }
        }
        function applySelection(remove) {
            try {
                if (stale || !selectedPort) throw new Error("The draft or controller settings changed. Reopen relative-source review.")
                const draft = JSON.parse(baseText)
                const sources = (draft.relative_sources || []).filter(source => source.port !== selectedPort.port)
                if (!remove) {
                    const device = candidates[relativeDeviceChoice.currentIndex]
                    if (!device) throw new Error("Choose a prepared source first.")
                    sources.push({port: selectedPort.port, device: JSON.parse(JSON.stringify(device))})
                }
                draft.relative_sources = sources
                const candidate = JSON.stringify(draft, null, 2)
                const review = JSON.parse(dialog.settingsModel.fbneo_assignment_review_json(candidate))
                if (review.error || (!remove && review.relative_source_error))
                    throw new Error(review.error || review.relative_source_error)
                editor.text = candidate
                dialog.refreshAssignments()
                dialog.statusText = "Relative-source draft updated. Other assignments are preserved. No settings were saved. Relative launch requires the opt-in frontend on 64-bit Linux; runtime behavior is unverified."
                close()
            } catch (error) { errorText = String(error) }
        }
        function omitGamepad() {
            try {
                if (!canOmitGamepad) throw new Error("This port is not eligible to omit its gamepad. Existing assignments must be resolved explicitly.")
                const draft = JSON.parse(baseText)
                const player = draft.players.find(player => player.port === selectedPlayer.port)
                if (!player || player.assignments.length) throw new Error("Gamepad assignments changed; reopen review.")
                player.controller_id = ""
                const candidate = JSON.stringify(draft, null, 2)
                const review = JSON.parse(dialog.settingsModel.fbneo_assignment_review_json(candidate))
                if (review.error || review.relative_source_error
                        || (review.relative_only_ports || []).indexOf(player.port) < 0)
                    throw new Error(review.error || review.relative_source_error || "Relative source no longer covers the complete port.")
                editor.text = candidate
                dialog.refreshAssignments()
                dialog.statusText = "Gamepad selection removed from this relative-only port's draft. Native port/device and other mappings are unchanged. Relative launch requires the opt-in frontend on 64-bit Linux. Use Replace controller to choose a gamepad again."
                close()
            } catch (error) { errorText = String(error) }
        }
        ScrollView {
            id: relativeSelectionScroll
            anchors.fill: parent
            ColumnLayout {
                width: relativeSelectionScroll.availableWidth
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Choose an exact saved device for the entire inspected mouse portion of one port. Candidates contain only required controls. This changes the text draft; it does not open devices or enable launch." }
                ComboBox {
                    id: relativePortChoice
                    Layout.fillWidth: true
                    model: relativeSelection.ports.map(port => "Player " + (port.port + 1) + " · native port " + port.port)
                    Accessible.name: "FBNeo relative source port"
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                    text: {
                        const port = relativeSelection.selectedPort
                        if (!port) return "No inspected mouse ports or saved selections."
                        const source = relativeSelection.selectedSources.find(source => source.port === port.port)
                        return "Current source: " + (source ? source.device.event_path + "\n" + source.device.input_identity : "none")
                            + "\n" + port.required.length + " inspected mouse targets; " + port.unsupported.length + " unsupported conversions."
                    }
                }
                ComboBox {
                    id: relativeDeviceChoice
                    Layout.fillWidth: true
                    model: relativeSelection.candidates.map(device => device.event_path + " · " + device.input_identity)
                    displayText: currentIndex < 0 ? "Choose a prepared saved device" : currentText
                    Accessible.name: "Exact prepared relative device"
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                    text: relativeDeviceChoice.currentIndex >= 0 ? JSON.stringify(relativeSelection.candidates[relativeDeviceChoice.currentIndex], null, 2) : ""
                }
                Button {
                    text: "Use source in draft"
                    enabled: !relativeSelection.stale && relativeDeviceChoice.currentIndex >= 0
                    onClicked: relativeSelection.applySelection(false)
                }
                Button {
                    text: "Remove this port's source"
                    enabled: !relativeSelection.stale && relativeSelection.selectedPort !== null
                        && relativeSelection.selectedSources.some(source => source.port === relativeSelection.selectedPort.port)
                    onClicked: relativeSelection.applySelection(true)
                }
                Button {
                    text: "Use this port without a gamepad"
                    enabled: relativeSelection.canOmitGamepad
                    onClicked: relativeSelection.omitGamepad()
                }
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    text: !relativeSelection.selectedPlayer ? "" : !relativeSelection.selectedPlayer.controller_id
                        ? "No gamepad selected. Keep a valid relative source for this port, or use Replace controller to add a gamepad. Removing its only source leaves an incomplete draft."
                        : "Gamepad omission requires a selected source covering every inspected port input and no retained gamepad assignments. Mixed-input ports still need their gamepad."
                }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; text: relativeSelection.stale ? "Draft changed. Close and reopen this review." : relativeSelection.errorText }
            }
        }
    }
    Dialog {
        id: replaceController
        property string baseText: ""
        property string errorText: ""
        property var players: []
        readonly property var sources: {
            dialog.settingsModel.controller_revision
            return JSON.parse(dialog.settingsModel.fbneo_source_controllers_json())
        }
        onSourcesChanged: replacementSource.currentIndex = -1
        title: "Replace FBNeo player controller"
        width: dialog.width - 40
        modal: true
        standardButtons: Dialog.Cancel
        function loadAndOpen() {
            try {
                const draft = JSON.parse(editor.text)
                if (!Array.isArray(draft.players) || !draft.players.length) throw new Error("Load a setup with player assignments first.")
                players = draft.players
                baseText = editor.text
                errorText = ""
                replacementPort.currentIndex = -1
                replacementSource.currentIndex = -1
                open()
            } catch (error) { dialog.statusText = "Cannot replace controller: " + error }
        }
        contentItem: ColumnLayout {
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Native port/device choices and control assignments are retained. A replacement with a different layout may need assignment repairs. Saved calibration is not a live connection check." }
            ComboBox {
                id: replacementPort
                Layout.fillWidth: true
                model: replaceController.players.map(player => "Player " + (player.port + 1) + " — " + player.controller_id)
                displayText: currentIndex < 0 ? "Choose player to replace…" : currentText
                Accessible.name: "FBNeo player to replace"
            }
            ComboBox {
                id: replacementSource
                Layout.fillWidth: true
                model: replaceController.sources.map(source => source.id + " — " + source.layout)
                displayText: currentIndex < 0 ? "Choose saved controller…" : currentText
                Accessible.name: "Replacement saved controller"
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; text: replaceController.errorText }
            Button {
                text: "Apply to draft and review"
                enabled: replacementPort.currentIndex >= 0 && replacementSource.currentIndex >= 0
                onClicked: {
                    try {
                        if (editor.text !== replaceController.baseText) throw new Error("The draft changed; close and reopen this editor.")
                        const draft = JSON.parse(editor.text)
                        const source = replaceController.sources[replacementSource.currentIndex]
                        if (!source) throw new Error("Select a saved controller.")
                        if (draft.players.some((player, index) => index !== replacementPort.currentIndex && player.controller_id === source.id)) throw new Error("That controller is already assigned to another player.")
                        draft.players[replacementPort.currentIndex].controller_id = source.id
                        editor.text = JSON.stringify(draft, null, 2)
                        dialog.statusText = "Controller replaced in the draft. Repair any missing or incompatible controls before staging. Native assignments were retained."
                        dialog.refreshAssignments()
                        replaceController.close()
                        assignmentEditor.open()
                    } catch (error) { replaceController.errorText = String(error) }
                }
            }
        }
    }
    Dialog {
        id: reportImport
        property bool inputsEdited: true
        property bool startedHere: false
        property var sourceControllers: []
        property var devicePorts: []
        property string deviceContextSnapshot: ""
        function loadDeviceChoices() {
            try {
                const result = JSON.parse(dialog.settingsModel.fbneo_device_choices_json(importContext.text))
                if (result.error) throw new Error(result.error)
                devicePorts = result.ports
                deviceContextSnapshot = importContext.text
                errorText = "Device choices loaded from supplied topology. These are source-defined choices, not a live core observation."
            } catch (error) {
                devicePorts = []
                errorText = "Cannot load devices: " + error
            }
        }
        function selectDevice(port, device) {
            try {
                if (importContext.text !== deviceContextSnapshot)
                    throw new Error("Context changed; reload device choices")
                const request = JSON.parse(importRequest.text)
                if (!Array.isArray(request.devices)) throw new Error("Request must contain a devices array")
                request.devices = request.devices.filter(item => item.port !== port)
                request.devices.push({port: port, device: device})
                request.devices.sort((left, right) => left.port - right.port)
                importRequest.text = JSON.stringify(request, null, 2)
                errorText = "Device selected in request draft. Reinspect before using a mapping; external adapters may still be required."
            } catch (error) { errorText = "Cannot select device: " + error }
        }
        function selectedDeviceIndex(port, choices) {
            try {
                const selected = JSON.parse(importRequest.text).devices.filter(item => item.port === port)
                return selected.length === 1 ? choices.findIndex(item => item.id === selected[0].device) : -1
            } catch (error) { return -1 }
        }
        function refreshSourceControllers() {
            try {
                sourceControllers = JSON.parse(dialog.settingsModel.fbneo_source_controllers_json())
            } catch (error) {
                sourceControllers = []
                errorText = "Cannot read saved controllers: " + error
            }
            // Array order may change after calibration edits. Require an
            // explicit choice instead of retaining an index into the old list.
            contextController.currentIndex = -1
        }
        onOpened: refreshSourceControllers()
        function assignContextController() {
            try {
                const context = JSON.parse(importContext.text)
                if (!context.controllers || Array.isArray(context.controllers)
                        || typeof context.controllers !== "object")
                    throw new Error("Context must contain a controllers object")
                const source = sourceControllers[contextController.currentIndex]
                if (!source) throw new Error("Select a saved controller first")
                const port = String(contextPort.value)
                if (Object.keys(context.controllers).some(key => key !== port
                        && context.controllers[key] === source.id))
                    throw new Error("That exact controller is already assigned to another port")
                context.controllers[port] = source.id
                importContext.text = JSON.stringify(context, null, 2)
                errorText = "Controller assigned in context draft. No hardware was opened; launch rechecks identity and calibration."
            } catch (error) { errorText = "Cannot assign controller: " + error }
        }
        property string errorText: ""
        property string dependencyRequestSnapshot: ""
        property bool dependenciesLoaded: false
        property string loadedContentDependencies: ""
        property string loadedSystemFiles: ""
        readonly property bool dependencyEditsPending: dependenciesLoaded
            && (dependencyContent.text !== loadedContentDependencies
                || dependencySystem.text !== loadedSystemFiles)
        function loadDependencies() {
            try {
                const request = JSON.parse(importRequest.text)
                for (const key of ["content_dependencies", "system_files"]) {
                    if (!Array.isArray(request[key]) || request[key].some(path =>
                            typeof path !== "string" || /[\r\n]/.test(path)))
                        throw new Error(key + " must be an array of single-line paths; use Request JSON for paths containing line breaks")
                }
                dependencyContent.text = request.content_dependencies.join("\n")
                dependencySystem.text = request.system_files.join("\n")
                loadedContentDependencies = dependencyContent.text
                loadedSystemFiles = dependencySystem.text
                dependencyRequestSnapshot = importRequest.text
                dependenciesLoaded = true
                errorText = "Dependency lists loaded. Edits affect the request only after Apply dependency lists."
            } catch (error) {
                dependenciesLoaded = false
                errorText = "Cannot load dependencies: " + error
            }
        }
        function applyDependencies() {
            try {
                if (!dependenciesLoaded || importRequest.text !== dependencyRequestSnapshot)
                    throw new Error("Request changed; reload dependency lists before applying")
                const request = JSON.parse(importRequest.text)
                function paths(text) {
                    // Do not trim: whitespace can be part of a real filename.
                    const values = text.split("\n").filter(path => path.length > 0)
                    if (values.length > 256 || values.some(path =>
                            !path.startsWith("/") || /[\r\u0000]/.test(path)))
                        throw new Error("Use at most 256 absolute paths per list, one per line")
                    if (new Set(values).size !== values.length)
                        throw new Error("Remove duplicate paths from each list")
                    return values
                }
                request.content_dependencies = paths(dependencyContent.text)
                request.system_files = paths(dependencySystem.text)
                if (request.content_dependencies.includes(request.content))
                    throw new Error("Primary content is already staged; do not list it as a dependency")
                importRequest.text = JSON.stringify(request, null, 2)
                dependencyRequestSnapshot = importRequest.text
                loadedContentDependencies = dependencyContent.text
                loadedSystemFiles = dependencySystem.text
                errorText = "Dependency lists applied to request. Files and staged-name collisions are validated before inspection; no core was run."
            } catch (error) { errorText = "Cannot apply dependencies: " + error }
        }
        title: "FBNeo inspection / report import"
        onClosed: {
            nativeConsent.close()
            if (dialog.settingsModel.fbneo_inspection_active)
                dialog.settingsModel.cancel_fbneo_inspection()
        }
        width: dialog.width - 20
        height: dialog.height - 20
        modal: true
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            Label {
                Layout.fillWidth: true
                text: "Supply request and application context. Import an existing report without running a core, or explicitly run a trusted-core inspection to create a new draft."
                wrapMode: Text.WordWrap
            }
            TabBar {
                id: importTabs
                Layout.fillWidth: true
                TabButton { text: "Request JSON" }
                TabButton { text: "Report JSON" }
                TabButton { text: "Application context" }
                TabButton { text: "Dependency files" }
                TabButton { text: "Devices" }
            }
            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: importTabs.currentIndex
                ScrollView {
                    TextArea {
                        id: importRequest
                        textFormat: TextEdit.PlainText
                        wrapMode: TextEdit.NoWrap
                        selectByMouse: true
                        font.family: "monospace"
                        readOnly: dialog.settingsModel.fbneo_import_busy
                        Accessible.name: "Existing FBNeo helper request JSON"
                        onTextChanged: reportImport.inputsEdited = true
                    }
                }
                ScrollView {
                    TextArea {
                        id: importReport
                        textFormat: TextEdit.PlainText
                        wrapMode: TextEdit.NoWrap
                        selectByMouse: true
                        font.family: "monospace"
                        readOnly: dialog.settingsModel.fbneo_import_busy
                        Accessible.name: "Existing FBNeo helper report JSON"
                        onTextChanged: reportImport.inputsEdited = true
                    }
                }
                ColumnLayout {
                    Label {
                        Layout.fillWidth: true
                        text: "Required context keys: emulator_id, helper, hardware (other, nes or msx_or_spectrum), driver_players, mahjong_keyboards, and controllers (zero-based port numbers mapped to exact saved controller IDs). Use source-confirmed topology."
                        wrapMode: Text.WordWrap
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: "Port (0-based)" }
                        SpinBox {
                            id: contextPort
                            from: 0
                            to: 5
                            enabled: !dialog.settingsModel.fbneo_import_busy
                        }
                        ComboBox {
                            id: contextController
                            Layout.fillWidth: true
                            displayText: currentIndex < 0 ? "Choose saved controller…" : currentText
                            model: reportImport.sourceControllers.map(item => item.id + " · " + item.layout)
                            enabled: !dialog.settingsModel.fbneo_import_busy
                        }
                        Button {
                            text: "Assign controller"
                            enabled: !dialog.settingsModel.fbneo_import_busy
                                && contextController.currentIndex >= 0 && importContext.text.trim().length > 0
                            onClicked: reportImport.assignContextController()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        text: "Choices are saved Linux calibrations, not detected connections. Controller-setting changes refresh this list and clear the picker selection, but preserve assignments already in the context draft. Assign only source-confirmed ports."
                        wrapMode: Text.WordWrap
                    }
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        TextArea {
                            id: importContext
                            textFormat: TextEdit.PlainText
                            wrapMode: TextEdit.NoWrap
                            selectByMouse: true
                            font.family: "monospace"
                            readOnly: dialog.settingsModel.fbneo_import_busy
                            Accessible.name: "FBNeo inspection application context JSON"
                            onTextChanged: reportImport.inputsEdited = true
                        }
                    }
                }
                ColumnLayout {
                    Label {
                        Layout.fillWidth: true
                        text: "Explicit files only, one absolute path per line. Content dependencies (for example reviewed parent/BIOS archives) are copied beside the primary content; system files are copied into the core's system directory. No directory scanning or filename matching is performed. Paths containing line breaks must be edited in Request JSON."
                        wrapMode: Text.WordWrap
                    }
                    Label { text: "Content dependencies" }
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        TextArea {
                            id: dependencyContent
                            textFormat: TextEdit.PlainText
                            wrapMode: TextEdit.NoWrap
                            selectByMouse: true
                            readOnly: !reportImport.dependenciesLoaded || dialog.settingsModel.fbneo_import_busy
                            Accessible.name: "FBNeo content dependency absolute paths"
                        }
                    }
                    Label { text: "System files" }
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        TextArea {
                            id: dependencySystem
                            textFormat: TextEdit.PlainText
                            wrapMode: TextEdit.NoWrap
                            selectByMouse: true
                            readOnly: !reportImport.dependenciesLoaded || dialog.settingsModel.fbneo_import_busy
                            Accessible.name: "FBNeo system file absolute paths"
                        }
                    }
                    RowLayout {
                        Button {
                            text: "Load / reset lists from request"
                            enabled: !dialog.settingsModel.fbneo_import_busy
                            onClicked: reportImport.loadDependencies()
                        }
                        Button {
                            text: "Apply dependency lists"
                            enabled: reportImport.dependenciesLoaded && !dialog.settingsModel.fbneo_import_busy
                                && importRequest.text === reportImport.dependencyRequestSnapshot
                            onClicked: reportImport.applyDependencies()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: reportImport.dependenciesLoaded
                            && importRequest.text !== reportImport.dependencyRequestSnapshot
                        text: "Request JSON changed. Reload the lists before applying; this discards unapplied list edits."
                        wrapMode: Text.WordWrap
                    }
                }
                ColumnLayout {
                    Label {
                        Layout.fillWidth: true
                        text: "Load choices after supplying source-confirmed hardware/player counts in Application context. Selection changes Request JSON only. Keyboard, mouse and coordinate devices can still require unfinished external adapters."
                        wrapMode: Text.WordWrap
                    }
                    Button {
                        text: "Load device choices from context"
                        enabled: !dialog.settingsModel.fbneo_import_busy
                        onClicked: reportImport.loadDeviceChoices()
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: reportImport.devicePorts.length > 0
                            && importContext.text !== reportImport.deviceContextSnapshot
                        text: "Context changed. Reload choices before selecting a device."
                        wrapMode: Text.WordWrap
                    }
                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        spacing: 10
                        model: reportImport.devicePorts
                        ScrollBar.vertical: ScrollBar {}
                        delegate: RowLayout {
                            id: deviceRow
                            required property var modelData
                            width: ListView.view.width
                            Label { text: "Port " + deviceRow.modelData.port }
                            ComboBox {
                                Layout.fillWidth: true
                                model: deviceRow.modelData.devices.map(item => item.label + " · ID " + item.id)
                                currentIndex: reportImport.selectedDeviceIndex(deviceRow.modelData.port, deviceRow.modelData.devices)
                                enabled: !dialog.settingsModel.fbneo_import_busy
                                    && importContext.text === reportImport.deviceContextSnapshot
                                    && !reportImport.dependencyEditsPending
                                onActivated: reportImport.selectDevice(deviceRow.modelData.port,
                                    deviceRow.modelData.devices[currentIndex].id)
                            }
                        }
                    }
                }
            }
            Label {
                Layout.fillWidth: true
                text: reportImport.errorText || dialog.settingsModel.fbneo_import_status
                textFormat: Text.PlainText
                wrapMode: Text.WrapAnywhere
            }
            Label {
                Layout.fillWidth: true
                visible: reportImport.dependencyEditsPending
                text: "Dependency list edits are not applied. Apply them or reset the lists before inspecting, importing, or using a draft."
                wrapMode: Text.WordWrap
            }
            Label {
                Layout.fillWidth: true
                visible: dialog.draftChanged
                text: "The setup editor has text edits. Stage or discard them before using an imported draft."
                wrapMode: Text.WordWrap
            }
            RowLayout {
                BusyIndicator { running: dialog.settingsModel.fbneo_import_busy; visible: running }
                Button {
                    text: "Run trusted-core inspection…"
                    enabled: !dialog.settingsModel.fbneo_import_busy && importRequest.text.trim().length > 0
                        && importContext.text.trim().length > 0
                        && !reportImport.dependencyEditsPending
                    onClicked: {
                        try {
                            const request = JSON.parse(importRequest.text)
                            const context = JSON.parse(importContext.text)
                            nativeConsent.requestText = importRequest.text
                            nativeConsent.contextText = importContext.text
                            nativeConsent.identityText = "Core: " + request.core + "\nSHA256: " + request.core_sha256
                                + "\nHelper: " + context.helper + "\nContent: " + request.content
                            nativeConsent.open()
                        } catch (error) { reportImport.errorText = "Invalid inspection JSON: " + error }
                    }
                }
                Button {
                    text: "Cancel inspection"
                    visible: dialog.settingsModel.fbneo_inspection_active
                    onClicked: dialog.settingsModel.cancel_fbneo_inspection()
                }
                Button {
                    text: "Validate and import report"
                    enabled: !dialog.settingsModel.fbneo_import_busy && importRequest.text.trim().length > 0
                        && importReport.text.trim().length > 0 && importContext.text.trim().length > 0
                        && !reportImport.dependencyEditsPending
                    onClicked: {
                        reportImport.errorText = dialog.settingsModel.begin_fbneo_inspection_import(
                            importRequest.text, importReport.text, importContext.text)
                        if (!reportImport.errorText) {
                            reportImport.startedHere = true
                            reportImport.inputsEdited = false
                        }
                    }
                }
                Button {
                    text: "Use imported draft"
                    enabled: reportImport.startedHere && !reportImport.inputsEdited && !dialog.draftChanged
                        && !dialog.settingsModel.fbneo_import_busy && dialog.settingsModel.fbneo_import_draft.length > 0
                        && !reportImport.dependencyEditsPending
                    onClicked: {
                        dialog.selectedKey = ""
                        dialog.loadedText = ""
                        editor.text = dialog.settingsModel.fbneo_import_draft
                        choices.currentIndex = -1
                        dialog.statusText = "Imported draft loaded. Use Edit assignments, then Stage setup and Save settings."
                        reportImport.close()
                    }
                }
            }
        }
        Dialog {
            id: nativeConsent
            property string requestText: ""
            property string contextText: ""
            property string identityText: ""
            title: "Run this trusted native core?"
            modal: true
            standardButtons: Dialog.Yes | Dialog.No
            width: Math.min(640, reportImport.width - 20)
            contentItem: Label {
                text: "This executes native code in a separate worker for one idle inspection frame, with a 30-second timeout and private content/save copies. It is not a security sandbox. Continue only if you trust this core and helper.\n\n" + nativeConsent.identityText
                textFormat: Text.PlainText
                wrapMode: Text.WrapAnywhere
            }
            onAccepted: {
                reportImport.errorText = dialog.settingsModel.begin_fbneo_inspection(requestText, contextText)
                if (!reportImport.errorText) {
                    reportImport.startedHere = true
                    reportImport.inputsEdited = importRequest.text !== requestText || importContext.text !== contextText
                }
            }
        }
    }
    Dialog {
        id: absoluteSelection
        property var ports: []
        property var devices: []
        title: "Calibrated absolute aim"
        modal: true
        width: dialog.width - 40
        standardButtons: Dialog.Close
        onOpened: {
            try {
                ports = JSON.parse(editor.text).players.filter(player => player.device === 1029).map(player => player.port)
                devices = JSON.parse(dialog.settingsModel.absolute_device_settings_json())
                absoluteStatus.text = ""
            } catch (error) { ports = []; devices = []; absoluteStatus.text = String(error) }
        }
        function assign(remove) {
            try {
                if (absolutePort.currentIndex < 0) throw new Error("Select an inspected Arcade Gun port.")
                const port = ports[absolutePort.currentIndex]
                const draft = JSON.parse(editor.text)
                const player = draft.players.find(player => player.port === port && player.device === 1029)
                if (!player) throw new Error("Selected native device changed.")
                draft.absolute_sources = (draft.absolute_sources || []).filter(source => source.port !== port)
                if (!remove) {
                    if (absoluteDevice.currentIndex < 0) throw new Error("Select a saved absolute calibration.")
                    const device = devices[absoluteDevice.currentIndex]
                    draft.absolute_sources.push({port: port, device: device})
                    const isAim = address => (address.device === 5 || address.device === 1029)
                        && address.index === 0 && (address.id === 0 || address.id === 1)
                    player.assignments = player.assignments.filter(assignment => !isAim(assignment.target))
                    const targets = draft.expected_queries.concat(draft.expected_descriptors.map(item => item.address))
                        .filter(address => address.port === port)
                    if (targets.length && targets.every(isAim)) player.controller_id = ""
                }
                editor.text = JSON.stringify(draft, null, 2)
                absoluteStatus.text = "Draft updated. Stage setup and Save settings to keep it."
            } catch (error) { absoluteStatus.text = String(error) }
        }
        contentItem: ColumnLayout {
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Assign saved rectangular calibration to an already-inspected Arcade Gun port. Applying replaces this port's gamepad aim assignments in the draft; buttons stay unchanged. Move both axes during launch preparation. Leaving the calibrated rectangle stops the input bridge: physical offscreen/reload protocols are not supplied. No live devices are opened by this editor."
            }
            ComboBox { id: absolutePort; Layout.fillWidth: true; model: absoluteSelection.ports.map(port => "Native port " + (port + 1)) }
            ComboBox { id: absoluteDevice; Layout.fillWidth: true; model: absoluteSelection.devices.map(device => device.event_path + " · " + device.input_identity) }
            RowLayout {
                Button { text: "Apply to draft"; enabled: absoluteDevice.currentIndex >= 0 && absolutePort.currentIndex >= 0; onClicked: absoluteSelection.assign(false) }
                Button { text: "Remove from draft"; enabled: absolutePort.currentIndex >= 0; onClicked: absoluteSelection.assign(true) }
            }
            Label { id: absoluteStatus; Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText }
        }
    }
    Dialog {
        id: keyboardConsent
        title: "Use the frontend keyboard?"
        modal: true
        width: dialog.width - 40
        standardButtons: Dialog.Ok | Dialog.Cancel
        contentItem: Label {
            text: "This replaces supported gamepad-to-key assignments in the draft with RetroArch's native keyboard input. Keys absent from the frontend's Linux key table (including colon and double quote) still require explicit mappings. A fully passthrough-backed port needs no gamepad. The frontend may combine connected keyboards; this does not select or exclusively capture one device. Keyboard hotkeys are disabled in the private game session. Discard the draft to recover saved mappings."
            wrapMode: Text.WordWrap
        }
        onAccepted: dialog.setKeyboardPassthrough(true)
    }
    Dialog {
        id: mappingPreview
        property var address: null
        property string partName: ""
        readonly property var row: address ? dialog.assignmentRows.find(row => dialog.sameAddress(row.target.address, address)) || null : null
        readonly property var part: row ? row.parts.find(part => part.part === partName) || null : null
        readonly property var catalog: JSON.parse(dialog.settingsModel.controller_catalog_json())
        readonly property var layout: row ? catalog.layouts.find(layout => layout.id === row.source_layout) || null : null
        title: "FBNeo source controller → native input"
        width: dialog.width - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; text: mappingPreview.row ? "Destination: " + mappingPreview.row.target.descriptions.join(" / ") + "\n" + mappingPreview.row.target.id + " · " + mappingPreview.partName + " · " + mappingPreview.row.target.encoding : "Target is no longer in the current review." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; text: "Source: " + (mappingPreview.layout ? mappingPreview.layout.name : "No controller layout") }
            ComboBox {
                Layout.fillWidth: true
                visible: !!mappingPreview.row && (mappingPreview.row.axis_pairs || []).length > 0
                model: ["Assign both halves of a measured axis…"].concat(mappingPreview.row ? (mappingPreview.row.axis_pairs || []).map(pair => pair.label) : [])
                currentIndex: 0
                onActivated: {
                    if (currentIndex > 0) dialog.assignAxisPair(mappingPreview.address, mappingPreview.row.axis_pairs[currentIndex - 1])
                    currentIndex = 0
                }
                Accessible.name: "Assign a complete calibrated axis or its reversed direction"
            }
            ControllerMappingView {
                Layout.fillWidth: true
                visible: !!mappingPreview.part && !!mappingPreview.part.destination_control
                settingsModel: dialog.settingsModel
                sourceLayout: mappingPreview.layout
                destinationLayout: mappingPreview.part ? mappingPreview.catalog.layouts.find(layout => layout.id === mappingPreview.part.destination_layout) || null : null
                rows: !mappingPreview.part || !mappingPreview.part.destination_control ? [] : [{
                    target_id: mappingPreview.part.destination_control,
                    target: mappingPreview.row.target.descriptions.join(" / ") || mappingPreview.row.target.id,
                    physical_id: mappingPreview.part.effective_source || null,
                    physical: mappingPreview.part.effective_source || "Unassigned",
                    output: mappingPreview.row.target.id + " / " + mappingPreview.partName,
                    reason: "Frontend channel geometry, not original cabinet geometry. The exact native address and input part remain the destination identity."
                }]
                onControlActivated: function(side, controlId) {
                    if (side === 0 && mappingPreview.part && mappingPreview.part.choices.some(choice => choice.id === controlId))
                        dialog.assignSource(mappingPreview.address, mappingPreview.partName, controlId)
                }
            }
            Item {
                id: sourcePanel
                visible: !mappingPreview.part || !mappingPreview.part.destination_control
                Layout.fillWidth: true
                Layout.preferredHeight: width * 500 / 900
                Image {
                    anchors.fill: parent
                    fillMode: Image.Stretch
                    sourceSize.width: Math.max(1, Math.ceil(width * Screen.devicePixelRatio))
                    sourceSize.height: Math.max(1, Math.ceil(height * Screen.devicePixelRatio))
                    source: mappingPreview.layout ? dialog.settingsModel.controller_diagram(mappingPreview.layout.id, mappingPreview.part ? mappingPreview.part.effective_source || "" : "") : ""
                    Accessible.name: "Selected FBNeo physical source control"
                }
                Repeater {
                    model: mappingPreview.layout ? mappingPreview.layout.controls : []
                    delegate: MouseArea {
                        required property var modelData
                        enabled: mappingPreview.part !== null && mappingPreview.part.choices.some(choice => choice.id === modelData.id)
                        x: (modelData.x * 8 + 50) * sourcePanel.width / 900 - width / 2
                        y: (modelData.y * 4 + 35) * sourcePanel.height / 500 - height / 2
                        width: Math.max(24, sourcePanel.width * 55 / 900)
                        height: Math.max(24, sourcePanel.height * 55 / 500)
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: dialog.assignSource(mappingPreview.address, mappingPreview.partName, modelData.id)
                        ToolTip.visible: containsMouse
                        ToolTip.text: modelData.label
                    }
                }
            }
            ComboBox {
                Layout.fillWidth: true
                model: [mappingPreview.part && mappingPreview.part.inherited_from ? "Use alias assignment: " + mappingPreview.part.effective_source : "Unassigned"].concat(mappingPreview.part ? mappingPreview.part.choices.map(choice => choice.label + " [" + choice.id + "]") : [])
                currentIndex: mappingPreview.part && mappingPreview.part.source ? mappingPreview.part.choices.findIndex(choice => choice.id === mappingPreview.part.source) + 1 : 0
                enabled: mappingPreview.part !== null
                onActivated: dialog.assignSource(mappingPreview.address, mappingPreview.partName, currentIndex > 0 ? mappingPreview.part.choices[currentIndex - 1].id : "")
                Accessible.name: "Physical control assigned to the selected FBNeo native input part"
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Click a calibrated control or use the list. The destination is an exact native input address, not a reconstructed cabinet layout. Changes affect the draft only; they do not verify live input or replace missing peripheral adapters." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: !!mappingPreview.part && mappingPreview.part.destination_layout === "fbneo-lightgun-buttons"; text: "This panel maps gun buttons only. Aim coordinates and offscreen status still require their own input adapter; assigning Offscreen shot does not provide them." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; visible: !!mappingPreview.part && !!mappingPreview.part.alias_source; text: !mappingPreview.part ? "" : "Shared channel source: " + mappingPreview.part.alias_source + (mappingPreview.part.inherited_from ? " from " + JSON.stringify(mappingPreview.part.inherited_from) + ". Edit that target to change the inherited binding; clearing this target does not clear its alias." : ". Explicit bindings on both aliases must agree at launch; pressure/digital fallback must not be duplicated.") }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: !!mappingPreview.part && (mappingPreview.part.alias_assignment_count || 0) > 1
                text: "The shared-channel owner has duplicate assignments. The displayed source is only its first assignment; repair the owner before staging."
            }
            Button {
                text: "View shared-channel owner…"
                visible: !!mappingPreview.part && !!mappingPreview.part.alias_target
                onClicked: {
                    const part = mappingPreview.part
                    mappingPreview.address = part.alias_target
                    mappingPreview.partName = part.alias_part
                }
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; text: dialog.assignmentError }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; visible: !!mappingPreview.part && !!mappingPreview.part.source_error; text: mappingPreview.part ? "Selected source: " + (mappingPreview.part.source_error || "") : "" }
        }
    }
    Dialog {
        id: assignmentEditor
        onOpened: { attentionOnly.checked = false; assignmentSearch.text = "" }
        onClosed: { mappingPreview.close(); mouseDestination.close() }
        title: "FBNeo physical assignments"
        width: dialog.width - 20
        height: dialog.height - 20
        modal: true
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            Label {
                Layout.fillWidth: true
                text: "Choose saved calibrated controls for each target part. These are draft edits, not live validation. Both halves of a full axis must use opposite gestures on the same measured axis."
                wrapMode: Text.WordWrap
            }
            Button { text: "Select calibrated absolute aim…"; onClicked: absoluteSelection.open() }
            Button {
                text: dialog.keyboardPassthroughActive ? "Disable frontend keyboard passthrough" : "Use frontend keyboard…"
                visible: dialog.keyboardPassthroughActive || dialog.assignmentRows.some(row => row.target.address.device === 3)
                onClicked: {
                    if (dialog.keyboardPassthroughActive) dialog.setKeyboardPassthrough(false)
                    else keyboardConsent.open()
                }
            }
            Label {
                Layout.fillWidth: true
                text: dialog.assignmentError
                visible: text.length > 0
                textFormat: Text.PlainText
                wrapMode: Text.WrapAnywhere
            }
            RowLayout {
                Layout.fillWidth: true
                CheckBox { id: attentionOnly; text: "Needs attention only" }
                TextField {
                    id: assignmentSearch
                    Layout.fillWidth: true
                    placeholderText: "Find action or native input ID…"
                    Accessible.name: "Filter FBNeo native inputs"
                }
            }
            RowLayout {
                Layout.fillWidth: true
                visible: dialog.staleAssignments.length > 0
                ComboBox {
                    id: staleChoice
                    Layout.fillWidth: true
                    model: dialog.staleAssignments.map(entry => "Player " + (entry.player_port + 1)
                        + " · " + JSON.stringify(entry.assignment.target) + " / " + entry.assignment.part
                        + " ← " + entry.assignment.source + " · " + entry.reason)
                    displayText: currentIndex < 0 ? "Choose stale assignment to remove…" : currentText
                    Accessible.name: "Stale FBNeo assignment"
                }
                Button {
                    text: "Remove selected stale assignment"
                    enabled: staleChoice.currentIndex >= 0
                    onClicked: dialog.removeStaleAssignment(dialog.staleAssignments[staleChoice.currentIndex])
                }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Showing " + targetList.count + " / " + dialog.assignmentRows.length + " native inputs. Filters do not change mappings or establish launch readiness."
            }
            ListView {
                id: targetList
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                spacing: 14
                model: dialog.assignmentRows.filter(row => {
                    if (attentionOnly.checked && !dialog.assignmentNeedsAttention(row)) return false
                    const query = assignmentSearch.text.trim().toLowerCase()
                    return !query || (row.target.descriptions.join(" / ") + " " + row.target.id).toLowerCase().indexOf(query) >= 0
                })
                ScrollBar.vertical: ScrollBar {}
                delegate: ColumnLayout {
                    id: targetRow
                    required property var modelData
                    width: targetList.width
                    Label {
                        Layout.fillWidth: true
                        text: "Player " + (targetRow.modelData.target.address.port + 1) + " · "
                            + (targetRow.modelData.target.descriptions.join(" / ") || "Unlabeled native input")
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                        font.bold: true
                    }
                    Label {
                        Layout.fillWidth: true
                        text: targetRow.modelData.target.id + " · " + targetRow.modelData.target.encoding
                            + " · controller " + targetRow.modelData.controller_id
                        textFormat: Text.PlainText
                        wrapMode: Text.WrapAnywhere
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: !!targetRow.modelData.calibration_error
                        text: (targetRow.modelData.calibration_error || "") + ". Use Replace controller in the setup editor, or repair its saved calibration. This player's mappings are unavailable; other players can still be reviewed."
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                    ComboBox {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.target.address.device === 3 && !targetRow.modelData.keyboard_passthrough
                        model: ["Choose internal keyboard channel…"].concat(dialog.keyboardChannels)
                        currentIndex: targetRow.modelData.keyboard_channel === null || targetRow.modelData.keyboard_channel === undefined
                            || targetRow.modelData.keyboard_channel >= 16 ? 0 : targetRow.modelData.keyboard_channel + 1
                        onActivated: dialog.assignKeyboardChannel(targetRow.modelData.target.address, currentIndex - 1)
                        Accessible.name: "Internal frontend channel for this native keyboard key"
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.target.address.device === 3 && !targetRow.modelData.keyboard_passthrough
                        wrapMode: Text.WordWrap
                        text: "Choose a unique internal channel, then its calibrated physical source below. The picker supports 16 button channels; advanced JSON supports 8 additional paired analog directions. More than 24 required keys cannot fit this adapter. Native key IDs are unchanged."
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.keyboard_passthrough === true
                        wrapMode: Text.WordWrap
                        text: "Source: frontend keyboard → this native key. No gamepad channel is consumed. This is native passthrough, not calibrated key remapping or exclusive device selection. Runtime behavior is unverified."
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.requires_external_adapter && targetRow.modelData.target.address.device !== 3
                        text: "Requires a separate input adapter; a gamepad binding cannot represent this target."
                            + (targetRow.modelData.relative_requirement
                                ? " Relative-device output required: event type " + targetRow.modelData.relative_requirement.event_type
                                    + ", code " + targetRow.modelData.relative_requirement.code
                                    + ". Matching saved devices: " + (targetRow.modelData.relative_candidates || []).map(device => device.event_path).join(", ")
                                    + ". Saved capability matches only; device identity, frontend routing and launch integration are not verified."
                                : "")
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.relative_source_selected === true
                        textFormat: Text.PlainText
                        wrapMode: Text.WrapAnywhere
                        text: {
                            const source = targetRow.modelData.relative_source
                            if (!source) return ""
                            const requirement = targetRow.modelData.relative_requirement
                            return "Prepared relative source: " + source.event_path + "\nIdentity: " + source.input_identity
                                + "\nOutput event type " + requirement.event_type + " / code " + requirement.code
                                + (targetRow.modelData.relative_launch_pending ? "\nPlatform requirements remain unmet." : "")
                                + "\nSaved configuration only. Launch requires the opt-in frontend and live identity, routing and focus checks; runtime behavior is unverified."
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: !!targetRow.modelData.absolute_source
                        wrapMode: Text.WrapAnywhere
                        textFormat: Text.PlainText
                        text: targetRow.modelData.absolute_source ? "Absolute aim source: " + targetRow.modelData.absolute_source.event_path
                            + "\nIdentity: " + targetRow.modelData.absolute_source.input_identity
                            + "\nSaved calibration → owned virtual gamepad aim. Runtime behavior is unverified; hardware offscreen/reload is not supplied." : ""
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: targetRow.modelData.pressure_alias
                        text: "This pressure binding also supplies its same-ID digital fallback. Do not duplicate the same axis assignment on both addresses."
                        wrapMode: Text.WordWrap
                    }
                    Button {
                        visible: !!targetRow.modelData.mouse_destination_control
                        text: "Show mouse destination"
                        onClicked: {
                            mouseDestination.controlId = targetRow.modelData.mouse_destination_control
                            mouseDestination.nativeId = targetRow.modelData.target.id
                            mouseDestination.mappingRow = targetRow.modelData
                            mouseDestination.open()
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: !!targetRow.modelData.lightgun_start_alias
                        text: "Legacy lightgun Pause and Start share one native channel. Assign either address; if both are assigned, they must use the same physical gesture."
                        wrapMode: Text.WordWrap
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: !!targetRow.modelData.arcade_coordinate_alias
                        text: "Arcade Gun coordinates share the advertised left analog-axis channel. Assign both measured axis halves on either address; duplicate assignments must use the same physical gestures. This is absolute stick positioning, not mouse movement or physical lightgun calibration."
                        wrapMode: Text.WordWrap
                    }
                    Repeater {
                        model: targetRow.modelData.parts
                        delegate: ColumnLayout {
                            id: partRow
                            required property var modelData
                            Layout.fillWidth: true
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: partRow.modelData.part }
                                Button {
                                    text: "Visual map…"
                                    onClicked: {
                                        mappingPreview.address = targetRow.modelData.target.address
                                        mappingPreview.partName = partRow.modelData.part
                                        mappingPreview.open()
                                    }
                                }
                                ComboBox {
                                    Layout.fillWidth: true
                                    model: [partRow.modelData.inherited_from ? "Use alias assignment: " + partRow.modelData.effective_source : "Unassigned"].concat(partRow.modelData.choices.map(choice => choice.label + " [" + choice.id + "]"))
                                    currentIndex: partRow.modelData.source
                                        ? partRow.modelData.choices.findIndex(choice => choice.id === partRow.modelData.source) + 1 : 0
                                    onActivated: dialog.assignSource(targetRow.modelData.target.address,
                                        partRow.modelData.part, currentIndex > 0 ? partRow.modelData.choices[currentIndex - 1].id : "")
                                }
                            }
                            Label {
                                Layout.fillWidth: true
                                wrapMode: Text.WordWrap
                                visible: (partRow.modelData.duplicate_count || 0) > 0
                                text: "Conflicting draft: " + (1 + (partRow.modelData.duplicate_count || 0)) + " assignments exist for this native input part. Only the first is displayed. Choose a source or Unassigned to replace/remove all of them; staging will reject unresolved duplicates."
                            }
                            Label {
                                Layout.fillWidth: true
                                wrapMode: Text.WordWrap
                                textFormat: Text.PlainText
                                visible: (partRow.modelData.alias_assignment_count || 0) > 1
                                text: "Shared-channel owner has " + partRow.modelData.alias_assignment_count + " assignments: " + JSON.stringify(partRow.modelData.alias_target) + " / " + partRow.modelData.alias_part + ". Open Visual map to navigate to that owner and repair it."
                            }
                            Label {
                                Layout.fillWidth: true
                                visible: !!partRow.modelData.effective_source
                                    && !partRow.modelData.choices.some(choice => choice.id === partRow.modelData.effective_source)
                                text: "Effective source is not eligible for this input part: " + (partRow.modelData.effective_source || "") + "\n" + (partRow.modelData.source_error || "Review its saved calibration.")
                                textFormat: Text.PlainText
                                wrapMode: Text.WrapAnywhere
                            }
                        }
                    }
                }
            }
        }
    }
    Dialog {
        id: mouseDestination
        property var mappingRow: null
        readonly property var sourceRoute: {
            const row = mappingRow
            if (!row || !row.relative_source_selected || !row.relative_source || !row.relative_requirement) return null
            const source = row.relative_source, requirement = row.relative_requirement
            const button = requirement.event_type === 1
            const motion = source.motion
            const output = button ? requirement.code - 0x110 + 1 : requirement.code
            const physical = button ? source.buttons.find(pair => pair[1] === requirement.code) : null
            if (button && !physical) return null
            return {kind: button ? "button" : "axis", player: row.target.address.port + 1,
                output: output, title: "FBNeo · " + row.target.id,
                details: (row.target.descriptions || []).join(" / ") + "\nNative callback: " + row.target.id,
                event_path: source.event_path, input_identity: source.input_identity,
                physical_button: button ? physical[0] : null,
                physical_axis: button ? null : motion.swap_xy ? output ^ 1 : output,
                sensitivity_percent: output === 0 ? motion.x_percent : motion.y_percent,
                inverted: output === 0 ? motion.invert_x : motion.invert_y}
        }
        onClosed: mappingRow = null
        property string controlId: ""
        property string nativeId: ""
        title: "FBNeo relative input mapping"
        width: Math.min(dialog.width - 40, 800)
        modal: true
        standardButtons: Dialog.Close
        height: Math.min(dialog.height - 30, 640)
        contentItem: ScrollView {
            id: mouseMappingScroll
            ColumnLayout {
                width: mouseMappingScroll.availableWidth
                ControllerRelativeMappingView {
                    Layout.fillWidth: true
                    visible: mouseDestination.sourceRoute !== null
                    routes: mouseDestination.sourceRoute ? [mouseDestination.sourceRoute] : []
                }
                Image {
                    visible: mouseDestination.sourceRoute === null
                    Layout.fillWidth: true
                    Layout.preferredHeight: width * 500 / 900
                    fillMode: Image.PreserveAspectFit
                    source: mouseDestination.visible ? dialog.settingsModel.controller_diagram("fbneo-mouse-channels", mouseDestination.controlId) : ""
                    sourceSize.width: Math.max(1, Math.ceil(width * Screen.devicePixelRatio))
                    sourceSize.height: Math.max(1, Math.ceil(height * Screen.devicePixelRatio))
                    Accessible.name: "Relative mouse destination: " + mouseDestination.controlId
                }
                Label {
                    Layout.fillWidth: true
                    text: "Native input: " + mouseDestination.nativeId
                    textFormat: Text.PlainText
                    wrapMode: Text.WrapAnywhere
                }
                Label {
                    Layout.fillWidth: true
                    text: "Saved logical mapping, not a physical cabinet replica. Live routing and input behavior are unverified. Targets without a prepared source show destination geometry only; unsupported conversions remain unresolved."
                    wrapMode: Text.WordWrap
                }
            }
        }
    }
    Dialog {
        id: removal
        property string keyToRemove: ""
        title: "Remove FBNeo setup?"
        modal: true
        standardButtons: Dialog.Yes | Dialog.No
        Label {
            text: "This removes only the staged setup. Game files and physical calibration stay unchanged."
            width: 400
            wrapMode: Text.WordWrap
        }
        onAccepted: {
            const error = dialog.settingsModel.remove_fbneo_controller_setup(keyToRemove)
            if (error) { dialog.statusText = error; return }
            dialog.selectedKey = ""
            dialog.loadedText = ""
            editor.text = ""
            dialog.refresh()
            dialog.statusText = "Removal staged. Save settings to keep it."
        }
    }
}
