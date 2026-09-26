pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls

Column {
    id: pane
    required property var backend
    spacing: 10
    readonly property var modes: ["emulator", "off", "casual", "hardcore"]
    Text { text: "RetroAchievements"; color: "#f4f7fb"; font.pixelSize: 18; font.bold: true }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
        text: "Earn achievements in supported RetroArch cores. RetroArch handles game recognition, unlock notifications, progress and leaderboards. An internet connection and a free RetroAchievements account are required."
    }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#6bd4c7"
        text: pane.backend.username ? "Signed in as " + pane.backend.username : "No Lunchbox account connected"
    }
    LbTextField {
        id: username
        objectName: "achievementUsername"
        width: parent.width
        placeholderText: "RetroAchievements username"
        text: pane.backend.username
        enabled: !pane.backend.busy
        Accessible.name: "RetroAchievements username"
    }
    LbTextField {
        id: password
        objectName: "achievementPassword"
        width: parent.width
        placeholderText: "Password (used only to sign in)"
        echoMode: TextInput.Password
        enabled: !pane.backend.busy
        Accessible.name: "RetroAchievements password"
        onVisibleChanged: if (!visible) clear()
    }
    Flow {
        width: parent.width; spacing: 8
        LbButton {
            text: "Sign in"
            objectName: "achievementSignIn"
            enabled: !pane.backend.busy && username.text.trim().length > 0 && password.text.length > 0
            onClicked: { pane.backend.sign_in(username.text, password.text); password.clear() }
        }
        LbButton { text: "Sign out"; enabled: !pane.backend.busy && !!pane.backend.username; onClicked: pane.backend.sign_out() }
        LbButton { text: "Create account"; onClicked: Qt.openUrlExternally("https://retroachievements.org/createaccount.php") }
        LbButton {
            text: "My profile"; enabled: !!pane.backend.username
            onClicked: Qt.openUrlExternally("https://retroachievements.org/user/" + encodeURIComponent(pane.backend.username))
        }
    }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
        text: "Your password is not saved. The login token is kept in your operating system's credential store, not in the settings database. Sign in again if the token expires."
    }
    Text { text: "Default for RetroArch games"; color: "#f4f7fb" }
    LbComboBox {
        objectName: "achievementDefaultMode"
        width: parent.width
        model: ["Keep RetroArch's own settings", "Off", "Casual — save states allowed", "Hardcore — no state loading or cheats"]
        currentIndex: Math.max(0, pane.modes.indexOf(pane.backend.default_mode))
        enabled: !pane.backend.busy
        onActivated: pane.backend.choose_default(pane.modes[currentIndex])
    }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
        text: "Casual and Hardcore use the account above. Individual games can override this default under Settings & mappings → RetroAchievements. Changes apply on the next launch. Standalone emulators keep their own achievement setup."
    }
    Text {
        width: parent.width; wrapMode: Text.WordWrap; color: "#a7b4c4"
        text: "Hardcore disables auto-resume and rewind; normal in-game saves (SRAM) still work. Enabled cheats block a Hardcore launch. RetroArch may reject unsupported cores or core options. Patched games need a recognized achievement set for that exact version."
    }
    Flow {
        width: parent.width; spacing: 8
        LbButton { text: "Supported games"; onClicked: Qt.openUrlExternally("https://retroachievements.org/gameList.php") }
        LbButton { text: "Supported emulators & cores"; onClicked: Qt.openUrlExternally("https://docs.retroachievements.org/general/emulator-support-and-issues.html") }
    }
    Text { width: parent.width; wrapMode: Text.WordWrap; color: "#6bd4c7"; text: pane.backend.message; visible: text.length > 0 }
}
