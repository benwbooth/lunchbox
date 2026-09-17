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
    // Hovered wire (list row or diagram hotspot). While set, the canvas
    // isolates that single connection circuit-style; clicks pin it via
    // selectedIndex. -1 restores the normal all/selected rendering.
    property int hoveredIndex: -1
    onHoveredIndexChanged: connections.requestPaint()
    property string focusedSourceControl: ""
    readonly property var selected: rows.length && selectedIndex >= 0 ? rows[Math.min(selectedIndex, rows.length - 1)] : null
    onRowsChanged: { focusedSourceControl = ""; selectedIndex = 0; hoveredIndex = -1; connections.requestPaint() }
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
    function escTooltip(text) {
        return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }
    function controlTooltip(side, control) {
        const SOURCE = "#ffb454"
        const DEST = "#62dac8"
        const DIM = "#8a93a3"
        const BAD = "#e57474"
        const paint = (text, color) => "<font color=\"" + color + "\"><b>" + escTooltip(text) + "</b></font>"
        const matches = rows.filter(row => rowMatchesControl(row, side, control.id))
        const owner = side === 0 ? sourceOwner(control.id) : control.id
        const repeat = side === 0 && owner !== control.id
            ? "<br><font color=\"" + DIM + "\">Hardware repeat of " + escTooltip(sourceLayout.controls.find(entry => entry.id === owner).label) + "; shares its input.</font>"
            : ""
        const plain = text => String(text).replace(/<[^>]*>/g, "")
        if (!matches.length) {
            const label = paint(control.label, side === 0 ? SOURCE : DEST)
            return { rich: label + "<br><font color=\"" + DIM + "\">No assignment in this view</font>" + repeat,
                     plain: plain(label) + "\nNo assignment in this view" }
        }
        const lines = matches.map(row => {
            const from = paint(row.physical_id ? row.physical : "—", SOURCE)
            const to = paint(targetLabel(row), DEST)
            let line = side === 0 ? from + " drives " + to : to + " driven by " + from
            if (row.output) line += " <font color=\"" + DIM + "\">[emulator: " + escTooltip(row.output) + "]</font>"
            if (!row.physical_id) line += " <font color=\"" + BAD + "\">· UNMAPPED</font>"
            if (gapReason(row)) line += " <font color=\"" + BAD + "\">· NEEDS CALIBRATION: " + escTooltip(gapReason(row)) + "</font>"
            return line
        })
        const tail = matches.length > 1
            ? "<br><font color=\"" + DIM + "\">Click repeatedly to cycle through these assignments.</font>"
            : ""
        const rich = lines.join("<br>") + tail + repeat
        return { rich: rich, plain: plain(rich).replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">") }
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
    // Flowchart lane for the wire at sort position `order` of `count`
    // wires across the channel [x0, x1]. Distinct lanes keep vertical
    // trunks from overlapping; deterministic in row order.
    function laneXFor(order, count, x0, x1) {
        if (count <= 1) return (x0 + x1) / 2
        const lo = Math.min(x0, x1) + 14
        const hi = Math.max(x0, x1) - 14
        if (hi <= lo) return (x0 + x1) / 2
        return lo + (hi - lo) * order / (count - 1)
    }
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
                            // Smaller than the drawn control so dense clusters
                            // (C buttons sit ~26px apart) stop flickering.
                            width: Math.max(16, panel.width * 40 / 900)
                            height: Math.max(16, stage.diagramHeight * 40 / 500)
                            hoverEnabled: true
                            activeFocusOnTab: true
                            Accessible.role: Accessible.Button
                            Accessible.name: (panel.index === 0 ? "Source: " : "Destination: ") + modelData.label
                            Accessible.description: view.controlTooltip(panel.index, modelData).plain
                            Accessible.onPressAction: controlHotspot.clicked()
                            background: Rectangle {
                                color: "transparent"
                                radius: 4
                                border.width: controlHotspot.activeFocus || view.controlHasGap(panel.index, controlHotspot.modelData.id) ? 2 : 0
                                border.color: controlHotspot.activeFocus ? "#ffb454" : "#e57474"
                            }
                            onClicked: { view.chooseControl(panel.index, modelData.id); view.controlActivated(panel.index, modelData.id) }
                            onHoveredChanged: {
                                if (!hovered) {
                                    // Clear only when the hover still belongs
                                    // to this control: a neighbor hotspot may
                                    // have claimed it first, and overlapping
                                    // hotspots must not steal it back and
                                    // cause flicker.
                                    const current = view.hoveredIndex >= 0
                                        && view.hoveredIndex < view.rows.length
                                        ? view.rows[view.hoveredIndex] : null
                                    if (current && view.rowMatchesControl(current, panel.index, modelData.id))
                                        view.hoveredIndex = -1
                                    return
                                }
                                view.hoveredIndex = view.rows.findIndex(
                                    row => view.rowMatchesControl(row, panel.index, modelData.id))
                            }
                            ToolTip {
                                visible: controlHotspot.hovered || controlHotspot.activeFocus
                                text: view.controlTooltip(panel.index, controlHotspot.modelData).plain
                                contentItem: Text {
                                    // Constrain the popup so long mappings
                                    // wrap inside it instead of overflowing.
                                    width: Math.min(implicitWidth, 420)
                                    wrapMode: Text.Wrap
                                    text: view.controlTooltip(panel.index, controlHotspot.modelData).rich
                                    textFormat: Text.RichText
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
                ctx.lineJoin = "round"
                ctx.lineCap = "round"
                function endpoints(row) {
                    if (!row || !row.physical_id) return null
                    const from = view.point(0, row.physical_id)
                    const to = view.point(1, row.target_id)
                    if (!from || !to) return null
                    return {from: from, to: to}
                }
                // Flowchart routing: exit horizontally, share no vertical
                // trunk (one lane per wire, ordered by midpoint), enter
                // horizontally. Reads as a circuit, not a nest.
                function traceWire(row, laneX, laneY) {
                    const ends = endpoints(row)
                    if (!ends) return
                    ctx.beginPath()
                    ctx.moveTo(ends.from.x, ends.from.y)
                    if (stage.stacked) {
                        ctx.lineTo(ends.from.x, laneY)
                        ctx.lineTo(ends.to.x, laneY)
                    } else {
                        ctx.lineTo(laneX, ends.from.y)
                        ctx.lineTo(laneX, ends.to.y)
                    }
                    ctx.lineTo(ends.to.x, ends.to.y)
                    ctx.stroke()
                }
                function dotAt(point, radius) {
                    ctx.beginPath(); ctx.arc(point.x, point.y, radius, 0, Math.PI * 2); ctx.fill()
                }
                const order = view.rows
                    .map((row, index) => ({row: row, index: index}))
                    .filter(entry => endpoints(entry.row))
                    .sort((a, b) => {
                        const ay = (endpoints(a.row).from.y + endpoints(a.row).to.y) / 2
                        const by = (endpoints(b.row).from.y + endpoints(b.row).to.y) / 2
                        return ay - by || a.index - b.index
                    })
                const leftPanel = diagrams.itemAt(0)
                const rightPanel = diagrams.itemAt(1)
                const lanes = order.map((entry, position) => {
                    let lane = 0
                    if (stage.stacked && leftPanel && rightPanel) {
                        const y0 = leftPanel.y + leftPanel.height
                        const y1 = rightPanel.y
                        lane = view.laneXFor(position, order.length, y0, y1)
                    } else if (leftPanel && rightPanel) {
                        lane = view.laneXFor(position, order.length,
                                             leftPanel.x + leftPanel.width, rightPanel.x)
                    }
                    return {entry: entry, lane: lane}
                })
                const focus = view.hoveredIndex >= 0 && view.hoveredIndex < view.rows.length
                    ? view.hoveredIndex
                    : (view.selectedIndex >= 0 && view.selectedIndex < view.rows.length
                       ? view.selectedIndex : -1)
                ctx.strokeStyle = "#5b6b7c"
                ctx.globalAlpha = 0.45
                ctx.lineWidth = 1.2
                for (const item of lanes) {
                    if (item.entry.index === focus) continue
                    traceWire(item.entry.row, item.lane, item.lane)
                    const ends = endpoints(item.entry.row)
                    if (ends) { ctx.fillStyle = ctx.strokeStyle; dotAt(ends.from, 3); dotAt(ends.to, 3) }
                }
                if (focus >= 0) {
                    const item = lanes.find(item => item.entry.index === focus)
                    if (item) {
                        ctx.strokeStyle = view.gapReason(view.rows[focus]) ? "#e57474" : "#ffb454"
                        ctx.globalAlpha = 1
                        ctx.lineWidth = 3
                        traceWire(item.entry.row, item.lane, item.lane)
                        const ends = endpoints(item.entry.row)
                        if (ends) { ctx.fillStyle = ctx.strokeStyle; dotAt(ends.from, 5); dotAt(ends.to, 5) }
                    }
                }
                ctx.globalAlpha = 1
            }
        }
    }
    Label {
        Layout.fillWidth: true
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        text: {
            const assigned = view.rows.filter(row => !!row.physical_id).length
            return view.rows.length ? "Displayed assignments: " + assigned + "/" + view.rows.length
                + " have a source · " + (view.rows.length - assigned) + " unmapped."
                + " Hover a control to isolate its wire; click to pin it. This counts only this view, not whole-game coverage or runtime readiness."
                : "No assignments in this view."
        }
    }
    ColumnLayout {
        visible: !view.simple
        Layout.fillWidth: true
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
        text: "Click a control or Tab to it and press Space to highlight its assignment. Hover any control to isolate its wire. Activate a shared control repeatedly to cycle through its connections. Schematics show catalog geometry; they are not product photographs or runtime verification."
        opacity: 0.7
    }
    }
}
