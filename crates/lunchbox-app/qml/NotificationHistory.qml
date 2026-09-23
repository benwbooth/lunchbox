import QtQuick

QtObject {
    id: history

    property var entries: []
    readonly property int count: entries.length

    function initialize(serialized) {
        let restored = []
        try {
            const parsed = JSON.parse(serialized)
            if (Array.isArray(parsed)) {
                for (const entry of parsed) {
                    if (entry && typeof entry.message === "string"
                            && typeof entry.when === "string"
                            && typeof entry.good === "boolean") {
                        restored.push({ message: entry.message.slice(0, 2000),
                                        when: entry.when,
                                        good: entry.good })
                    }
                }
            }
        } catch (error) {
            // Corrupt saved UI preferences must not prevent the app opening.
        }
        entries = restored.slice(-100).reverse()
    }

    function append(message, good, when) {
        const next = entries.slice()
        next.unshift({ message: String(message).slice(0, 2000),
                       good: Boolean(good),
                       when: when || new Date().toISOString() })
        entries = next.slice(0, 100)
    }

    function clear() {
        entries = []
    }

    function serialized() {
        return JSON.stringify(entries.slice().reverse())
    }
}
