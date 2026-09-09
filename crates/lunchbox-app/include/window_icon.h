#pragma once

#include <QGuiApplication>
#include <QIcon>
#include <QFont>
#include <QQuickWindow>
#include <QString>
#include "text_pixel_alignment.h"

namespace lunchbox {
inline void configureTextRendering()
{
    // Match native text with the host's font defaults. Forcing unhinted glyphs
    // or disabling subpixel antialiasing changes the desktop font appearance.
    // Install before loading QML so styled controls inherit the same renderer.
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
    installTextPixelAlignment();
}

inline void setApplicationWindowIcon(const QString& resourcePath)
{
    QGuiApplication::setWindowIcon(QIcon(resourcePath));
}
} // namespace lunchbox
