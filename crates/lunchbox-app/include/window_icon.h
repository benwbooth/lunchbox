#pragma once

#include <QGuiApplication>
#include <QIcon>
#include <QString>

namespace lunchbox {
inline void setApplicationWindowIcon(const QString& resourcePath)
{
    QGuiApplication::setWindowIcon(QIcon(resourcePath));
}
} // namespace lunchbox
