pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Popup {
    id: setup

    required property var settingsModel
    required property var steamGridDbModel
    required property var igdbModel
    required property var emuMoviesModel
    property color ink: "#f4f7fb"
    property color muted: "#8d99aa"
    property color panel: "#131923"
    property color panelRaised: "#1a2230"
    property color line: "#283244"
    property color accent: "#ffb454"
    property color accentCool: "#62d6c6"
    property bool uiProbe: false
    property string screenshotOutput: ""

    signal chooseDirectory(string field)
    signal openSettings(string section)
    signal finishRequested()

    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.NoAutoClose
    onOpened: {
        steamGridDbModel.initialize()
        igdbModel.initialize()
        emuMoviesModel.initialize()
    }

    component SetupCard: Rectangle {
        default property alias body: cardBody.data
        property string eyebrow: ""
        property string heading: ""
        Layout.fillWidth: true
        implicitHeight: cardBody.implicitHeight + 34
        radius: 12
        color: setup.panelRaised
        border.color: setup.line

        ColumnLayout {
            id: cardBody
            anchors.fill: parent
            anchors.margins: 17
            spacing: 9
            Text {
                Layout.fillWidth: true
                text: parent.parent.eyebrow
                color: setup.accent
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.1
            }
            Text {
                Layout.fillWidth: true
                text: parent.parent.heading
                color: setup.ink
                font.pixelSize: 16
                font.weight: Font.Bold
            }
        }
    }

    component AccountRow: Rectangle {
        property string serviceName: ""
        property string description: ""
        property bool configured: false
        property string section: ""
        signal setupRequested(string section)

        Layout.fillWidth: true
        implicitHeight: 64
        radius: 9
        color: "#101721"
        border.color: configured ? "#28594f" : setup.line

        RowLayout {
            anchors.fill: parent
            anchors.margins: 11
            spacing: 11
            Rectangle {
                Layout.preferredWidth: 10
                Layout.preferredHeight: 10
                radius: 5
                color: parent.parent.configured ? setup.accentCool : "#637085"
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Text {
                    Layout.fillWidth: true
                    text: parent.parent.parent.serviceName
                    color: setup.ink
                    font.pixelSize: 11
                    font.weight: Font.Bold
                }
                Text {
                    Layout.fillWidth: true
                    text: parent.parent.parent.description
                    color: setup.muted
                    font.pixelSize: 9
                    elide: Text.ElideRight
                }
            }
            Text {
                text: parent.parent.configured ? "CONNECTED" : "OPTIONAL"
                color: parent.parent.configured ? setup.accentCool : setup.muted
                font.pixelSize: 8
                font.weight: Font.Bold
                font.letterSpacing: 0.8
            }
            Button {
                text: parent.parent.configured ? "MANAGE" : "SET UP"
                flat: parent.parent.configured
                onClicked: parent.parent.setupRequested(parent.parent.section)
            }
        }
    }

    background: Rectangle {
        color: "#090e15"
    }

    contentItem: Item {
        id: setupSurface

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 78
            color: "#0d141e"
            border.color: setup.line
            Text {
                anchors.left: parent.left
                anchors.leftMargin: Math.max(28, (parent.width - 1120) / 2)
                anchors.verticalCenter: parent.verticalCenter
                text: "LUNCHBOX"
                color: setup.accent
                font.pixelSize: 18
                font.weight: Font.Black
                font.letterSpacing: 2.4
            }
            Text {
                anchors.right: parent.right
                anchors.rightMargin: Math.max(28, (parent.width - 1120) / 2)
                anchors.verticalCenter: parent.verticalCenter
                text: "FIRST-RUN SETUP"
                color: setup.muted
                font.pixelSize: 9
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }
        }

        ScrollView {
            anchors.fill: parent
            anchors.topMargin: 78
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: Math.min(1120, setup.width - 56)
                x: Math.max(28, (setup.width - width) / 2)
                spacing: 15

                Item { Layout.preferredHeight: 19 }

                Text {
                    Layout.fillWidth: true
                    text: "YOUR GAMES, READY WHEN YOU ARE"
                    color: setup.ink
                    font.pixelSize: 30
                    font.weight: Font.Black
                    font.letterSpacing: 0.3
                }
                Text {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 820
                    text: "Bring an existing collection, browse games available through Minerva, or use both. These choices only establish sensible starting points and can be changed later."
                    color: setup.muted
                    font.pixelSize: 13
                    lineHeight: 1.25
                    wrapMode: Text.WordWrap
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 15

                    SetupCard {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        eyebrow: "01  ·  LOCAL COLLECTION"
                        heading: "Choose where your games live"
                        Text {
                            Layout.fillWidth: true
                            text: setup.settingsModel.rom_directory.length > 0
                                  ? setup.settingsModel.rom_directory
                                  : "No ROM library selected yet"
                            color: setup.settingsModel.rom_directory.length > 0
                                   ? setup.accentCool : setup.muted
                            font.pixelSize: 10
                            elide: Text.ElideMiddle
                        }
                        Button {
                            Layout.fillWidth: true
                            text: setup.settingsModel.rom_directory.length > 0
                                  ? "CHANGE ROM FOLDER" : "CHOOSE ROM FOLDER"
                            onClicked: setup.chooseDirectory("rom")
                        }
                    }

                    SetupCard {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        eyebrow: "02  ·  DOWNLOADS"
                        heading: "Connect the download engine"
                        Text {
                            Layout.fillWidth: true
                            text: (setup.settingsModel.qbittorrent_use_https
                                   ? "https://" : "http://")
                                  + setup.settingsModel.qbittorrent_host + ":"
                                  + setup.settingsModel.qbittorrent_port
                            color: setup.muted
                            font.pixelSize: 10
                            elide: Text.ElideRight
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Text {
                                Layout.fillWidth: true
                                text: setup.settingsModel.connection_ok
                                      ? "READY" : "NOT TESTED"
                                color: setup.settingsModel.connection_ok
                                       ? setup.accentCool : setup.muted
                                font.pixelSize: 9
                                font.weight: Font.Bold
                            }
                            Button {
                                text: "CONFIGURE"
                                onClicked: setup.openSettings("qbittorrent")
                            }
                        }
                    }
                }

                SetupCard {
                    eyebrow: "03  ·  MEDIA ACCOUNTS  ·  OPTIONAL"
                    heading: "Add richer artwork, videos, and manuals"
                    Text {
                        Layout.fillWidth: true
                        text: "Lunchbox works without these accounts. When a feature needs one, it will explain why and take you directly to its settings. Credentials stay in the operating-system credential store."
                        color: setup.muted
                        font.pixelSize: 10
                        wrapMode: Text.WordWrap
                    }
                    AccountRow {
                        serviceName: "SteamGridDB"
                        description: "Modern covers, backgrounds, and clear logos"
                        configured: setup.steamGridDbModel.api_key_saved
                        section: "steamgriddb"
                        onSetupRequested: section => setup.openSettings(section)
                    }
                    AccountRow {
                        serviceName: "IGDB"
                        description: "Covers, artwork, screenshots, and metadata"
                        configured: setup.igdbModel.credentials_saved
                        section: "igdb"
                        onSetupRequested: section => setup.openSettings(section)
                    }
                    AccountRow {
                        serviceName: "EmuMovies"
                        description: "Gameplay videos, manuals, and FTP media packs"
                        configured: setup.emuMoviesModel.credentials_saved
                        section: "emumovies"
                        onSetupRequested: section => setup.openSettings(section)
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 78
                    radius: 12
                    color: "#111a24"
                    border.color: setup.accent
                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        spacing: 14
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text {
                                Layout.fillWidth: true
                                text: "READY TO EXPLORE"
                                color: setup.ink
                                font.pixelSize: 13
                                font.weight: Font.Bold
                            }
                            Text {
                                Layout.fillWidth: true
                                text: "You can import local games or configure more services at any time from Settings."
                                color: setup.muted
                                font.pixelSize: 10
                            }
                        }
                        Button {
                            Layout.preferredWidth: 190
                            Layout.preferredHeight: 44
                            text: setup.settingsModel.busy ? "SAVING…" : "FINISH SETUP"
                            enabled: setup.settingsModel.initialized
                                     && !setup.settingsModel.busy
                            onClicked: setup.finishRequested()
                        }
                    }
                }

                Item { Layout.preferredHeight: 23 }
            }
        }
    }

    Timer {
        interval: 700
        running: setup.uiProbe && setup.opened
        repeat: false
        onTriggered: {
            if (setup.screenshotOutput.length === 0) {
                console.warn("LUNCHBOX_ONBOARDING_UI_READY")
                Qt.quit()
                return
            }
            setupSurface.grabToImage(function(result) {
                if (!result.saveToFile(setup.screenshotOutput)) {
                    console.error("LUNCHBOX_ONBOARDING_UI_FAILED screenshot="
                                  + setup.screenshotOutput)
                    Qt.exit(2)
                    return
                }
                console.warn("LUNCHBOX_ONBOARDING_UI_READY screenshot="
                             + setup.screenshotOutput)
                Qt.quit()
            })
        }
    }
}
