import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "RetroAchievements"
    when: windowShown
    visible: true
    width: 720; height: 1040
    Rectangle { anchors.fill: parent; color: "#101823"; z: -1 }
    QtObject {
        id: account
        property string default_mode: "casual"
        property string game_mode: "inherit"
        property string username: ""
        property string message: ""
        property bool busy: false
        property string selected: ""
        property string submitted: ""
        function select_game(game) { selected = game }
        function choose_game(mode) { game_mode = mode }
        function choose_default(mode) { default_mode = mode }
        function sign_in(user, password) { submitted = user + ":" + password }
    }
    Component { id: paneComponent; Lunchbox.RetroAchievementsPane { width: 480; backend: account; gameId: "game"; retroarch: true } }
    Component { id: settingsComponent; Lunchbox.RetroAchievementsSettings { width: 680; backend: account; x: 20; y: 20 } }
    function init() { account.default_mode = "casual"; account.game_mode = "inherit"; account.username = ""; account.busy = false; account.submitted = "" }
    function test_game_accordion_inheritance_and_lock() {
        const pane = createTemporaryObject(paneComponent, testCase)
        compare(pane.expanded, false)
        compare(account.selected, "game")
        mouseClick(findChild(pane, "achievementAccordion"))
        compare(pane.expanded, true)
        const combo = findChild(pane, "achievementGameMode")
        compare(combo.currentIndex, 0)
        verify(combo.displayText.indexOf("Casual") >= 0)
        account.default_mode = "hardcore"
        verify(combo.displayText.indexOf("Hardcore") >= 0)
        pane.locked = true
        compare(combo.enabled, false)
        pane.gameId = "next"
        compare(pane.expanded, false)
    }
    function test_settings_sign_in_clears_password_and_fits() {
        const settings = createTemporaryObject(settingsComponent, testCase)
        const user = findChild(settings, "achievementUsername")
        const password = findChild(settings, "achievementPassword")
        const signIn = findChild(settings, "achievementSignIn")
        compare(signIn.enabled, false)
        user.text = "Player"
        password.text = "not-a-real-password"
        compare(signIn.enabled, true)
        mouseClick(signIn)
        compare(account.submitted, "Player:not-a-real-password")
        compare(password.text, "")
        wait(50)
        verify(settings.height < testCase.height - 40)
        grabImage(testCase).save("/tmp/lunchbox-achievements-settings.png")
    }
}
