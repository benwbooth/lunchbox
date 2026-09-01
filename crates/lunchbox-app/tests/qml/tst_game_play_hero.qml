import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    id: testCase
    name: "GamePlayHero"
    when: windowShown

    Component {
        id: heroComponent
        Lunchbox.GamePlayHero {
            width: 440
            local: true
            loading: false
            canLaunch: true
            discoveryBusy: false
            launchBusy: false
            gameRunning: false
            emulatorName: "RetroArch · Mesen"
            platform: "Nintendo Entertainment System"
            launchStatus: "Ready"
            preferenceScope: ""
            emulatorOptionCount: 2
            selectedEmulatorOption: 0
            emulatorLabelAt: function(index) {
                return index === 0 ? "RetroArch · Mesen" : "Mesen"
            }
            ink: "#f4f7fb"
            muted: "#94a0b3"
            line: "#2b384b"
            accentCool: "#5de2d2"
        }
    }

    function test_installed_game_exposes_prominent_play_state() {
        const hero = createTemporaryObject(heroComponent, testCase)
        verify(hero)
        verify(hero.local)
        verify(hero.canLaunch)
        compare(hero.emulatorOptionCount, 2)
        compare(hero.emulatorLabelAt(1), "Mesen")
        verify(hero.implicitHeight > 100)
    }

    function test_non_local_game_hides_play_section() {
        const hero = createTemporaryObject(heroComponent, testCase, { local: false })
        verify(hero)
        compare(hero.visible, false)
        compare(hero.height, 0)
    }

    function test_missing_emulator_exposes_install_state() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            canLaunch: false,
            emulatorName: "No compatible emulator",
            emulatorOptionCount: 0,
            selectedEmulatorOption: -1
        })
        verify(hero)
        verify(hero.emulatorMissing)
        verify(hero.local)
    }

    function test_launch_preparation_can_be_cancelled() {
        const hero = createTemporaryObject(heroComponent, testCase, {
            launchBusy: true
        })
        verify(hero)
        let cancelled = false
        hero.cancelLaunchRequested.connect(function() { cancelled = true })
        const action = findChild(hero, "launchAction")
        verify(action)
        compare(action.text, "CANCEL PREPARATION")
        verify(action.enabled)
        action.clicked()
        verify(cancelled)
        verify(hero.launchBusy)
    }
}
