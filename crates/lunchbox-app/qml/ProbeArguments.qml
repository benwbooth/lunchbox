import QtQml

QtObject {
    function isProbeRun(argv) {
        for (let index = 1; index < argv.length; ++index) {
            const argument = String(argv[index])
            if (/^--[a-z0-9-]*probe$/.test(argument))
                return true
        }
        return false
    }
}
