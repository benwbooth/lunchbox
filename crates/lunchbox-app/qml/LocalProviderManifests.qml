import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property var manifestModel
    property color ink: "#edf3f8"
    property color muted: "#91a0b4"
    property color line: "#273244"
    property color panel: "#111924"
    property color accent: "#f4a442"
    property color accentCool: "#62d7c6"
    property int pendingRemoveIndex: -1

    Layout.fillWidth: true
    spacing: 9

    RowLayout {
        Layout.fillWidth: true
        spacing: 12

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                text: "LOCAL PROVIDER CATALOGS"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            Text {
                Layout.fillWidth: true
                text: "Index user-managed local .torrent collections in bulk. Lunchbox validates metadata and makes exact game payloads discoverable; importing a manifest never starts a download."
                color: root.muted
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
        }

        Button {
            objectName: "localProviderImport"
            Layout.preferredWidth: 156
            text: "Import manifest…"
            enabled: !root.manifestModel.busy
            onClicked: root.manifestModel.choose_and_import()
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: 42
        radius: 8
        color: Qt.rgba(1, 1, 1, 0.025)
        border.color: root.line
        border.width: 1

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            spacing: 10
            BusyIndicator {
                Layout.preferredWidth: 22
                Layout.preferredHeight: 22
                running: root.manifestModel.busy
                visible: running
            }
            Text {
                Layout.fillWidth: true
                text: root.manifestModel.message
                color: root.muted
                font.pixelSize: 10
                elide: Text.ElideMiddle
            }
            Text {
                visible: root.manifestModel.entry_count > 0
                text: root.manifestModel.entry_count + " PROVIDER"
                      + (root.manifestModel.entry_count === 1 ? "" : "S")
                color: root.accentCool
                font.pixelSize: 9
                font.weight: Font.Bold
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: root.manifestModel.entry_count > 0 ? 252 : 100
        radius: 9
        color: root.panel
        border.color: root.line
        border.width: 1

        ListView {
            id: providerList
            objectName: "localProviderList"
            anchors.fill: parent
            anchors.margins: 7
            clip: true
            spacing: 5
            boundsBehavior: Flickable.StopAtBounds
            model: {
                root.manifestModel.revision
                return root.manifestModel.entry_count
            }
            ScrollBar.vertical: ScrollBar { }

            delegate: Rectangle {
                id: providerRow
                required property int index
                readonly property bool available: root.manifestModel.file_available_at(index)
                width: providerList.width - 10
                height: 70
                radius: 7
                color: "#151e29"
                border.color: available ? root.line : "#9b583f"
                border.width: 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 8
                    spacing: 9

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 7
                            Text {
                                Layout.fillWidth: true
                                text: root.manifestModel.name_at(providerRow.index)
                                color: root.ink
                                font.pixelSize: 11
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                text: root.manifestModel.offer_count_at(providerRow.index)
                                      + " CATALOG"
                                      + (root.manifestModel.offer_count_at(providerRow.index) === 1
                                         ? "" : "S")
                                color: root.accentCool
                                font.pixelSize: 8
                                font.weight: Font.Bold
                            }
                        }
                        Text {
                            Layout.fillWidth: true
                            text: root.manifestModel.provider_id_at(providerRow.index)
                                  + "  ·  version "
                                  + root.manifestModel.version_at(providerRow.index)
                            color: root.muted
                            font.pixelSize: 9
                            elide: Text.ElideRight
                        }
                        Text {
                            id: manifestPath
                            Layout.fillWidth: true
                            text: providerRow.available
                                  ? root.manifestModel.path_at(providerRow.index)
                                  : "Manifest file is unavailable · choose Resync after restoring it"
                            color: providerRow.available ? "#6f7e91" : "#ff9b91"
                            font.pixelSize: 8
                            elide: Text.ElideMiddle
                            ToolTip.visible: pathHover.hovered
                            ToolTip.text: root.manifestModel.path_at(providerRow.index)
                            HoverHandler { id: pathHover }
                        }
                    }

                    Button {
                        Layout.preferredWidth: 62
                        Layout.preferredHeight: 32
                        text: "Terms"
                        visible: root.manifestModel.terms_url_at(providerRow.index).length > 0
                        enabled: visible
                        onClicked: Qt.openUrlExternally(
                                       root.manifestModel.terms_url_at(providerRow.index))
                    }
                    Button {
                        objectName: "localProviderResync-" + providerRow.index
                        Layout.preferredWidth: 74
                        Layout.preferredHeight: 32
                        text: "Resync"
                        enabled: providerRow.available && !root.manifestModel.busy
                        onClicked: root.manifestModel.resync_at(providerRow.index)
                    }
                    Button {
                        objectName: "localProviderRemove-" + providerRow.index
                        Layout.preferredWidth: 74
                        Layout.preferredHeight: 32
                        text: "Remove"
                        enabled: !root.manifestModel.busy
                        onClicked: {
                            root.pendingRemoveIndex = providerRow.index
                            removeDialog.open()
                        }
                    }
                }
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 36, 620)
            spacing: 6
            visible: root.manifestModel.entry_count === 0
                     && !root.manifestModel.busy
            Text {
                width: parent.width
                text: "NO PROVIDER CATALOGS INSTALLED"
                color: root.ink
                font.pixelSize: 11
                font.weight: Font.Bold
                horizontalAlignment: Text.AlignHCenter
            }
            Text {
                width: parent.width
                text: "A manifest is a small versioned JSON index beside your .torrent files. Import one above; individual games remain reviewable before download."
                color: root.muted
                font.pixelSize: 10
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }

        DropArea {
            anchors.fill: parent
            enabled: !root.manifestModel.busy
            onDropped: function(drop) {
                if (drop.urls.length === 1)
                    root.manifestModel.import_path(drop.urls[0])
            }
        }
    }

    Text {
        Layout.fillWidth: true
        text: "The manifest and .torrent files remain in your custody. Removing a provider deletes only Lunchbox's catalog index; it never deletes torrent files, qBittorrent jobs, or ROMs."
        color: root.muted
        font.pixelSize: 9
        wrapMode: Text.WordWrap
    }

    Dialog {
        id: removeDialog
        objectName: "localProviderRemoveDialog"
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(520, parent ? parent.width - 40 : 520)
        modal: true
        title: "Remove provider catalog?"
        standardButtons: Dialog.Cancel | Dialog.Ok

        contentItem: Text {
            width: removeDialog.availableWidth
            text: root.pendingRemoveIndex >= 0
                  ? "Remove " + root.manifestModel.name_at(root.pendingRemoveIndex)
                    + " from Lunchbox? Its manifest, torrent files, qBittorrent jobs, and downloaded games will remain untouched."
                  : "Remove this provider from Lunchbox?"
            color: root.ink
            font.pixelSize: 11
            wrapMode: Text.WordWrap
        }

        onAccepted: {
            if (root.pendingRemoveIndex >= 0)
                root.manifestModel.remove_at(root.pendingRemoveIndex)
            root.pendingRemoveIndex = -1
        }
        onRejected: root.pendingRemoveIndex = -1
    }
}
