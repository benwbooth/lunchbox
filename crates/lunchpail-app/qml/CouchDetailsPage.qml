pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// A ten-foot presentation, not a second desktop sidebar. Management tools
// open on demand; the library and its selected card remain underneath.
Rectangle {
    id: page
    required property var details
    property string gameTitle: ""
    property string platform: ""
    property url coverUrl: ""
    property string primaryAction: "Play"
    property bool favorite: false
    property bool ready: false
    property color background: "#101620"
    property color panel: "#182230"
    property color ink: "#f4f7fb"
    property color muted: "#95a2b6"
    property color accent: "#62dac8"
    property int tabIndex: 0
    property int navigationArea: 0 // action rail, tabs, content
    property int actionIndex: 0
    property int toolIndex: 0
    readonly property var tabs: ["Overview", "Play & setup", "Media", "Activity"]
    readonly property var tools: tabIndex === 1 ? [
        { label: "Display & save states", hint: "Display shaders, bezels and automatic resume", key: "display" },
        { label: "Controllers", hint: "Players and button mappings", key: "controllers" },
        { label: "Translations & mods", hint: "Community patches and cheats", key: "mods" },
        { label: "RetroAchievements", hint: "Achievement mode and account", key: "achievements" },
        { label: "ROMs & save files", hint: "Versions, backups and uninstall", key: "files" },
        { label: "Advanced game tools", hint: "Every remaining game option", key: "advanced" }
    ] : tabIndex === 2 ? [
        { label: "View artwork", hint: "Full-size artwork gallery", key: "artwork" },
        { label: "Play video", hint: "Gameplay preview", key: "video" },
        { label: "Find better media", hint: "Choose replacement artwork", key: "find-media" },
        { label: "Manual & music", hint: "Cached media and downloads", key: "media" }
    ] : []
    signal closeRequested()
    signal primaryRequested()
    signal favoriteRequested()
    signal versionsRequested()
    signal manageRequested(string section)

    objectName: "couchDetailsPage"
    color: background
    function reset() { tabIndex = 0; navigationArea = 0; actionIndex = 0; toolIndex = 0; overview.contentY = 0 }
    function chooseTab(index) { tabIndex = Math.max(0, Math.min(tabs.length - 1, index)); toolIndex = 0; overview.contentY = 0 }
    function activateRail() {
        if (actionIndex === 0) primaryRequested()
        else if (actionIndex === 1) favoriteRequested()
        else if (ready) versionsRequested()
    }
    function handleNavigation(action) {
        if (action === "back") closeRequested()
        else if (action === "favorite") favoriteRequested()
        else if (action === "page_left" || action === "page_right") {
            chooseTab((tabIndex + (action === "page_left" ? tabs.length - 1 : 1)) % tabs.length)
            navigationArea = 1
        } else if (action === "left") {
            if (navigationArea === 1 && tabIndex > 0) chooseTab(tabIndex - 1)
            else if (navigationArea === 2 && tools.length && toolIndex % 2 === 1) toolIndex--
            else navigationArea = 0
        } else if (action === "right") {
            if (navigationArea === 0) navigationArea = 1
            else if (navigationArea === 1) chooseTab(Math.min(tabs.length - 1, tabIndex + 1))
            else if (tools.length) toolIndex = Math.min(tools.length - 1, toolIndex + 1)
        } else if (action === "up" || action === "down") {
            const step = action === "up" ? -1 : 1
            if (navigationArea === 0) actionIndex = Math.max(0, Math.min(2, actionIndex + step))
            else if (navigationArea === 1) { if (step > 0) navigationArea = 2 }
            else if (tools.length) {
                if (step < 0 && toolIndex < 2) navigationArea = 1
                else toolIndex = Math.max(0, Math.min(tools.length - 1, toolIndex + step * 2))
            } else if (step < 0 && overview.contentY <= 0) navigationArea = 1
            else overview.contentY = Math.max(0, Math.min(Math.max(0, overview.contentHeight - overview.height), overview.contentY + step * 110))
        } else if (action === "accept") {
            if (navigationArea === 0) activateRail()
            else if (navigationArea === 1) navigationArea = 2
            else if (tools.length && ready) manageRequested(tools[toolIndex].key)
            else if (tabIndex === 3 && ready) manageRequested("activity")
        } else if (action === "home") reset()
        else return false
        return true
    }

    RowLayout {
        id: header
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 38 }
        spacing: 24
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 7
            Text { text: page.platform; color: page.accent; font.pixelSize: 15; font.weight: Font.DemiBold }
            Text {
                Layout.fillWidth: true
                text: page.gameTitle; color: page.ink; font.pixelSize: 36; font.weight: Font.Bold
                minimumPixelSize: 22; fontSizeMode: Text.Fit; maximumLineCount: 2; wrapMode: Text.WordWrap
            }
        }
        LbRoundButton { text: "×"; implicitWidth: 48; implicitHeight: 48; Accessible.name: "Back to games"; onClicked: page.closeRequested() }
    }
    RowLayout {
        anchors { left: parent.left; right: parent.right; top: header.bottom; bottom: help.top; margins: 38 }
        spacing: 42
        ColumnLayout {
            id: rail
            Layout.preferredWidth: Math.min(340, page.width * 0.28)
            Layout.fillHeight: true
            spacing: 12
            Item {
                Layout.fillWidth: true; Layout.fillHeight: true
                Image {
                    id: cover
                    anchors.fill: parent; source: page.coverUrl; fillMode: Image.PreserveAspectFit
                    asynchronous: true; sourceSize: Qt.size(480, 680); cache: true
                }
                Text { anchors.centerIn: parent; visible: cover.status !== Image.Ready; text: page.gameTitle.charAt(0); color: page.muted; font.pixelSize: 96 }
            }
            Repeater {
                model: [page.primaryAction, page.favorite ? "Remove favorite" : "Add favorite", "Other releases"]
                delegate: LbButton {
                    required property int index
                    required property string modelData
                    objectName: "couchDetailsAction" + index
                    Layout.fillWidth: true; Layout.preferredHeight: 54
                    text: modelData
                    highlighted: page.navigationArea === 0 && page.actionIndex === index
                    positive: index === 0 && page.primaryAction === "Play"
                    contentItem: LbButtonLabel { control: parent; pixelSize: 18 }
                    enabled: index !== 2 || (page.ready && page.details.variant_count > 1)
                    onClicked: { page.navigationArea = 0; page.actionIndex = index; page.activateRail() }
                }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true; Layout.fillHeight: true
            spacing: 22
            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: page.tabs
                    delegate: LbButton {
                        required property string modelData
                        required property int index
                        objectName: "couchDetailsTab" + index
                        Layout.fillWidth: true; Layout.preferredHeight: 48
                        text: modelData; highlighted: page.tabIndex === index
                        contentItem: LbButtonLabel { control: parent; pixelSize: 17 }
                        onClicked: { page.chooseTab(index); page.navigationArea = 1 }
                    }
                }
            }
            Item {
                Layout.fillWidth: true; Layout.fillHeight: true
                MomentumFlickable {
                    id: overview
                    anchors.fill: parent; clip: true
                    visible: page.tabIndex === 0 || page.tabIndex === 3
                    contentWidth: width; contentHeight: copy.implicitHeight
                    ScrollBar.vertical: LbScrollBar { policy: ScrollBar.AsNeeded }
                    Column {
                        id: copy
                        width: overview.width - 20; spacing: 24
                        Text {
                            width: parent.width
                            text: !page.ready ? "Loading game information…" : page.tabIndex === 0
                                  ? (page.details.description || "No description is available for this release.")
                                  : page.details.activity_visible ? page.details.play_count + " plays · " + page.details.play_time : "You haven’t played this game yet."
                            color: page.ink; font.pixelSize: 21; lineHeight: 1.4; wrapMode: Text.WordWrap
                        }
                        Flow {
                            width: parent.width; spacing: 18
                            Repeater {
                                model: !page.ready ? [] : page.tabIndex === 0
                                    ? [page.details.release_date, page.details.genre, page.details.developer, page.details.players ? page.details.players + " players" : "", page.details.rating ? "★ " + page.details.rating : ""]
                                    : [page.details.last_played ? "Last played " + page.details.last_played : "", (page.details.completion_state || "").replace(/-/g, " ")]
                                delegate: Text { required property var modelData; visible: !!modelData; text: modelData || ""; color: page.muted; font.pixelSize: 17; width: Math.min(implicitWidth, copy.width); wrapMode: Text.WordWrap }
                            }
                        }
                        Text {
                            width: parent.width; visible: page.tabIndex === 3 && page.ready && !!page.details.notes
                            text: page.details.notes || ""; color: page.muted; font.pixelSize: 19; wrapMode: Text.WordWrap
                        }
                        LbButton {
                            visible: page.tabIndex === 3; width: parent.width; height: 52
                            text: "View play sessions"; enabled: page.ready
                            onClicked: page.manageRequested("activity")
                        }
                    }
                }
                GridLayout {
                    anchors { left: parent.left; right: parent.right; top: parent.top }
                    visible: page.tools.length > 0
                    columns: 2; columnSpacing: 16; rowSpacing: 16
                    Repeater {
                        model: page.tools
                        delegate: LbButton {
                            required property int index
                            required property var modelData
                            objectName: "couchDetailsTool" + index
                            Layout.fillWidth: true; Layout.preferredHeight: 118
                            highlighted: page.navigationArea === 2 && page.toolIndex === index
                            enabled: page.ready
                            text: modelData.label
                            contentItem: Item {
                                Column {
                                    anchors.centerIn: parent
                                    width: parent.width
                                    spacing: 10
                                    Text { width: parent.width; text: parent.parent.parent.modelData.label; color: page.ink; font.pixelSize: 20; minimumPixelSize: 16; fontSizeMode: Text.Fit; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                                    Text { width: parent.width; text: parent.parent.parent.modelData.hint; color: page.muted; font.pixelSize: 15; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap }
                                }
                            }
                            onClicked: { page.navigationArea = 2; page.toolIndex = index; page.manageRequested(modelData.key) }
                        }
                    }
                }
            }
        }
    }
    Text {
        id: help
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: 26 }
        text: "D-pad / arrows  Navigate     A / Enter  Select     LB / RB  Change tab     B / Escape  Back"
        color: page.muted; font.pixelSize: 14; horizontalAlignment: Text.AlignHCenter
    }
}
