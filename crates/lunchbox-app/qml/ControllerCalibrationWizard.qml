import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: wizard
    required property var settingsModel
    required property var gamepad
    readonly property var catalog: JSON.parse(settingsModel.controller_catalog_json())
    property bool guided: false
    property string deviceId: ""
    property string deviceName: ""
    property int layoutIndex: 0
    property int step: 0
    property var bindings: ({})
    property string baselineCalibration: ""
    property string status: ""
    property bool waitingForRelease: false
    property string targetedControlId: ""
    property bool targetedComplete: false
    property string pendingControlId: ""
    property bool pendingHadBinding: false
    property var pendingOriginalBinding: null
    property bool showPreview: false
    property bool reviewingSaved: false
    property int profileIndex: 0
    readonly property var layout: catalog.layouts[layoutIndex]
    readonly property var calibrationControls: layout ? layout.controls.filter(control => !control.repeat_of) : []
    readonly property var nonrecordableRecords: layout ? Object.keys(bindings).filter(id =>
        !calibrationControls.some(control => control.id === id)).sort() : []
    onNonrecordableRecordsChanged: obsoleteRecord.currentIndex = -1
    readonly property var requiredControls: calibrationControls.filter(control => !control.optional)
    readonly property var recordingErrors: {
        const errors = {}
        if (!visible || !layout) return errors
        for (const id of Object.keys(bindings)) {
            if (id === pendingControlId) continue
            try {
                const error = settingsModel.validate_controller_capture(layout.id, id, JSON.stringify(bindings[id]))
                if (error) errors[id] = error
            } catch (failure) { errors[id] = "Recording validation unavailable: " + failure }
        }
        return errors
    }
    readonly property var missingRequiredControls: requiredControls.filter(control =>
        !bindings[control.id] || pendingControlId === control.id || !!recordingErrors[control.id])
    readonly property var currentControl: step < calibrationControls.length ? calibrationControls[step] : null
    readonly property var preview: showPreview && layout && catalog.emulator_profiles[profileIndex] ? JSON.parse(settingsModel.controller_mapping_preview(
        layout.id, JSON.stringify(bindings), catalog.emulator_profiles[profileIndex].id)) : ({ rows: [], warnings: [] })
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(900, parent ? parent.width - 32 : 900)
    height: Math.min(guided && !layout ? 440 : 850, parent ? parent.height - 32 : 850)
    padding: guided ? 24 : 12
    modal: true
    closePolicy: Popup.CloseOnEscape
    title: guided ? "Set up " + deviceName : "Calibrate " + deviceName
    Timer {
        interval: 1500
        running: wizard.visible && Qt.application.arguments.includes("--controller-font-review")
        onTriggered: {
            const label = physicalLayout.contentItem
            console.log("LAYOUT_FONT", label.text, JSON.stringify(label.font),
                        "renderType", label.renderType, "scene", JSON.stringify(label.mapToItem(null, 0, 0)))
            wizard.contentItem.parent.grabToImage(result => result.saveToFile("/tmp/lunchbox-real-layout-font.png"))
        }
    }

    function isPressure(control) {
        return control && control.analog && (control.pressure || control.group === "shoulder")
    }
    function removeNonrecordableRecord(id) {
        if (waitingForRelease || !layout || nonrecordableRecords.indexOf(id) < 0) return
        const next = Object.assign({}, bindings)
        delete next[id]
        bindings = next
        targetedControlId = ""
        targetedComplete = true
        status = "Removed " + id + " from this draft only. Recording is paused. Other records were kept; Cancel leaves saved calibration unchanged."
    }
    function repeatRecordOwner(id) {
        if (waitingForRelease || !layout || nonrecordableRecords.indexOf(id) < 0) return ""
        const sources = layout.controls.filter(control => control.id === id)
        if (sources.length !== 1 || !sources[0].repeat_of) return ""
        const owner = sources[0].repeat_of
        if (layout.controls.filter(control => control.id === owner).length !== 1
                || calibrationControls.filter(control => control.id === owner).length !== 1
                || Object.prototype.hasOwnProperty.call(bindings, owner)) return ""
        return owner
    }
    function moveRepeatRecord(id) {
        const owner = repeatRecordOwner(id)
        if (!owner) return
        try {
            const error = settingsModel.validate_controller_capture(layout.id, owner, JSON.stringify(bindings[id]))
            if (error) {
                status = "Cannot move this recording to " + owner + ": " + error + " All records were kept."
                return
            }
        } catch (failure) {
            status = "Recording validation unavailable: " + failure + ". All records were kept."
            return
        }
        const next = Object.assign({}, bindings)
        next[owner] = next[id]
        delete next[id]
        bindings = next
        targetedControlId = ""
        targetedComplete = true
        status = "Moved " + id + " to its declared base " + owner + " in this draft only, without changing its measurement. Recording is paused; full calibration validation still applies when saving. Cancel leaves saved calibration unchanged."
    }
    function controlPrompt(control) {
        if (!layout) return "Choose a physical controller layout before recording inputs."
        if (!control) return "End of layout. Review any unrecorded controls and the mapping before using this calibration."
        if (isPressure(control)) return "Press " + control.label + " gradually to full pressure, then release completely. A digital-only button cannot supply this control."
        if (control.analog) return "Move " + control.label + " fully in the highlighted direction, then release."
        return "Press " + control.label + "."
    }
    function openFor(id, name) {
        clearPendingCapture()
        deviceId = id; deviceName = name
        const savedText = settingsModel.controller_calibration_json(id)
        const saved = JSON.parse(savedText)
        baselineCalibration = savedText
        layoutIndex = catalog.layouts.findIndex(item => item.id === saved.layout)
        bindings = layoutIndex >= 0 && saved.os === catalog.host_os ? (saved.bindings || {}) : ({})
        step = 0; waitingForRelease = false; showPreview = false
        reviewingSaved = layoutIndex >= 0 && saved.os === catalog.host_os && Object.keys(bindings).length > 0
        targetedControlId = ""; targetedComplete = reviewingSaved
        status = layoutIndex < 0
            ? "Choose the picture that matches your controller. Then press each highlighted button. Your existing setup is kept until you save."
            : saved.os !== catalog.host_os
            ? "Saved measurements belong to another OS. Record this layout on the current host; existing saved calibration stays unchanged until you apply."
            : reviewingSaved ? "Your layout and recorded buttons are saved. Re-record only if you want to change them."
            : "Choose the closest physical layout, then press the highlighted control."
        open()
    }
    function focusSavedControl(layoutId, controlId) {
        reviewingSaved = false
        const mismatch = () => {
            cancelPendingCapture()
            targetedControlId = ""
            targetedComplete = true
            showPreview = false
            status = "The requested control does not match this layout. Recording is paused; choose the correct layout/control or explicitly resume. Existing bindings were preserved."
            return false
        }
        if (!layout || layout.id !== layoutId) return mismatch()
        const controls = layout.controls.filter(control => control.id === controlId)
        if (controls.length !== 1) return mismatch()
        // Repeated diagram positions share their declared calibration owner.
        const owner = controls[0].repeat_of || controlId
        const indices = []
        calibrationControls.forEach((control, index) => {
            if (control.id === owner) indices.push(index)
        })
        if (indices.length !== 1) return mismatch()
        cancelPendingCapture()
        step = indices[0]
        targetedControlId = owner
        targetedComplete = false
        waitingForRelease = false
        showPreview = false
        status = controlPrompt(currentControl)
        return true
    }
    function resetLayout(index) {
        reviewingSaved = false
        if (!Number.isInteger(index) || index < 0 || index >= catalog.layouts.length) {
            status = "Choose an available physical controller layout."
            return
        }
        clearPendingCapture()
        targetedControlId = ""; targetedComplete = false
        layoutIndex = index; bindings = ({}); step = 0; waitingForRelease = false
        status = controlPrompt(currentControl)
    }
    function clearPendingCapture() {
        pendingControlId = ""
        pendingHadBinding = false
        pendingOriginalBinding = null
        waitingForRelease = false
    }
    function cancelPendingCapture() {
        if (pendingControlId) {
            const next = Object.assign({}, bindings)
            if (pendingHadBinding) next[pendingControlId] = pendingOriginalBinding
            else delete next[pendingControlId]
            bindings = next
        }
        clearPendingCapture()
    }
    function receiveInput() {
        if (!visible || !currentControl || waitingForRelease || showPreview || targetedComplete) return
        if (targetedControlId && currentControl.id !== targetedControlId) return
        if (settingsModel.controller_key_for_input(gamepad.last_device_key) !== deviceId) {
            const incoming = settingsModel.controller_key_for_input(gamepad.last_device_key)
            let incomingName = "another controller"
            for (let index = 0; index < settingsModel.controller_count(); index++) {
                if (settingsModel.controller_key_at(index) === incoming)
                    incomingName = settingsModel.controller_name_at(index)
            }
            status = "Input detected from " + incomingName + ", but this setup is recording " + deviceName + ". Close this dialog and choose Set up on the controller you are holding."
            return
        }
        if (gamepad.last_capture_started !== true) {
            status = "A previous gesture is still being measured. Release all controls and let them settle, then press only the highlighted control."
            return
        }
        let input
        try { input = JSON.parse(gamepad.last_binding) } catch (error) { return }
        if (!input) return
        const physicalAxis = input.native && (input.native.code >>> 16) === 3
        // GilRs can report a real pressure axis as ButtonPressed. Match the
        // measured binding emitted on release: logical positive travel, with
        // physical polarity retained separately in native.direction.
        if (physicalAxis && input.kind === "button") {
            input.kind = "axis"
            input.direction = 1
        }
        if (isPressure(currentControl) && !physicalAxis) {
            status = "This control requires a measured physical pressure axis. A digital button or an unmeasured logical axis cannot supply proportional pressure."
            return
        }
        if (currentControl.analog && input.kind !== "axis") {
            status = controlPrompt(currentControl)
            return
        }
        const sharedPressureAxis = physicalAxis && Object.keys(bindings).find(id => {
            if (id === currentControl.id) return false
            const previous = bindings[id]
            const control = layout.controls.find(item => item.id === id)
            return previous.native && previous.native.code === input.native.code
                && (isPressure(currentControl) || isPressure(control))
        })
        if (sharedPressureAxis) {
            const control = layout.controls.find(item => item.id === sharedPressureAxis)
            status = "This physical axis already supplies " + control.label + ". Each pressure control needs its own independent axis; opposite halves cannot supply two pressure controls."
            return
        }
        const duplicate = Object.keys(bindings).find(id => id !== currentControl.id
            && bindings[id].code === input.code && bindings[id].kind === input.kind
            && bindings[id].direction === input.direction)
        if (duplicate) {
            const previousControl = layout.controls.find(control => control.id === duplicate)
            const label = previousControl ? previousControl.label : duplicate + " (not present in this layout)"
            status = "This input is already assigned to " + label + ". Go back to correct it, or skip a duplicate hardware button."
            return
        }
        let next = Object.assign({}, bindings)
        pendingControlId = currentControl.id
        pendingHadBinding = Object.prototype.hasOwnProperty.call(bindings, currentControl.id)
        pendingOriginalBinding = pendingHadBinding ? bindings[currentControl.id] : null
        next[currentControl.id] = input
        bindings = next
        waitingForRelease = true
        status = "Recorded " + currentControl.label + " → " + input.logical + " (code " + input.code + "). "
            + (isPressure(currentControl) ? "Press to full pressure, then release completely and let it settle."
                : physicalAxis ? "Move fully in the highlighted direction, then release and let it settle." : "Release all controls to continue.")
    }
    function receiveNeutral() {
        if (!visible || !waitingForRelease) return
        if (settingsModel.controller_key_for_input(gamepad.neutral_device_key) !== deviceId) return
        if (!currentControl || currentControl.id !== pendingControlId) {
            cancelPendingCapture()
            status = "Recording was interrupted. The previous binding was restored; select the control and try again."
            return
        }
        const recorded = currentControl ? bindings[currentControl.id] : null
        let completed = null
        try { completed = JSON.parse(gamepad.neutral_binding || "null") } catch (error) {}
        let error = gamepad.neutral_error || ""
        const matches = recorded && completed && recorded.code === completed.code
            && recorded.kind === completed.kind && recorded.direction === completed.direction
            && ((!recorded.native && !completed.native)
                || (recorded.native && completed.native && recorded.native.code === completed.native.code))
        // Raw native axis direction is refined from measured travel at release;
        // compare its physical code, not that provisional direction estimate.
        if (!error && recorded && recorded.native && (recorded.native.code >>> 16) === 3
            && (!matches || !completed.axis))
            error = isPressure(currentControl)
                ? "Could not measure pressure. Press gradually to full pressure, release completely, then try again."
                : "Could not measure this axis. Move fully in the highlighted direction, then release it and try again."
        if (!error && !matches)
            error = "The completed recording does not match the selected input. Release all controls, then press only the highlighted control and try again."
        if (!error) {
            try {
                error = settingsModel.validate_controller_capture(layout.id, pendingControlId, JSON.stringify(completed))
            } catch (failure) { error = "Could not validate the completed recording: " + failure }
        }
        if (!error) {
            // Release-time measurements refine native direction. Validate the
            // completed identity, not only the provisional press event.
            const conflict = Object.keys(bindings).find(id => {
                if (id === pendingControlId) return false
                const previous = bindings[id]
                const sameLogical = previous.code === completed.code
                    && previous.kind === completed.kind && previous.direction === completed.direction
                const sameNative = previous.native && completed.native
                    && previous.native.code === completed.native.code
                    && previous.native.direction === completed.native.direction
                return sameLogical || sameNative
            })
            if (conflict) {
                const control = layout.controls.find(item => item.id === conflict)
                error = "The completed input is already assigned to " + (control ? control.label : conflict)
                    + ". Choose an independent input or correct the existing assignment."
            }
        }
        if (error) {
            const restored = pendingHadBinding
            cancelPendingCapture()
            status = error + (restored ? " The previous binding was kept." : " No incomplete binding was kept.")
            return
        }
        let next = Object.assign({}, bindings)
        next[currentControl.id] = completed
        bindings = next
        clearPendingCapture()
        if (targetedControlId) {
            targetedComplete = true
            status = "Selected control recorded. Other saved bindings were kept. Use calibration to apply, or record this control again. Input recording is paused."
            return
        }
        step++
        status = controlPrompt(currentControl)
    }
    function skip() {
        if (!currentControl || targetedComplete) return
        cancelPendingCapture()
        let next = Object.assign({}, bindings)
        delete next[currentControl.id]
        bindings = next
        if (targetedControlId) {
            waitingForRelease = false
            targetedComplete = true
            status = "Selected control skipped; its binding was removed from this wizard draft. Other bindings were kept. Use calibration to apply or Cancel to discard. Input recording is paused."
            return
        }
        step++; waitingForRelease = false
        status = "Skipped controls remain unavailable; no input is invented for them."
    }
    Connections {
        target: wizard.gamepad
        function onInput_revisionChanged() { wizard.receiveInput() }
        function onNeutral_revisionChanged() { wizard.receiveNeutral() }
    }

    contentItem: ScrollView {
        id: scroll
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scroll.availableWidth
            spacing: 12
            ColumnLayout {
                visible: wizard.guided
                Layout.fillWidth: true
                spacing: 14
                Label { text: wizard.reviewingSaved ? "Your saved buttons" : wizard.layout ? "Follow the highlighted controls" : "Choose your controller’s layout"; font.pixelSize: 22; font.bold: true }
                PixelAlignedText {
                    Layout.fillWidth: true
                    font: wizard.font; color: "#a4c6db"
                    text: "Controller: " + wizard.deviceName
                    wrapMode: Text.WordWrap; elide: Text.ElideNone
                }
                TextField { id: layoutSearch; Layout.fillWidth: true; placeholderText: "Find a layout (Xbox, PlayStation, arcade…)" }
                ComboBox {
                    id: physicalLayout
                    Layout.fillWidth: true
                    readonly property var layouts: wizard.catalog.layouts.filter(item => !layoutSearch.text || item.name.toLowerCase().includes(layoutSearch.text.toLowerCase()))
                    model: layouts
                    textRole: "name"
                    currentIndex: wizard.layout ? layouts.findIndex(item => item.id === wizard.layout.id) : -1
                    displayText: currentIndex < 0 ? "Select a physical layout" : currentText
                    // This picker is not editable. Use a label instead of the
                    // desktop theme's disabled TextInput for its display text.
                    contentItem: PixelAlignedText {
                        objectName: "physicalLayoutDisplay"
                        text: physicalLayout.displayText
                        font: physicalLayout.font
                        color: physicalLayout.palette.buttonText
                        leftPadding: physicalLayout.mirrored ? 0 : 8
                        rightPadding: physicalLayout.mirrored ? 8 : 0
                        elide: Text.ElideRight
                    }
                    enabled: !wizard.waitingForRelease
                    onActivated: wizard.resetLayout(wizard.catalog.layouts.findIndex(item => item.id === layouts[currentIndex].id))
                }
                Image {
                    visible: !!wizard.layout && wizard.layout.id !== "brawler64"
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(320, width * 500 / 900)
                    source: wizard.layout ? wizard.settingsModel.controller_diagram(wizard.layout.id, wizard.currentControl ? wizard.currentControl.id : "") : ""
                    sourceSize.width: Math.max(1, Math.ceil(width * Screen.devicePixelRatio))
                    sourceSize.height: Math.max(1, Math.ceil(height * Screen.devicePixelRatio))
                    fillMode: Image.PreserveAspectFit
                }
                Brawler64Diagram {
                    visible: !!wizard.layout && wizard.layout.id === "brawler64"
                    Layout.fillWidth: true
                    Layout.preferredHeight: width * 500 / 900
                    activeControl: wizard.currentControl ? wizard.currentControl.id : ""
                }
                Label {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    font.pixelSize: 22; font.bold: true
                    text: !wizard.layout ? "Choose a layout to begin."
                        : wizard.waitingForRelease ? "Release the control."
                        : wizard.reviewingSaved ? "Saved layout · " + Object.keys(wizard.bindings).length + " recorded inputs"
                        : wizard.currentControl ? wizard.controlPrompt(wizard.currentControl) : "Layout recorded. Save your controller."
                }
                ProgressBar { Layout.fillWidth: true; from: 0; to: Math.max(1, wizard.calibrationControls.length); value: wizard.step }
                Label { text: Object.keys(wizard.bindings).length + " inputs recorded"; visible: !!wizard.layout }
                RowLayout {
                    Button {
                        text: "Re-record buttons"
                        visible: wizard.reviewingSaved
                        onClicked: { wizard.reviewingSaved = false; wizard.targetedComplete = false; wizard.status = wizard.controlPrompt(wizard.currentControl) }
                    }
                    Button { text: "Back"; enabled: !wizard.reviewingSaved && wizard.step > 0 && !wizard.waitingForRelease; onClicked: { wizard.step--; wizard.targetedComplete = false } }
                    Button { text: "Skip this control"; enabled: !wizard.reviewingSaved && !!wizard.currentControl && !wizard.waitingForRelease; onClicked: wizard.skip() }
                }
                PixelAlignedText {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    font: wizard.font
                    color: wizard.palette.windowText
                    elide: Text.ElideNone
                    text: wizard.status; visible: text.length > 0
                }
            }
            ColumnLayout {
            Layout.fillWidth: true
            visible: !wizard.guided
            ComboBox {
                Layout.fillWidth: true
                model: wizard.catalog.layouts
                textRole: "name"
                currentIndex: wizard.layoutIndex
                displayText: currentIndex < 0 ? "Choose a physical controller layout" : currentText
                onActivated: wizard.resetLayout(currentIndex)
                Accessible.name: "Physical controller layout"
            }
            Label {
                Layout.fillWidth: true
                visible: !!wizard.layout && !!wizard.layout.notes
                text: wizard.layout ? wizard.layout.notes || "" : ""
                color: "#a6b6c7"
                wrapMode: Text.WordWrap
            }
            Image {
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(330, wizard.height * 0.40)
                source: wizard.layout ? wizard.settingsModel.controller_diagram(wizard.layout.id,
                    wizard.currentControl ? wizard.currentControl.id : "") : ""
                sourceSize.width: Math.ceil(width * Screen.devicePixelRatio)
                sourceSize.height: Math.ceil(height * Screen.devicePixelRatio)
                fillMode: Image.PreserveAspectFit
                Accessible.name: "Controller diagram: " + (!wizard.layout ? "choose a layout" : wizard.targetedComplete ? "recording paused" : wizard.currentControl ? "press " + wizard.currentControl.label : "end of layout")
            }
            Label {
                Layout.fillWidth: true
                text: !wizard.layout ? "Choose a physical layout" : wizard.targetedComplete ? "Recording paused"
                    : wizard.currentControl ? "Press " + wizard.currentControl.label
                    + (wizard.currentControl.optional ? " (optional)" : "") : "End of layout"
                font.pixelSize: 24
                font.bold: true
                color: "#ffb454"
                wrapMode: Text.WordWrap
            }
            ProgressBar {
                Layout.fillWidth: true
                visible: wizard.requiredControls.length > 0
                from: 0; to: Math.max(1, wizard.requiredControls.length)
                value: wizard.requiredControls.length - wizard.missingRequiredControls.length
            }
            Label {
                Layout.fillWidth: true
                visible: !!wizard.layout
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
                text: {
                    const total = wizard.requiredControls.length
                    const recorded = total - wizard.missingRequiredControls.length
                    return "Required layout controls with individually valid recording records: " + recorded + " / " + total
                        + (total ? " (" + (100 * recorded / total).toFixed(1) + "%)." : " (percentage unavailable).")
                        + " Optional controls and pending capture are excluded. This checks each record separately, not cross-control conflicts, complete native measurements, whole-game coverage or live input behavior."
                }
            }
            Button {
                text: "Repair next missing or invalid required control"
                enabled: !!wizard.layout && !wizard.waitingForRelease && wizard.missingRequiredControls.length > 0
                onClicked: {
                    const control = wizard.missingRequiredControls[0]
                    if (!wizard.waitingForRelease && control && wizard.layout)
                        wizard.focusSavedControl(wizard.layout.id, control.id)
                }
            }
            Label {
                Layout.fillWidth: true
                visible: !!wizard.layout
                text: "Or choose any control to record or repair. Other bindings are kept; recording pauses after the selected control."
                wrapMode: Text.WordWrap
            }
            ComboBox {
                Layout.fillWidth: true
                enabled: !!wizard.layout && !wizard.waitingForRelease
                model: wizard.calibrationControls.map(control => ({
                    controlId: control.id,
                    name: control.label + (control.optional ? " (optional)" : " (required)")
                        + (wizard.pendingControlId === control.id ? " — capture pending"
                            : wizard.recordingErrors[control.id] ? " — invalid recording"
                            : wizard.bindings[control.id] ? " — record valid individually" : " — no recording")
                }))
                textRole: "name"
                currentIndex: wizard.currentControl ? wizard.step : -1
                displayText: currentIndex >= 0 ? currentText : "Choose a control to record"
                Accessible.name: "Physical control to record or repair"
                onActivated: index => {
                    if (wizard.waitingForRelease || !wizard.layout) return
                    const control = wizard.calibrationControls[index]
                    if (control && !wizard.focusSavedControl(wizard.layout.id, control.id))
                        wizard.status = "This control could not be uniquely located in the current layout. No bindings were changed."
                }
            }
            Label {
                Layout.fillWidth: true
                visible: !!wizard.currentControl && !!wizard.recordingErrors[wizard.currentControl.id]
                text: wizard.currentControl ? wizard.recordingErrors[wizard.currentControl.id] || "" : ""
                textFormat: Text.PlainText
                wrapMode: Text.WordWrap
            }
            Label {
                Layout.fillWidth: true
                visible: wizard.nonrecordableRecords.length > 0
                text: "Some retained records are not independent recordable controls in this layout. A repeat record can move to its declared base only if that base has no recording and validation succeeds. Otherwise remove an obsolete record explicitly and record its current base if needed. Saved calibration is unchanged until you apply."
                wrapMode: Text.WordWrap
            }
            RowLayout {
                Layout.fillWidth: true
                visible: wizard.nonrecordableRecords.length > 0
                ComboBox {
                    id: obsoleteRecord
                    Layout.fillWidth: true
                    model: wizard.nonrecordableRecords
                    enabled: !wizard.waitingForRelease
                    displayText: currentIndex < 0 ? "Choose an obsolete or repeat-alias record" : currentText
                    Accessible.name: "Nonrecordable calibration record to repair in the draft"
                }
                Button {
                    text: "Move to base control"
                    enabled: obsoleteRecord.currentIndex >= 0
                        && !!wizard.repeatRecordOwner(wizard.nonrecordableRecords[obsoleteRecord.currentIndex])
                    onClicked: wizard.moveRepeatRecord(wizard.nonrecordableRecords[obsoleteRecord.currentIndex])
                }
                Button {
                    text: "Remove selected draft record"
                    enabled: !wizard.waitingForRelease && obsoleteRecord.currentIndex >= 0
                    onClicked: wizard.removeNonrecordableRecord(wizard.nonrecordableRecords[obsoleteRecord.currentIndex])
                }
            }
            Label {
                Layout.fillWidth: true
                text: wizard.status
                wrapMode: Text.WordWrap
                color: "#bfcddb"
            }
            RowLayout {
                Button {
                    text: "Back"
                    enabled: wizard.step > 0 && !wizard.targetedControlId
                    onClicked: {
                        wizard.cancelPendingCapture()
                        wizard.step--
                        wizard.status = wizard.targetedComplete
                            ? "Recording is paused. Choose Record this control again to capture the highlighted control."
                            : "Press the highlighted control to replace its binding."
                    }
                }
                Button { text: "Skip / not present"; enabled: !!wizard.currentControl && !wizard.targetedComplete; onClicked: wizard.skip() }
                Button {
                    text: "Record this control again"
                    visible: wizard.targetedComplete
                    enabled: !!wizard.layout && !!wizard.currentControl
                    onClicked: {
                        wizard.focusSavedControl(wizard.layout.id, wizard.currentControl.id)
                    }
                }
                Button { text: "Start over"; enabled: !!wizard.layout; onClicked: wizard.resetLayout(wizard.layoutIndex) }
            }
            CheckBox {
                text: "Preview system / emulator mapping"
                enabled: !!wizard.layout
                checked: wizard.showPreview
                onToggled: wizard.showPreview = checked
            }
            ColumnLayout {
                visible: wizard.showPreview
                Layout.fillWidth: true
                ComboBox {
                    Layout.fillWidth: true
                    model: wizard.catalog.emulator_profiles
                    textRole: "name"
                    currentIndex: wizard.profileIndex
                    onActivated: wizard.profileIndex = currentIndex
                    Accessible.name: "Emulator input contract"
                }
                Label {
                    Layout.fillWidth: true
                    visible: wizard.preview.transport === "duckstation-settings"
                    text: "Outputs below are DuckStation setting names, not SDL button numbers. Launch also needs a compatible native runtime setup."
                    wrapMode: Text.WordWrap
                    color: "#a6b6c7"
                }
                Label {
                    Layout.fillWidth: true
                    text: wizard.preview.automatic_launch_ready
                        ? "Mapping ready — controller and runtime checked again at launch"
                        : wizard.preview.native_runtime_required
                            ? "Guided native mapping — runtime setup and physical calibration required; see details below"
                            : "Preview only — missing physical inputs or launch adapter"
                    color: "#ffb454"
                    wrapMode: Text.WordWrap
                }
                ControllerMappingView {
                    Layout.fillWidth: true
                    settingsModel: wizard.settingsModel
                    sourceLayout: wizard.layout
                    destinationLayout: wizard.catalog.emulator_profiles[wizard.profileIndex]
                        ? wizard.catalog.layouts.find(layout => layout.id === wizard.catalog.emulator_profiles[wizard.profileIndex].target_layout) || null : null
                    rows: wizard.preview.rows
                }
                Repeater {
                    model: wizard.preview.rows
                    delegate: Label {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.target + " ← " + modelData.physical + " → " + modelData.output
                            + (modelData.input ? "" : " · MISSING")
                        wrapMode: Text.WordWrap
                        color: modelData.input ? "#dbe5ed" : "#ffb454"
                    }
                }
                Label {
                    Layout.fillWidth: true
                    text: wizard.preview.warnings.join("\n")
                    color: "#a6b6c7"
                    wrapMode: Text.WordWrap
                }
            }
        }
    }
    }
    footer: DialogButtonBox {
        Button { text: wizard.reviewingSaved ? "Close" : "Cancel"; DialogButtonBox.buttonRole: DialogButtonBox.RejectRole; onClicked: wizard.close() }
        Button {
            visible: !wizard.reviewingSaved
            text: wizard.guided ? "Save controller" : "Use calibration"
            enabled: !!wizard.layout && Object.keys(wizard.bindings).length > 0 && !wizard.waitingForRelease
            DialogButtonBox.buttonRole: DialogButtonBox.ActionRole
            onClicked: {
                if (!wizard.layout) { wizard.status = "Choose a physical layout before saving."; return }
                const error = wizard.settingsModel.save_controller_calibration_if_unchanged(wizard.deviceId, wizard.layout.id, JSON.stringify(wizard.bindings), wizard.baselineCalibration)
                if (error.length) wizard.status = error
                else if (wizard.guided) {
                    const failure = wizard.settingsModel.persist_controller_calibration(wizard.deviceId)
                    if (failure) {
                        wizard.baselineCalibration = wizard.settingsModel.controller_calibration_json(wizard.deviceId)
                        wizard.status = failure
                    } else wizard.close()
                } else wizard.close()
            }
        }
    }
}
