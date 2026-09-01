import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root
    objectName: "bulkMetadataEditor"

    required property var metadataModel
    property int visibleGameCount: 0
    property string scopeLabel: "Current library view"
    property color ink: "#edf3f8"
    property color muted: "#91a0b4"
    property color line: "#273244"
    property color panel: "#111924"
    property color panelRaised: "#17212d"
    property color accent: "#f4a442"
    property color accentCool: "#62d7c6"
    property bool reviewing: false
    property bool submitted: false
    readonly property bool restoreMode: actionCombo.currentValue === "restore"
    readonly property string fieldKey: fieldCombo.currentValue || "developer"
    readonly property string fieldLabel: fieldCombo.currentText || "Developer"
    readonly property bool canReview: visibleGameCount > 0
                                           && !metadataModel.bulk_metadata_busy
                                           && (restoreMode
                                               || valueField.text.trim().length > 0)
    readonly property var fieldChoices: [
        { key: "developer", label: "Developer" },
        { key: "publisher", label: "Publisher" },
        { key: "genre", label: "Genre" },
        { key: "series", label: "Series" },
        { key: "play_mode", label: "Play mode" },
        { key: "region", label: "Region" },
        { key: "release_type", label: "Release type" },
        { key: "release_status", label: "Release status" },
        { key: "age_rating", label: "Age rating" },
        { key: "players", label: "Players" },
        { key: "version", label: "Version" }
    ]
    readonly property var actionChoices: [
        { key: "set", label: "Set one value" },
        { key: "restore", label: "Restore catalog value" }
    ]

    function openForScope() {
        reviewing = false
        submitted = false
        fieldCombo.currentIndex = 0
        actionCombo.currentIndex = 0
        valueField.clear()
        open()
        valueField.forceActiveFocus()
    }

    modal: true
    focus: true
    width: Math.min(720, parent ? parent.width - 48 : 720)
    height: Math.min(620, parent ? parent.height - 48 : 620)
    padding: 22
    closePolicy: metadataModel.bulk_metadata_busy ? Popup.NoAutoClose
                                                  : Popup.CloseOnEscape

    background: Rectangle {
        color: root.panel
        radius: 14
        border.color: root.line
        border.width: 1
    }

    header: Rectangle {
        implicitHeight: 74
        color: root.panelRaised
        radius: 14

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 22
            anchors.right: parent.right
            anchors.rightMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
                width: parent.width
                text: "BULK EDIT METADATA"
                color: root.ink
                font.pixelSize: 17
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            Text {
                width: parent.width
                text: root.scopeLabel + "  ·  "
                      + root.visibleGameCount.toLocaleString() + " exact game "
                      + (root.visibleGameCount === 1 ? "record" : "records")
                color: root.muted
                font.pixelSize: 10
                elide: Text.ElideRight
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: 14

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 66
            radius: 10
            color: Qt.rgba(root.accentCool.r, root.accentCool.g,
                           root.accentCool.b, 0.07)
            border.color: Qt.rgba(root.accentCool.r, root.accentCool.g,
                                  root.accentCool.b, 0.36)

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                spacing: 11
                Text {
                    text: "✓"
                    color: root.accentCool
                    font.pixelSize: 20
                    font.weight: Font.Bold
                }
                Text {
                    Layout.fillWidth: true
                    text: "Identity-safe: the current filtered game IDs are snapshotted only when Apply is clicked. Titles are never fuzzy-matched."
                    color: root.ink
                    font.pixelSize: 11
                    lineHeight: 1.25
                    wrapMode: Text.WordWrap
                }
            }
        }

        ColumnLayout {
            visible: !root.reviewing
            Layout.fillWidth: true
            spacing: 12

            Text {
                text: "FIELD"
                color: root.accent
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            ComboBox {
                id: fieldCombo
                objectName: "bulkMetadataField"
                Layout.fillWidth: true
                Layout.preferredHeight: 42
                model: root.fieldChoices
                textRole: "label"
                valueRole: "key"
                onActivated: root.submitted = false
            }

            Text {
                text: "CHANGE"
                color: root.accent
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            ComboBox {
                id: actionCombo
                objectName: "bulkMetadataAction"
                Layout.fillWidth: true
                Layout.preferredHeight: 42
                model: root.actionChoices
                textRole: "label"
                valueRole: "key"
                onActivated: root.submitted = false
            }

            Text {
                visible: !root.restoreMode
                text: "VALUE"
                color: root.accent
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            TextField {
                id: valueField
                objectName: "bulkMetadataValue"
                visible: !root.restoreMode
                Layout.fillWidth: true
                Layout.preferredHeight: 44
                maximumLength: root.fieldKey === "players" ? 100
                               : root.fieldKey === "age_rating" ? 100
                               : root.fieldKey === "genre"
                                 || root.fieldKey === "series"
                                 || root.fieldKey === "developer"
                                 || root.fieldKey === "publisher" ? 1000 : 500
                placeholderText: "Enter the value for every visible game"
                selectByMouse: true
                onTextEdited: root.submitted = false
                onAccepted: {
                    if (root.canReview)
                        root.reviewing = true
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.restoreMode
                      ? "Only the " + root.fieldLabel.toLowerCase()
                        + " override will be removed. Every other personal edit stays intact."
                      : "This creates one personal " + root.fieldLabel.toLowerCase()
                        + " override for each visible game. Canonical catalog records are not rewritten."
                color: root.muted
                font.pixelSize: 11
                lineHeight: 1.3
                wrapMode: Text.WordWrap
            }
        }

        ColumnLayout {
            visible: root.reviewing
            Layout.fillWidth: true
            spacing: 12

            Text {
                text: "REVIEW CHANGE"
                color: root.accent
                font.pixelSize: 10
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 126
                radius: 10
                color: root.panelRaised
                border.color: root.line

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 14
                    spacing: 7
                    Text {
                        Layout.fillWidth: true
                        text: root.restoreMode
                              ? "Restore catalog " + root.fieldLabel.toLowerCase()
                              : "Set " + root.fieldLabel.toLowerCase() + " to ‘"
                                + valueField.text.trim() + "’"
                        color: root.ink
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                        wrapMode: Text.WordWrap
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Scope: " + root.visibleGameCount.toLocaleString()
                              + " visible game "
                              + (root.visibleGameCount === 1 ? "record" : "records")
                              + " in " + root.scopeLabel
                        color: root.muted
                        font.pixelSize: 11
                        wrapMode: Text.WordWrap
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "The transaction either saves every exact record or saves none."
                        color: root.accentCool
                        font.pixelSize: 10
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            radius: 9
            color: Qt.rgba(1, 1, 1, 0.025)
            border.color: root.line

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 10
                BusyIndicator {
                    Layout.preferredWidth: 24
                    Layout.preferredHeight: 24
                    running: root.metadataModel.bulk_metadata_busy
                    visible: running
                }
                Text {
                    id: operationMessage
                    objectName: "bulkMetadataMessage"
                    Layout.fillWidth: true
                    text: root.metadataModel.bulk_metadata_message
                    color: root.submitted ? root.accentCool : root.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }
        }
    }

    footer: Rectangle {
        implicitHeight: 72
        color: "transparent"
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 1
            color: root.line
        }
        Row {
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 9
            HeaderButton {
                objectName: "bulkMetadataCancel"
                width: 104
                text: root.reviewing ? "Back" : "Cancel"
                enabled: !root.metadataModel.bulk_metadata_busy
                onClicked: {
                    if (root.reviewing) {
                        root.reviewing = false
                    } else {
                        root.close()
                    }
                }
            }
            HeaderButton {
                objectName: "bulkMetadataReview"
                width: 138
                visible: !root.reviewing
                text: "Review change"
                active: true
                enabled: root.canReview
                onClicked: root.reviewing = true
            }
            HeaderButton {
                objectName: "bulkMetadataApply"
                width: 138
                visible: root.reviewing
                text: root.metadataModel.bulk_metadata_busy ? "Applying…" : "Apply to view"
                active: true
                enabled: root.canReview
                onClicked: {
                    root.submitted = true
                    root.metadataModel.apply_bulk_metadata(
                                root.fieldKey, valueField.text, root.restoreMode)
                }
            }
        }
    }
}
