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

    spacing: 8

    function focusPrimary() {
        developerId.forceActiveFocus()
    }

    Text {
        text: "ARTWORK SOURCES · SCREENSCRAPER"
        color: root.accent
        font.pixelSize: 10
        font.weight: Font.Bold
        font.letterSpacing: 1.2
    }

    Text {
        Layout.fillWidth: true
        text: "Optional checksum-oriented game media. ScreenScraper issues developer credentials only for approved API integrations; configure this source only when you are authorized to use them. Game identity is always reviewed explicitly; a linked game can then fill missing media automatically."
        color: root.muted
        font.pixelSize: 11
        wrapMode: Text.WordWrap
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        TextField {
            id: developerId
            Layout.fillWidth: true
            placeholderText: root.providerModel.credentials_saved
                             ? "Saved developer ID (leave blank to use)"
                             : "Developer ID"
            enabled: !root.providerModel.busy
        }

        SecretField {
            id: developerPassword
            Layout.fillWidth: true
            placeholderText: root.providerModel.credentials_saved
                             ? "Saved developer password (leave blank to use)"
                             : "Developer password"
            enabled: !root.providerModel.busy
            ink: root.ink
            muted: root.muted
            line: root.line
            accent: root.accent
        }
    }

    Text {
        Layout.fillWidth: true
        text: "Optional member credentials can raise the service limits associated with your ScreenScraper account. Enter both fields or leave both blank."
        color: root.muted
        font.pixelSize: 9
        wrapMode: Text.WordWrap
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        TextField {
            id: memberId
            Layout.fillWidth: true
            placeholderText: root.providerModel.credentials_saved
                             ? "Saved member username (leave blank to use)"
                             : "Member username (optional)"
            enabled: !root.providerModel.busy
        }

        SecretField {
            id: memberPassword
            Layout.fillWidth: true
            placeholderText: root.providerModel.credentials_saved
                             ? "Saved member password (leave blank to use)"
                             : "Member password (optional)"
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

        Item { Layout.fillWidth: true }

        Button {
            text: "Test connection"
            enabled: !root.providerModel.busy
                     && (root.providerModel.credentials_saved
                         || (developerId.text.length > 0
                             && developerPassword.text.length > 0))
            onClicked: root.providerModel.test_connection(
                           developerId.text, developerPassword.text,
                           memberId.text, memberPassword.text)
        }

        Button {
            text: root.providerModel.busy ? "Testing…" : "Save & test credentials"
            enabled: !root.providerModel.busy
                     && developerId.text.length > 0
                     && developerPassword.text.length > 0
                     && ((memberId.text.length === 0
                          && memberPassword.text.length === 0)
                         || (memberId.text.length > 0
                             && memberPassword.text.length > 0))
            onClicked: root.providerModel.save_and_test_credentials(
                           developerId.text, developerPassword.text,
                           memberId.text, memberPassword.text)
        }

        Button {
            visible: root.providerModel.credentials_saved
            text: "Clear saved"
            enabled: !root.providerModel.busy
            onClicked: {
                developerId.text = ""
                developerPassword.text = ""
                memberId.text = ""
                memberPassword.text = ""
                root.providerModel.clear_credentials()
            }
        }
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 8

        BusyIndicator {
            visible: root.providerModel.busy
            running: visible
            Layout.preferredWidth: 18
            Layout.preferredHeight: 18
        }

        Text {
            Layout.fillWidth: true
            text: root.providerModel.message
            color: root.muted
            font.pixelSize: 10
            wrapMode: Text.WordWrap
        }

        Button {
            text: "API documentation ↗"
            flat: true
            onClicked: Qt.openUrlExternally("https://www.screenscraper.fr/webapi2.php")
        }
    }

    Text {
        Layout.fillWidth: true
        text: "Credentials are kept in the operating-system credential store. API search, media enumeration, validation, and publication run outside the Qt thread."
        color: root.muted
        font.pixelSize: 9
        wrapMode: Text.WordWrap
    }
}
