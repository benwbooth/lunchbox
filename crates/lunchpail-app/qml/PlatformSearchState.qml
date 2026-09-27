import QtQuick

QtObject {
    id: state

    property bool initialized: false
    property string currentPlatform: ""
    property var searches: ({})

    function initialize(serialized, platform, legacyText) {
        let restored = ({})
        let valid = false
        if (typeof serialized === "string" && serialized.length > 0) {
            try {
                const parsed = JSON.parse(serialized)
                if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
                    for (const key in parsed) {
                        if (Object.prototype.hasOwnProperty.call(parsed, key)
                                && typeof parsed[key] === "string")
                            restored[key] = parsed[key]
                    }
                    valid = true
                }
            } catch (error) {
                // A damaged preference should not prevent library startup.
            }
        }
        if (!valid && typeof legacyText === "string" && legacyText.length > 0)
            restored[platform] = legacyText
        searches = restored
        currentPlatform = platform
        initialized = true
        return textFor(platform)
    }

    function textFor(platform) {
        const text = searches[platform]
        return typeof text === "string" ? text : ""
    }

    function update(text) {
        if (!initialized)
            return
        const next = ({})
        for (const key in searches) {
            if (Object.prototype.hasOwnProperty.call(searches, key))
                next[key] = searches[key]
        }
        next[currentPlatform] = text
        searches = next
    }

    function switchTo(platform, visibleText) {
        if (!initialized || platform === currentPlatform)
            return visibleText
        update(visibleText)
        currentPlatform = platform
        return textFor(platform)
    }

    function serialized() {
        return JSON.stringify(searches)
    }
}
