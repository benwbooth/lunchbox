import QtQuick
import QtQuick.Controls
import QtQuick3D

Item {
    id: root

    required property url frontSource
    property url backSource: frontSource
    property bool autoRotate: true
    property real yaw: -18
    property real pitch: -5
    property real cameraDistance: 310
    property real pointerX: 0
    property real pointerY: 0

    function resetView() {
        yaw = -18
        pitch = -5
        cameraDistance = 310
        autoRotate = true
    }

    Rectangle {
        anchors.fill: parent
        color: "#101722"
        gradient: Gradient {
            GradientStop { position: 0; color: "#1b2735" }
            GradientStop { position: 1; color: "#090e16" }
        }
    }

    View3D {
        anchors.fill: parent
        environment: SceneEnvironment {
            backgroundMode: SceneEnvironment.Transparent
            antialiasingMode: SceneEnvironment.MSAA
            antialiasingQuality: SceneEnvironment.High
        }

        PerspectiveCamera {
            id: camera
            z: root.cameraDistance
            clipNear: 1
            clipFar: 1000
        }

        Node {
            id: box
            eulerRotation.x: root.pitch
            eulerRotation.y: root.yaw

            Model {
                source: "#Rectangle"
                z: 7.5
                scale: Qt.vector3d(1.35, 1.9, 1)
                materials: PrincipledMaterial {
                    lighting: PrincipledMaterial.NoLighting
                    baseColorMap: Texture {
                        source: root.frontSource
                        generateMipmaps: true
                    }
                }
            }

            Model {
                source: "#Rectangle"
                z: -7.5
                eulerRotation.y: 180
                scale: Qt.vector3d(1.35, 1.9, 1)
                materials: PrincipledMaterial {
                    lighting: PrincipledMaterial.NoLighting
                    baseColorMap: Texture {
                        source: root.backSource.toString().length > 0
                                ? root.backSource : root.frontSource
                        generateMipmaps: true
                    }
                }
            }

            Model {
                source: "#Rectangle"
                x: -67.5
                eulerRotation.y: -90
                scale: Qt.vector3d(0.15, 1.9, 1)
                materials: spineMaterial
            }

            Model {
                source: "#Rectangle"
                x: 67.5
                eulerRotation.y: 90
                scale: Qt.vector3d(0.15, 1.9, 1)
                materials: spineMaterial
            }

            Model {
                source: "#Rectangle"
                y: 95
                eulerRotation.x: 90
                scale: Qt.vector3d(1.35, 0.15, 1)
                materials: edgeMaterial
            }

            Model {
                source: "#Rectangle"
                y: -95
                eulerRotation.x: -90
                scale: Qt.vector3d(1.35, 0.15, 1)
                materials: edgeMaterial
            }
        }

        PrincipledMaterial {
            id: spineMaterial
            lighting: PrincipledMaterial.NoLighting
            baseColor: "#202a38"
            roughness: 0.78
        }

        PrincipledMaterial {
            id: edgeMaterial
            lighting: PrincipledMaterial.NoLighting
            baseColor: "#354255"
            roughness: 0.7
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
        onPressed: mouse => {
            root.autoRotate = false
            root.pointerX = mouse.x
            root.pointerY = mouse.y
        }
        onPositionChanged: mouse => {
            if (!pressed)
                return
            root.yaw += (mouse.x - root.pointerX) * 0.5
            root.pitch = Math.max(-55, Math.min(55,
                                                root.pitch
                                                + (mouse.y - root.pointerY) * 0.35))
            root.pointerX = mouse.x
            root.pointerY = mouse.y
        }
        onDoubleClicked: root.resetView()
        onWheel: wheel => {
            root.cameraDistance = Math.max(200, Math.min(650,
                                            root.cameraDistance
                                            - wheel.angleDelta.y * 0.35))
            wheel.accepted = true
        }
    }

    Timer {
        interval: 16
        repeat: true
        running: root.visible && root.autoRotate
        onTriggered: root.yaw = (root.yaw + 0.12) % 360
    }

    Rectangle {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: 10
        width: interactionHint.implicitWidth + 18
        height: 26
        radius: 8
        color: "#c90a0f18"
        border.color: "#4f6077"

        Text {
            id: interactionHint
            anchors.centerIn: parent
            text: "DRAG TO ROTATE · WHEEL TO ZOOM"
            color: "#cbd5e3"
            font.pixelSize: 8
            font.weight: Font.Bold
            font.letterSpacing: 0.6
        }
    }

    Row {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 8
        spacing: 5

        RoundButton {
            width: 30
            height: 30
            text: root.autoRotate ? "Ⅱ" : "▶"
            Accessible.name: root.autoRotate ? "Pause box rotation" : "Resume box rotation"
            onClicked: root.autoRotate = !root.autoRotate
        }
        RoundButton {
            width: 30
            height: 30
            text: "↺"
            Accessible.name: "Reset box view"
            onClicked: root.resetView()
        }
    }
}
