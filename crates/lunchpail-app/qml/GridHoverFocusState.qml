import QtQml

QtObject {
    id: state

    // Grid focus survives mouse clicks and controller use. It cannot by itself
    // tell us which input is currently driving the card presentation.
    property bool pointerActive: false
    property bool pointerPositionKnown: false
    property real lastSceneX: 0
    property real lastSceneY: 0

    function navigationFocused() {
        pointerActive = false
    }

    function pointerMoved(sceneX, sceneY) {
        if (!Number.isFinite(sceneX) || !Number.isFinite(sceneY))
            return
        if (!pointerPositionKnown
                || Math.abs(sceneX - lastSceneX) >= 1
                || Math.abs(sceneY - lastSceneY) >= 1)
            pointerActive = true
        pointerPositionKnown = true
        lastSceneX = sceneX
        lastSceneY = sceneY
    }

    function pointerActivated() {
        pointerActive = true
    }

    function controllerFocusElsewhere(gridFocused, currentIndex, tileIndex) {
        return gridFocused && !pointerActive && currentIndex !== tileIndex
    }

    function focusedCard(gridFocused, currentIndex, tileIndex) {
        return gridFocused && !pointerActive && currentIndex === tileIndex
    }
}
