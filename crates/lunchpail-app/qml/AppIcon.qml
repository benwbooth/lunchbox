import QtQuick

Image {
    id: root

    source: "qrc:/qt/qml/Lunchpail/qml/icons/lunchpail.svg"
    fillMode: Image.PreserveAspectFit
    asynchronous: false
    cache: true
    smooth: true
    mipmap: true
    Accessible.name: "Lunchpail"
}
