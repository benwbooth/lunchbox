import QtQuick

QtObject {
    // A collection is a navigation destination, not a global library filter.
    property string globalFilter: ""
    property string collection: ""
    readonly property string effectiveFilter: collection.length > 0 ? collection : globalFilter

    function select(key) {
        collection = key === "favorites" || key === "recent" ? key : ""
    }

    function leave() {
        collection = ""
    }

    function restore(key, platform) {
        select(platform.length === 0 ? key : "")
    }
}
