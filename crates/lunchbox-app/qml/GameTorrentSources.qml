import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Column {
    id: root

    required property var detailsModel
    property bool showAddSource: false
    property bool installed: false
    property bool alternativesExpanded: false
    property color ink: "#f4f7fb"
    property color muted: "#95a2b6"
    property color line: "#2b3647"
    property color accent: "#f2a13b"
    property color accentCool: "#62dac8"
    property int revision: detailsModel ? detailsModel.detail_revision : 0
    readonly property int matchingSourceCount: {
        revision
        return detailsModel ? detailsModel.download_source_count() : 0
    }
    readonly property int registeredSourceCount:
        detailsModel ? detailsModel.registered_torrent_source_count : 0
    readonly property bool ranking: detailsModel ? detailsModel.torrent_loading : false
    readonly property bool shouldShowSources:
        (!installed || alternativesExpanded)
        && (showAddSource || ranking || matchingSourceCount > 0
            || registeredSourceCount > 0)

    signal addSourceRequested()
    signal reviewCandidateRequested(int candidateIndex)

    objectName: "gameTorrentSources"
    visible: shouldShowSources
    height: visible ? implicitHeight : 0
    spacing: 9

    RowLayout {
        width: parent.width
        spacing: 8

        Text {
            Layout.fillWidth: true
            text: root.installed ? "OTHER TORRENT SOURCES" : "TORRENT SOURCES"
            color: "#687488"
            font.pixelSize: 10
            font.weight: Font.Bold
            font.letterSpacing: 1.2
        }
        Rectangle {
            visible: root.registeredSourceCount > 0
            Layout.preferredWidth: registeredSourceBadge.implicitWidth + 16
            Layout.preferredHeight: 22
            radius: 7
            color: "#1a2d2b"
            border.color: "#315c54"
            Text {
                id: registeredSourceBadge
                objectName: "registeredTorrentSourceCount"
                anchors.centerIn: parent
                text: root.registeredSourceCount + " REGISTERED"
                color: root.accentCool
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 0.5
            }
        }
    }

    Text {
        objectName: "torrentSourceSummary"
        width: parent.width
        text: root.registeredSourceCount > 0
              ? root.registeredSourceCount
                + (root.registeredSourceCount === 1
                   ? " personal torrent is indexed for "
                   : " personal torrents are indexed for ")
                + root.detailsModel.platform
                + ". Exact candidates from those sources and Minerva appear below."
              : root.matchingSourceCount > 0
                ? "Exact candidates from Minerva appear below."
                : root.ranking
                  ? "Ranking exact game payloads…"
                  : "No exact game payload is indexed yet."
        color: root.muted
        font.pixelSize: 10
        lineHeight: 1.25
        wrapMode: Text.WordWrap
    }

    Repeater {
        id: sourceRepeater
        objectName: "matchingTorrentSources"
        model: root.matchingSourceCount

        delegate: Rectangle {
            id: sourceSection
            required property int index
            property int sourceRevision: root.revision
            readonly property int bundleIndex: {
                sourceRevision
                return root.detailsModel.download_source_bundle_at(index)
            }
            readonly property int candidateCount: {
                sourceRevision
                return root.detailsModel.bundle_file_count_at(bundleIndex)
            }
            objectName: "torrentSource-" + index
            width: root.width
            height: sourceContents.implicitHeight + 24
            radius: 10
            color: "#141d29"
            border.color: index === 0 && candidateCount > 0
                          ? root.accentCool : root.line

            Column {
                id: sourceContents
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                anchors.topMargin: 12
                spacing: 8

                RowLayout {
                    width: parent.width
                    spacing: 8
                    Text {
                        Layout.fillWidth: true
                        text: {
                            sourceSection.sourceRevision
                            return root.detailsModel.bundle_title_at(
                                        sourceSection.bundleIndex)
                        }
                        color: root.ink
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        visible: sourceSection.index === 0
                                 && sourceSection.candidateCount > 0
                        Layout.preferredWidth: bestSourceText.implicitWidth + 14
                        Layout.preferredHeight: 20
                        radius: 6
                        color: "#17342f"
                        border.color: root.accentCool
                        Text {
                            id: bestSourceText
                            anchors.centerIn: parent
                            text: "BEST SOURCE"
                            color: root.accentCool
                            font.pixelSize: 8
                            font.weight: Font.Bold
                            font.letterSpacing: 0.6
                        }
                    }
                }
                Text {
                    width: parent.width
                    text: {
                        sourceSection.sourceRevision
                        return root.detailsModel.bundle_detail_at(
                                    sourceSection.bundleIndex)
                    }
                    color: root.muted
                    font.pixelSize: 10
                    elide: Text.ElideRight
                }

                Repeater {
                    model: sourceSection.candidateCount
                    delegate: Rectangle {
                        id: fileRow
                        required property int index
                        property int fileRevision: root.revision
                        objectName: "torrentCandidate-" + index
                        width: sourceContents.width
                        height: Math.max(70, fileName.implicitHeight
                                         + fileInfo.implicitHeight + 26)
                        radius: 8
                        color: sourceSection.index === 0 && index === 0
                               ? "#172b2a" : "#192330"
                        border.color: index === 0 ? root.accentCool : "#2a3748"

                        Column {
                            anchors.left: parent.left
                            anchors.right: getButton.left
                            anchors.leftMargin: 11
                            anchors.rightMargin: 9
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 4
                            Text {
                                id: fileName
                                width: parent.width
                                text: {
                                    fileRow.fileRevision
                                    return root.detailsModel.bundle_file_name_at(
                                                sourceSection.bundleIndex,
                                                fileRow.index)
                                }
                                color: root.ink
                                font.pixelSize: 11
                                wrapMode: Text.WrapAnywhere
                            }
                            Text {
                                id: fileInfo
                                width: parent.width
                                text: {
                                    fileRow.fileRevision
                                    return root.detailsModel.bundle_file_detail_at(
                                                sourceSection.bundleIndex,
                                                fileRow.index)
                                }
                                color: root.muted
                                font.pixelSize: 10
                                elide: Text.ElideRight
                            }
                        }
                        HeaderButton {
                            id: getButton
                            objectName: "getTorrentCandidate-" + fileRow.index
                            anchors.right: parent.right
                            anchors.rightMargin: 9
                            anchors.verticalCenter: parent.verticalCenter
                            text: root.detailsModel.download_busy ? "…" : "GET"
                            enabled: !root.detailsModel.download_busy
                                     && sourceSection.candidateCount > 0
                            implicitWidth: 62
                            implicitHeight: 34
                            leftPadding: 8
                            rightPadding: 8
                            onClicked: {
                                const selected = root.detailsModel.select_bundle_file(
                                                   sourceSection.bundleIndex,
                                                   fileRow.index)
                                if (selected >= 0)
                                    root.reviewCandidateRequested(selected)
                            }
                        }
                    }
                }
            }
        }
    }

    Button {
        id: addSourceButton
        objectName: "addTorrentSourceButton"
        width: parent.width
        height: 36
        visible: root.showAddSource
        text: root.registeredSourceCount > 0 || root.matchingSourceCount > 0
              ? "ADD ANOTHER TORRENT SOURCE" : "ADD TORRENT SOURCE"
        font.pixelSize: 9
        font.weight: Font.Bold
        onClicked: root.addSourceRequested()
        background: Rectangle {
            radius: 8
            color: parent.down ? "#28364a" : "#1b2532"
            border.color: root.line
        }
        contentItem: Text {
            text: parent.text
            color: root.ink
            font: parent.font
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        ToolTip.visible: hovered
        ToolTip.text: "Review one or more torrents and index them for "
                      + root.detailsModel.platform + "."
    }
}
