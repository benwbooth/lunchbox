pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: dialog
    objectName: "gameFileIdentityDialog"

    required property var identityModel
    property color ink: "#edf3f8"
    property color muted: "#8e9bad"
    property color panel: "#111822"
    property color panelRaised: "#182230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d7c6"
    property bool samePlatformOnly: true
    property bool historyExpanded: false

    function begin(gameUid, title, platform) {
        identityModel.load_game(gameUid, title, platform)
        targetSearch.text = title
        samePlatformOnly = true
        identityModel.search_targets(title, platform)
        open()
        targetSearch.forceActiveFocus()
    }

    modal: true
    anchors.centerIn: parent
    width: Math.min(920, parent ? parent.width - 48 : 920)
    height: Math.min(760, parent ? parent.height - 48 : 760)
    padding: 18
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        radius: 15
        color: dialog.panel
        border.color: dialog.line
    }

    header: Rectangle {
        width: parent.width
        height: 82
        radius: 15
        color: dialog.panelRaised

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 22
            anchors.rightMargin: 18
            spacing: 14

            Rectangle {
                Layout.preferredWidth: 42
                Layout.preferredHeight: 42
                radius: 11
                color: Qt.rgba(dialog.accentCool.r, dialog.accentCool.g,
                               dialog.accentCool.b, 0.12)
                border.color: Qt.rgba(dialog.accentCool.r, dialog.accentCool.g,
                                     dialog.accentCool.b, 0.52)
                Text {
                    anchors.centerIn: parent
                    text: "↔"
                    color: dialog.accentCool
                    font.pixelSize: 18
                    font.weight: Font.Bold
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    text: "ROM IDENTITY"
                    color: dialog.ink
                    font.pixelSize: 19
                    font.weight: Font.Bold
                    font.letterSpacing: 0.7
                }
                Text {
                    Layout.fillWidth: true
                    text: dialog.identityModel.game_title + " · "
                          + dialog.identityModel.game_platform
                    color: dialog.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }
            }

            HeaderButton {
                text: "×"
                implicitWidth: 38
                leftPadding: 0
                rightPadding: 0
                Accessible.name: "Close ROM identity manager"
                onClicked: dialog.close()
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: 12

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: dialog.identityModel.file_count > 1 ? 112 : 82
            radius: 11
            color: "#0d141e"
            border.color: dialog.line

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    Text {
                        Layout.fillWidth: true
                        text: dialog.identityModel.file_count > 1
                              ? "CHOOSE IMPORTED FILE" : "IMPORTED FILE"
                        color: dialog.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        text: dialog.identityModel.file_count + " RECORD"
                              + (dialog.identityModel.file_count === 1 ? "" : "S")
                        color: dialog.accentCool
                        font.pixelSize: 8
                        font.weight: Font.Bold
                    }
                }

                ListView {
                    id: fileList
                    objectName: "identityFileList"
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    orientation: ListView.Horizontal
                    spacing: 7
                    model: dialog.identityModel.file_count
                    currentIndex: dialog.identityModel.selected_file
                    boundsBehavior: Flickable.StopAtBounds
                    ScrollBar.horizontal: ScrollBar {
                        policy: fileList.contentWidth > fileList.width
                                ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
                    }
                    HorizontalWheelHandler { scroller: fileList }

                    delegate: Button {
                        id: fileButton
                        required property int index
                        width: Math.min(360, Math.max(210, fileList.width * 0.47))
                        height: 50
                        text: dialog.identityModel.file_label_at(fileButton.index)
                        onClicked: dialog.identityModel.select_file(fileButton.index)
                        ToolTip.visible: hovered
                        ToolTip.text: dialog.identityModel.file_path_at(fileButton.index)
                        background: Rectangle {
                            radius: 8
                            color: fileButton.index === dialog.identityModel.selected_file
                                   ? "#1b3131" : fileButton.down ? "#253244" : "#17202d"
                            border.color: fileButton.index === dialog.identityModel.selected_file
                                          ? dialog.accentCool : dialog.line
                        }
                        contentItem: Column {
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 2
                            Text {
                                width: parent.width
                                text: fileButton.text
                                color: dialog.ink
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
                                elide: Text.ElideMiddle
                            }
                            Text {
                                width: parent.width
                                text: dialog.identityModel.file_detail_at(fileButton.index)
                                color: dialog.muted
                                font.pixelSize: 8
                                elide: Text.ElideRight
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            ClearableSearchField {
                id: targetSearch
                objectName: "identityTargetSearch"
                Layout.fillWidth: true
                Layout.preferredHeight: 42
                searchIconVisible: true
                leftPadding: 40
                placeholderText: "Search exact catalog game…"
                color: dialog.ink
                placeholderTextColor: dialog.muted
                onTextEdited: searchDelay.restart()
                onAccepted: dialog.identityModel.search_targets(
                                text, dialog.samePlatformOnly
                                      ? dialog.identityModel.game_platform : "")
                onClearRequested: dialog.identityModel.search_targets("", "")
                background: Rectangle {
                    radius: 9
                    color: "#0d141e"
                    border.color: targetSearch.activeFocus
                                  ? dialog.accentCool : dialog.line
                }
            }

            CheckBox {
                id: platformScope
                objectName: "identitySamePlatformOnly"
                text: "THIS PLATFORM"
                checked: dialog.samePlatformOnly
                onToggled: {
                    dialog.samePlatformOnly = checked
                    dialog.identityModel.search_targets(
                                targetSearch.text,
                                checked ? dialog.identityModel.game_platform : "")
                }
                contentItem: Text {
                    leftPadding: platformScope.indicator.width + platformScope.spacing
                    text: platformScope.text
                    color: dialog.ink
                    font.pixelSize: 9
                    font.weight: Font.Bold
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Timer {
            id: searchDelay
            interval: 180
            repeat: false
            onTriggered: dialog.identityModel.search_targets(
                             targetSearch.text,
                             dialog.samePlatformOnly
                                   ? dialog.identityModel.game_platform : "")
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 180
            radius: 11
            color: "#0d141e"
            border.color: dialog.line

            ListView {
                id: candidateList
                objectName: "identityCandidateList"
                anchors.fill: parent
                anchors.margins: 1
                clip: true
                model: dialog.identityModel.candidate_count
                currentIndex: dialog.identityModel.selected_candidate
                boundsBehavior: Flickable.StopAtBounds
                keyNavigationEnabled: true
                ScrollBar.vertical: ScrollBar { policy: ScrollBar.AlwaysOn }
                AcceleratedWheelHandler { scroller: candidateList }

                delegate: ItemDelegate {
                    id: candidateRow
                    required property int index
                    objectName: "identityCandidateRow-" + index
                    width: candidateList.width
                    height: 64
                    onClicked: dialog.identityModel.select_candidate(candidateRow.index)
                    background: Rectangle {
                        color: candidateRow.index === dialog.identityModel.selected_candidate
                               ? "#1b3131" : candidateRow.hovered ? "#151f2c" : "transparent"
                        border.color: candidateRow.index === dialog.identityModel.selected_candidate
                                      ? dialog.accentCool : "#202a39"
                    }
                    contentItem: RowLayout {
                        spacing: 12
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2
                            Text {
                                Layout.fillWidth: true
                                text: dialog.identityModel.candidate_title_at(candidateRow.index)
                                color: dialog.ink
                                font.pixelSize: 12
                                font.weight: Font.DemiBold
                                elide: Text.ElideRight
                            }
                            Text {
                                Layout.fillWidth: true
                                text: dialog.identityModel.candidate_platform_at(candidateRow.index)
                                      + " · "
                                      + dialog.identityModel.candidate_match_at(candidateRow.index)
                                color: dialog.muted
                                font.pixelSize: 9
                                elide: Text.ElideRight
                            }
                        }
                        Text {
                            visible: candidateRow.index
                                     === dialog.identityModel.selected_candidate
                            text: "SELECTED"
                            color: dialog.accentCool
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.8
                        }
                    }
                }

                Text {
                    anchors.centerIn: parent
                    width: parent.width - 48
                    visible: dialog.identityModel.candidate_count === 0
                    text: targetSearch.text.length < 2
                          ? "Search by title to find the exact catalog identity."
                          : "No candidates in this scope. Try all platforms or another title."
                    color: dialog.muted
                    font.pixelSize: 11
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                }
            }
        }

        Rectangle {
            objectName: "identityReviewPanel"
            Layout.fillWidth: true
            Layout.preferredHeight: dialog.identityModel.selected_candidate >= 0 ? 76 : 46
            radius: 10
            color: dialog.identityModel.selected_candidate >= 0 ? "#182a2a" : "#171f2b"
            border.color: dialog.identityModel.selected_candidate >= 0
                          ? "#315c54" : dialog.line

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 12
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text {
                        Layout.fillWidth: true
                        text: dialog.identityModel.selected_candidate >= 0
                              ? "REVIEW EXACT TARGET" : "SELECT A TARGET ABOVE"
                        color: dialog.identityModel.selected_candidate >= 0
                               ? dialog.accentCool : dialog.muted
                        font.pixelSize: 8
                        font.weight: Font.Bold
                        font.letterSpacing: 0.8
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: dialog.identityModel.selected_candidate >= 0
                        text: dialog.identityModel.candidate_title_at(
                                  dialog.identityModel.selected_candidate)
                              + " · "
                              + dialog.identityModel.candidate_platform_at(
                                  dialog.identityModel.selected_candidate)
                        color: dialog.ink
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: dialog.identityModel.selected_candidate >= 0
                        text: "Stable ID: " + dialog.identityModel.candidate_game_uid_at(
                                  dialog.identityModel.selected_candidate)
                        color: dialog.muted
                        font.pixelSize: 8
                        elide: Text.ElideMiddle
                    }
                }
                Button {
                    objectName: "confirmIdentityRelinkButton"
                    Layout.preferredWidth: 154
                    Layout.preferredHeight: 40
                    text: "RELINK ROM"
                    enabled: dialog.identityModel.selected_file >= 0
                             && dialog.identityModel.selected_candidate >= 0
                    onClicked: dialog.identityModel.relink_selected()
                    background: Rectangle {
                        radius: 8
                        color: !parent.enabled ? "#202835"
                               : parent.down ? "#257f71" : "#2da58f"
                        border.color: parent.enabled ? "#63dbc6" : dialog.line
                    }
                    contentItem: Text {
                        text: parent.text
                        color: parent.enabled ? "#071713" : dialog.muted
                        font.pixelSize: 9
                        font.weight: Font.Bold
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }

        Button {
            id: historyToggle
            objectName: "identityHistoryToggle"
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            text: (dialog.historyExpanded ? "▾" : "▸") + " IDENTITY HISTORY · "
                  + dialog.identityModel.history_count
            onClicked: dialog.historyExpanded = !dialog.historyExpanded
            background: Rectangle {
                radius: 8
                color: historyToggle.down ? "#202b3a" : "#171f2b"
                border.color: dialog.line
            }
            contentItem: Text {
                text: parent.text
                color: dialog.muted
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 0.7
                horizontalAlignment: Text.AlignLeft
                verticalAlignment: Text.AlignVCenter
                leftPadding: 12
            }
        }

        ListView {
            id: historyList
            objectName: "identityHistoryList"
            Layout.fillWidth: true
            Layout.preferredHeight: dialog.historyExpanded
                                    ? Math.min(126, contentHeight) : 0
            visible: dialog.historyExpanded
            clip: true
            model: dialog.identityModel.history_count
            spacing: 4
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

            delegate: Rectangle {
                id: historyRow
                required property int index
                width: historyList.width
                height: 58
                radius: 7
                color: "#151e2a"
                border.color: dialog.line
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 11
                    anchors.rightMargin: 9
                    spacing: 10
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        Text {
                            Layout.fillWidth: true
                            text: dialog.identityModel.history_summary_at(historyRow.index)
                            color: dialog.ink
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: new Date(dialog.identityModel.history_occurred_at(
                                               historyRow.index) * 1000).toLocaleString()
                                  + " · "
                                  + dialog.identityModel.history_detail_at(historyRow.index)
                            color: dialog.muted
                            font.pixelSize: 8
                            elide: Text.ElideRight
                        }
                    }
                    Button {
                        objectName: "identityUndoButton-" + historyRow.index
                        visible: dialog.identityModel.history_undoable_at(historyRow.index)
                        text: "UNDO"
                        onClicked: dialog.identityModel.undo_at(historyRow.index)
                        background: Rectangle {
                            radius: 7
                            color: parent.down ? "#3a3040" : "#272331"
                            border.color: "#695374"
                        }
                        contentItem: Text {
                            text: parent.text
                            color: "#dabee5"
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                    }
                }
            }
        }

        Text {
            objectName: "identityMessage"
            Layout.fillWidth: true
            text: dialog.identityModel.message
            color: text.indexOf("Could not") === 0 ? "#f3a49c" : dialog.muted
            font.pixelSize: 9
            wrapMode: Text.WordWrap
        }
    }
}
