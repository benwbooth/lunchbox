#include "text_pixel_alignment.h"
#include <QQmlComponent>
#include <QQmlEngine>
#include <QElapsedTimer>
#include <QThread>
#include <cstdio>

int main(int argc, char** argv) {
    QGuiApplication app(argc, argv);
    lunchbox::installTextPixelAlignment();
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
    QQmlEngine engine;
    QQmlComponent component(&engine);
    component.setData(R"(
        import QtQuick
        import QtQuick.Controls
        Window {
            id: testWindow
            width: 600; height: 300; visible: true
            property real offset: 0.35
            Item {
                x: testWindow.offset; y: 0.65
                Label { objectName: "label"; text: "Not set up yet"; x: 12; y: 12 }
                TextField { objectName: "field"; text: "Select a physical layout"; x: 12; y: 51 }
                TextEdit { objectName: "edit"; text: "Wrapped text"; x: 12; y: 95 }
            }
        }
    )", QUrl());
    auto* root = component.create();
    if (!root) { qWarning() << component.errors(); return 1; }
    auto* window = qobject_cast<QQuickWindow*>(root);
    auto settle = [&] {
        window->requestUpdate();
        QElapsedTimer timer; timer.start();
        while (timer.elapsed() < 150) { app.processEvents(); QThread::msleep(1); }
    };
    auto verify = [&] {
        settle();
        for (const auto* name : {"label", "field", "edit"}) {
            auto* item = root->findChild<QQuickItem*>(name);
            if (!item || !item->property("_lunchboxTextPixelTranslation").isValid()) return false;
            const auto point = item->mapToScene(QPointF(0, item->baselineOffset()));
            const auto dpr = window->effectiveDevicePixelRatio();
            if (std::abs(point.x() * dpr - std::round(point.x() * dpr)) > 0.001
                || std::abs(point.y() * dpr - std::round(point.y() * dpr)) > 0.001
                || item->x() != 12 || item->scale() != 1) return false;
        }
        return true;
    };
    const bool initial = verify();
    root->setProperty("offset", 0.8);
    const bool moved = verify();
    delete root;
    std::printf("global text alignment: initial=%d moved=%d\n", initial, moved);
    return initial && moved ? 0 : 1;
}
