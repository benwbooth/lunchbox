import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    id: root

    required property var providerModel
    required property color ink
    required property color muted
    required property color line
    required property color accent
    signal manualSyncRequested()

    spacing: 9

    function focusPrimary() {
        provider.forceActiveFocus()
    }

    function clearDraftSecrets() {
        accessToken.text = ""
        refreshToken.text = ""
        clientId.text = ""
        clientSecret.text = ""
    }

    Text {
        text: "SAVE & STATE SYNCHRONIZATION"
        color: root.accent
        font.pixelSize: 10
        font.weight: Font.Bold
        font.letterSpacing: 1.2
    }

    Text {
        Layout.fillWidth: true
        text: "Synchronize the captured save and save-state folders for the exact emulator runtime before launch and after exit. Content hashes and a shared ancestor determine changes; timestamps are shown for review but never pick a winner automatically."
        color: root.muted
        font.pixelSize: 11
        wrapMode: Text.WordWrap
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 9

        Text { text: "Provider"; color: root.ink; font.pixelSize: 11 }
        ComboBox {
            id: provider
            Layout.fillWidth: true
            textRole: "label"
            valueRole: "value"
            enabled: !root.providerModel.busy
            model: [
                { label: "Local folder", value: "local_folder" },
                { label: "Google Drive", value: "google_drive" },
                { label: "Dropbox", value: "dropbox" },
                { label: "OneDrive", value: "one_drive" }
            ]
            currentIndex: root.providerModel.provider === "google_drive" ? 1
                          : root.providerModel.provider === "dropbox" ? 2
                          : root.providerModel.provider === "one_drive" ? 3 : 0
        }
        ComboBox {
            id: authMode
            Layout.preferredWidth: 190
            visible: provider.currentValue !== "local_folder"
            textRole: "label"
            valueRole: "value"
            enabled: !root.providerModel.busy
            model: [
                { label: "Refresh token", value: "refresh" },
                { label: "Access token", value: "access" }
            ]
        }
    }

    Text {
        Layout.fillWidth: true
        visible: provider.currentValue !== "local_folder"
        text: authMode.currentValue === "refresh"
              ? "Use credentials from a registered desktop OAuth application. Google Drive and Dropbox require its client secret; OneDrive permits a public-client refresh token without one."
              : "Access tokens are useful for short-lived testing and usually expire. Lunchbox stores the token only in the operating-system credential store."
        color: root.muted
        font.pixelSize: 9
        wrapMode: Text.WordWrap
    }

    SecretField {
        id: accessToken
        Layout.fillWidth: true
        visible: provider.currentValue !== "local_folder"
                 && authMode.currentValue === "access"
        placeholderText: "OAuth access token"
        enabled: !root.providerModel.busy
        ink: root.ink
        muted: root.muted
        line: root.line
        accent: root.accent
    }

    SecretField {
        id: refreshToken
        Layout.fillWidth: true
        visible: provider.currentValue !== "local_folder"
                 && authMode.currentValue === "refresh"
        placeholderText: "OAuth refresh token"
        enabled: !root.providerModel.busy
        ink: root.ink
        muted: root.muted
        line: root.line
        accent: root.accent
    }

    RowLayout {
        Layout.fillWidth: true
        visible: provider.currentValue !== "local_folder"
                 && authMode.currentValue === "refresh"
        spacing: 8

        TextField {
            id: clientId
            Layout.fillWidth: true
            placeholderText: "OAuth client ID"
            enabled: !root.providerModel.busy
        }
        SecretField {
            id: clientSecret
            Layout.fillWidth: true
            placeholderText: provider.currentValue === "one_drive"
                             ? "Client secret (optional for public client)"
                             : "OAuth client secret"
            enabled: !root.providerModel.busy
            ink: root.ink
            muted: root.muted
            line: root.line
            accent: root.accent
        }
    }

    RowLayout {
        Layout.fillWidth: true
        visible: provider.currentValue === "local_folder"
        spacing: 8

        TextField {
            id: localFolderRoot
            Layout.fillWidth: true
            text: root.providerModel.local_folder_root
            placeholderText: "Absolute folder used by Syncthing, Nextcloud, rsync, or another tool"
            enabled: !root.providerModel.busy
            onTextEdited: root.providerModel.local_folder_root = text
        }
        Button {
            text: "Choose…"
            enabled: !root.providerModel.busy
            onClicked: root.providerModel.choose_local_folder()
        }
    }

    Text {
        Layout.fillWidth: true
        visible: provider.currentValue === "local_folder"
        text: "RetroArch writes playable .srm and .state files to Lunchbox's local data folder. After play, Lunchbox backs them up here under saves/v1/{emulator}/{runtime} as versioned blobs and manifests; this folder is a sync backup, not RetroArch's live save path. The folder must already exist and be an absolute path."
        color: root.muted
        font.pixelSize: 9
        wrapMode: Text.WordWrap
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        Switch {
            text: "Synchronize automatically around game sessions"
            checked: root.providerModel.automatic_enabled
            enabled: root.providerModel.credentials_saved
                     && !root.providerModel.busy
            onToggled: root.providerModel.set_automatic(checked)
        }
        Item { Layout.fillWidth: true }
        Button {
            text: "Sync selected emulator now"
            enabled: root.providerModel.credentials_saved
                     && !root.providerModel.busy
            onClicked: root.manualSyncRequested()
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        BusyIndicator {
            visible: root.providerModel.busy
            running: visible
            Layout.preferredWidth: 20
            Layout.preferredHeight: 20
        }
        Text {
            Layout.fillWidth: true
            text: root.providerModel.message
            color: root.providerModel.status === "error" ? "#f29a91" : root.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8
        Button {
            text: "Test saved connection"
            visible: root.providerModel.credentials_saved
            enabled: !root.providerModel.busy
            onClicked: root.providerModel.test_saved_connection()
        }
        Button {
            text: "Remove saved connection"
            visible: root.providerModel.credentials_saved
            enabled: !root.providerModel.busy
            onClicked: {
                root.clearDraftSecrets()
                root.providerModel.clear_credentials()
            }
        }
        Item { Layout.fillWidth: true }
        Button {
            text: root.providerModel.busy ? "VERIFYING…"
                  : provider.currentValue === "local_folder"
                    ? "SAVE && VERIFY FOLDER" : "SAVE && VERIFY CONNECTION"
            enabled: !root.providerModel.busy
                     && ((provider.currentValue === "local_folder"
                          && localFolderRoot.text.length > 0)
                         || (authMode.currentValue === "access"
                          && accessToken.text.length > 0)
                         || (authMode.currentValue === "refresh"
                             && refreshToken.text.length > 0
                             && clientId.text.length > 0
                             && (provider.currentValue === "one_drive"
                                 || clientSecret.text.length > 0)))
            onClicked: root.providerModel.save_and_test_credentials(
                           provider.currentValue,
                           authMode.currentValue === "access" ? accessToken.text : "",
                           authMode.currentValue === "refresh" ? refreshToken.text : "",
                           authMode.currentValue === "refresh" ? clientId.text : "",
                           authMode.currentValue === "refresh" ? clientSecret.text : "",
                           provider.currentValue === "local_folder" ? localFolderRoot.text : "")
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 2
        Text {
            Layout.fillWidth: true
            text: "Provider transport is implemented with Apache OpenDAL. Local folders need no account credentials and can be managed by Syncthing, Nextcloud, rsync, or another sync tool. Save blobs and manifests are content-addressed; each installation owns its device pointer, and local replacements retain a recovery copy."
            color: root.muted
            font.pixelSize: 9
            wrapMode: Text.WordWrap
        }
        Button {
            text: "OpenDAL ↗"
            flat: true
            onClicked: Qt.openUrlExternally("https://opendal.apache.org/")
        }
    }

    Connections {
        target: root.providerModel
        function onRevisionChanged() {
            if (root.providerModel.credentials_saved
                    && root.providerModel.status === "idle")
                root.clearDraftSecrets()
        }
    }
}
