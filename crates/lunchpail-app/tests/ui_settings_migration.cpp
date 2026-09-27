#include "ui_settings_migration.h"
#include <QTemporaryDir>
#include <cstdio>

int main(int argc, char** argv) {
    QCoreApplication app(argc, argv);
    QTemporaryDir directory;
    if (!directory.isValid()) return 1;
    QSettings legacy(directory.filePath("old.ini"), QSettings::IniFormat);
    QSettings current(directory.filePath("new.ini"), QSettings::IniFormat);
    legacy.setValue("Notifications/historyJson", "saved history");
    legacy.setValue("WindowPlacement/normalWidth", 1969);
    legacy.setValue("LibraryNavigation/searchTextByPlatform", "saved searches");
    current.setValue("WindowPlacement/normalWidth", 1800);
    if (!lunchpail::importLegacyUiSettings(legacy, current)) return 2;
    if (current.value("Notifications/historyJson").toString() != "saved history"
        || current.value("LibraryNavigation/searchTextByPlatform").toString() != "saved searches"
        || current.value("WindowPlacement/normalWidth").toInt() != 1800) return 3;
    current.remove("Notifications/historyJson");
    if (!lunchpail::importLegacyUiSettings(legacy, current)
        || current.contains("Notifications/historyJson")) return 4;
    if (legacy.value("Notifications/historyJson").toString() != "saved history") return 5;
    std::puts("UI settings migration: preserves history, searches, edits, and cleared settings");
    return 0;
}
