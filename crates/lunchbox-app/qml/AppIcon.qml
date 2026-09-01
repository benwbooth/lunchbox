import QtQuick

Image {
    id: root

    source: "qrc:/qt/qml/Lunchbox/qml/icons/lunchbox.svg"
    fillMode: Image.PreserveAspectFit
    asynchronous: false
    cache: true
    smooth: true
    mipmap: true
    Accessible.name: "Lunchbox"
}
