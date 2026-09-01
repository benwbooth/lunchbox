import QtQuick
import QtQuick.Window
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "GameWindowBehavior"

    QtObject {
        id: applicationWindow
        property int visibility: Window.Windowed
        property int minimizeCalls: 0
        property int normalCalls: 0
        property int maximizeCalls: 0
        property int fullScreenCalls: 0
        property int activateCalls: 0

        function showMinimized() {
            visibility = Window.Minimized
            minimizeCalls += 1
        }
        function showNormal() {
            visibility = Window.Windowed
            normalCalls += 1
        }
        function showMaximized() {
            visibility = Window.Maximized
            maximizeCalls += 1
        }
        function showFullScreen() {
            visibility = Window.FullScreen
            fullScreenCalls += 1
        }
        function requestActivate() {
            activateCalls += 1
        }
    }

    Lunchbox.GameWindowBehavior {
        id: behavior
        applicationWindow: applicationWindow
    }

    function init() {
        behavior.gameRunning = false
        behavior.minimizeEnabled = false
        behavior.minimizedForRunningGame = false
        behavior.visibilityBeforeGameMinimize = Window.Windowed
        applicationWindow.visibility = Window.Windowed
        applicationWindow.minimizeCalls = 0
        applicationWindow.normalCalls = 0
        applicationWindow.maximizeCalls = 0
        applicationWindow.fullScreenCalls = 0
        applicationWindow.activateCalls = 0
    }

    function test_disabled_leaves_window_alone() {
        behavior.gameRunning = true

        compare(applicationWindow.visibility, Window.Windowed)
        compare(applicationWindow.minimizeCalls, 0)
        verify(!behavior.minimizedForRunningGame)
    }

    function test_minimizes_and_restores_previous_maximized_state() {
        applicationWindow.visibility = Window.Maximized
        behavior.minimizeEnabled = true
        behavior.gameRunning = true

        compare(applicationWindow.visibility, Window.Minimized)
        compare(applicationWindow.minimizeCalls, 1)
        verify(behavior.minimizedForRunningGame)

        behavior.gameRunning = false

        compare(applicationWindow.visibility, Window.Maximized)
        compare(applicationWindow.maximizeCalls, 1)
        compare(applicationWindow.activateCalls, 1)
        verify(!behavior.minimizedForRunningGame)
    }

    function test_does_not_restore_window_that_user_already_minimized() {
        applicationWindow.visibility = Window.Minimized
        behavior.minimizeEnabled = true
        behavior.gameRunning = true
        behavior.gameRunning = false

        compare(applicationWindow.minimizeCalls, 0)
        compare(applicationWindow.normalCalls, 0)
        compare(applicationWindow.maximizeCalls, 0)
        compare(applicationWindow.fullScreenCalls, 0)
        compare(applicationWindow.activateCalls, 0)
        compare(applicationWindow.visibility, Window.Minimized)
    }

    function test_restores_full_screen_state() {
        applicationWindow.visibility = Window.FullScreen
        behavior.minimizeEnabled = true
        behavior.gameRunning = true
        behavior.gameRunning = false

        compare(applicationWindow.fullScreenCalls, 1)
        compare(applicationWindow.visibility, Window.FullScreen)
    }
}
