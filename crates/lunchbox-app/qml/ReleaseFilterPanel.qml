import QtQuick

Column {
    id: panel

    required property var library
    property color ink: "#f4f7fb"
    property color muted: "#687488"
    property color line: "#2a3545"
    signal filtersChanged()

    width: parent ? parent.width : 320
    spacing: 2

    Text {
        width: parent.width
        topPadding: 7
        text: "RELEASE REGION"
        color: panel.muted
        font.pixelSize: 9
        font.weight: Font.Bold
        font.letterSpacing: 1
    }

    FilterToggle {
        objectName: "usaReleaseFilter"
        label: "United States releases"
        description: "Families with exact US or North America region metadata"
        checked: panel.library.usa_release_filter
        onToggled: {
            panel.library.set_release_filters(!checked,
                                              panel.library.japan_release_filter,
                                              panel.library.adult_release_filter,
                                              panel.library.non_retail_release_filter)
            panel.filtersChanged()
        }
    }

    FilterToggle {
        objectName: "japanReleaseFilter"
        label: "Japanese releases"
        description: "Exact Japan metadata, linked romanized names, or Japanese script"
        checked: panel.library.japan_release_filter
        onToggled: {
            panel.library.set_release_filters(panel.library.usa_release_filter,
                                              !checked,
                                              panel.library.adult_release_filter,
                                              panel.library.non_retail_release_filter)
            panel.filtersChanged()
        }
    }

    Rectangle {
        width: parent.width
        height: 1
        color: panel.line
    }

    Text {
        width: parent.width
        topPadding: 7
        text: "RELEASE CATEGORY"
        color: panel.muted
        font.pixelSize: 9
        font.weight: Font.Bold
        font.letterSpacing: 1
    }

    FilterToggle {
        objectName: "adultReleaseFilter"
        label: "Adult releases"
        description: "Show only adult-rated or explicitly adult releases"
        checked: panel.library.adult_release_filter
        onToggled: {
            panel.library.set_release_filters(panel.library.usa_release_filter,
                                              panel.library.japan_release_filter,
                                              !checked,
                                              panel.library.non_retail_release_filter)
            panel.filtersChanged()
        }
    }

    FilterToggle {
        objectName: "nonRetailReleaseFilter"
        label: "Homebrew / pirate releases"
        description: "Homebrew, ROM hacks, unlicensed, bootleg and pirate releases"
        checked: panel.library.non_retail_release_filter
        onToggled: {
            panel.library.set_release_filters(panel.library.usa_release_filter,
                                              panel.library.japan_release_filter,
                                              panel.library.adult_release_filter,
                                              !checked)
            panel.filtersChanged()
        }
    }
}
