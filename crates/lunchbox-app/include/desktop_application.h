#pragma once

#include <QApplication>
#include <QGuiApplication>

#include "cxx-qt-lib/qcoreapplication.h"

namespace lunchbox {
inline std::unique_ptr<QGuiApplication>
newDesktopApplication(const QVector<QByteArray>& args)
{
    // KDE's org.kde.desktop Quick Controls style paints through QStyle.
    // QApplication supplies the active KDE widget style, while retaining the
    // QGuiApplication interface used by the rest of the Rust/Qt bridge.
    auto* argsData = new rust::cxxqtlib1::ApplicationArgsData(args);
    auto app = std::make_unique<QApplication>(argsData->size(), argsData->data());
    argsData->setParent(app.get());
    return app;
}
} // namespace lunchbox
