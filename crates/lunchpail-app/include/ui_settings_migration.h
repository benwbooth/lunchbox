#pragma once

#include <QCoreApplication>
#include <QDir>
#include <QSettings>

namespace lunchpail {
inline bool importLegacyUiSettings(QSettings& legacy, QSettings& current)
{
    const QString marker = QStringLiteral("Migration/lunchpailUiSettingsV1");
    legacy.setFallbacksEnabled(false);
    current.setFallbacksEnabled(false);
    if (current.value(marker, false).toBool())
        return true;
    const auto keys = legacy.allKeys();
    if (legacy.status() != QSettings::NoError)
        return false;
    for (const auto& key : keys) {
        // Never overwrite preferences already changed in Lunchpail.
        if (!current.contains(key))
            current.setValue(key, legacy.value(key));
    }
    current.sync();
    if (current.status() != QSettings::NoError)
        return false;
    current.setValue(marker, true);
    current.sync();
    return current.status() == QSettings::NoError;
}

inline bool migrateUiSettings()
{
    QSettings current;
#ifdef Q_OS_LINUX
    if (qEnvironmentVariable("FLATPAK_ID") == QStringLiteral("io.github.benwbooth.Lunchpail")) {
        QSettings legacy(QDir::homePath() + QStringLiteral(
            "/.var/app/io.github.benwbooth.Lunchbox/config/Lunchbox/Lunchbox.conf"),
            QSettings::IniFormat);
        return importLegacyUiSettings(legacy, current);
    }
#endif
    // QSettings uses the organization domain on Apple platforms and the
    // organization name elsewhere. Let Qt handle plist/registry/INI storage.
    QString organization = QStringLiteral("Lunchbox");
#ifdef Q_OS_DARWIN
    if (!QCoreApplication::organizationDomain().isEmpty())
        organization = QCoreApplication::organizationDomain();
#endif
    QSettings legacy(QSettings::defaultFormat(), QSettings::UserScope,
                     organization, QStringLiteral("Lunchbox"));
    return importLegacyUiSettings(legacy, current);
}
} // namespace lunchpail
