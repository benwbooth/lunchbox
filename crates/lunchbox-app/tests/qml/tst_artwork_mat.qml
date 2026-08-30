import QtQuick
import QtTest
import "../../qml" as Lunchbox

TestCase {
    name: "ArtworkMat"
    when: windowShown

    Component {
        id: matComponent
        Lunchbox.ArtworkMat {
            width: 200
            height: 280
            fallbackColor: "#39c7d8"
            neutralColor: "#080c12"
        }
    }

    function test_fallback_color_exists_only_without_artwork() {
        const mat = createTemporaryObject(matComponent, this)
        verify(mat !== null)
        const fallback = findChild(mat, "fallbackColorLayer")
        verify(fallback !== null)

        compare(mat.artworkPresent, false)
        compare(mat.fallbackVisible, true)
        compare(fallback.color.toString(), "#39c7d8")

        mat.artworkPresent = true
        compare(mat.fallbackVisible, false)
        compare(fallback.visible, false)
        compare(mat.color.toString(), "#080c12")
    }

    function test_failed_or_removed_artwork_can_restore_the_fallback() {
        const mat = createTemporaryObject(matComponent, this,
                                          { "artworkPresent": true })
        verify(mat !== null)
        compare(mat.fallbackVisible, false)

        mat.artworkPresent = false
        compare(mat.fallbackVisible, true)
        compare(findChild(mat, "fallbackColorLayer").color.toString(), "#39c7d8")
    }
}
