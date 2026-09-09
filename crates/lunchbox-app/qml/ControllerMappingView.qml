import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: view
    Brawler64Geometry { id: brawlerGeometry }
    required property var settingsModel
    property var sourceLayout: null
    property var destinationLayout: null
    property var rows: []
    property bool simple: false
    property var nativeRoutes: []
    // Supplied by callers that have checked saved native measurements. Empty
    // means no gap evidence supplied, not verified runtime readiness.
    property var physicalGaps: []
    signal controlActivated(int side, string controlId)
    property int selectedIndex: 0
    property string focusedSourceControl: ""
    readonly property var selected: rows.length && selectedIndex >= 0 ? rows[Math.min(selectedIndex, rows.length - 1)] : null
    onRowsChanged: { focusedSourceControl = ""; selectedIndex = 0; connections.requestPaint() }
    onSelectedChanged: { focusedSourceControl = ""; connections.requestPaint() }
    onPhysicalGapsChanged: connections.requestPaint()
    onSourceLayoutChanged: { focusedSourceControl = ""; Qt.callLater(() => connections.requestPaint()) }
    onDestinationLayoutChanged: Qt.callLater(() => connections.requestPaint())
    Component.onCompleted: Qt.callLater(() => connections.requestPaint())

    function sourceOwner(id) {
        const controls = sourceLayout ? sourceLayout.controls : []
        const matches = controls.filter(control => control.id === id)
        if (matches.length !== 1 || !matches[0].repeat_of) return id
        const owner = matches[0].repeat_of
        const owners = controls.filter(control => control.id === owner && !control.repeat_of)
        return owners.length === 1 ? owner : id
    }
    function rowMatchesControl(row, side, id) {
        // Hardware repeat positions share a physical input, not a new output.
        // Destination identities stay exact: they describe emulator actions.
        return side === 0 ? sourceOwner(row.physical_id) === sourceOwner(id) : row.target_id === id
    }
    function chooseControl(side, id) {
        focusedSourceControl = ""
        const indices = []
        rows.forEach((row, index) => {
            if (rowMatchesControl(row, side, id)) indices.push(index)
        })
        if (!indices.length) {
            selectedIndex = -1
            if (side === 0) focusedSourceControl = sourceOwner(id)
            return
        }
        const current = indices.indexOf(selectedIndex)
        selectedIndex = indices[(current + 1) % indices.length]
    }
    function gapReason(row) {
        if (!row) return ""
        const gap = physicalGaps.find(entry => entry.target_id === row.target_id)
        return gap ? gap.reason || "Saved physical calibration needs attention" : ""
    }
    readonly property var setupGapIndices: {
        const indices = []
        rows.forEach((row, index) => {
            if (!row.physical_id || gapReason(row).length > 0) indices.push(index)
        })
        return indices
    }
    function chooseNextSetupGap() {
        if (!setupGapIndices.length) return
        const next = setupGapIndices.find(index => index > selectedIndex)
        selectedIndex = next === undefined ? setupGapIndices[0] : next
    }
    function controlHasGap(side, id) {
        return rows.some(row => rowMatchesControl(row, side, id) && gapReason(row).length > 0)
    }
    function controlTooltip(side, control) {
        const matches = rows.filter(row => rowMatchesControl(row, side, control.id))
        const owner = side === 0 ? sourceOwner(control.id) : control.id
        const heading = control.label + (owner !== control.id
            ? " · Hardware repeat of " + sourceLayout.controls.find(entry => entry.id === owner).label
                + "; not an independent input. Connection lines use the base control."
            : "")
        if (!matches.length) return heading + " · No assignment in this view"
        return heading + "\n" + matches.map(row => row.physical + " → " + targetLabel(row)
            + (row.physical_id ? "" : " · UNMAPPED")
            + (gapReason(row) ? " · NEEDS CALIBRATION: " + gapReason(row) : "")).join("\n")
            + (matches.length > 1 ? "\nClick repeatedly to cycle through these assignments." : "")
    }
    function targetLabel(row) {
        const names = []
        for (const route of nativeRoutes) {
            if (route.target_id === row.target_id && route.label && names.indexOf(route.label) < 0)
                names.push(route.label)
        }
        return names.length ? names.join(" / ") + " [" + row.target + "]" : row.target
    }
    function diagramGap(row) {
        if (!row) return ""
        const reasons = []
        const sourceId = row.physical_id ? sourceOwner(row.physical_id) : ""
        const sourceControls = sourceLayout ? sourceLayout.controls : []
        const destinationControls = destinationLayout ? destinationLayout.controls : []
        if (!sourceId) reasons.push("no physical source assigned")
        else if (sourceControls.filter(control => control.id === sourceId).length !== 1)
            reasons.push("source geometry missing or ambiguous: " + sourceId)
        if (!row.target_id || destinationControls.filter(control => control.id === row.target_id).length !== 1)
            reasons.push("destination geometry missing or ambiguous: " + (row.target_id || "unspecified"))
        return reasons.join("; ")
    }
    readonly property var diagramGaps: rows.map(row => ({row: row, reason: diagramGap(row)}))
        .filter(entry => entry.reason.length > 0)
    function point(side, id) {
        const layout = side === 0 ? sourceLayout : destinationLayout
        const resolvedId = side === 0 ? sourceOwner(id) : id
        const matches = layout ? layout.controls.filter(control => control.id === resolvedId) : []
        const control = matches.length === 1 ? matches[0] : null
        const panel = diagrams.itemAt(side)
        if (!control || !panel) return null
        const position = layout.id === "brawler64" ? brawlerGeometry.point(resolvedId) : {x: control.x * 8 + 50, y: control.y * 4 + 35}
        if (!position) return null
        return panel.mapToItem(connections, position.x * panel.width / 900,
                              stage.titleHeight + 8 + position.y * stage.diagramHeight / 500)
    }
    Item {
        id: stage
        readonly property bool stacked: width < 700
        readonly property real panelWidth: Math.max(0, stacked ? width : (width - 36) / 2)
        readonly property real diagramHeight: panelWidth * 500 / 900
        property real sourceTitleHeight: 24
        property real destinationTitleHeight: 24
        readonly property real titleHeight: Math.max(24, sourceTitleHeight, destinationTitleHeight)
        readonly property real panelHeight: titleHeight + 8 + diagramHeight
        Layout.fillWidth: true
        Layout.preferredHeight: stacked ? panelHeight * 2 + 28 : panelHeight
        onWidthChanged: Qt.callLater(() => connections.requestPaint())
        onHeightChanged: Qt.callLater(() => connections.requestPaint())
        Item {
            anchors.fill: parent
            Repeater {
                id: diagrams
                model: [view.sourceLayout, view.destinationLayout]
                onItemAdded: Qt.callLater(() => connections.requestPaint())
                delegate: Item {
                    id: panel
                    required property int index
                    required property var modelData
                    readonly property real titleHeight: panelTitle.implicitHeight
                    function updateTitleHeight() {
                        if (index === 0) stage.sourceTitleHeight = titleHeight
                        else stage.destinationTitleHeight = titleHeight
                    }
                    onTitleHeightChanged: updateTitleHeight()
                    Component.onCompleted: updateTitleHeight()
                    x: !stage.stacked && index === 1 ? stage.panelWidth + 36 : 0
                    y: stage.stacked && index === 1 ? stage.panelHeight + 28 : 0
                    width: stage.panelWidth
                    height: stage.panelHeight
                    Label {
                        id: panelTitle
                        width: parent.width
                        text: (panel.index === 0 ? "Source · " : "Destination · ")
                            + (panel.modelData ? panel.modelData.name : "Choose a layout")
                        textFormat: Text.PlainText
                        wrapMode: Text.WordWrap
                    }
                    Image {
                        visible: !panel.modelData || panel.modelData.id !== "brawler64"
                        y: stage.titleHeight + 8
                        width: parent.width
                        height: stage.diagramHeight
                        fillMode: Image.Stretch
                        sourceSize.width: Math.max(1, Math.ceil(width * Screen.devicePixelRatio))
                        sourceSize.height: Math.max(1, Math.ceil(height * Screen.devicePixelRatio))
                        source: panel.modelData ? view.settingsModel.controller_diagram(panel.modelData.id,
                            panel.index === 0 ? view.focusedSourceControl || (view.selected ? view.sourceOwner(view.selected.physical_id || "") : "")
                                : (view.selected ? view.selected.target_id : "")) : ""
                        Accessible.name: (panel.index === 0 ? "Source " : "Destination ") + (panel.modelData ? panel.modelData.name : "layout")
                    }
                    Brawler64Diagram {
                        visible: !!panel.modelData && panel.modelData.id === "brawler64"
                        y: stage.titleHeight + 8
                        width: parent.width; height: stage.diagramHeight
                        activeControl: panel.index === 0 ? view.focusedSourceControl || (view.selected ? view.sourceOwner(view.selected.physical_id || "") : "") : (view.selected ? view.selected.target_id : "")
                    }
                    Repeater {
                        model: panel.modelData ? panel.modelData.controls : []
                        delegate: AbstractButton {
                            id: controlHotspot
                            required property var modelData
                            readonly property var position: panel.modelData.id === "brawler64" ? brawlerGeometry.point(modelData.id) : {x: modelData.x * 8 + 50, y: modelData.y * 4 + 35}
                            x: position.x * panel.width / 900 - width / 2
                            y: stage.titleHeight + 8 + position.y * stage.diagramHeight / 500 - height / 2
                            width: Math.max(20, panel.width * 55 / 900)
                            height: Math.max(20, stage.diagramHeight * 55 / 500)
                            hoverEnabled: true
                            activeFocusOnTab: true
                            Accessible.role: Accessible.Button
                            Accessible.name: (panel.index === 0 ? "Source: " : "Destination: ") + modelData.label
                            Accessible.description: view.controlTooltip(panel.index, modelData)
                            Accessible.onPressAction: controlHotspot.clicked()
                            background: Rectangle {
                                color: "transparent"
                                radius: 4
                                border.width: controlHotspot.activeFocus || view.controlHasGap(panel.index, controlHotspot.modelData.id) ? 2 : 0
                                border.color: controlHotspot.activeFocus ? "#ffb454" : "#e57474"
                            }
                            onClicked: { view.chooseControl(panel.index, modelData.id); view.controlActivated(panel.index, modelData.id) }
                            ToolTip {
                                visible: controlHotspot.hovered || controlHotspot.activeFocus
                                text: view.controlTooltip(panel.index, controlHotspot.modelData)
                                contentItem: Text {
                                    text: view.controlTooltip(panel.index, controlHotspot.modelData)
                                    textFormat: Text.PlainText
                                    color: controlHotspot.palette.toolTipText
                                }
                            }
                        }
                    }
                }
            }
        }
        Canvas {
            id: connections
            anchors.fill: parent
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                function drawConnection(row, highlighted) {
                    if (!row || !row.physical_id) return
                    const from = view.point(0, row.physical_id)
                    const to = view.point(1, row.target_id)
                    if (!from || !to) return
                    ctx.strokeStyle = view.gapReason(row) ? "#e57474" : highlighted ? "#ffb454" : "#708090"
                    ctx.globalAlpha = highlighted ? 1 : 0.4
                    ctx.lineWidth = highlighted ? 2.5 : 1
                    ctx.beginPath()
                    ctx.moveTo(from.x, from.y)
                    if (stage.stacked) {
                        const middle = (from.y + to.y) / 2
                        ctx.bezierCurveTo(from.x, middle, to.x, middle, to.x, to.y)
                    } else {
                        const middle = (from.x + to.x) / 2
                        ctx.bezierCurveTo(middle, from.y, middle, to.y, to.x, to.y)
                    }
                    ctx.stroke()
                    ctx.fillStyle = ctx.strokeStyle
                    for (const point of [from, to]) {
                        ctx.beginPath(); ctx.arc(point.x, point.y, 4, 0, Math.PI * 2); ctx.fill()
                    }
                    ctx.globalAlpha = 1
                }
                if (allConnections.checked) {
                    for (const row of view.rows) drawConnection(row, false)
                }
                drawConnection(view.selected, true)
            }
        }
    }
    ColumnLayout {
    visible: !view.simple
    Layout.fillWidth: true
    CheckBox {
        id: allConnections
        text: "Show all mapping connections"
        checked: !view.simple
        onCheckedChanged: connections.requestPaint()
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        text: {
            const assigned = view.rows.filter(row => !!row.physical_id).length
            return view.rows.length ? "Displayed assignments: " + assigned + "/" + view.rows.length
                + " have a source · " + (view.rows.length - assigned) + " unmapped."
                + " This counts only this view, not whole-game coverage or runtime readiness."
                : "No assignments in this view."
        }
    }
    ComboBox {
        Layout.fillWidth: true
        model: view.rows.map(row => row.physical + " → " + view.targetLabel(row)
            + (row.physical_id ? "" : " · UNMAPPED")
            + (view.diagramGap(row) ? " · DIAGRAM INCOMPLETE" : "")
            + (view.gapReason(row) ? " · NEEDS CALIBRATION" : ""))
        currentIndex: view.rows.length ? view.selectedIndex : -1
        onActivated: view.selectedIndex = currentIndex
        Accessible.name: "Mapping connection to highlight"
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        text: {
            const total = view.rows.length
            const drawable = total - view.diagramGaps.length
            return total ? "Diagram coverage: " + drawable + "/" + total + " assignments ("
                + Math.round(1000 * drawable / total) / 10 + "%) have both endpoints represented."
                + " This measures this diagram only, not calibration, game coverage, or runtime behavior."
                : "Diagram coverage: no assignments to measure."
        }
    }
    Label {
        Layout.fillWidth: true
        visible: view.diagramGap(view.selected).length > 0
        text: "Selected connection cannot be fully drawn: " + view.diagramGap(view.selected)
            + ". The mapping remains listed; missing geometry does not remove or remap it."
        textFormat: Text.PlainText
        wrapMode: Text.WordWrap
        color: "#e57474"
    }
    Button {
        visible: view.diagramGaps.length > 0
        text: "Next undrawn connection (" + view.diagramGaps.length + ")"
        onClicked: {
            const indices = []
            view.rows.forEach((row, index) => { if (view.diagramGap(row)) indices.push(index) })
            if (!indices.length) return
            const next = indices.find(index => index > view.selectedIndex)
            view.selectedIndex = next === undefined ? indices[0] : next
        }
        Accessible.description: "Highlight the next assignment with an unassigned source or missing or ambiguous diagram geometry."
    }
    Label {
        Layout.fillWidth: true
        visible: view.gapReason(view.selected).length > 0
        text: "Needs calibration: " + view.gapReason(view.selected)
            + ". Repair the saved controller calibration or choose another controller. A drawn connection does not prove a measured native input."
        textFormat: Text.PlainText
        wrapMode: Text.WordWrap
        color: "#e57474"
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: view.selected ? view.selected.reason + (view.selected.output ? " · Emulator output: " + view.selected.output : "")
            : view.focusedSourceControl ? "Selected source control: " + view.focusedSourceControl + ". No assignment in this view."
            : view.rows.length ? "Choose a mapped connection." : "No mapping rows available."
    }
    CheckBox {
        id: nativeDetails
        visible: view.nativeRoutes.length > 0
        text: "Show technical native-field details"
        checked: false
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        visible: nativeDetails.checked && text.length > 0
        textFormat: Text.PlainText
        text: view.selected ? view.nativeRoutes.filter(route => route.target_id === view.selected.target_id).map(route => {
            const assignment = route.assignment
            const field = assignment.field
            const mode = assignment.output ? "button " + (assignment.sequence || "standard") + " · " + assignment.output + (route.explicit ? " · explicit override" : " · default route") : assignment.controller_aim ? "controller aim" : assignment.relative_velocity ? "stick velocity" : "absolute axis"
            const state = route.analog_state
            const behavior = !field.analog ? "" : !state ? "\nNative analog behavior: not captured; reinspect for details." : "\nNative analog behavior: key step " + state.keydelta + (state.keydelta === 0 ? " (single-step press behavior)" : "") + " · center step " + (state.centerdelta === null ? "not applicable/exposed" : state.centerdelta) + " · sensitivity " + state.sensitivity + "% · reverse " + state.reverse + " · reset " + state.reset + " · wraps " + state.wraps
            return "Native field: " + (route.label ? route.label + " · " : "") + field.tag + " / " + field.input_type + " · mask " + field.mask + " · default " + field.defvalue + " · " + mode + (assignment.output ? "" : " · range " + (assignment.stick_range || "full")) + behavior
        }).join("\n") : ""
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        text: "Click a control or Tab to it and press Space to highlight its assignment. Activate a shared control repeatedly to cycle through its connections. You can also choose an assignment from the list. Schematics show catalog geometry; they are not product photographs or runtime verification."
        opacity: 0.7
    }
    }
}
