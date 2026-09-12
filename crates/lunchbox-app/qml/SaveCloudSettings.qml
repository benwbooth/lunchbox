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
        text: "CLOUD SAVES & STATES"
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
                { label: "Google Drive", value: "google_drive" },
                { label: "Dropbox", value: "dropbox" },
                { label: "OneDrive", value: "one_drive" }
            ]
            currentIndex: root.providerModel.provider === "dropbox" ? 1
                          : root.providerModel.provider === "one_drive" ? 2 : 0
        }
        ComboBox {
            id: authMode
            Layout.preferredWidth: 190
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
        visible: authMode.currentValue === "access"
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
        visible: authMode.currentValue === "refresh"
        placeholderText: "OAuth refresh token"
        enabled: !root.providerModel.busy
        ink: root.ink
        muted: root.muted
        line: root.line
        accent: root.accent
    }

    RowLayout {
        Layout.fillWidth: true
        visible: authMode.currentValue === "refresh"
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
            text: root.providerModel.busy ? "VERIFYING…" : "SAVE && VERIFY CONNECTION"
            enabled: !root.providerModel.busy
                     && ((authMode.currentValue === "access"
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
                           authMode.currentValue === "refresh" ? clientSecret.text : "")
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 2
        Text {
            Layout.fillWidth: true
            text: "Provider transport is implemented with Apache OpenDAL. Save blobs and manifests are content-addressed; each installation owns its device pointer, and local replacements retain a recovery copy."
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
