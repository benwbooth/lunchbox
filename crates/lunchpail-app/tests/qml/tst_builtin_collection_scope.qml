import QtQuick
import QtTest
import "../../qml" as Lunchpail

TestCase {
    name: "BuiltInCollectionScope"

    Lunchpail.BuiltInCollectionScope { id: scope }

    function init() {
        scope.globalFilter = ""
        scope.leave()
    }

    function test_collection_does_not_enable_global_favorites() {
        scope.select("favorites")
        compare(scope.effectiveFilter, "favorites")
        compare(scope.globalFilter, "")
        scope.leave()
        compare(scope.effectiveFilter, "")
    }

    function test_collection_preserves_existing_filter_data() {
        return [
            { tag: "unfiltered", filter: "" },
            { tag: "installed", filter: "local" },
            { tag: "downloadable", filter: "downloadable" },
            { tag: "explicit favorites", filter: "favorites" }
        ]
    }

    function test_collection_preserves_existing_filter(data) {
        scope.globalFilter = data.filter
        scope.select("favorites")
        compare(scope.globalFilter, data.filter)
        scope.select("recent")
        compare(scope.effectiveFilter, "recent")
        compare(scope.globalFilter, data.filter)
        scope.leave()
        compare(scope.effectiveFilter, data.filter)
    }

    function test_restore_collection_separately_from_filter() {
        scope.globalFilter = "local"
        scope.restore("favorites", "")
        compare(scope.effectiveFilter, "favorites")
        compare(scope.globalFilter, "local")
        scope.restore("favorites", "Arcade")
        compare(scope.collection, "")
        compare(scope.effectiveFilter, "local")
    }

    function test_unknown_collection_does_not_become_a_global_filter() {
        scope.restore("not-a-collection", "")
        compare(scope.collection, "")
        compare(scope.effectiveFilter, "")
    }
}
