import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog
    required property var settingsModel
    signal calibrationRequested(string controllerId, string sourceLayout, string physicalControl)
    property var setups: []
    property string loadedText: ""
    property string statusText: ""
    property string reviewedText: ""
    property bool reviewHasUnhandled: true
    property var reviewPlayers: []
    // Navigation intent only, retained separately from invalidatable evidence.
    property int preferredReviewPort: 0
    property string lastReviewContext: ""
    property var reviewExceptions: []
    property string reviewSummary: ""
    property string reviewCoverage: ""
    property var reviewRelativeRoutes: []
    readonly property int mappingRevision: settingsModel.controller_revision
    onMappingRevisionChanged: {
        // Calibration and inventory changes can alter resolved rows without
        // changing the per-game draft. Never retain that review as authority.
        if (reviewedText.length > 0) {
            reviewedText = ""
            reviewHasUnhandled = true
            reviewPlayers = []
            reviewExceptions = []
            reviewSummary = ""
            reviewCoverage = ""
            reviewRelativeRoutes = []
            review.close()
            statusText = "Controller settings or inventory changed. Review the current source/destination mappings again before staging. Your draft is unchanged."
        }
    }
    readonly property bool draftChanged: editor.text !== loadedText
    readonly property var presetIds: ["automatic", "six_button", "eight_button", "neo_geo", "fixed_channels"]
    readonly property var presetLabels: ["Automatic arcade layout (recommended)", "Six buttons: 1 2 3 / 4 5 6", "Eight buttons: 1 2 3 7 / 4 5 6 8", "Neo Geo: A B C D", "Keep legacy gamepad layout"]
    title: "MAME per-game controller setup"
    width: Math.min(900, parent ? parent.width - 40 : 900)
    height: Math.min(720, parent ? parent.height - 40 : 720)
    modal: true
    closePolicy: Popup.NoAutoClose
    function refresh() {
        try { setups = JSON.parse(settingsModel.mame_controller_setups_json()) }
        catch (error) { statusText = "Cannot read MAME setups: " + error }
    }
    function loadAndOpen() { refresh(); open() }
    function calibrateReviewedPlayer(focusSelected) {
        if (editor.text !== reviewedText || !review.player) {
            statusText = "Review the current draft and select a player before opening calibration."
            return
        }
        const controllerId = review.player.controller
        if (!controllerId) {
            statusText = "This reviewed player has no controller identity. Choose its controller first."
            return
        }
        const physicalControl = focusSelected ? mappingView.focusedSourceControl
            || (mappingView.selected ? mappingView.selected.physical_id || "" : "") : ""
        const sourceLayout = focusSelected ? review.player.source_layout || "" : ""
        if (focusSelected && (!physicalControl || !sourceLayout)) {
            statusText = "Select a source control in a known layout, or open the player's general calibration instead."
            return
        }
        review.close()
        // Keep the per-game draft, but do not retain its review as authority
        // across a calibration workflow, including a cancelled one.
        reviewedText = ""
        reviewHasUnhandled = true
        statusText = "Opening this player's controller calibration. The game draft is unchanged; review assignments again afterward."
        calibrationRequested(controllerId, sourceLayout, physicalControl)
    }
    function applyCompletedInspection() {
        try {
            const result = JSON.parse(settingsModel.apply_mame_inspection_json(editor.text))
            if (result.error) throw new Error(result.error)
            editor.text = result.configuration
            reviewedText = ""
            reviewHasUnhandled = true
            statusText = "Inspected snapshot, complete dependency manifest and hashes copied into the matching draft. Review the manifest and assignments before staging."
            return true
        } catch (error) {
            statusText = "Cannot apply inspection: " + error
            return false
        }
    }
    function fieldLabel(field, labels) {
        const entry = labels.find(entry => entry.field.tag === field.tag && entry.field.input_type === field.input_type && entry.field.mask === field.mask && entry.field.defvalue === field.defvalue)
        return (entry && entry.label ? entry.label + " · " : "") + field.input_type + " · " + field.tag + " · mask " + field.mask + " · default " + field.defvalue
    }
    function fieldCoverageSummary(coverage) {
        if (!coverage) return "Native field coverage is unavailable for this review."
        const percent = value => value === null || value === undefined
            ? " (percentage unavailable)" : " (" + value.toFixed(1) + "%)"
        let summary = "This game's inspected user inputs: " + coverage.mapped + " / " + coverage.user_fields
            + " mapped" + percent(coverage.mapped_percent) + "; " + coverage.disabled
            + " deliberately disabled; " + coverage.unresolved + " unresolved; "
            + coverage.native_service + " service inputs retained on native bindings."
        if (coverage.assignment_fields !== undefined) {
            summary += "\nAssignment scope: " + coverage.mapped + " / " + coverage.assignment_fields
                + " mapped" + percent(coverage.assignment_percent)
                + ". This excludes disabled and native-service inputs, but includes every unresolved input."
        }
        return summary + "\nMachine settings/internal signals are excluded from both denominators."
            + " These counts apply only to this inspected game/mode, not all MAME games, physical calibration or verified playability."
    }
    function layoutCoverageSummary(coverage) {
        if (!coverage) return "Selected-player destination-layout coverage is unavailable."
        return "Destination profiles: " + coverage.generated + " / " + coverage.selected_players
            + " selected gamepad players" + (coverage.generated_percent === null ? " (percentage unavailable)"
                : " (" + coverage.generated_percent.toFixed(1) + "%)")
            + "; " + coverage.failed + " layout errors; " + coverage.no_active_profile + " without an active profile."
            + " This counts generated layouts, not calibration, game support or playability."
    }
    function playerPresetDescription(port, draftText) {
        try {
            const draft = JSON.parse(draftText)
            const layouts = draft.player_digital_layouts || {}
            const overridden = Object.prototype.hasOwnProperty.call(layouts, String(port))
            const preset = overridden ? layouts[String(port)] : draft.digital_layout || "fixed_channels"
            const index = presetIds.indexOf(preset)
            if (index < 0) return "Preset selection is unavailable."
            return (overridden ? "Player override: " : "Inherited shared preset: ") + presetLabels[index]
        } catch (_) { return "Preset selection is unavailable until the draft is reviewed." }
    }
    function editReviewedSwitch(route) {
        try {
            if (editor.text !== reviewedText) throw new Error("The draft changed. Review it again before editing this connection.")
            if (!route || !route.assignment.output) throw new Error("Choose a button-controlled game action.")
            const draft = JSON.parse(editor.text)
            const assignment = route.assignment
            if (!review.player || (review.player.switch_actions || []).filter(other =>
                    switchEditor.sameField(other.assignment.field, assignment.field)
                    && other.assignment.source_player === assignment.source_player
                    && other.assignment.output === assignment.output
                    && (other.assignment.sequence || "standard") === (assignment.sequence || "standard")).length !== 1)
                throw new Error("Choose an exact button action from the current player's review.")
            switchEditor.fields = draft.reviewed_snapshot.fields.filter(field => switchEditor.sameField(field, assignment.field))
            if (switchEditor.fields.length !== 1) throw new Error("The reviewed native field no longer matches the draft.")
            switchEditor.labels = draft.reviewed_snapshot.field_labels || []
            switchEditor.ports = Object.keys(draft.players).map(Number).sort((a, b) => a - b)
            switchEditor.channels = JSON.parse(settingsModel.mame_digital_channels_json())
            switchEditor.baseText = editor.text
            switchEditor.status = "Editing this game action only. Choose its frontend button channel, then review the resulting physical mapping. Shared actions are not changed together."
            switchField.currentIndex = 0
            switchSequence.currentIndex = assignment.sequence === "decrement" ? 1 : 0
            switchEditor.loadSelection()
            // Default routes have no saved override to load yet.
            switchPort.currentIndex = switchEditor.ports.indexOf(assignment.source_player)
            switchChannel.currentIndex = switchEditor.channels.findIndex(channel => channel.output === assignment.output)
            review.close()
            switchEditor.open()
        } catch (error) { dialog.statusText = "Cannot edit reviewed connection: " + error }
    }
    function editableUnresolvedSwitch(field) {
        return field && !field.analog && ["controller", "keyboard", "misc"].indexOf(field.class) >= 0
    }
    function editUnresolvedSwitch(entry) {
        try {
            if (editor.text !== reviewedText) throw new Error("The draft changed. Review it again before assigning an unresolved switch.")
            if (!entry || !editableUnresolvedSwitch(entry.field))
                throw new Error("Choose an unresolved controller, keyboard or auxiliary switch.")
            if (!reviewExceptions.some(other => switchEditor.sameField(other.field, entry.field)))
                throw new Error("This field is no longer in the unresolved review.")
            const draft = JSON.parse(editor.text)
            switchEditor.fields = draft.reviewed_snapshot.fields.filter(field => switchEditor.sameField(field, entry.field)
                && editableUnresolvedSwitch(field) && field.class === entry.field.class)
            if (switchEditor.fields.length !== 1) throw new Error("The unresolved field no longer uniquely matches the inspected draft.")
            switchEditor.labels = draft.reviewed_snapshot.field_labels || []
            switchEditor.ports = Object.keys(draft.players).map(Number).sort((a, b) => a - b)
            switchEditor.channels = JSON.parse(settingsModel.mame_digital_channels_json())
            switchEditor.baseText = editor.text
            switchEditor.status = "Choose a source player and button channel for this unresolved game input. This changes the draft only; review its physical mapping before staging."
            switchField.currentIndex = 0
            switchSequence.currentIndex = 0
            switchEditor.loadSelection()
            review.close()
            switchEditor.open()
        } catch (error) { dialog.statusText = "Cannot assign unresolved switch: " + error }
    }
    function hasButtonOverride(route) {
        if (!route || !route.assignment || route.assignment.field.analog
                || route.assignment.field.class !== "controller"
                || (route.assignment.sequence || "standard") !== "standard") return false
        try {
            const draft = JSON.parse(editor.text)
            return (draft.digital_assignments || []).some(entry =>
                switchEditor.sameField(entry.field, route.assignment.field)
                && (entry.sequence || "standard") === "standard")
        } catch (_) { return false }
    }
    function restoreReviewedButton(route) {
        try {
            if (editor.text !== reviewedText || !hasButtonOverride(route)) throw new Error("Review the current draft and choose an overridden button first.")
            const draft = JSON.parse(editor.text)
            draft.digital_assignments = draft.digital_assignments.filter(entry =>
                !(switchEditor.sameField(entry.field, route.assignment.field)
                  && (entry.sequence || "standard") === "standard"))
            editor.text = JSON.stringify(draft, null, 2)
            reviewedText = ""
            reviewHasUnhandled = true
            review.close()
            statusText = "Removed this button override from the draft. The selected preset will resolve it again; automatic allocation may change other default channels. Review all mappings before staging. Other explicit overrides were kept."
        } catch (error) { statusText = "Cannot restore button default: " + error }
    }
    function isOwnNumberedOverride(entry, port) {
        return entry && entry.field && entry.source_player === port
            && !entry.field.analog && entry.field.class === "controller"
            && new RegExp("^P" + port + "_BUTTON([1-9]|1[0-6])$").test(entry.field.input_type)
            && (entry.sequence || "standard") === "standard"
    }
    function numberedOverrideCount(port) {
        try {
            const draft = JSON.parse(editor.text)
            return (draft.digital_assignments || []).filter(entry => isOwnNumberedOverride(entry, port)).length
        } catch (_) { return 0 }
    }
    function restorePlayerNumberedButtons(port) {
        try {
            if (editor.text !== reviewedText || !review.player || review.player.port !== port)
                throw new Error("Review the current draft and choose the player again before restoring buttons.")
            const draft = JSON.parse(editor.text)
            if (!Object.prototype.hasOwnProperty.call(draft.players, String(port)))
                throw new Error("This source player is no longer selected.")
            const assignments = draft.digital_assignments || []
            const removed = assignments.filter(entry => isOwnNumberedOverride(entry, port))
            if (!removed.length) throw new Error("This player has no own numbered-button overrides to restore.")
            for (const entry of removed) {
                if (draft.reviewed_snapshot.fields.filter(field => switchEditor.sameField(field, entry.field)).length !== 1)
                    throw new Error("An overridden field no longer uniquely matches the inspected draft.")
            }
            draft.digital_assignments = assignments.filter(entry => !isOwnNumberedOverride(entry, port))
            editor.text = JSON.stringify(draft, null, 2)
            reviewedText = ""
            reviewHasUnhandled = true
            review.close()
            statusText = "Removed " + removed.length + " numbered-button overrides for player " + port
                + " from the draft. Directions, service inputs, analog assignments, cross-player routes and other players were preserved. Review again: the preset may reallocate default channels. Stage and Save settings only after review."
        } catch (error) { statusText = "Cannot restore player buttons: " + error }
    }
    function swappableAction(route, routes) {
        if (!route || !route.assignment) return false
        const a = route.assignment
        return !a.field.analog && a.field.class === "controller"
            && /^P[1-8]_BUTTON([1-9]|1[0-6])$/.test(a.field.input_type)
            && (a.sequence || "standard") === "standard"
            && ["South", "East", "West", "North", "LeftBumper", "RightBumper", "LeftStick", "RightStick", "LeftTrigger", "RightTrigger"].indexOf(a.output) >= 0
            && routes.filter(other => switchEditor.sameField(other.assignment.field, a.field)
                && other.assignment.source_player === a.source_player
                && other.assignment.output === a.output
                && (other.assignment.sequence || "standard") === "standard").length === 1
            && routes.filter(other => other.assignment.output === a.output && other.assignment.source_player === a.source_player).length === 1
    }
    function swapReviewedActions(first, second) {
        try {
            if (editor.text !== reviewedText || !review.player) throw new Error("Review the current draft before swapping.")
            const routes = review.swapRoutes
            if (!swappableAction(first, routes) || !swappableAction(second, routes)) throw new Error("Choose two independent ordinary button actions.")
            const a = first.assignment, b = second.assignment
            if (a.output === b.output || a.source_player !== b.source_player) throw new Error("Choose different channels on the same source player.")
            const draft = JSON.parse(editor.text)
            // Freeze numbered actions so sparse/twin allocation cannot move
            // them. Leave direction fields intact for layout inference.
            const frozen = routes.filter(route => route.assignment.output && route.assignment.source_player === a.source_player
                && /^P[1-8]_BUTTON([1-9]|1[0-6])$/.test(route.assignment.field.input_type)).map(route => {
                const entry = JSON.parse(JSON.stringify(route.assignment))
                if (draft.reviewed_snapshot.fields.filter(field => switchEditor.sameField(field, entry.field)).length !== 1) throw new Error("A reviewed field no longer uniquely matches the inspected draft.")
                if (switchEditor.sameField(entry.field, a.field)) entry.output = b.output
                else if (switchEditor.sameField(entry.field, b.field)) entry.output = a.output
                return entry
            })
            draft.digital_assignments = (draft.digital_assignments || []).filter(entry => !frozen.some(other =>
                switchEditor.sameField(entry.field, other.field) && (entry.sequence || "standard") === (other.sequence || "standard"))).concat(frozen)
            editor.text = JSON.stringify(draft, null, 2)
            reviewedText = ""
            reviewHasUnhandled = true
            review.close()
            statusText = "Button channels swapped in this draft. Other numbered actions on this source player were preserved as overrides. Review the new physical mapping before staging."
        } catch (error) { dialog.statusText = "Cannot swap button actions: " + error }
    }
    onClosed: { playerControllers.close(); newSetup.close(); removal.close(); review.close(); inspection.close(); analogEditor.close(); switchEditor.close(); relativePreview.close(); presetComparison.close(); settingsModel.cancel_mame_inspection() }
    contentItem: ColumnLayout {
        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: "Start with an explicit MAME setup, persistent paths and calibrated player identities. Native inspection can discover split/nonmerged ROM, BIOS and CHD dependencies from exact folders using the selected core. New-setup guidance and merged-set discovery remain unfinished. Runtime and devices are rechecked at launch."
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Shared per-game button layout" }
            ComboBox {
                id: digitalPreset
                Layout.fillWidth: true
                model: dialog.presetLabels
                currentIndex: {
                    try {
                        const draft = JSON.parse(editor.text)
                        return dialog.presetIds.indexOf(draft.digital_layout || "fixed_channels")
                    } catch (_) { return -1 }
                }
                enabled: editor.text.trim().length > 0
                onActivated: {
                    try {
                        const draft = JSON.parse(editor.text)
                        draft.digital_layout = dialog.presetIds[index]
                        editor.text = JSON.stringify(draft, null, 2)
                        dialog.reviewedText = ""
                        dialog.reviewHasUnhandled = true
                        const overriddenPorts = Object.keys(draft.player_digital_layouts || {}).sort((a, b) => Number(a) - Number(b))
                        dialog.statusText = "Shared layout changed in this draft only. Review assignments, stage, then Save settings."
                            + (overriddenPorts.length ? " Player-specific presets were kept for ports " + overriddenPorts.join(", ") + ". Change those in Player controllers." : "")
                    } catch (error) { dialog.statusText = "Load a valid setup before choosing its layout: " + error }
                }
            }
        }
        Button {
            text: "Compare shared presets…"
            enabled: editor.text.trim().length > 0
            onClicked: presetComparison.loadAndOpen()
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: "Eight-button rows retain the six-button positions and add a rightmost column. Only active game controls are required. Choose a preset, then Review assignments to see source and destination controls and swap buttons. A saved per-game setup takes precedence over automatic defaults."
        }
        RowLayout {
            Layout.fillWidth: true
            Button { text: "New setup…"; enabled: !dialog.draftChanged; onClicked: { newSetup.errorText = ""; newSetup.open() } }
            Button { text: "Inspect native fields…"; onClicked: inspection.open() }
            ComboBox {
                id: choices
                Layout.fillWidth: true
                model: dialog.setups.map(item => item.machine + " — " + item.content + (item.needs_reinspection ? " [needs reinspection]" : ""))
            }
            Button {
                text: "Load"
                enabled: !dialog.draftChanged && choices.currentIndex >= 0
                onClicked: {
                    const selected = dialog.setups[choices.currentIndex]
                    editor.text = dialog.settingsModel.mame_controller_setup_json(selected.key)
                    dialog.loadedText = editor.text
                    dialog.reviewedText = ""
                    dialog.reviewHasUnhandled = true
                    dialog.statusText = selected.needs_reinspection
                        ? "Saved snapshot schema " + selected.snapshot_schema + " needs reinspection. Your setup and assignments are preserved, but launch is blocked. Inspect native fields, use the completed inspection, review assignments, then stage and Save settings."
                        : "Loaded staged setup; not a runtime verification."
                }
            }
            Button {
                text: "Remove…"
                enabled: choices.currentIndex >= 0
                onClicked: {
                    removal.keyToRemove = dialog.setups[choices.currentIndex].key
                    removal.open()
                }
            }
        }
        Button {
            text: "Change player controllers…"
            enabled: editor.text.trim().length > 0
            onClicked: playerControllers.loadAndOpen()
        }
        CheckBox {
            id: advancedControls
            text: "Advanced controls (analog axes, keyboard and service inputs)"
            checked: false
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            visible: advancedControls.checked
            text: "These tools are for unusual game inputs. Hiding them does not remove saved assignments or bypass unresolved-input checks. Analog assignments use saved calibration; this editor does not calibrate devices."
        }
        Button {
            text: advancedControls.checked ? "Edit button / incremental-axis assignments…" : "Edit game buttons…"
            enabled: editor.text.trim().length > 0
            onClicked: {
                try {
                    const draft = JSON.parse(editor.text)
                    switchEditor.fields = draft.reviewed_snapshot.fields.filter(field => (!field.analog && (field.class === "controller" || (advancedControls.checked && (field.class === "misc" || field.class === "keyboard")))) || (advancedControls.checked && field.analog && field.class === "controller" && /^P[1-8]_(AD_STICK_[XYZ]|PADDLE|PADDLE_V|POSITIONAL|POSITIONAL_V|PEDAL|PEDAL2|PEDAL3|DIAL|DIAL_V|TRACKBALL_[XY]|MOUSE_[XY]|LIGHTGUN_[XY])$/.test(field.input_type)))
                    switchEditor.labels = draft.reviewed_snapshot.field_labels || []
                    switchEditor.ports = Object.keys(draft.players).map(Number).sort((a, b) => a - b)
                    switchEditor.channels = JSON.parse(dialog.settingsModel.mame_digital_channels_json())
                    if (!switchEditor.fields.length || !switchEditor.ports.length) throw new Error("No inspected switch fields or selected source players.")
                    switchEditor.baseText = editor.text
                    switchEditor.status = ""
                    switchField.currentIndex = 0
                    switchSequence.currentIndex = 0
                    switchEditor.loadSelection()
                    switchEditor.open()
                } catch (error) { dialog.statusText = "Cannot edit digital assignments: " + error }
            }
        }
        Button {
            text: "Edit analog channel assignments…"
            visible: advancedControls.checked
            enabled: editor.text.trim().length > 0
            onClicked: {
                try {
                    const draft = JSON.parse(editor.text)
                    analogEditor.fields = draft.reviewed_snapshot.fields.filter(field => field.analog && field.class === "controller"
                        && /^P[1-8]_(AD_STICK_[XYZ]|PADDLE|PADDLE_V|POSITIONAL|POSITIONAL_V|PEDAL|PEDAL2|PEDAL3|DIAL|DIAL_V|TRACKBALL_[XY]|MOUSE_[XY]|LIGHTGUN_[XY])$/.test(field.input_type))
                    analogEditor.labels = draft.reviewed_snapshot.field_labels || []
                    analogEditor.ports = Object.keys(draft.players).map(Number).sort((a, b) => a - b)
                    if (!analogEditor.fields.length || !analogEditor.ports.length) throw new Error("No supported inspected analog fields or selected source players.")
                    analogEditor.baseText = editor.text
                    analogEditor.status = ""
                    analogField.currentIndex = 0
                    analogEditor.open()
                    analogEditor.loadSelection()
                } catch (error) { dialog.statusText = "Cannot edit analog assignments: " + error }
            }
        }
        ScrollView {
            id: setupSummary
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !showSetupJson.checked
            clip: true
            Label {
                width: setupSummary.availableWidth
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
                text: {
                    if (!editor.text.trim().length)
                        return "Create a new setup or load an existing one. Then choose your button layout and player controllers, review the visual assignments, and stage the setup."
                    try {
                        const draft = JSON.parse(editor.text)
                        if (!draft || typeof draft !== "object" || !draft.players
                                || typeof draft.players !== "object" || Array.isArray(draft.players))
                            throw new Error("Expected a setup with player controllers.")
                        const ports = Object.keys(draft.players).sort((a, b) => Number(a) - Number(b))
                        const lines = ["Machine: " + (draft.machine || "Not specified"),
                            "Content: " + (draft.content || "Not specified"), ""]
                        for (const port of ports) {
                            lines.push("Player " + port + ": " + draft.players[port])
                            lines.push(dialog.playerPresetDescription(port, editor.text))
                        }
                        if (!ports.length) lines.push("No player controllers selected.")
                        lines.push("", dialog.draftChanged ? "Draft has unstaged changes." : "No changes since this draft was loaded or staged.")
                        lines.push(dialog.reviewedText === editor.text
                            ? (dialog.reviewHasUnhandled ? "Review found inputs that still need setup." : "Current draft reviewed; staging is available.")
                            : "Review assignments to see the source and destination diagrams and current mapping gaps.")
                        lines.push("Staging is separate from Save settings. This summary is not runtime verification.")
                        return lines.join("\n")
                    } catch (error) {
                        return "The draft cannot be summarized: " + error
                            + "\nShow setup JSON to repair it, or revert the editor. Your text has been preserved."
                    }
                }
            }
        }
        CheckBox {
            id: showSetupJson
            text: "Show setup JSON (advanced)"
            checked: false
            Accessible.description: "Show or hide the raw setup editor without changing the draft or its assignments."
        }
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: showSetupJson.checked
            TextArea {
                id: editor
                placeholderText: "Paste complete MAME setup JSON"
                selectByMouse: true
                wrapMode: TextEdit.NoWrap
                font.family: "monospace"
            }
        }
        Label {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            text: dialog.statusText
        }
        RowLayout {
            Button {
                text: "Review assignments"
                enabled: editor.text.trim().length > 0
                onClicked: {
                    try {
                        const result = JSON.parse(dialog.settingsModel.mame_assignment_review_json(editor.text))
                        if (result.error) { dialog.statusText = result.error; return }
                        const draft = JSON.parse(editor.text)
                        const reviewContext = JSON.stringify([draft.emulator_id, draft.core, draft.core_sha256,
                            draft.content, draft.content_sha256, draft.machine, draft.library])
                        const retainedPlayer = reviewContext === dialog.lastReviewContext
                            ? result.players.findIndex(player => player.port === dialog.preferredReviewPort) : -1
                        let lines = ["Saved calibration preview only — no device or runtime verification.", ""]
                        dialog.reviewCoverage = dialog.fieldCoverageSummary(result.field_coverage)
                        const pendingMouseButtons = (result.native_mouse_routing || {}).button_assignments || []
                        if (pendingMouseButtons.length) {
                            const mode = (result.native_mouse_routing || {}).game_mouse_mode || {}
                            dialog.reviewCoverage += "\n" + pendingMouseButtons.length
                                + " mouse-button assignments are planned. Launch requires the opt-in frontend and core, validated private UI configuration, and live source identity checks. Runtime behavior is unverified."
                            if ((result.native_mouse_routing || {}).platform_error)
                                dialog.reviewCoverage += "\n" + result.native_mouse_routing.platform_error
                            dialog.reviewCoverage += "\nCore mouse mode: " + (mode.declared === true
                                ? "required mode declared in saved inspection; runtime behavior and saved UI-binding isolation remain unverified."
                                : mode.error || "missing mode evidence; reinspect the opt-in core with mouse input enabled.")
                            for (const assignment of pendingMouseButtons) {
                                const field = assignment.field
                                dialog.reviewCoverage += "\nPlayer " + assignment.source_player + " / mouse button "
                                    + assignment.output_button + " → " + field.input_type + " / " + field.tag
                                    + " / mask " + field.mask + " / default " + field.defvalue
                            }
                        }
                        dialog.reviewRelativeRoutes = ((result.native_mouse_routing || {}).routes || []).map(route => {
                            const assignment = route.assignment
                            const field = assignment.field
                            const description = "Player " + assignment.source_player + " · " + route.event_path
                                + "\nIdentity: " + route.input_identity
                                + "\nPhysical " + (route.physical_axis === 0 ? "X" : "Y")
                                + " → " + route.sensitivity_percent + "%" + (route.inverted ? " · inverted" : "")
                                + " → output " + (assignment.output_axis === 0 ? "X" : "Y")
                                + " → " + dialog.fieldLabel(field, [])
                                + "\n" + field.tag + " / " + field.input_type + " / mask " + field.mask
                                + " / default " + field.defvalue
                                + "\nSaved axis mapping only; runtime input unverified."
                            return Object.assign({}, route, {description: description, fieldLabel: dialog.fieldLabel(field, [])})
                        })
                        dialog.reviewRelativeRoutes = dialog.reviewRelativeRoutes.concat(
                            ((result.native_mouse_routing || {}).button_routes || []).map(route => {
                                const field = route.assignment.field
                                const description = "Player " + route.assignment.source_player + " · " + route.event_path
                                    + "\nIdentity: " + route.input_identity
                                    + "\nPhysical button code 0x" + route.physical_button.toString(16)
                                    + " → native mouse button " + route.assignment.output_button
                                    + " → " + dialog.fieldLabel(field, [])
                                    + "\n" + field.tag + " / " + field.input_type + " / mask " + field.mask
                                    + " / default " + field.defvalue + "\nSaved button mapping only; runtime input unverified."
                                return Object.assign({}, route, {description: description, fieldLabel: dialog.fieldLabel(field, [])})
                            }))
                        lines.push(...dialog.reviewRelativeRoutes.map(route => route.description))
                        for (const port of (result.native_mouse_routing || {}).relative_only_ports || []) {
                            dialog.reviewCoverage += "\nRelative-only player " + port.port + ": "
                                + (port.error || "no gamepad-channel requirements; physical runtime behavior remains unverified.")
                        }
                        lines.push(dialog.reviewCoverage, "")
                        for (const player of result.players) {
                            lines.push("Player " + player.port + " — " + player.controller)
                            lines.push(player.layout)
                            lines.push(dialog.playerPresetDescription(player.port, editor.text))
                            for (const row of player.rows)
                                lines.push(row.target + " ← " + (row.physical || "UNMAPPED") + " → " + row.output + "\n  " + row.reason)
                            const switchGroups = {}
                            for (const route of player.native_routes || []) {
                                const assignment = route.assignment
                                if (!assignment.output) continue
                                if (!switchGroups[assignment.output]) switchGroups[assignment.output] = []
                                switchGroups[assignment.output].push(assignment.field.input_type + " / " + assignment.field.tag + " / " + (assignment.sequence || "standard") + (route.explicit ? " [override]" : " [default]"))
                            }
                            for (const output of Object.keys(switchGroups)) {
                                if (switchGroups[output].length > 1) lines.push("SHARED SWITCH " + output + ": " + switchGroups[output].join("; "))
                            }
                            for (const warning of player.warnings) lines.push("Note: " + warning)
                            lines.push("")
                        }
                        for (const assignment of result.digital_assignments || []) {
                            const field = assignment.field
                            lines.push("BUTTON OVERRIDE: " + field.tag + " / " + field.input_type + " (mask " + field.mask + ", default " + field.defvalue + ") / " + (assignment.sequence || "standard") + " ← source player " + assignment.source_player + " / " + assignment.output)
                        }
                        if ((result.analog_assignments || []).length) {
                            for (const calibration of result.analog_calibrations || []) {
                                lines.push("Analog calibration — source player " + calibration.port)
                                if (calibration.error) lines.push("NEEDS CALIBRATION: " + calibration.error)
                                for (const row of calibration.rows) lines.push(row.target + " ← " + row.physical + " → " + row.output)
                                for (const warning of calibration.warnings) lines.push("Note: " + warning)
                            }
                            lines.push("ANALOG CHANNEL CHOICES — saved measurement checks only, live input unverified:")
                            for (const assignment of result.analog_assignments) {
                                const field = assignment.field
                                lines.push(field.tag + " / " + field.input_type + " (mask " + field.mask + ", default " + field.defvalue + ") ← source player " + assignment.source_player + " / " + assignment.channel + " / " + (assignment.stick_range || "full") + (assignment.relative_velocity ? " · STICK VELOCITY, NOT PHYSICAL DELTAS" : "") + (assignment.controller_aim ? " · CONTROLLER AIM, NOT PHYSICAL GUN CAPTURE" : ""))
                            }
                            lines.push(result.analog_pending ? "Resolve the analog calibration errors before staging." : "Saved analog measurements satisfy the preparation checks. Staging does not verify runtime behavior; fresh native fields and live input identity/bounds are checked at launch.", "")
                        }
                        if (result.unhandled.length) {
                            lines.push("UNHANDLED NATIVE FIELDS — digital launch remains blocked:")
                            for (const field of result.unhandled) {
                                const entry = (result.unresolved_controls || []).find(entry => switchEditor.sameField(entry.field, field))
                                lines.push(field.tag + " / " + field.input_type + " [" + field.class + "]"
                                    + (entry && entry.guidance ? "\n  " + entry.guidance : ""))
                            }
                        }
                        lines.push("Preserved machine settings: " + (result.preserved_settings || []).length + " DIP/configuration fields (not controller bindings).")
                        const internal = result.preserved_internal || []
                        lines.push("Preserved native internal signals: " + internal.length + " (not controller bindings).")
                        for (const field of internal) lines.push("  " + field.tag + " / " + field.input_type + " [internal]")
                        for (const field of result.preserved_service || [])
                            lines.push("NATIVE SERVICE BINDING PRESERVED: " + field.tag + " / " + field.input_type + " / mask " + field.mask + " / default " + field.defvalue)
                        reviewText.text = lines.join("\n")
                        dialog.reviewPlayers = result.players
                        dialog.reviewExceptions = result.unresolved_controls || []
                        const missingSources = result.players.reduce((count, player) => count + player.rows.filter(row => !row.physical_id).length, 0)
                        dialog.reviewSummary = missingSources + " displayed controls have no source assignment; "
                            + result.unhandled.length + " native inputs need extra setup."
                            + ((result.preserved_service || []).length ? " " + result.preserved_service.length
                                + " cabinet service inputs keep native bindings (not calibrated button mappings)." : "")
                            + (result.analog_pending ? " Analog calibration also needs attention." : "")
                            + (result.calibration_pending ? " One or more players have unavailable calibration; staging is blocked." : "")
                            + (result.physical_pending ? " A selected player's layout or physical mapping is incomplete. Check per-player warnings; staging is blocked." : "")
                            + " This is a saved-mapping review, not a live-input check."
                        dialog.reviewSummary += "\n" + dialog.layoutCoverageSummary(result.layout_coverage)
                        technicalReport.checked = false
                        reviewPlayer.currentIndex = retainedPlayer >= 0 ? retainedPlayer : result.players.length ? 0 : -1
                        dialog.preferredReviewPort = reviewPlayer.currentIndex >= 0
                            ? result.players[reviewPlayer.currentIndex].port : 0
                        dialog.lastReviewContext = reviewContext
                        dialog.reviewedText = editor.text
                        dialog.reviewHasUnhandled = result.unhandled.length > 0 || result.analog_pending === true || result.calibration_pending === true || result.physical_pending === true
                        review.open()
                    } catch (error) { dialog.statusText = "Cannot review assignments: " + error }
                }
            }
            Button {
                text: "Use completed inspection"
                enabled: !dialog.settingsModel.mame_inspection_busy && dialog.settingsModel.mame_inspection_result.length > 0
                onClicked: dialog.applyCompletedInspection()
            }
            Button {
                text: "Stage setup"
                enabled: editor.text.trim().length > 0 && dialog.reviewedText === editor.text && !dialog.reviewHasUnhandled
                onClicked: {
                    const error = dialog.settingsModel.save_mame_controller_setup(editor.text)
                    if (error) { dialog.statusText = error; return }
                    dialog.loadedText = editor.text
                    dialog.refresh()
                    dialog.statusText = "Setup staged. Save settings to persist it. No runtime was started."
                }
            }
            Button {
                text: "Revert editor"
                enabled: dialog.draftChanged
                onClicked: { editor.text = dialog.loadedText; dialog.statusText = "Editor reverted to its last loaded or staged text." }
            }
            Item { Layout.fillWidth: true }
            Button { text: "Close (keep draft)"; onClicked: dialog.close() }
        }
    }
    Dialog {
        id: presetComparison
        property string baseText: ""
        property int baseRevision: -1
        property string report: ""
        property int completedChoices: 0
        property int generation: 0
        property bool comparing: false
        property var generatedChoices: []
        property var generatedPreviews: ({})
        readonly property var previewPlayers: stale ? [] : generatedPreviews[generatedChoices[comparedPreset.currentIndex]] || []
        readonly property var previewPlayer: previewPlayers[comparedPlayer.currentIndex] || null
        onPreviewPlayersChanged: {
            const preferred = previewPlayers.findIndex(player => player.port === comparisonPort)
            comparedPlayer.currentIndex = preferred >= 0 ? preferred : (previewPlayers.length ? 0 : -1)
        }
        property int comparisonPort: 0
        readonly property var choiceIds: comparisonPort > 0 ? ["inherit"].concat(dialog.presetIds) : dialog.presetIds
        readonly property var choiceLabels: comparisonPort > 0 ? ["Inherit shared preset (remove this override)"].concat(dialog.presetLabels) : dialog.presetLabels
        readonly property string comparisonScope: comparisonPort > 0 ? "Player " + comparisonPort + " override" : "Shared preset"
        onGeneratedChoicesChanged: comparedPreset.currentIndex = -1
        readonly property bool stale: baseText !== editor.text || baseRevision !== dialog.mappingRevision
        onStaleChanged: { if (stale) cancelComparison() }
        onClosed: cancelComparison()
        title: comparisonScope + " comparison"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Close
        function cancelComparison() {
            comparisonStep.stop()
            comparing = false
            ++generation
        }
        function candidateDraft(index) {
            const preset = choiceIds[index]
            if (!preset) throw new Error("Choose a valid preset comparison result.")
            const draft = JSON.parse(baseText)
            if (comparisonPort > 0) {
                if (!Object.prototype.hasOwnProperty.call(draft.players || {}, String(comparisonPort)))
                    throw new Error("The compared player is not selected in this draft.")
                draft.player_digital_layouts = Object.assign({}, draft.player_digital_layouts || {})
                if (preset === "inherit") {
                    delete draft.player_digital_layouts[String(comparisonPort)]
                    if (!Object.keys(draft.player_digital_layouts).length) delete draft.player_digital_layouts
                } else {
                    draft.player_digital_layouts[String(comparisonPort)] = preset
                }
            } else {
                draft.digital_layout = preset
            }
            return draft
        }
        function loadAndOpen(port) {
            cancelComparison()
            comparisonPort = port === undefined ? 0 : port
            baseText = editor.text
            baseRevision = dialog.mappingRevision
            completedChoices = 0
            generatedChoices = []
            generatedPreviews = ({})
            const lines = ["Draft-only comparison. No setup was changed or staged, and no device or emulator was started.",
                comparisonPort > 0
                    ? "Only player " + comparisonPort + "'s preset override is varied. Shared settings and all other players are preserved. The complete setup is checked; unrelated player errors can still block generation."
                    : "Per-player preset overrides are preserved. Only players inheriting the shared preset are affected.", ""]
            try {
                if (!Number.isInteger(comparisonPort) || comparisonPort < 0 || comparisonPort > 8)
                    throw new Error("Invalid comparison player.")
                const original = JSON.parse(baseText)
                if (comparisonPort > 0 && !Object.prototype.hasOwnProperty.call(original.players || {}, String(comparisonPort)))
                    throw new Error("The compared player is not selected.")
                const overrides = original.player_digital_layouts || {}
                const inherited = Object.keys(original.players || {}).filter(port =>
                    !Object.prototype.hasOwnProperty.call(overrides, port)).sort((a, b) => Number(a) - Number(b))
                if (comparisonPort === 0) lines.push("Players inheriting the shared preset: " + (inherited.length ? inherited.join(", ") : "none; shared preset changes have no player effect"), "")
                comparing = true
            } catch (error) { lines.push("Cannot compare presets: " + error) }
            report = lines.join("\n")
            open()
            if (comparing) comparisonStep.start()
        }
        function compareNext() {
            if (!visible || stale || !comparing) { cancelComparison(); return }
            const token = generation
            const index = completedChoices
            if (index >= choiceIds.length) { comparing = false; return }
            const lines = [choiceLabels[index]]
            let generated = false
            let players = []
            try {
                const draft = candidateDraft(index)
                const result = JSON.parse(dialog.settingsModel.mame_assignment_review_json(JSON.stringify(draft)))
                if (result.error) throw new Error(result.error)
                if (!Array.isArray(result.players)) throw new Error("Player preview data is unavailable.")
                players = result.players
                lines.push(dialog.layoutCoverageSummary(result.layout_coverage))
                const layoutErrors = players.filter(player => !!player.layout_error)
                lines.push(layoutErrors.length
                    ? "Partial review: " + layoutErrors.length + " player layout(s) could not be generated."
                    : "Profile generation succeeded; this is not launch readiness.")
                for (const player of result.players || [])
                    lines.push("Player " + player.port + ": " + (player.layout_error
                        ? "LAYOUT ERROR: " + player.layout_error : player.target_layout || "no active destination profile"))
                if (scopeLayoutErrors(index, players).length)
                    lines.push("This preset cannot be applied from comparison: a player affected by this choice has a layout error.")
                lines.push(dialog.fieldCoverageSummary(result.field_coverage))
                if (result.physical_pending || result.calibration_pending)
                    lines.push("Physical assignments or saved calibration still need attention.")
                generated = true
            } catch (error) { lines.push("Cannot generate this draft's profiles: " + error) }
            if (token !== generation || !visible || stale || !comparing) return
            if (generated) {
                const previews = Object.assign({}, generatedPreviews)
                previews[index] = players
                generatedPreviews = previews
                generatedChoices = generatedChoices.concat([index])
            }
            report += "\n" + lines.join("\n") + "\n"
            ++completedChoices
            comparing = completedChoices < choiceIds.length
            if (comparing) comparisonStep.start()
        }
        function scopeLayoutErrors(index, players) {
            const draft = candidateDraft(index)
            const overrides = draft.player_digital_layouts || {}
            return players.filter(player => !!player.layout_error && (comparisonPort > 0
                ? player.port === comparisonPort
                : !Object.prototype.hasOwnProperty.call(overrides, String(player.port))))
        }
        readonly property bool selectedPresetUsable: {
            const index = generatedChoices[comparedPreset.currentIndex]
            if (stale || comparing || index === undefined || completedChoices !== choiceIds.length) return false
            try { return scopeLayoutErrors(index, generatedPreviews[index] || []).length === 0 }
            catch (_) { return false }
        }
        function applySelection() {
            if (!selectedPresetUsable) return
            const index = generatedChoices[comparedPreset.currentIndex]
            if (index === undefined || !choiceIds[index]) return
            try {
                const draft = candidateDraft(index)
                // Preserve unrelated overrides and all exact-field assignments.
                // Applying never reuses comparison as review.
                editor.text = JSON.stringify(draft, null, 2)
                dialog.reviewedText = ""
                dialog.reviewHasUnhandled = true
                dialog.statusText = comparisonScope + " applied to the draft. Unrelated presets and field assignments were preserved. Review assignments again, stage, then Save settings."
                close()
            } catch (error) { report += "\nCannot apply preset: " + error }
        }
        function calibratePreview(focusControl) {
            if (stale || comparing || completedChoices !== choiceIds.length || !previewPlayer) return
            try {
                const player = previewPlayer
                const draft = JSON.parse(baseText)
                if (!player.controller || (draft.players || {})[String(player.port)] !== player.controller)
                    throw new Error("The preview controller does not match this draft's player.")
                const sourceLayout = focusControl ? player.source_layout || "" : ""
                const control = focusControl ? comparedMapping.focusedSourceControl
                    || (comparedMapping.selected ? comparedMapping.selected.physical_id || "" : "") : ""
                if (focusControl && (!sourceLayout || !control))
                    throw new Error("Highlight a source control in a known layout first.")
                cancelComparison()
                generatedPreviews = ({})
                generatedChoices = []
                completedChoices = 0
                dialog.reviewedText = ""
                dialog.reviewHasUnhandled = true
                dialog.statusText = "Opening calibration from the candidate preview. No preset was applied and the game draft is unchanged. Compare or review again afterward."
                close()
                dialog.calibrationRequested(player.controller, sourceLayout, control)
            } catch (error) { report += "\nCannot open calibration: " + error }
        }
        Timer {
            id: comparisonStep
            interval: 1
            repeat: false
            onTriggered: presetComparison.compareNext()
        }
        contentItem: ColumnLayout {
            RowLayout {
                Layout.fillWidth: true
                ComboBox {
                    id: comparedPreset
                    Layout.fillWidth: true
                    model: presetComparison.generatedChoices.map(index => presetComparison.choiceLabels[index]
                        + (presetComparison.scopeLayoutErrors(index, presetComparison.generatedPreviews[index] || []).length
                            ? " — incompatible layout" : ""))
                    displayText: currentIndex < 0 ? "Choose a reviewed preset candidate" : currentText
                    enabled: !presetComparison.stale && !presetComparison.comparing
                        && presetComparison.completedChoices === presetComparison.choiceIds.length
                    Accessible.name: "Generated " + presetComparison.comparisonScope + " to apply to the MAME draft"
                }
                Button {
                    text: "Use in draft"
                    enabled: presetComparison.selectedPresetUsable
                    onClicked: presetComparison.applySelection()
                }
            }
            Label {
                Layout.fillWidth: true
                text: "Using a preset changes only " + presetComparison.comparisonScope.toLowerCase() + " in the draft. Generation success does not resolve calibration or prove launch readiness; review is required again."
                wrapMode: Text.WordWrap
            }
            Label {
                Layout.fillWidth: true
                text: (presetComparison.comparing ? "Comparing: " : "Compared: ")
                    + presetComparison.completedChoices + " / " + presetComparison.choiceIds.length
                    + " choices. Results below are profile checks, not verified support."
                wrapMode: Text.WordWrap
            }
            Label {
                Layout.fillWidth: true
                visible: presetComparison.stale
                wrapMode: Text.WordWrap
                text: "This comparison is stale because the draft or controller settings changed. Close and compare again."
            }
            ScrollView {
                id: comparisonScroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ColumnLayout {
                    width: comparisonScroll.availableWidth
                    ComboBox {
                        id: comparedPlayer
                        Layout.fillWidth: true
                        visible: presetComparison.previewPlayers.length > 0
                        model: presetComparison.previewPlayers.map(player => "Player " + player.port + " · " + player.controller)
                        Accessible.name: "Player to visualize for the compared preset"
                    }
                    ControllerMappingView {
                        id: comparedMapping
                        Layout.fillWidth: true
                        visible: presetComparison.previewPlayer !== null && !!presetComparison.previewPlayer.target_layout
                        settingsModel: dialog.settingsModel
                        sourceLayout: presetComparison.previewPlayer
                            ? review.catalog.layouts.find(layout => layout.id === presetComparison.previewPlayer.source_layout) || null : null
                        destinationLayout: presetComparison.previewPlayer
                            ? review.catalog.layouts.find(layout => layout.id === presetComparison.previewPlayer.target_layout) || null : null
                        rows: presetComparison.previewPlayer ? presetComparison.previewPlayer.rows || [] : []
                        nativeRoutes: presetComparison.previewPlayer ? presetComparison.previewPlayer.native_routes || [] : []
                        physicalGaps: presetComparison.previewPlayer ? presetComparison.previewPlayer.physical_gaps || [] : []
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        visible: presetComparison.previewPlayer !== null
                        enabled: !presetComparison.stale && !presetComparison.comparing
                            && presetComparison.completedChoices === presetComparison.choiceIds.length
                        Button {
                            text: "Calibrate previewed controller…"
                            onClicked: presetComparison.calibratePreview(false)
                        }
                        Button {
                            text: "Calibrate highlighted source…"
                            enabled: presetComparison.previewPlayer !== null && !!presetComparison.previewPlayer.source_layout
                                && (!!comparedMapping.focusedSourceControl || (comparedMapping.selected !== null && !!comparedMapping.selected.physical_id))
                            onClicked: presetComparison.calibratePreview(true)
                        }
                    }
                    Repeater {
                        model: presetComparison.previewPlayer ? presetComparison.previewPlayer.warnings || [] : []
                        delegate: Label {
                            required property string modelData
                            Layout.fillWidth: true
                            text: "Note: " + modelData
                            textFormat: Text.PlainText
                            wrapMode: Text.WordWrap
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: presetComparison.previewPlayer !== null && !presetComparison.previewPlayer.target_layout
                        text: "This player has no active destination profile for the selected candidate. No visual mapping is established."
                        wrapMode: Text.WordWrap
                    }
                    TextArea {
                        Layout.fillWidth: true
                        text: presetComparison.report
                        textFormat: TextEdit.PlainText
                        readOnly: true
                        selectByMouse: true
                        wrapMode: TextEdit.Wrap
                    }
                }
            }
        }
    }
    Dialog {
        id: switchEditor
        readonly property bool analogFieldSelected: !!fields[switchField.currentIndex] && fields[switchField.currentIndex].analog
        readonly property string selectedSequence: analogFieldSelected ? (["increment", "decrement"][switchSequence.currentIndex] || "") : "standard"
        property var labels: []
        property var fields: []
        property var ports: []
        property var channels: []
        property string baseText: ""
        property string status: ""
        property string channelPreview: ""
        property var candidatePlayers: []
        property var originalPlayers: []
        property bool showOriginalMapping: false
        function currentChannelPlayer() {
            const port = ports[switchPort.currentIndex]
            const reviewed = originalPlayers.length ? originalPlayers
                : dialog.reviewedText === baseText && editor.text === baseText ? dialog.reviewPlayers : []
            const players = reviewed.filter(player => player.port === port)
            return players.length === 1 ? players[0] : null
        }
        function otherSwitchRoutes(channel) {
            const player = currentChannelPlayer()
            if (!channel || !player || !Array.isArray(player.switch_actions)) return null
            const field = fields[switchField.currentIndex]
            return player.switch_actions.filter(route => {
                const assignment = route.assignment
                return assignment && assignment.output === channel.output
                    && !(field && sameField(assignment.field, field)
                        && (assignment.sequence || "standard") === selectedSequence)
            })
        }
        function channelChoiceLabel(channel) {
            if (!channel) return "Choose a button channel"
            const player = currentChannelPlayer()
            const routes = otherSwitchRoutes(channel)
            const occupancy = routes === null ? " · switch usage unavailable"
                : routes.length ? " · SHARED with " + routes.length + " other switch route(s)"
                : " · no other switch routes"
            if (!player) return channel.output + occupancy + " · current physical mapping unavailable"
            const rows = (player.rows || []).filter(row => row.output === channel.output)
            if (rows.length !== 1) return channel.output + occupancy + " · no unique current connection"
            const row = rows[0]
            if (!row.physical_id) return channel.output + occupancy + " · UNMAPPED"
            const gap = (player.physical_gaps || []).some(entry => entry.target_id === row.target_id)
            return channel.output + occupancy + " · " + row.physical + " [" + row.physical_id + "]"
                + (gap ? " · needs calibration" : "")
        }
        readonly property var candidatePlayer: candidatePlayers[switchPreviewPlayer.currentIndex] || null
        readonly property var displayedPlayer: {
            if (!showOriginalMapping || !candidatePlayer) return candidatePlayer
            const matches = originalPlayers.filter(player => player.port === candidatePlayer.port
                && player.controller === candidatePlayer.controller)
            return matches.length === 1 ? matches[0] : null
        }
        function clearChannelPreview() {
            channelPreview = ""
            candidatePlayers = []
            originalPlayers = []
            showOriginalMapping = false
        }
        onClosed: clearChannelPreview()
        function retainCandidate(result, before, selection) {
            if (!visible || previewSelection !== selection) throw new Error("The preview selection changed. Preview again.")
            if (!Array.isArray(result.players)) throw new Error("Candidate player layouts are unavailable.")
            if (!Array.isArray(before.players)) throw new Error("Current player layouts are unavailable.")
            originalPlayers = before.players
            candidatePlayers = result.players
            const selected = candidatePlayers.findIndex(player => player.port === ports[switchPort.currentIndex])
            switchPreviewPlayer.currentIndex = selected >= 0 ? selected : candidatePlayers.length ? 0 : -1
        }
        readonly property bool numberedFieldSelected: isNumberedField(fields[switchField.currentIndex])
        function isNumberedField(field) {
            return !!field && !field.analog && field.class === "controller"
                && /^P[1-8]_BUTTON([1-9]|1[0-6])$/.test(field.input_type)
        }
        function preserveNumberedRoutes(draft, before, editedField) {
            if (!numberedFieldSelected || !keepNumberedChannels.checked) return 0
            if (!Array.isArray(before.players)) throw new Error("Current player routes are unavailable; no draft changes were made.")
            let added = 0
            const seen = new Set()
            for (const player of before.players) {
                if (!Array.isArray(player.switch_actions)) throw new Error("Complete button routes are unavailable.")
                for (const route of player.switch_actions) {
                    const entry = route.assignment
                    if (!entry || !entry.output || !isNumberedField(entry.field)
                            || (entry.sequence || "standard") !== "standard" || sameField(entry.field, editedField)) continue
                    const key = JSON.stringify([entry.field.tag, entry.field.input_type, entry.field.mask, entry.field.defvalue])
                    if (seen.has(key)) throw new Error("A numbered button has ambiguous resolved routes.")
                    seen.add(key)
                    if (draft.reviewed_snapshot.fields.filter(field => sameField(field, entry.field)).length !== 1)
                        throw new Error("A numbered button no longer uniquely matches the inspected draft.")
                    const existing = draft.digital_assignments.filter(other => sameField(other.field, entry.field)
                        && (other.sequence || "standard") === "standard")
                    if (existing.length > 1 || (existing.length === 1
                            && (existing[0].output !== entry.output || existing[0].source_player !== entry.source_player)))
                        throw new Error("A saved numbered-button override disagrees with the current review.")
                    if (!existing.length) {
                        draft.digital_assignments.push(JSON.parse(JSON.stringify(entry)))
                        added++
                    }
                }
            }
            return added
        }
        readonly property string previewSelection: JSON.stringify([baseText, editor.text, fields[switchField.currentIndex],
            selectedSequence, ports[switchPort.currentIndex], channels[switchChannel.currentIndex], dialog.mappingRevision,
            keepNumberedChannels.checked])
        onPreviewSelectionChanged: clearChannelPreview()
        readonly property bool hasSelectedOverride: {
            try {
                const draft = JSON.parse(baseText)
                return (draft.digital_assignments || []).filter(assignment =>
                    sameField(assignment.field, fields[switchField.currentIndex])
                    && (assignment.sequence || "standard") === selectedSequence).length === 1
            } catch (_) { return false }
        }
        function previewOverrideRemoval() {
            clearChannelPreview()
            const selection = previewSelection
            try {
                if (editor.text !== baseText || !hasSelectedOverride)
                    throw new Error("Choose an existing override in the unchanged draft.")
                const field = fields[switchField.currentIndex]
                const draft = JSON.parse(baseText)
                draft.digital_assignments = draft.digital_assignments.filter(assignment =>
                    !(sameField(assignment.field, field) && (assignment.sequence || "standard") === selectedSequence))
                const before = JSON.parse(dialog.settingsModel.mame_assignment_review_json(baseText))
                if (before.error) throw new Error("Current draft: " + before.error)
                const after = JSON.parse(dialog.settingsModel.mame_assignment_review_json(JSON.stringify(draft)))
                if (after.error) throw new Error("Proposed removal: " + after.error)
                const lines = ["Proposed removal: " + dialog.fieldLabel(field, draft.reviewed_snapshot.field_labels || [])
                    + " · " + selectedSequence]
                lines.push(...switchChanges(before, after, draft.reviewed_snapshot.field_labels || []))
                if (field.analog) lines.push("Removing a button direction can leave it disabled while another direction remains, or leave the analog input unresolved when the last direction is removed.")
                lines.push("Preview only; no override was removed. Ordinary inputs return to default allocation where available. Review all mappings after removal; this is not physical calibration or runtime verification.")
                channelPreview = lines.join("\n")
                retainCandidate(after, before, selection)
            } catch (error) { channelPreview = "Cannot preview override removal: " + error }
        }
        function unresolvedChanges(before, after, fieldLabels) {
            function indexFields(result) {
                if (!Array.isArray(result.unresolved_controls)) throw new Error("Unresolved-field comparison is unavailable.")
                const indexed = new Map()
                for (const entry of result.unresolved_controls) {
                    const field = entry.field
                    const key = JSON.stringify([field.tag, field.input_type, field.mask, field.defvalue])
                    if (indexed.has(key)) throw new Error("Duplicate unresolved native field identity.")
                    indexed.set(key, entry)
                }
                return indexed
            }
            const oldFields = indexFields(before), newFields = indexFields(after)
            const added = Array.from(newFields.keys()).filter(key => !oldFields.has(key)).sort()
            const removed = Array.from(oldFields.keys()).filter(key => !newFields.has(key)).sort()
            const lines = ["Unresolved fields: " + oldFields.size + " → " + newFields.size
                + " (" + added.length + " newly unresolved, " + removed.length + " no longer unresolved)."]
            for (const key of added) {
                const entry = newFields.get(key)
                lines.push("NEEDS SETUP: " + dialog.fieldLabel(entry.field, fieldLabels)
                    + (entry.guidance ? "\n  " + entry.guidance : ""))
            }
            for (const key of removed) lines.push("NO LONGER UNRESOLVED: "
                + dialog.fieldLabel(oldFields.get(key).field, fieldLabels))
            if (removed.length) lines.push("Leaving the unresolved list can mean mapped, deliberately disabled or preserved by policy. It does not prove physical input coverage or playability.")
            return lines
        }
        function switchChanges(before, after, fieldLabels) {
            function indexRoutes(result) {
                const indexed = new Map()
                for (const player of result.players) {
                    if (!Array.isArray(player.switch_actions)) throw new Error("Complete switch comparison is unavailable.")
                    for (const route of player.switch_actions) {
                        const field = route.assignment.field
                        const key = JSON.stringify([field.tag, field.input_type, field.mask, field.defvalue,
                            route.assignment.sequence || "standard"])
                        if (indexed.has(key)) throw new Error("A native switch sequence has duplicate resolved routes.")
                        indexed.set(key, route)
                    }
                }
                return indexed
            }
            const oldRoutes = indexRoutes(before), newRoutes = indexRoutes(after)
            const keys = Array.from(new Set(Array.from(oldRoutes.keys()).concat(Array.from(newRoutes.keys())))).sort()
            const describeSource = route => route ? "player " + route.assignment.source_player + " / "
                + route.assignment.output + (route.explicit ? " (override)" : " (default)") : "no resolved switch route"
            const changes = []
            for (const key of keys) {
                const oldRoute = oldRoutes.get(key), newRoute = newRoutes.get(key)
                if (describeSource(oldRoute) === describeSource(newRoute)) continue
                const assignment = (newRoute || oldRoute).assignment
                changes.push("• " + dialog.fieldLabel(assignment.field, fieldLabels)
                    + " · " + (assignment.sequence || "standard") + "\n  "
                    + describeSource(oldRoute) + " → " + describeSource(newRoute))
            }
            return ["Switch-route changes across all selected players: " + changes.length + "."]
                .concat(changes)
                .concat(["No resolved switch route does not mean the input is disabled; it may require another mapping mode."])
                .concat(unresolvedChanges(before, after, fieldLabels))
        }
        function previewChannelEffects() {
            clearChannelPreview()
            const selection = previewSelection
            try {
                if (editor.text !== baseText) throw new Error("The setup draft changed. Reopen this form.")
                const field = fields[switchField.currentIndex]
                const channel = channels[switchChannel.currentIndex]
                const port = ports[switchPort.currentIndex]
                if (!field || !channel || port === undefined || !selectedSequence)
                    throw new Error("Choose a field, source player, channel and direction first.")
                const draft = JSON.parse(baseText)
                draft.digital_assignments = (draft.digital_assignments || []).filter(assignment =>
                    !(sameField(assignment.field, field) && (assignment.sequence || "standard") === selectedSequence))
                draft.digital_assignments.push({field: field, source_player: port, output: channel.output, sequence: selectedSequence})
                const before = JSON.parse(dialog.settingsModel.mame_assignment_review_json(baseText))
                if (before.error) throw new Error("Cannot resolve the current draft for comparison: " + before.error)
                const preserved = preserveNumberedRoutes(draft, before, field)
                const result = JSON.parse(dialog.settingsModel.mame_assignment_review_json(JSON.stringify(draft)))
                if (result.error) throw new Error(result.error)
                const changes = switchChanges(before, result, draft.reviewed_snapshot.field_labels || [])
                const player = result.players.find(player => player.port === port)
                if (!player || !Array.isArray(player.switch_actions)) throw new Error("Resolved switch actions are unavailable.")
                const describe = route => dialog.fieldLabel(route.assignment.field, draft.reviewed_snapshot.field_labels || [])
                    + " · " + (route.assignment.sequence || "standard")
                    + (route.explicit ? " · override" : " · default");
                const same = player.switch_actions.filter(route => route.assignment.output === channel.output)
                const opposite = channel.opposite_output ? player.switch_actions.filter(route =>
                    route.assignment.output === channel.opposite_output) : []
                const lines = ["Proposed source player " + port + " / " + channel.output + ": " + same.length
                    + " resolved switch actions, including this proposed assignment."]
                if (numberedFieldSelected && keepNumberedChannels.checked)
                    lines.push("Preserves other resolved numbered-button channels; " + preserved + " default routes become explicit overrides.")
                for (const route of same) lines.push("• " + describe(route))
                if (same.length > 1) lines.push("These actions share one channel and can activate together.")
                if (channel.opposite_output) {
                    lines.push("Opposing axis channel " + channel.opposite_output + ": " + opposite.length + " actions.")
                    for (const route of opposite) lines.push("• " + describe(route))
                }
                const analog = channel.axis_channel ? (result.analog_assignments || []).filter(assignment =>
                    assignment.source_player === port && assignment.channel === channel.axis_channel) : []
                for (const assignment of analog) lines.push("Shared analog axis: "
                    + dialog.fieldLabel(assignment.field, draft.reviewed_snapshot.field_labels || []))
                lines.push("")
                lines.push(...changes)
                lines.push("Preview only; the draft is unchanged. This compares switch routes, not physical calibration or every analog effect. Review the complete mapping after applying. No physical input or runtime behavior was verified.")
                channelPreview = lines.join("\n")
                retainCandidate(result, before, selection)
            } catch (error) { channelPreview = "Cannot preview channel effects: " + error }
        }
        function channelBehavior(channelIndex, portIndex, draftText) {
            const channel = channels[channelIndex]
            if (!channel) return ""
            const behavior = channel.opposite_output
                ? "Shares a bipolar axis with " + channel.opposite_output + ". Opposing button inputs cancel to neutral; analog travel on this axis also affects both switches."
                : channel.axis_channel
                ? "Shares trigger pressure channel " + channel.axis_channel + ". The native axis threshold determines switch activation."
                : "This is a digital switch channel."
            const port = ports[portIndex]
            if (port === undefined) return behavior
            try {
                const draft = JSON.parse(draftText)
                const switches = (draft.digital_assignments || []).filter(assignment => assignment.source_player === port)
                const same = switches.filter(assignment => assignment.output === channel.output).length
                const opposite = channel.opposite_output ? switches.filter(assignment => assignment.output === channel.opposite_output).length : 0
                const analog = channel.axis_channel ? (draft.analog_assignments || []).filter(assignment => assignment.source_player === port && assignment.channel === channel.axis_channel).length : 0
                return behavior + "\nSaved draft assignments on this source port: " + same + " same-channel switches"
                    + (channel.opposite_output ? ", " + opposite + " opposing switches" : "")
                    + (channel.axis_channel ? ", " + analog + " analog fields sharing this axis" : "")
                    + ". These counts exclude inferred default routes; review the complete map before staging."
            } catch (error) { return behavior + "\nCannot read draft sharing: " + String(error) }
        }
        function sameField(a, b) {
            return a && b && a.tag === b.tag && a.input_type === b.input_type && a.mask === b.mask && a.defvalue === b.defvalue
        }
        function loadSelection() {
            try {
                const draft = JSON.parse(baseText)
                const assignments = draft.digital_assignments || []
                if (!Array.isArray(assignments)) throw new Error("digital_assignments must be a list.")
                const saved = assignments.find(assignment => sameField(assignment.field, fields[switchField.currentIndex]) && (assignment.sequence || "standard") === selectedSequence)
                switchPort.currentIndex = saved ? ports.indexOf(saved.source_player) : -1
                switchChannel.currentIndex = saved ? channels.findIndex(channel => channel.output === saved.output) : -1
            } catch (error) { status = String(error) }
        }
        function updateAssignment(remove) {
            try {
                if (editor.text !== baseText) throw new Error("The setup draft changed. Close and reopen this form.")
                const field = fields[switchField.currentIndex]
                if (!field) throw new Error("Choose an inspected switch field.")
                if (remove && !hasSelectedOverride) throw new Error("This field and sequence have no unique saved override to remove.")
                if (!selectedSequence) throw new Error("Choose increment or decrement.")
                const channel = channels[switchChannel.currentIndex]
                const port = ports[switchPort.currentIndex]
                if (!remove && (!channel || port === undefined)) throw new Error("Choose both a source player and a button channel.")
                const draft = JSON.parse(baseText)
                const assignments = draft.digital_assignments || []
                if (!Array.isArray(assignments)) throw new Error("digital_assignments must be a list.")
                if (!remove && field.analog && (draft.analog_assignments || []).some(assignment => sameField(assignment.field, field))) throw new Error("This field already has an analog channel assignment. Remove it in the analog editor before choosing button-driven control.")
                draft.digital_assignments = assignments.filter(assignment => !(sameField(assignment.field, field) && (assignment.sequence || "standard") === selectedSequence))
                if (!remove) draft.digital_assignments.push({field: field, source_player: port, output: channel.output, sequence: selectedSequence})
                let preserved = 0
                if (!remove && numberedFieldSelected && keepNumberedChannels.checked) {
                    const before = JSON.parse(dialog.settingsModel.mame_assignment_review_json(baseText))
                    if (before.error) throw new Error("Cannot preserve current button channels: " + before.error)
                    preserved = preserveNumberedRoutes(draft, before, field)
                }
                editor.text = JSON.stringify(draft, null, 2)
                baseText = editor.text
                dialog.reviewedText = ""
                dialog.reviewHasUnhandled = true
                const shared = !remove && draft.digital_assignments.filter(assignment => assignment.source_player === port && assignment.output === channel.output).length > 1
                const otherDirection = draft.digital_assignments.some(assignment => sameField(assignment.field, field))
                status = remove ? (field.analog ? (otherDirection ? "Direction removed; the other direction is preserved. The removed direction is disabled in button-driven mode." : "Last button direction removed. The analog field needs another assignment before launch.") : "Override removed. The normal native mapping applies again; this does not disable the field.") : "Override saved in draft. Review physical calibration and native routes before staging." + (shared ? " Multiple sequences share this button; opposite directions on one axis can cancel or interact." : "")
                loadSelection()
                if (!remove && numberedFieldSelected && keepNumberedChannels.checked)
                    status += " Other resolved numbered-button channels were preserved; " + preserved + " defaults became overrides. Layout geometry and physical mappings still require review."
            } catch (error) { status = String(error) }
        }
        title: analogFieldSelected ? "MAME button-driven axis" : "MAME button assignment"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            ScrollView {
                id: switchFormScroll
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: availableWidth
                ColumnLayout {
                    width: switchFormScroll.availableWidth
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: switchEditor.analogFieldSelected ? "Choose a source button for one direction of this game axis. This changes the draft only." : "Choose the game action, source player and button channel. Preview its effects, then apply the override and review the mapping." }
                    ComboBox { id: switchField; Layout.fillWidth: true; model: switchEditor.fields.map(field => dialog.fieldLabel(field, switchEditor.labels)); onActivated: { switchSequence.currentIndex = 0; switchEditor.loadSelection() } Accessible.name: "Inspected button-controlled field" }
                    ComboBox { id: switchSequence; Layout.fillWidth: true; visible: switchEditor.analogFieldSelected; model: ["Increment", "Decrement"]; onActivated: switchEditor.loadSelection(); Accessible.name: "Button-driven analog direction" }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: switchEditor.analogFieldSelected; text: "Native keydelta, centering, wrapping and sensitivity determine motion. This is incremental button control, not measured analog travel or physical mouse/gun capture. Each direction is edited separately; unassigned directions and the standard axis sequence are disabled while any button direction remains assigned." }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: !!switchEditor.fields[switchField.currentIndex] && switchEditor.fields[switchField.currentIndex].class === "keyboard"; text: "Keyboard keys require fresh owner/enable-state evidence and emulated keyboard mode. This maps individual keys, not text entry." }
                    CheckBox { id: switchTechnical; text: "Show native channel details"; checked: false }
                    ComboBox { id: switchPort; Layout.fillWidth: true; model: switchEditor.ports.map(port => "Source player " + port); Accessible.name: "Digital source player" }
                    ComboBox {
                        id: switchChannel
                        Layout.fillWidth: true
                        model: switchEditor.channels
                        textRole: "output"
                        displayText: switchEditor.channelChoiceLabel(switchEditor.channels[currentIndex])
                        Accessible.name: "Digital source channel and current physical connection"
                        delegate: ItemDelegate {
                            required property var modelData
                            required property int index
                            width: switchChannel.width
                            text: switchEditor.channelChoiceLabel(modelData)
                            highlighted: switchChannel.highlightedIndex === index
                        }
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        text: "Button labels describe the current draft's saved physical connections and other switch routes, not the proposed result. No other switch routes does not prove independent physical capacity or absence of analog ownership. Preview channel effects to inspect the new layout; missing labels do not prevent choosing a channel."
                    }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        textFormat: Text.PlainText
                        text: {
                            const routes = switchEditor.otherSwitchRoutes(switchEditor.channels[switchChannel.currentIndex])
                            if (routes === null || !routes.length) return ""
                            return "This channel currently also drives:\n" + routes.map(route => {
                                const assignment = route.assignment
                                const field = assignment.field
                                return (route.label ? route.label + " · " : "") + field.input_type
                                    + " / " + field.tag + " / mask " + field.mask + " / default " + field.defvalue
                                    + " · " + (assignment.sequence || "standard")
                                    + (route.explicit ? " · explicit" : " · default")
                            }).join("\n") + "\nSharing may activate these actions together. Preview is required because default allocation can change."
                        }
                        visible: text.length > 0
                    }
                    CheckBox {
                        id: keepNumberedChannels
                        visible: switchEditor.numberedFieldSelected
                        checked: true
                        text: "Keep other numbered-button channels unchanged"
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: keepNumberedChannels.visible
                        wrapMode: Text.WordWrap
                        text: "Preserves resolved numbered-button channels across selected players as explicit overrides. Directions and non-button defaults are not frozen. Choosing an occupied channel can still make actions share a button; review the resulting physical layout. Restore buttons to preset removes these overrides."
                    }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText; visible: switchTechnical.checked && switchChannel.currentIndex >= 0; text: "Native channel: " + (switchEditor.channels[switchChannel.currentIndex] || {}).item }
                    Label {
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                        textFormat: Text.PlainText
                        text: {
                            const behavior = switchEditor.channelBehavior(switchChannel.currentIndex, switchPort.currentIndex, switchEditor.baseText)
                            return switchTechnical.checked ? behavior : behavior.split("\n")[0]
                        }
                        visible: text.length > 0
                    }
                    Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Channels can also drive existing default mappings or other explicit fields. Review shared actions before use, especially service, reset or coin controls. This is not physical calibration or runtime verification." }
                    Button {
                        text: "Preview channel effects"
                        enabled: editor.text === switchEditor.baseText && switchField.currentIndex >= 0
                            && switchPort.currentIndex >= 0 && switchChannel.currentIndex >= 0
                            && switchEditor.selectedSequence.length > 0
                        onClicked: switchEditor.previewChannelEffects()
                    }
                    Button {
                        text: "Preview override removal"
                        enabled: editor.text === switchEditor.baseText && switchEditor.hasSelectedOverride
                        onClicked: switchEditor.previewOverrideRemoval()
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: switchEditor.candidatePlayers.length > 0
                        text: "Proposed mapping only — the draft is unchanged. Inspect each player's connections and warnings before applying, then review again before staging."
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                    ComboBox {
                        id: switchPreviewPlayer
                        Layout.fillWidth: true
                        visible: switchEditor.candidatePlayers.length > 0
                        model: switchEditor.candidatePlayers.map(player => "Player " + player.port + " · " + player.controller)
                        Accessible.name: "Player to inspect in the proposed button mapping"
                    }
                    CheckBox {
                        visible: switchEditor.candidatePlayers.length > 0
                        text: "Show current draft mapping (before this change)"
                        checked: switchEditor.showOriginalMapping
                        onToggled: switchEditor.showOriginalMapping = checked
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: switchEditor.candidatePlayers.length > 0
                        text: switchEditor.showOriginalMapping ? "CURRENT DRAFT — before the proposed edit" : "PROPOSED — not yet applied"
                        textFormat: Text.PlainText
                    }
                    ControllerMappingView {
                        Layout.fillWidth: true
                        visible: switchEditor.displayedPlayer !== null && !!switchEditor.displayedPlayer.target_layout
                        settingsModel: dialog.settingsModel
                        sourceLayout: switchEditor.displayedPlayer
                            ? review.catalog.layouts.find(layout => layout.id === switchEditor.displayedPlayer.source_layout) || null : null
                        destinationLayout: switchEditor.displayedPlayer
                            ? review.catalog.layouts.find(layout => layout.id === switchEditor.displayedPlayer.target_layout) || null : null
                        rows: switchEditor.displayedPlayer ? switchEditor.displayedPlayer.rows || [] : []
                        nativeRoutes: switchEditor.displayedPlayer ? switchEditor.displayedPlayer.native_routes || [] : []
                        physicalGaps: switchEditor.displayedPlayer ? switchEditor.displayedPlayer.physical_gaps || [] : []
                    }
                    Label {
                        Layout.fillWidth: true
                        visible: switchEditor.candidatePlayer !== null
                            && (!switchEditor.displayedPlayer || !switchEditor.displayedPlayer.target_layout)
                        text: "No uniquely matched destination diagram is available for this player in the selected version. Consult the route report; no physical mapping completeness is established."
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                    Repeater {
                        model: switchEditor.displayedPlayer ? switchEditor.displayedPlayer.warnings || [] : []
                        delegate: Label {
                            required property string modelData
                            Layout.fillWidth: true
                            text: "Note: " + modelData
                            textFormat: Text.PlainText
                            wrapMode: Text.WordWrap
                        }
                    }
                    ScrollView {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 140
                        visible: switchEditor.channelPreview.length > 0
                        TextArea {
                            text: switchEditor.channelPreview
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.Wrap
                            textFormat: TextEdit.PlainText
                            Accessible.name: "Proposed MAME channel sharing, including default assignments"
                        }
                    }
                }
            }
            RowLayout {
                Button { text: "Add / replace override"; onClicked: switchEditor.updateAssignment(false) }
                Button { text: "Remove override"; enabled: editor.text === switchEditor.baseText && switchEditor.hasSelectedOverride; onClicked: switchEditor.updateAssignment(true) }
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: switchEditor.status }
        }
    }
    Dialog {
        id: relativePreview
        property string baseText: ""
        property int baseRevision: -1
        property var fields: []
        property var ports: []
        property var devices: []
        property string report: ""
        property var queuedAxes: []
        property var retainedButtons: []
        property var buttonFields: []
        property var buttonSourceOverrides: ({})
        readonly property bool mouseInspected: {
            try { return (JSON.parse(editor.text).reviewed_snapshot.mouse_state || {}).enabled === true }
            catch (_) { return false }
        }
        property string draftConfiguration: ""
        readonly property int savedCount: {
            try {
                const draft = JSON.parse(editor.text)
                return (draft.relative_assignments || []).length + (draft.relative_button_assignments || []).length
            }
            catch (_) { return 0 }
        }
        onQueuedAxesChanged: { report = ""; draftConfiguration = "" }
        onRetainedButtonsChanged: { report = ""; draftConfiguration = "" }
        onButtonSourceOverridesChanged: { report = ""; draftConfiguration = "" }
        readonly property bool stale: baseText !== editor.text || baseRevision !== dialog.mappingRevision
        readonly property string selection: JSON.stringify([stale, relativeField.currentIndex,
            relativePort.currentIndex, relativeAxis.currentIndex, relativeDevice.currentIndex, nativeOnly.checked])
        onSelectionChanged: { report = ""; draftConfiguration = "" }
        onClosed: { relativeTuning.close(); fields = []; buttonFields = []; buttonSourceOverrides = ({}); devices = []; queuedAxes = []; retainedButtons = []; report = ""; draftConfiguration = "" }
        function loadQueuedAxis(index) {
            if (stale) throw new Error("Review the current draft again before editing.")
            const row = queuedAxes[index]
            if (!row) throw new Error("Choose an existing axis assignment.")
            const fieldIndex = fields.findIndex(entry => switchEditor.sameField(entry.field, row.assignment.field))
            const portIndex = ports.indexOf(row.assignment.source_player)
            const deviceIndex = row.device ? devices.findIndex(device => JSON.stringify(device) === JSON.stringify(row.device)) : -1
            if (fieldIndex < 0 || portIndex < 0 || [0, 1].indexOf(row.assignment.output_axis) < 0
                    || (row.device && deviceIndex < 0))
                throw new Error("This axis assignment no longer matches the available fields or sources.")
            relativeField.currentIndex = fieldIndex
            relativePort.currentIndex = portIndex
            relativeAxis.currentIndex = row.assignment.output_axis
            relativeDevice.currentIndex = deviceIndex
            nativeOnly.checked = !row.device
            queuedRelativeAxis.currentIndex = index
            report = "Loaded the selected axis. Edit its source or output, then Add / replace axis and preview the complete batch. No draft changes were made."
            draftConfiguration = ""
        }
        function loadAndOpen(selectedAssignment) {
            if (dialog.reviewedText !== editor.text) return
            try {
                baseText = editor.text
                baseRevision = dialog.mappingRevision
                const draft = JSON.parse(baseText)
                retainedButtons = draft.relative_button_assignments || []
                buttonFields = draft.reviewed_snapshot.fields.filter(field => !field.analog
                    && (field.class === "controller" || field.class === "misc"))
                buttonSourceOverrides = ({})
                newMouseButtonField.currentIndex = -1
                mouseButtonOutput.currentIndex = -1
                fields = draft.reviewed_snapshot.fields.filter(field => field.analog && field.class === "controller"
                    && /^P[1-8]_(DIAL|DIAL_V|TRACKBALL_[XY]|MOUSE_[XY])$/.test(field.input_type)).map(field => ({field: field}))
                ports = [1, 2, 3, 4, 5, 6, 7, 8]
                devices = JSON.parse(dialog.settingsModel.relative_device_settings_json())
                for (const source of draft.relative_sources || []) {
                    if (!devices.some(device => JSON.stringify(device) === JSON.stringify(source.device)))
                        devices = devices.concat([source.device])
                }
                relativeField.currentIndex = -1; relativePort.currentIndex = -1
                relativeDevice.currentIndex = -1; relativeAxis.currentIndex = -1
                nativeOnly.checked = false
                queuedAxes = (draft.relative_assignments || []).map(assignment => {
                    const source = (draft.relative_sources || []).find(source => source.source_player === assignment.source_player)
                    return {assignment: assignment, device: source ? source.device : null}
                })
                report = ""; draftConfiguration = ""
                if (selectedAssignment && selectedAssignment.output_button !== undefined) {
                    const index = retainedButtons.findIndex(assignment => switchEditor.sameField(assignment.field, selectedAssignment.field)
                        && assignment.source_player === selectedAssignment.source_player
                        && assignment.output_button === selectedAssignment.output_button)
                    const fieldIndex = buttonFields.findIndex(field => switchEditor.sameField(field, selectedAssignment.field))
                    const portIndex = ports.indexOf(selectedAssignment.source_player)
                    const source = (draft.relative_sources || []).find(source => source.source_player === selectedAssignment.source_player)
                    const deviceIndex = source ? devices.findIndex(device => JSON.stringify(device) === JSON.stringify(source.device)) : -1
                    if (index < 0 || fieldIndex < 0 || portIndex < 0 || deviceIndex < 0)
                        throw new Error("The selected button route no longer matches its fields or sources. Review the draft again.")
                    existingMouseButton.currentIndex = index
                    newMouseButtonField.currentIndex = fieldIndex
                    relativePort.currentIndex = portIndex
                    relativeDevice.currentIndex = deviceIndex
                    mouseButtonOutput.currentIndex = selectedAssignment.output_button - 1
                    report = "Loaded the selected mouse button. Edit its source or output, then Add / replace button and preview the complete batch. No draft changes were made."
                } else if (selectedAssignment) {
                    const index = queuedAxes.findIndex(row => switchEditor.sameField(row.assignment.field, selectedAssignment.field)
                        && row.assignment.source_player === selectedAssignment.source_player
                        && row.assignment.output_axis === selectedAssignment.output_axis)
                    if (index < 0) throw new Error("The selected route no longer exists. Review the draft again.")
                    loadQueuedAxis(index)
                }
                review.close()
                open()
            } catch (error) { dialog.statusText = "Cannot open relative preview: " + error }
        }
        function queueSelection() {
            report = ""
            try {
                if (stale) throw new Error("Review the current draft again before previewing.")
                const entry = fields[relativeField.currentIndex], port = ports[relativePort.currentIndex]
                if (!entry || port === undefined || relativeAxis.currentIndex < 0)
                    throw new Error("Choose a native field, source player and output axis.")
                const device = nativeOnly.checked ? null : devices[relativeDevice.currentIndex]
                if (!nativeOnly.checked && !device) throw new Error("Choose a saved relative device or explicitly select native-only preview.")
                const row = {assignment: {field: entry.field, source_player: port, output_axis: relativeAxis.currentIndex}, device: device}
                const next = queuedAxes.filter(other => !switchEditor.sameField(other.assignment.field, entry.field))
                queuedAxes = next.concat([row])
            } catch (error) { report = "Cannot add preview axis: " + error }
        }
        function queueMouseButton() {
            try {
                if (stale || nativeOnly.checked) throw new Error("Use a current saved-source preview for mouse buttons.")
                const field = buttonFields[newMouseButtonField.currentIndex]
                const port = ports[relativePort.currentIndex]
                const device = devices[relativeDevice.currentIndex]
                const output = mouseButtonOutput.currentIndex + 1
                if (!field || port === undefined || !device || output < 1 || output > 5)
                    throw new Error("Choose a native switch, source player, device and mouse output button.")
                const axes = queuedAxes.filter(row => row.assignment.source_player === port)
                if (axes.some(row => JSON.stringify(row.device) !== JSON.stringify(device)))
                    throw new Error("This port's axes use a different source. Use the source-wide replacement action first.")
                const next = retainedButtons.filter(entry => !switchEditor.sameField(entry.field, field))
                const overrides = Object.assign({}, buttonSourceOverrides)
                overrides[String(port)] = {source_player: port, device: JSON.parse(JSON.stringify(device))}
                buttonSourceOverrides = overrides
                retainedButtons = next.concat([{field: field, source_player: port, output_button: output}])
                report = "Added/replaced the queued mouse-button mapping. The selected device is shared by this port's buttons. Preview the full batch to check capabilities and field conflicts. Staging and click launch remain unavailable."
            } catch (error) { report = "Cannot queue mouse button: " + error }
        }
        function deviceLabel(device) {
            const motion = device.motion || {x_percent: 100, y_percent: 100}
            return device.event_path + " · " + device.input_identity
                + " · X " + motion.x_percent + "%" + (motion.invert_x ? " inverted" : "")
                + " / Y " + motion.y_percent + "%" + (motion.invert_y ? " inverted" : "")
                + (motion.swap_xy ? " · X/Y swapped" : "")
                + (device.exclusive_source ? " · EXCLUSIVE CAPTURE" : " · shared capture")
        }
        function replacePlayerSource() {
            try {
                if (stale || nativeOnly.checked) throw new Error("Use a current saved-device review to replace a source.")
                const port = ports[relativePort.currentIndex]
                const device = devices[relativeDevice.currentIndex]
                if (port === undefined || !device) throw new Error("Choose a source player and replacement device.")
                const count = queuedAxes.filter(row => row.assignment.source_player === port).length
                if (!count) throw new Error("The selected player has no queued relative axes.")
                queuedAxes = queuedAxes.map(row => row.assignment.source_player === port
                    ? {assignment: row.assignment, device: JSON.parse(JSON.stringify(device))} : row)
                report = "Replaced the source for " + count + " queued axes on player " + port
                    + ". Game fields and output axes were preserved; the selected device's tuning and capture settings were adopted."
                    + "\n" + deviceLabel(device)
                    + "\nPreview the complete replacement set to validate capabilities and device uniqueness before applying to the draft. No device was opened."
            } catch (error) { report = "Cannot replace source: " + error }
        }
        function generate() {
            report = ""
            draftConfiguration = ""
            try {
                if (stale) throw new Error("Review the current draft again before previewing.")
                if (nativeOnly.checked && JSON.stringify(retainedButtons)
                        !== JSON.stringify(JSON.parse(baseText).relative_button_assignments || []))
                    throw new Error("Button edits require a saved-source preview. Turn off Native-only preview.")
                const assignments = queuedAxes.map(row => row.assignment)
                let request = assignments
                if (!nativeOnly.checked) {
                    const sources = new Map()
                    for (const row of queuedAxes) {
                        if (!row.device) throw new Error("An axis was added without a device. Replace it with a device selection or choose native-only preview.")
                        const port = row.assignment.source_player
                        if (sources.has(port) && JSON.stringify(sources.get(port).device) !== JSON.stringify(row.device))
                            throw new Error("Player " + port + " has different device selections across axes. Replace individual axes or use Replace source for all queued player axes.")
                        sources.set(port, {source_player: port, device: row.device})
                    }
                    const originalSources = JSON.parse(baseText).relative_sources || []
                    for (const button of retainedButtons) {
                        const port = button.source_player
                        // A shared port has one source: queued axis settings
                        // also govern its retained buttons. Button-only ports
                        // keep their original exact source unchanged.
                        if (!sources.has(port)) {
                            const source = buttonSourceOverrides[String(port)]
                                || originalSources.find(source => source.source_player === port)
                            if (!source) throw new Error("Retained mouse button has no source for player " + port + ".")
                            sources.set(port, source)
                        }
                    }
                    request = {assignments: assignments, buttons: retainedButtons, sources: Array.from(sources.values())}
                }
                const result = JSON.parse(dialog.settingsModel.mame_relative_plan_preview_json(baseText, JSON.stringify(request)))
                if (result.error) throw new Error(result.error)
                report = JSON.stringify(result, null, 2)
                draftConfiguration = result.draft_configuration || ""
            } catch (error) { report = "Cannot preview relative assignment: " + error }
        }
        function applyDraft() {
            if (stale || nativeOnly.checked || !draftConfiguration) return
            editor.text = draftConfiguration
            dialog.reviewedText = ""
            dialog.reviewHasUnhandled = true
            dialog.statusText = "Applied the previewed relative axis/button batch and sources to the draft. Other mappings were preserved. Review again; button-bearing drafts still cannot be staged or launched. No device was opened."
            close()
        }
        Dialog {
            id: relativeTuning
            property var sourceRow: null
            property string batchText: ""
            property string errorText: ""
            title: "Per-game relative source tuning"
            modal: true
            width: Math.min(500, relativePreview.width - 20)
            standardButtons: Dialog.Close
            onClosed: { sourceRow = null; batchText = ""; errorText = "" }
            function loadAndOpen() {
                try {
                    if (relativePreview.stale) throw new Error("Review the current draft again.")
                    const row = relativePreview.queuedAxes[queuedRelativeAxis.currentIndex]
                    if (!row || !row.device) throw new Error("Select a queued axis with a saved device.")
                    sourceRow = row
                    batchText = JSON.stringify(relativePreview.queuedAxes)
                    const motion = row.device.motion || {x_percent: 100, y_percent: 100, invert_x: false, invert_y: false, swap_xy: false}
                    relativeGainX.value = motion.x_percent
                    relativeGainY.value = motion.y_percent
                    relativeInvertX.checked = motion.invert_x
                    relativeInvertY.checked = motion.invert_y
                    relativeSwap.checked = motion.swap_xy === true
                    errorText = ""
                    open()
                } catch (error) { relativePreview.report = "Cannot tune source: " + error }
            }
            contentItem: ColumnLayout {
                Label {
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: relativeTuning.sourceRow ? "Player " + relativeTuning.sourceRow.assignment.source_player
                        + ": " + relativeTuning.sourceRow.device.event_path
                        + "\nApplies to all queued axes for this source player. X/Y swap occurs before output sensitivity and inversion. The global saved device is unchanged." : ""
                    textFormat: Text.PlainText
                }
                RowLayout {
                    Label { text: "Output X %" }
                    SpinBox { id: relativeGainX; from: 1; to: 1000; value: 100; editable: true; Accessible.name: "Per-game output X sensitivity percent" }
                    CheckBox { id: relativeInvertX; text: "Invert X" }
                }
                RowLayout {
                    Label { text: "Output Y %" }
                    SpinBox { id: relativeGainY; from: 1; to: 1000; value: 100; editable: true; Accessible.name: "Per-game output Y sensitivity percent" }
                    CheckBox { id: relativeInvertY; text: "Invert Y" }
                }
                CheckBox { id: relativeSwap; text: "Swap physical X/Y before scaling" }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: relativeTuning.errorText }
                Button {
                    text: "Apply tuning to queued source"
                    onClicked: {
                        try {
                            if (relativePreview.stale || relativeTuning.batchText !== JSON.stringify(relativePreview.queuedAxes))
                                throw new Error("The draft or batch changed. Close and reopen tuning.")
                            const row = relativeTuning.sourceRow
                            if (!row || !row.device) throw new Error("No source selected.")
                            const port = row.assignment.source_player
                            const siblings = relativePreview.queuedAxes.filter(entry => entry.assignment.source_player === port)
                            if (siblings.some(entry => JSON.stringify(entry.device) !== JSON.stringify(row.device)))
                                throw new Error("This player's axes have conflicting devices or settings. Resolve the source selections first.")
                            const device = JSON.parse(JSON.stringify(row.device))
                            device.motion = {x_percent: relativeGainX.value, y_percent: relativeGainY.value,
                                invert_x: relativeInvertX.checked, invert_y: relativeInvertY.checked, swap_xy: relativeSwap.checked}
                            relativePreview.queuedAxes = relativePreview.queuedAxes.map(entry => entry.assignment.source_player === port
                                ? {assignment: entry.assignment, device: device} : entry)
                            if (!relativePreview.devices.some(entry => JSON.stringify(entry) === JSON.stringify(device)))
                                relativePreview.devices = relativePreview.devices.concat([device])
                            if (relativePreview.ports[relativePort.currentIndex] === port)
                                relativeDevice.currentIndex = relativePreview.devices.findIndex(entry => JSON.stringify(entry) === JSON.stringify(device))
                            relativePreview.report = "Updated tuning for " + siblings.length + " queued axes on player " + port
                                + ". Preview the complete replacement set before applying to the draft. Nothing was saved or captured."
                            relativeTuning.close()
                        } catch (error) { relativeTuning.errorText = String(error) }
                    }
                }
            }
        }
        title: "Relative axes — draft assignments"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ScrollView {
            id: relativeEditorScroll
            clip: true
            ColumnLayout {
                width: relativeEditorScroll.availableWidth
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "This batch starts with every relative axis in the draft. Add, replace or remove axes, then preview the complete replacement set. Existing mouse-button assignments are preserved, including sources on button-only ports. Replacing a shared port's source also changes the source used by its retained buttons. Emptying this axis batch removes axes, not buttons. Closing without applying discards edits. Nothing is saved or captured here." }
            Label {
                Layout.fillWidth: true
                visible: relativePreview.retainedButtons.length > 0
                wrapMode: Text.WordWrap
                text: relativePreview.retainedButtons.length + " mouse-button assignments are in this batch. Use the controls below to change an existing output or explicitly remove a mapping. Staging requires 64-bit Linux and the opt-in core mode; launch also checks private UI bindings and live routing. Runtime behavior is unverified."
            }
            ComboBox {
                id: existingMouseButton
                Layout.fillWidth: true
                visible: relativePreview.retainedButtons.length > 0
                model: relativePreview.retainedButtons.map(assignment => assignment.field.input_type
                    + " / " + assignment.field.tag + " / mask " + assignment.field.mask
                    + " / default " + assignment.field.defvalue + " ← player " + assignment.source_player
                    + " / mouse button " + assignment.output_button)
                Accessible.name: "Existing mouse-button mapping in the temporary batch"
                onCurrentIndexChanged: mouseButtonOutput.currentIndex = -1
            }
            ComboBox {
                id: mouseButtonOutput
                Layout.fillWidth: true
                model: ["Button 1 · left", "Button 2 · right", "Button 3 · middle", "Button 4 · side", "Button 5 · extra"]
                currentIndex: -1
                displayText: currentIndex < 0 ? "Choose a replacement output button" : currentText
                Accessible.name: "Replacement native mouse-button output"
            }
            RowLayout {
                visible: relativePreview.retainedButtons.length > 0
                Button {
                    text: "Change queued button output"
                    enabled: !relativePreview.stale && !nativeOnly.checked
                        && existingMouseButton.currentIndex >= 0 && mouseButtonOutput.currentIndex >= 0
                    onClicked: {
                        const index = existingMouseButton.currentIndex
                        const assignment = relativePreview.retainedButtons[index]
                        if (!assignment || relativePreview.stale) return
                        const output = mouseButtonOutput.currentIndex + 1
                        relativePreview.retainedButtons = relativePreview.retainedButtons.map((entry, row) => row === index
                            ? {field: entry.field, source_player: entry.source_player, output_button: output} : entry)
                        relativePreview.report = "Changed the queued output. Preview the complete batch to validate the saved device's button remap before applying."
                    }
                }
                Button {
                    text: "Remove queued button mapping"
                    enabled: !relativePreview.stale && !nativeOnly.checked && existingMouseButton.currentIndex >= 0
                    onClicked: {
                        const index = existingMouseButton.currentIndex
                        if (relativePreview.stale || !relativePreview.retainedButtons[index]) return
                        relativePreview.retainedButtons = relativePreview.retainedButtons.filter((_, row) => row !== index)
                        relativePreview.report = "Removed the mapping from this batch only. Preview again: the game field may revert to a gamepad default or become unresolved."
                    }
                }
            }
            ComboBox { id: relativeField; Layout.fillWidth: true; model: relativePreview.fields.map(entry => dialog.fieldLabel(entry.field, [])); displayText: currentIndex < 0 ? "Choose a relative field" : currentText; Accessible.name: "Native relative field" }
            ComboBox { id: relativePort; Layout.fillWidth: true; model: relativePreview.ports.map(port => "Source player " + port); displayText: currentIndex < 0 ? "Choose a source player" : currentText; Accessible.name: "Relative source player" }
            ComboBox { id: relativeAxis; Layout.fillWidth: true; model: ["Output X", "Output Y"]; displayText: currentIndex < 0 ? "Choose an output axis" : currentText; Accessible.name: "Post-transform relative output axis" }
            CheckBox { id: nativeOnly; text: "Native-only preview (skip saved-device validation)" }
            ComboBox { id: relativeDevice; Layout.fillWidth: true; enabled: !nativeOnly.checked; model: relativePreview.devices.map(device => relativePreview.deviceLabel(device)); displayText: currentIndex < 0 ? "Choose a saved relative device" : currentText; Accessible.name: "Exact saved relative device and tuning" }
            ComboBox {
                id: newMouseButtonField
                Layout.fillWidth: true
                model: relativePreview.buttonFields.map(field => field.input_type + " / " + field.tag
                    + " / mask " + field.mask + " / default " + field.defvalue)
                displayText: currentIndex < 0 ? "Choose a native switch for a mouse button" : currentText
                Accessible.name: "Exact inspected field for a new mouse-button assignment"
            }
            Button {
                text: "Add / replace mouse-button mapping in batch"
                enabled: !relativePreview.stale && !nativeOnly.checked && newMouseButtonField.currentIndex >= 0
                    && relativePort.currentIndex >= 0 && relativeDevice.currentIndex >= 0 && mouseButtonOutput.currentIndex >= 0
                onClicked: relativePreview.queueMouseButton()
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                text: relativePreview.devices[relativeDevice.currentIndex]
                    ? relativePreview.deviceLabel(relativePreview.devices[relativeDevice.currentIndex]) : ""
            }
            Button {
                text: "Replace source for all queued player axes"
                enabled: !relativePreview.stale && !nativeOnly.checked && relativeDevice.currentIndex >= 0
                    && relativePreview.queuedAxes.some(row => row.assignment.source_player === relativePreview.ports[relativePort.currentIndex])
                Accessible.description: "Adopt the selected device, including its tuning and exclusive-capture setting, for all queued axes of the selected source player. Does not apply or save the draft."
                onClicked: relativePreview.replacePlayerSource()
            }
            Button { text: "Add / replace axis in preview batch"; enabled: !relativePreview.stale; onClicked: relativePreview.queueSelection() }
            ComboBox {
                id: queuedRelativeAxis
                Layout.fillWidth: true
                model: relativePreview.queuedAxes.map(row => dialog.fieldLabel(row.assignment.field, [])
                    + " → player " + row.assignment.source_player + " / " + (row.assignment.output_axis === 0 ? "X" : "Y")
                    + " · " + (row.device ? row.device.event_path : "native only"))
                Accessible.name: "Temporary relative-axis preview batch"
            }
            Button {
                text: "Load selected axis for editing"
                enabled: !relativePreview.stale && queuedRelativeAxis.currentIndex >= 0
                onClicked: {
                    try { relativePreview.loadQueuedAxis(queuedRelativeAxis.currentIndex) }
                    catch (error) { relativePreview.report = "Cannot load axis: " + error }
                }
            }
            Button {
                text: "Tune selected axis's source…"
                enabled: !relativePreview.stale && !nativeOnly.checked
                    && !!(relativePreview.queuedAxes[queuedRelativeAxis.currentIndex] || {}).device
                onClicked: relativeTuning.loadAndOpen()
            }
            RowLayout {
                Button { text: "Remove selected preview axis"; enabled: !relativePreview.stale && queuedRelativeAxis.currentIndex >= 0; onClicked: relativePreview.queuedAxes = relativePreview.queuedAxes.filter((_, index) => index !== queuedRelativeAxis.currentIndex) }
                Button { text: "Preview replacement set"; enabled: !relativePreview.stale; onClicked: relativePreview.generate() }
                Button { text: "Apply replacement to draft"; enabled: !relativePreview.stale && !nativeOnly.checked && relativePreview.draftConfiguration.length > 0; onClicked: relativePreview.applyDraft() }
            }
            Label { visible: relativePreview.stale; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Draft or controller settings changed. Close and review again; previous preview data was cleared." }
            TextArea { Layout.fillWidth: true; Layout.minimumHeight: 160; text: relativePreview.report; readOnly: true; selectByMouse: true; wrapMode: TextEdit.Wrap; textFormat: TextEdit.PlainText }
            }
        }
    }
    Dialog {
        id: analogEditor
        property var labels: []
        property var fields: []
        property var ports: []
        property string baseText: ""
        property string status: ""
        readonly property var channels: ["left_x", "left_y", "right_x", "right_y", "left_pressure", "right_pressure"]
        readonly property var ranges: ["full", "reversed", "positive", "negative"]
        function sameField(a, b) {
            return a && b && a.tag === b.tag && a.input_type === b.input_type && a.mask === b.mask && a.defvalue === b.defvalue
        }
        function loadSelection() {
            try {
                const draft = JSON.parse(baseText)
                const field = fields[analogField.currentIndex]
                const saved = (draft.analog_assignments || []).find(assignment => sameField(assignment.field, field))
                analogPort.currentIndex = saved ? ports.indexOf(saved.source_player) : -1
                analogChannel.currentIndex = saved ? channels.indexOf(saved.channel) : -1
                analogRange.currentIndex = saved ? ranges.indexOf(saved.stick_range || "full") : 0
                velocityOptIn.checked = saved ? saved.relative_velocity === true : false
                aimOptIn.checked = saved ? saved.controller_aim === true : false
            } catch (error) { status = String(error) }
        }
        function updateAssignment(remove) {
            try {
                if (editor.text !== baseText) throw new Error("The setup draft changed. Close and reopen this form.")
                const field = fields[analogField.currentIndex]
                if (!field) throw new Error("Choose an inspected field.")
                if (!remove && (analogPort.currentIndex < 0 || analogChannel.currentIndex < 0)) throw new Error("Choose both a source player and a channel.")
                const draft = JSON.parse(baseText)
                const assignments = draft.analog_assignments || []
                if (!Array.isArray(assignments)) throw new Error("analog_assignments must be a list.")
                draft.analog_assignments = assignments.filter(assignment => !sameField(assignment.field, field))
                if (!remove) {
                    if ((draft.digital_assignments || []).some(assignment => sameField(assignment.field, field))) throw new Error("This field has button-driven assignments. Remove both directions in the button editor before choosing an analog channel.")
                    if (analogRange.currentIndex < 0) throw new Error("Choose a supported stick range.")
                    const relative = /^P[1-8]_(DIAL|DIAL_V|TRACKBALL_[XY]|MOUSE_[XY])$/.test(field.input_type)
                    const lightgun = /^P[1-8]_LIGHTGUN_[XY]$/.test(field.input_type)
                    if (lightgun && !aimOptIn.checked) throw new Error("Explicitly select controller aim for this field, or use a separate physical-gun adapter.")
                    if (lightgun && (analogChannel.currentIndex >= 4 || analogRange.currentIndex >= 2)) throw new Error("Controller aim requires a full/reversed stick axis.")
                    if (relative && !velocityOptIn.checked) throw new Error("Explicitly select stick velocity for this relative field, or use a separate physical-relative adapter.")
                    if (relative && (analogChannel.currentIndex >= 4 || analogRange.currentIndex >= 2)) throw new Error("Stick velocity requires a full/reversed stick axis, not pressure or a half-axis range.")
                    draft.analog_assignments.push({field: field, source_player: ports[analogPort.currentIndex], channel: channels[analogChannel.currentIndex], stick_range: analogChannel.currentIndex >= 4 ? "full" : ranges[analogRange.currentIndex], relative_velocity: relative && velocityOptIn.checked, controller_aim: lightgun && aimOptIn.checked})
                }
                editor.text = JSON.stringify(draft, null, 2)
                baseText = editor.text
                dialog.reviewedText = ""
                dialog.reviewHasUnhandled = true
                status = remove ? "Assignment removed from draft." : "Assignment added/replaced in draft. Review physical calibration next."
                loadSelection()
            } catch (error) { status = String(error) }
        }
        title: "MAME analog channel choices"
        width: dialog.width - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ColumnLayout {
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Choose a runtime-inspected analog field and its source channel. This edits the draft only; it does not calibrate or start a device. Physical relative devices and lightguns still need separate contracts." }
            ComboBox { id: analogField; Layout.fillWidth: true; model: analogEditor.fields.map(field => dialog.fieldLabel(field, analogEditor.labels)); onActivated: analogEditor.loadSelection(); Accessible.name: "Inspected analog field" }
            ComboBox { id: analogPort; Layout.fillWidth: true; model: analogEditor.ports.map(port => "Source player " + port); Accessible.name: "Analog source player" }
            ComboBox { id: analogChannel; Layout.fillWidth: true; model: ["Left stick X", "Left stick Y", "Right stick X", "Right stick Y", "Left pressure (L2)", "Right pressure (R2)"]; Accessible.name: "Analog source channel" }
            ComboBox { id: analogRange; Layout.fillWidth: true; enabled: analogChannel.currentIndex >= 0 && analogChannel.currentIndex < 4; model: ["Full stick axis", "Reversed full axis", "Positive half → full native range", "Negative half → full native range"]; Accessible.name: "Native stick range" }
            CheckBox { id: velocityOptIn; text: "Use centered stick displacement as native relative velocity"; visible: analogField.currentIndex >= 0 && /^P[1-8]_(DIAL|DIAL_V|TRACKBALL_[XY]|MOUSE_[XY])$/.test(analogEditor.fields[analogField.currentIndex].input_type) }
            CheckBox { id: aimOptIn; text: "Use calibrated stick position for native absolute aim"; visible: analogField.currentIndex >= 0 && /^P[1-8]_LIGHTGUN_[XY]$/.test(analogEditor.fields[analogField.currentIndex].input_type) }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: aimOptIn.visible; text: "This uses controller axes, not a physical gun or screen calibration. Trigger and auxiliary inputs still need mappings. Off-screen/reload behavior must be established for the specific game; this does not add it automatically." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: velocityOptIn.visible; text: "This is MAME's controller-driven velocity mode: holding the stick produces continuous movement with native sensitivity/reset behavior. It is not trackball, spinner or mouse delta capture." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Half-axis choices map stick center to the native minimum and one direction to the maximum, useful for pedals. Full axes keep center at midrange. Pressure channels always use their source-defined released-to-pressed range." }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; visible: analogField.currentIndex >= 0 && /^P[1-8]_POSITIONAL/.test(analogEditor.fields[analogField.currentIndex].input_type); text: "MAME converts absolute travel into this field's native positions. This is absolute position selection, not relative encoder motion; the driver's position count and wrapping behavior remain native." }
            RowLayout {
                Button { text: "Add / replace assignment"; onClicked: analogEditor.updateAssignment(false) }
                Button { text: "Remove assignment"; onClicked: analogEditor.updateAssignment(true) }
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: analogEditor.status }
        }
    }
    Dialog {
        id: review
        readonly property var catalog: JSON.parse(dialog.settingsModel.controller_catalog_json())
        readonly property var player: dialog.reviewPlayers[reviewPlayer.currentIndex] || null
        function playerGapLabel(entry) {
            if (entry.layout_error) return "layout error"
            if (entry.calibration_error) return "calibration unavailable"
            if (!entry.target_layout) return "no active destination profile"
            if ((entry.physical_gaps || []).length || (entry.rows || []).some(row => !row.physical_id))
                return "physical mapping needs setup"
            return ""
        }
        readonly property var playerGapIndices: {
            const indices = []
            dialog.reviewPlayers.forEach((entry, index) => {
                if (playerGapLabel(entry)) indices.push(index)
            })
            return indices
        }
        function nextPlayerGap() {
            if (dialog.reviewedText !== editor.text || !playerGapIndices.length) return
            const next = playerGapIndices.find(index => index > reviewPlayer.currentIndex)
            reviewPlayer.currentIndex = next === undefined ? playerGapIndices[0] : next
            dialog.preferredReviewPort = dialog.reviewPlayers[reviewPlayer.currentIndex].port
        }
        readonly property var ordinaryActions: player ? (player.switch_actions || []).filter(route =>
            !route.assignment.field.analog && route.assignment.field.class === "controller"
            && (route.assignment.sequence || "standard") === "standard") : []
        // Retain every switch owner for shared-channel rejection, including
        // auxiliary actions. Drawable routes only enrich the physical preview.
        readonly property var swapRoutes: player ? (player.switch_actions || []).map(action =>
            diagramRoute(action) || action) : []
        onOrdinaryActionsChanged: ordinaryAction.currentIndex = -1
        function diagramRoute(action) {
            if (!player || !action) return null
            const assignment = action.assignment
            const matches = (player.native_routes || []).filter(route =>
                route.assignment.output === assignment.output
                && route.assignment.source_player === assignment.source_player
                && (route.assignment.sequence || "standard") === (assignment.sequence || "standard")
                && switchEditor.sameField(route.assignment.field, assignment.field))
            if (matches.length !== 1 || !matches[0].target_id) return null
            return mappingView.rows.filter(row => row.target_id === matches[0].target_id).length === 1
                ? matches[0] : null
        }
        function showActionConnection(action) {
            if (dialog.reviewedText !== editor.text) return
            const route = diagramRoute(action)
            if (!route) return
            mappingView.selectedIndex = mappingView.rows.findIndex(row => row.target_id === route.target_id)
            // Shared channels can represent several native actions. Keep the
            // precise action selected rather than choosing the first label.
            reviewedAction.currentIndex = selectedSwitches.indexOf(route)
        }
        readonly property var selectedSwitches: player && mappingView.selected
            ? (player.native_routes || []).filter(route => route.target_id === mappingView.selected.target_id && route.assignment.output)
            : []
        onSelectedSwitchesChanged: reviewedAction.currentIndex = selectedSwitches.length === 1 ? 0 : -1
        readonly property var selectedAction: selectedSwitches[reviewedAction.currentIndex] || null
        property var diagramSwapFirst: null
        property string diagramSwapStatus: ""
        readonly property var swapFirst: diagramSwapFirst || selectedAction
        onPlayerChanged: cancelDiagramSwap()
        onClosed: cancelDiagramSwap()
        function cancelDiagramSwap() {
            diagramSwapFirst = null
            diagramSwapStatus = ""
            swapAction.currentIndex = -1
        }
        function beginDiagramSwap() {
            beginActionSwap(selectedAction)
        }
        function beginActionSwap(action) {
            if (dialog.reviewedText !== editor.text || !player
                    || !dialog.swappableAction(action, swapRoutes)) return
            diagramSwapFirst = action
            swapAction.currentIndex = -1
            diagramSwapStatus = "First action: " + (diagramSwapFirst.label || diagramSwapFirst.assignment.field.input_type)
                + ". Click another independent button in either diagram, or choose it below. Nothing is changed yet."
        }
        function chooseDiagramSwap(side, controlId) {
            if (!diagramSwapFirst || dialog.reviewedText !== editor.text) return
            // Resolve the whole hotspot, not whichever shared row the view
            // happened to cycle to. Ambiguity must remain an explicit choice.
            const rows = mappingView.rows.filter(row => mappingView.rowMatchesControl(row, side, controlId))
            const choices = swapChoices.filter(route => rows.some(row =>
                row.target_id === route.target_id && row.output === route.assignment.output))
            swapAction.currentIndex = rows.length === 1 && choices.length === 1
                ? swapChoices.indexOf(choices[0]) : -1
            diagramSwapStatus = swapAction.currentIndex >= 0
                ? "Second action selected. Check the before/after summary, then press Swap buttons to change the draft."
                : "That control does not identify one eligible second action. Choose another independent button or use the action picker. No mapping changed."
        }
        readonly property var swapChoices: player && dialog.swappableAction(swapFirst, swapRoutes)
            ? swapRoutes.filter(route => dialog.swappableAction(route, swapRoutes)
                && route.assignment.source_player === swapFirst.assignment.source_player
                && route.assignment.output !== swapFirst.assignment.output) : []
        onSwapChoicesChanged: swapAction.currentIndex = -1
        function swapControlLabel(route) {
            if (!route) return "Physical connection unavailable"
            const channel = route.assignment.output
            const drawable = diagramRoute(route)
            if (!drawable) return channel + " — physical connection unavailable"
            const matches = mappingView.rows.filter(row => row.target_id === drawable.target_id
                && row.output === channel)
            if (matches.length !== 1) return channel + " — physical connection unavailable"
            const row = matches[0]
            if (!row.physical_id) return "UNMAPPED — " + route.assignment.output
            return channel + " · " + row.physical + " [" + row.physical_id + "]"
                + (mappingView.gapReason(row) ? " — needs calibration" : "")
        }
        readonly property string swapSummary: {
            const first = swapFirst
            const second = swapChoices[swapAction.currentIndex]
            if (dialog.reviewedText !== editor.text || !first || !second) return ""
            const firstName = (first.label || first.assignment.field.input_type) + " [" + first.assignment.field.input_type + "]"
            const secondName = (second.label || second.assignment.field.input_type) + " [" + second.assignment.field.input_type + "]"
            return "Proposed swap using the current reviewed connections:\n"
                + firstName + ": " + swapControlLabel(first) + " → " + swapControlLabel(second) + "\n"
                + secondName + ": " + swapControlLabel(second) + " → " + swapControlLabel(first)
                + "\nNothing changes until Swap buttons. Review the resulting draft again before staging; this is not live input verification."
        }
        title: "MAME physical assignment preview"
        readonly property var unresolvedSwitches: dialog.reviewExceptions.filter(entry =>
            dialog.editableUnresolvedSwitch(entry.field)
            && (entry.field.class === "controller" || includeAuxiliarySwitches.checked))
        onUnresolvedSwitchesChanged: unresolvedButton.currentIndex = -1
        onOpened: includeAuxiliarySwitches.checked = false
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Close
        contentItem: ScrollView {
            id: reviewScroll
            clip: true
            ColumnLayout {
                width: reviewScroll.availableWidth
                Label {
                    Layout.fillWidth: true
                    text: dialog.reviewSummary
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    text: dialog.reviewCoverage
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                ControllerRelativeMappingView {
                    Layout.fillWidth: true
                    visible: dialog.reviewRelativeRoutes.length > 0
                    routes: dialog.reviewRelativeRoutes
                    editingEnabled: dialog.reviewedText === editor.text
                    onEditRequested: assignment => relativePreview.loadAndOpen(assignment)
                }
                Repeater {
                    model: dialog.reviewExceptions.slice(0, 8)
                    delegate: Label {
                        required property var modelData
                        Layout.fillWidth: true
                        text: "Needs setup: " + (modelData.label || modelData.field.input_type)
                            + (modelData.field.analog ? " · analog/peripheral input" : " · " + modelData.field.class + " input")
                            + (modelData.guidance ? "\n" + modelData.guidance : "")
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                }
                Label {
                    Layout.fillWidth: true
                    visible: dialog.reviewExceptions.length > 8
                    text: (dialog.reviewExceptions.length - 8) + " more unresolved inputs are listed in the technical report."
                    wrapMode: Text.WordWrap
                }
                CheckBox {
                    id: includeAuxiliarySwitches
                    text: "Include unresolved keyboard and auxiliary switches"
                    visible: dialog.reviewExceptions.some(entry => dialog.editableUnresolvedSwitch(entry.field)
                        && entry.field.class !== "controller")
                    checked: false
                }
                Button {
                    text: "Edit relative axes / mouse-button drafts…"
                    visible: relativePreview.mouseInspected || relativePreview.savedCount > 0 || dialog.reviewExceptions.some(entry => !!entry.relative_input)
                    enabled: dialog.reviewedText === editor.text
                    onClicked: relativePreview.loadAndOpen()
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: review.unresolvedSwitches.length > 0
                    ComboBox {
                        id: unresolvedButton
                        Layout.fillWidth: true
                        model: review.unresolvedSwitches.map(entry => dialog.fieldLabel(entry.field,
                            [{field: entry.field, label: entry.label || ""}]) + " · " + entry.field.class)
                        displayText: currentIndex < 0 ? "Choose an unresolved switch input" : currentText
                        Accessible.name: "Unresolved MAME switch input to assign"
                    }
                    Button {
                        text: "Assign button…"
                        enabled: unresolvedButton.currentIndex >= 0 && dialog.reviewedText === editor.text
                        onClicked: dialog.editUnresolvedSwitch(review.unresolvedSwitches[unresolvedButton.currentIndex])
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: (review.unresolvedSwitches[unresolvedButton.currentIndex] || {}).guidance || ""
                    visible: text.length > 0
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                ComboBox {
                    id: reviewPlayer
                    Layout.fillWidth: true
                    model: dialog.reviewPlayers.map(player => "Player " + player.port + " · " + player.controller
                        + (review.playerGapLabel(player) ? " · " + review.playerGapLabel(player) : ""))
                    onActivated: dialog.preferredReviewPort = dialog.reviewPlayers[currentIndex]
                        ? dialog.reviewPlayers[currentIndex].port : 0
                    Accessible.name: "MAME player mapping to visualize"
                }
                Button {
                    text: "Next player with a layout / physical gap (" + review.playerGapIndices.length + ")"
                    visible: review.playerGapIndices.length > 0
                    enabled: dialog.reviewedText === editor.text
                    onClicked: review.nextPlayerGap()
                    Accessible.description: "Navigate through reported player layout and physical mapping gaps. Other unresolved native inputs still require separate review."
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.player !== null
                    text: review.player ? dialog.playerPresetDescription(review.player.port, dialog.reviewedText)
                        + ". The destination diagram shows the resolved requirements; preset selection alone is not coverage verification." : ""
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Button {
                    text: "Compare this player's presets…"
                    enabled: review.player !== null && dialog.reviewedText === editor.text
                    onClicked: {
                        const port = review.player.port
                        review.close()
                        presetComparison.loadAndOpen(port)
                    }
                }
                Button {
                    text: "Calibrate this player's controller…"
                    enabled: review.player !== null && dialog.reviewedText === editor.text
                    onClicked: dialog.calibrateReviewedPlayer(false)
                }
                Button {
                    text: "Calibrate highlighted source control…"
                    visible: review.player !== null && !!review.player.source_layout
                        && (!!mappingView.focusedSourceControl || (mappingView.selected !== null && !!mappingView.selected.physical_id))
                    enabled: dialog.reviewedText === editor.text
                    onClicked: dialog.calibrateReviewedPlayer(true)
                }
                Button {
                    text: "Next control needing setup (" + mappingView.setupGapIndices.length + ")"
                    visible: review.player !== null && mappingView.setupGapIndices.length > 0
                    enabled: dialog.reviewedText === editor.text
                    Accessible.description: "Cycle through displayed controls with no saved source assignment or a reported calibration gap. This does not change mappings."
                    onClicked: mappingView.chooseNextSetupGap()
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: review.ordinaryActions.length > 0
                    ComboBox {
                        id: ordinaryAction
                        Layout.fillWidth: true
                        model: review.ordinaryActions.map(route => (route.label || route.assignment.field.input_type)
                            + " · " + route.assignment.output + " · " + route.assignment.field.input_type
                            + " / " + route.assignment.field.tag + " / mask " + route.assignment.field.mask
                            + " / default " + route.assignment.field.defvalue)
                        displayText: currentIndex < 0 ? "Choose any mapped controller action" : currentText
                        Accessible.name: "MAME controller action to edit without selecting a diagram connection"
                    }
                    Button {
                        text: "Show connection"
                        enabled: dialog.reviewedText === editor.text
                            && review.diagramRoute(review.ordinaryActions[ordinaryAction.currentIndex]) !== null
                        Accessible.description: "Highlight this exact game action's source and destination connection when a unique drawable route is available."
                        onClicked: review.showActionConnection(review.ordinaryActions[ordinaryAction.currentIndex])
                    }
                    Button {
                        text: "Edit action…"
                        enabled: ordinaryAction.currentIndex >= 0 && dialog.reviewedText === editor.text
                        onClicked: dialog.editReviewedSwitch(review.ordinaryActions[ordinaryAction.currentIndex])
                    }
                    Button {
                        text: "Swap this action…"
                        enabled: dialog.reviewedText === editor.text
                            && dialog.swappableAction(review.ordinaryActions[ordinaryAction.currentIndex], review.swapRoutes)
                        Accessible.description: "Choose this game's numbered button as the first swap action. Physical calibration is not required to edit channels; shared channels cannot be swapped."
                        onClicked: review.beginActionSwap(review.ordinaryActions[ordinaryAction.currentIndex])
                    }
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.ordinaryActions.length > 0
                    text: "Game actions can be edited even when physical calibration is unavailable. This list shows channel assignments only; missing calibration must still be repaired before staging."
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.player !== null
                    text: {
                        const rows = review.player ? review.player.rows || [] : []
                        const assigned = rows.filter(row => !!row.physical_id).length
                        return "Selected player's displayed controls: " + assigned + " / " + rows.length
                            + " have saved source assignments"
                            + (rows.length ? " (" + (100 * assigned / rows.length).toFixed(1) + "%)." : " (percentage unavailable).")
                            + " This does not establish native measurements or live input behavior."
                    }
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.player !== null
                    text: {
                        const coverage = review.player ? review.player.measurement_coverage : null
                        if (!coverage) return "Saved native measurement coverage: unavailable for this player. No completeness percentage is established."
                        return "Saved native measurements: " + coverage.measured + " / " + coverage.required
                            + (coverage.percent === null ? " (percentage unavailable)" : " (" + coverage.percent.toFixed(1) + "%)")
                            + "; " + coverage.missing + " required controls need measurement or assignment."
                            + " This is saved-data completeness, not live device verification or game playability."
                    }
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Repeater {
                    model: review.player ? review.player.warnings || [] : []
                    delegate: Label {
                        required property string modelData
                        Layout.fillWidth: true
                        text: "Note: " + modelData
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                }
                ControllerMappingView {
                    id: mappingView
                    Layout.fillWidth: true
                    visible: review.player !== null && !!review.player.target_layout
                    settingsModel: dialog.settingsModel
                    sourceLayout: review.player ? review.catalog.layouts.find(layout => layout.id === review.player.source_layout) || null : null
                    destinationLayout: review.player ? review.catalog.layouts.find(layout => layout.id === review.player.target_layout) || null : null
                    rows: review.player ? review.player.rows : []
                    nativeRoutes: review.player ? review.player.native_routes || [] : []
                    physicalGaps: review.player ? review.player.physical_gaps || [] : []
                    onControlActivated: (side, controlId) => review.chooseDiagramSwap(side, controlId)
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: review.selectedSwitches.length > 0
                    ComboBox {
                        id: reviewedAction
                        Layout.fillWidth: true
                        model: review.selectedSwitches.map(route => (route.label || route.assignment.field.input_type)
                            + " · " + (route.assignment.sequence || "standard")
                            + (review.selectedSwitches.length > 1 ? " · " + route.assignment.field.tag
                                + " / " + route.assignment.field.input_type + " / mask " + route.assignment.field.mask
                                + " / default " + route.assignment.field.defvalue : ""))
                        displayText: currentIndex < 0 ? "Choose the shared game action to edit" : currentText
                        Accessible.name: "Game action to change on the highlighted connection"
                    }
                    Button {
                        text: "Change button channel…"
                        enabled: reviewedAction.currentIndex >= 0 && dialog.reviewedText === editor.text
                        onClicked: dialog.editReviewedSwitch(review.selectedSwitches[reviewedAction.currentIndex])
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: review.swapChoices.length > 0
                    Button {
                        text: review.diagramSwapFirst ? "Cancel diagram selection" : "Choose swap in diagram"
                        enabled: dialog.reviewedText === editor.text
                        onClicked: {
                            if (review.diagramSwapFirst) review.cancelDiagramSwap()
                            else review.beginDiagramSwap()
                        }
                    }
                    ComboBox {
                        id: swapAction
                        Layout.fillWidth: true
                        model: review.swapChoices.map(route => (route.label || route.assignment.field.input_type)
                            + " · " + route.assignment.field.input_type + " · " + review.swapControlLabel(route))
                        displayText: currentIndex < 0 ? "Choose another button action to swap with" : currentText
                        Accessible.name: "Other arcade action for a channel swap"
                    }
                    Button {
                        text: "Swap buttons"
                        enabled: swapAction.currentIndex >= 0 && dialog.reviewedText === editor.text
                        onClicked: dialog.swapReviewedActions(review.swapFirst, review.swapChoices[swapAction.currentIndex])
                    }
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.diagramSwapStatus.length > 0
                    text: review.diagramSwapStatus
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.swapSummary.length > 0
                    text: review.swapSummary
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Button {
                    text: "Restore selected button to preset"
                    visible: dialog.hasButtonOverride(review.selectedAction)
                    enabled: dialog.reviewedText === editor.text
                    onClicked: dialog.restoreReviewedButton(review.selectedAction)
                }
                Button {
                    readonly property int overrideCount: review.player ? dialog.numberedOverrideCount(review.player.port) : 0
                    text: "Restore all numbered buttons to preset (" + overrideCount + " overrides)"
                    visible: overrideCount > 0
                    enabled: review.player !== null && dialog.reviewedText === editor.text
                    onClicked: dialog.restorePlayerNumberedButtons(review.player.port)
                }
                Label {
                    Layout.fillWidth: true
                    visible: review.player !== null && dialog.numberedOverrideCount(review.player.port) > 0
                    text: "Restore all removes this player's own numbered-button overrides from the draft, including those preserved by swaps. Cross-player routes and all non-numbered controls are kept. Review the preset's resulting assignments again before staging."
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "The diagram shows saved physical-to-frontend assignments. Native field modifiers and detailed calibration errors are available in the technical report. This is not live input verification." }
                CheckBox { id: technicalReport; text: "Show complete technical report" }
                TextArea { id: reviewText; visible: technicalReport.checked; Layout.fillWidth: true; readOnly: true; selectByMouse: true; wrapMode: TextEdit.Wrap }
            }
        }
    }
    Dialog {
        id: playerControllers
        property string baseText: ""
        property string errorText: ""
        property var players: []
        property var relativePorts: []
        readonly property var unusedPorts: [1, 2, 3, 4, 5, 6, 7, 8].filter(port => !players.some(player => Number(player.port) === port))
        title: "MAME player controllers"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Cancel
        function addPlayer(port) {
            try {
                if (editor.text !== baseText) throw new Error("The draft changed. Close and reopen this editor.")
                if (unusedPorts.indexOf(port) < 0) throw new Error("Choose an unused player port.")
                players = players.concat([{port: String(port), controller: "", layout: ""}]).sort((a, b) => Number(a.port) - Number(b.port))
                errorText = "Choose a saved controller for player " + port + ". Apply changes the draft only; review must confirm this game's controls before staging."
            } catch (error) { errorText = String(error) }
        }
        function removePlayer(port) {
            try {
                if (editor.text !== baseText) throw new Error("The draft changed. Close and reopen this editor.")
                if (players.length <= 1 && !relativePorts.length)
                    throw new Error("Keep at least one gamepad or configure a relative source before removing the last gamepad.")
                const draft = JSON.parse(baseText)
                if ((draft.digital_assignments || []).concat(draft.analog_assignments || [])
                        .some(assignment => assignment.source_player === Number(port)))
                    throw new Error("Player " + port + " owns explicit button/axis assignments. Reassign or remove those in the assignment editors before removing this player.")
                players = players.filter(player => player.port !== port)
                errorText = "Gamepad for player " + port + " removed from this editor. Apply to draft to keep it; Cancel leaves the draft unchanged. Relative assignments and sources are preserved. Review must confirm that remaining controls have a source."
            } catch (error) { errorText = String(error) }
        }
        function loadAndOpen() {
            try {
                const draft = JSON.parse(editor.text)
                if (!draft.players || typeof draft.players !== "object" || Array.isArray(draft.players)) throw new Error("The draft needs a player map.")
                const ports = Object.keys(draft.players)
                const relative = (draft.relative_sources || []).map(source => source.source_player)
                    .filter((port, index, all) => Number.isInteger(port) && port >= 1 && port <= 8
                        && all.indexOf(port) === index
                        && (draft.relative_assignments || []).concat(draft.relative_button_assignments || [])
                            .some(assignment => assignment.source_player === port))
                if ((!ports.length && !relative.length) || ports.length > 8 || ports.some(port => !/^[1-8]$/.test(port)))
                    throw new Error("Select a gamepad or configured relative source; player ports must be 1–8.")
                const layouts = draft.player_digital_layouts || {}
                if (typeof layouts !== "object" || Array.isArray(layouts)
                        || Object.keys(layouts).some(port => ports.indexOf(port) < 0 || dialog.presetIds.indexOf(layouts[port]) < 0))
                    throw new Error("Player layout overrides must name selected ports and supported presets.")
                players = ports.sort((a, b) => Number(a) - Number(b)).map(port => ({port: port, controller: draft.players[port], layout: layouts[port] || ""}))
                relativePorts = relative.sort((a, b) => a - b)
                baseText = editor.text
                errorText = ""
                open()
            } catch (error) { dialog.statusText = "Cannot edit player controllers: " + error }
        }
        contentItem: ScrollView {
            ColumnLayout {
                width: parent.width
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Add player ports, replace controllers or choose each player's destination preset. Inherit uses the shared per-game preset. Adding a port does not prove game support. Removing a port removes its layout override; explicit button/axis assignments must be handled before removal. Review the new mapping before staging." }
                Label {
                    Layout.fillWidth: true
                    visible: playerControllers.relativePorts.length > 0
                    wrapMode: Text.WordWrap
                    text: "Relative sources remain configured for ports " + playerControllers.relativePorts.join(", ")
                        + ". Removing a gamepad does not remove these sources or their axis assignments. Relative-only ports must have no remaining gamepad-channel requirements before staging."
                }
                RowLayout {
                    Layout.fillWidth: true
                    visible: playerControllers.unusedPorts.length > 0
                    ComboBox {
                        id: additionalPort
                        Layout.fillWidth: true
                        model: playerControllers.unusedPorts.map(port => "Player " + port)
                        Accessible.name: "Unused MAME player port"
                    }
                    Button {
                        text: "Add player port"
                        enabled: additionalPort.currentIndex >= 0
                        onClicked: playerControllers.addPlayer(playerControllers.unusedPorts[additionalPort.currentIndex])
                    }
                }
                Repeater {
                    model: playerControllers.players
                    ColumnLayout {
                        id: replacementRow
                        required property int index
                        required property var modelData
                        Layout.fillWidth: true
                        Label { text: "Player " + replacementRow.modelData.port }
                        ComboBox {
                            Layout.fillWidth: true
                            model: newSetup.controllerChoices.map(entry => entry.id + " — " + entry.layout)
                            currentIndex: newSetup.controllerChoices.findIndex(entry => entry.id === replacementRow.modelData.controller)
                            displayText: currentIndex < 0 ? (replacementRow.modelData.controller ? "Missing calibration: " + replacementRow.modelData.controller : "Choose saved calibration…") : currentText
                            Accessible.name: "Replacement controller for player " + replacementRow.modelData.port
                            onActivated: function(choiceIndex) {
                                const players = playerControllers.players.slice()
                                players[replacementRow.index] = {port: replacementRow.modelData.port, controller: newSetup.controllerChoices[choiceIndex].id, layout: replacementRow.modelData.layout}
                                playerControllers.players = players
                            }
                        }
                        ComboBox {
                            Layout.fillWidth: true
                            model: ["Inherit shared per-game preset"].concat(dialog.presetLabels)
                            currentIndex: replacementRow.modelData.layout ? dialog.presetIds.indexOf(replacementRow.modelData.layout) + 1 : 0
                            Accessible.name: "Destination layout preset for player " + replacementRow.modelData.port
                            onActivated: function(choiceIndex) {
                                const players = playerControllers.players.slice()
                                players[replacementRow.index] = {port: replacementRow.modelData.port,
                                    controller: replacementRow.modelData.controller,
                                    layout: choiceIndex === 0 ? "" : dialog.presetIds[choiceIndex - 1]}
                                playerControllers.players = players
                            }
                        }
                        Button {
                            text: "Remove gamepad"
                            enabled: playerControllers.players.length > 1 || playerControllers.relativePorts.length > 0
                            Accessible.name: "Remove MAME gamepad for player " + replacementRow.modelData.port
                            onClicked: playerControllers.removePlayer(replacementRow.modelData.port)
                        }
                    }
                }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: playerControllers.errorText }
                Button {
                    text: "Apply to draft"
                    onClicked: {
                        try {
                            if (editor.text !== playerControllers.baseText) throw new Error("The draft changed. Close and reopen this editor.")
                            const draft = JSON.parse(editor.text), players = {}, layouts = {}, used = [], unavailable = []
                            if (!playerControllers.players.length && !playerControllers.relativePorts.length)
                                throw new Error("Keep at least one gamepad or configured relative source.")
                            for (const player of playerControllers.players) {
                                if (typeof player.controller !== "string" || !player.controller.length || player.controller.length > 1024)
                                    throw new Error("Choose a controller identity for player " + player.port + ".")
                                if (!newSetup.controllerChoices.some(entry => entry.id === player.controller)) {
                                    if (!Object.prototype.hasOwnProperty.call(draft.players, player.port)
                                            || draft.players[player.port] !== player.controller)
                                        throw new Error("New or replacement controllers need a valid saved calibration for player " + player.port + ".")
                                    unavailable.push(player.port)
                                }
                                if (used.indexOf(player.controller) >= 0) throw new Error("Each player needs a different controller.")
                                used.push(player.controller)
                                players[player.port] = player.controller
                                if (player.layout) {
                                    if (dialog.presetIds.indexOf(player.layout) < 0) throw new Error("Choose a supported player layout preset.")
                                    layouts[player.port] = player.layout
                                }
                            }
                            if ((draft.digital_assignments || []).concat(draft.analog_assignments || [])
                                    .some(assignment => !Object.prototype.hasOwnProperty.call(players, String(assignment.source_player))))
                                throw new Error("An explicit assignment still references an unselected source player.")
                            draft.players = players
                            if (Object.keys(layouts).length) draft.player_digital_layouts = layouts
                            else delete draft.player_digital_layouts
                            editor.text = JSON.stringify(draft, null, 2)
                            dialog.reviewedText = ""
                            dialog.reviewHasUnhandled = true
                            dialog.statusText = "Player controllers and layout presets updated in the draft only. Review the new source/destination mappings before staging. Explicit input assignments were preserved."
                                + (unavailable.length ? " Existing controller identities with unavailable calibration were retained for players "
                                    + unavailable.join(", ") + "; repair their calibration before staging." : "")
                            playerControllers.close()
                        } catch (error) { playerControllers.errorText = String(error) }
                    }
                }
            }
        }
    }
    Dialog {
        id: newSetup
        property string errorText: ""
        property var selectedControllers: []
        readonly property var controllerChoices: {
            dialog.settingsModel.controller_revision
            try { return JSON.parse(dialog.settingsModel.saved_controller_choices_json()).filter(entry => entry.valid) }
            catch (_) { return [] }
        }
        title: "New MAME arcade setup"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        standardButtons: Dialog.Cancel
        function absolutePath(value, name) {
            const path = value.trim()
            if (!path.startsWith("/") || path === "/" || /[\x00-\x1f\x7f]/.test(path)
                    || path.split("/").slice(1).some(part => !part || part === "." || part === ".."))
                throw new Error(name + " must be a normalized absolute path, without a trailing slash.")
            return path
        }
        contentItem: ScrollView {
            ColumnLayout {
                width: parent.width
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Create a draft for an exact ZIP/7z arcade set. Use your configured emulator identity and select saved controller calibrations for each player. Inspection is still required; merged sets need advanced editing." }
                TextField { id: newEmulator; Layout.fillWidth: true; placeholderText: "Configured emulator ID"; Accessible.name: placeholderText }
                TextField { id: newCore; Layout.fillWidth: true; placeholderText: "Absolute path to MAME libretro core"; Accessible.name: placeholderText }
                TextField { id: newContent; Layout.fillWidth: true; placeholderText: "Absolute path to ROM ZIP/7z"; Accessible.name: placeholderText }
                TextField { id: newMachine; Layout.fillWidth: true; placeholderText: "Exact MAME machine shortname (not game title)"; Accessible.name: placeholderText }
                TextField { id: newLibrary; Layout.fillWidth: true; placeholderText: "Exact RetroArch library configuration name"; Accessible.name: placeholderText }
                RowLayout {
                    Label { text: "Players" }
                    SpinBox { id: newPlayerCount; from: 1; to: 8; value: 1; Accessible.name: "MAME player count" }
                }
                Repeater {
                    model: newPlayerCount.value
                    RowLayout {
                        id: newPlayerRow
                        required property int index
                        Layout.fillWidth: true
                        Label { text: "Player " + (index + 1) }
                        ComboBox {
                            Layout.fillWidth: true
                            model: newSetup.controllerChoices.map(entry => entry.id + " — " + entry.layout)
                            currentIndex: newSetup.controllerChoices.findIndex(entry => entry.id === newSetup.selectedControllers[index])
                            displayText: currentIndex < 0 ? "Choose saved calibration…" : currentText
                            Accessible.name: "Saved controller for player " + (index + 1)
                            onActivated: function(choiceIndex) {
                                const selected = newSetup.selectedControllers.slice()
                                selected[newPlayerRow.index] = newSetup.controllerChoices[choiceIndex].id
                                newSetup.selectedControllers = selected
                            }
                        }
                    }
                }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: newSetup.controllerChoices.length ? "Saved calibration does not prove a controller is connected. Each player needs a different controller; launch rechecks devices." : "No valid saved calibrations. Calibrate controllers in controller settings before creating a draft." }
                TextField { id: newStorage; Layout.fillWidth: true; placeholderText: "Absolute dedicated per-game storage directory"; Accessible.name: placeholderText }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: "Storage uses nvram, diff, states, snapshots, recordings, frontend-save and frontend-state subdirectories. No directories are created by this form. Existing game state elsewhere is not imported. The draft starts with Automatic arcade layout." }
                Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: newSetup.errorText }
                Button {
                    text: "Create draft"
                    enabled: !dialog.draftChanged
                    onClicked: {
                        try {
                            const core = newSetup.absolutePath(newCore.text, "Core")
                            const content = newSetup.absolutePath(newContent.text, "ROM")
                            const storage = newSetup.absolutePath(newStorage.text, "Storage")
                            const machine = newMachine.text.trim()
                            if (!/^[a-z0-9_]{1,64}$/.test(machine) || machine === "default") throw new Error("Enter an exact MAME machine shortname.")
                            const filename = content.split("/").pop()
                            if (filename !== machine + ".zip" && filename !== machine + ".7z") throw new Error("The archive filename must match the exact machine shortname (.zip or .7z).")
                            const emulator = newEmulator.text.trim(), library = newLibrary.text.trim()
                            if (!emulator || emulator.length > 512) throw new Error("Enter the configured emulator ID.")
                            const players = {}, used = []
                            for (let port = 1; port <= newPlayerCount.value; ++port) {
                                const controller = newSetup.selectedControllers[port - 1]
                                if (!newSetup.controllerChoices.some(entry => entry.id === controller)) throw new Error("Choose a saved calibration for player " + port + ".")
                                if (used.indexOf(controller) >= 0) throw new Error("Each player needs a different controller.")
                                players[String(port)] = controller
                                used.push(controller)
                            }
                            if (!library || library.length > 128 || library === "." || library === ".." || /[\/\\\x00-\x1f\x7f]/.test(library)) throw new Error("Enter the exact library configuration name, without path separators.")
                            const persistent = {}
                            for (const category of ["nvram", "diff", "states", "snapshots", "recordings"]) persistent[category] = storage + "/" + category
                            const draft = {emulator_id: emulator, core: core, content: content, machine: machine, library: library,
                                inputs: [{source: content, destination: "roms/" + filename}], persistent: persistent,
                                frontend_save: storage + "/frontend-save", frontend_state: storage + "/frontend-state",
                                players: players, digital_layout: "automatic"}
                            editor.text = JSON.stringify(draft, null, 2)
                            dialog.reviewedText = ""
                            dialog.reviewHasUnhandled = true
                            dialog.statusText = "New uninspected draft. Inspect native fields (add ROM search folders for dependencies), use the completed inspection, then review assignments. Nothing has been staged or saved."
                            newSetup.close()
                        } catch (error) { newSetup.errorText = String(error) }
                    }
                }
            }
        }
    }
    Dialog {
        id: inspection
        property string errorText: ""
        property string generatedDraft: ""
        readonly property bool draftRequestStale: generatedDraft.length > 0 && generatedDraft !== editor.text
        onDraftRequestStaleChanged: if (draftRequestStale) trustRuntime.checked = false
        onOpened: errorText = ""
        title: "MAME native inspection — advanced request"
        width: dialog.width - 40
        height: dialog.height - 40
        modal: true
        closePolicy: Popup.NoAutoClose
        contentItem: ColumnLayout {
            RowLayout {
                Layout.fillWidth: true
                TextField { id: nativeRuntimePath; Layout.fillWidth: true; placeholderText: "Absolute path to trusted native RetroArch"; enabled: !dialog.settingsModel.mame_inspection_busy; onTextChanged: { trustRuntime.checked = false; inspectionRequest.text = "" } }
                Button {
                    text: "Generate from setup draft"
                    enabled: !dialog.settingsModel.mame_inspection_busy && nativeRuntimePath.text.trim().length > 0
                    onClicked: {
                        inspection.errorText = ""
                        try {
                            const result = JSON.parse(dialog.settingsModel.mame_inspection_request_json(editor.text, nativeRuntimePath.text, discoveryRoots.text))
                            if (result.error) throw new Error(result.error)
                            const request = JSON.parse(result.request)
                            request.inspect_mouse = inspectNativeMouse.checked
                            if (request.inspect_mouse) {
                                // Pin the wrapper's polling option as well as the
                                // native class flag. Preserve all unrelated options.
                                request.core_options = request.core_options.split("\n")
                                    .filter(line => !/^\s*mame_mouse_enable\s*=/.test(line))
                                    .join("\n") + '\nmame_mouse_enable = "enabled"\n'
                            }
                            inspectionRequest.text = JSON.stringify(request, null, 2)
                            inspection.generatedDraft = editor.text
                            trustRuntime.checked = false
                        } catch (error) { inspectionRequest.text = ""; inspection.errorText = "Cannot generate request: " + error }
                    }
                }
            }
            CheckBox {
                id: inspectNativeMouse
                text: "Include enabled native mouse evidence in generated request"
                checked: false
                enabled: !dialog.settingsModel.mame_inspection_busy
                onToggled: { inspectionRequest.text = ""; trustRuntime.checked = false }
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Optional discovery queries this exact core for parent, BIOS, device and CHD dependencies before field inspection. Each required ROM-bearing set needs its own ZIP/7z; merged archives are not inferred. Leave folders empty to retain only the explicit manifest. Generate does not run native code; Run starts the trusted executable/core. Private staging is not an OS sandbox."
            }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: "Mouse evidence enables the native class for inspection only, not physical capture or launch routing. The request JSON is authoritative, including pasted requests. Changing the option requires regeneration and renewed trust confirmation."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.preferredHeight: 66
                TextArea {
                    id: discoveryRoots
                    placeholderText: "Optional ROM search folders: one absolute directory per line. Regenerate the request after changes."
                    selectByMouse: true
                    readOnly: dialog.settingsModel.mame_inspection_busy
                    onTextChanged: { trustRuntime.checked = false; inspectionRequest.text = "" }
                }
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                TextArea { id: inspectionRequest; placeholderText: "Paste complete inspection request JSON"; selectByMouse: true; readOnly: dialog.settingsModel.mame_inspection_busy; font.family: "monospace"; onTextChanged: trustRuntime.checked = false }
            }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: dialog.settingsModel.mame_inspection_status }
            Label { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: inspection.errorText; visible: text.length > 0 }
            Label {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: inspection.draftRequestStale
                text: "The setup draft changed after this request was generated. Generate a fresh request and confirm trust again before running inspection."
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                TextArea { text: dialog.settingsModel.mame_inspection_result; readOnly: true; selectByMouse: true; font.family: "monospace" }
            }
            CheckBox { id: trustRuntime; text: "I trust this executable/core and want to run this inspection"; enabled: !dialog.settingsModel.mame_inspection_busy }
            RowLayout {
                Button {
                    text: "Run inspection"
                    enabled: trustRuntime.checked && !inspection.draftRequestStale && !dialog.settingsModel.mame_inspection_busy && inspectionRequest.text.trim().length > 0
                    onClicked: {
                        inspection.errorText = ""
                        if (inspection.draftRequestStale) { inspection.errorText = "Regenerate the request from the current draft first."; return }
                        const error = dialog.settingsModel.begin_mame_inspection(inspectionRequest.text)
                        if (error) inspection.errorText = error
                    }
                }
                Button {
                    text: "Apply result to draft"
                    enabled: !dialog.settingsModel.mame_inspection_busy && dialog.settingsModel.mame_inspection_result.length > 0
                    onClicked: {
                        if (dialog.applyCompletedInspection()) inspection.close()
                        else inspection.errorText = dialog.statusText
                    }
                }
                Button { text: "Cancel"; enabled: dialog.settingsModel.mame_inspection_busy; onClicked: dialog.settingsModel.cancel_mame_inspection() }
                Button { text: "Close"; onClicked: { dialog.settingsModel.cancel_mame_inspection(); inspection.close() } }
            }
        }
    }
    Dialog {
        id: removal
        property string keyToRemove: ""
        title: "Remove this MAME setup?"
        modal: true
        standardButtons: Dialog.Yes | Dialog.No
        contentItem: Label { text: "Only staged controller settings change. Game files and calibrations are kept."; wrapMode: Text.WordWrap }
        onAccepted: {
            const error = dialog.settingsModel.remove_mame_controller_setup(keyToRemove)
            dialog.statusText = error || "Setup removed from staged settings. Save settings to persist the change."
            dialog.refresh()
        }
    }
}
