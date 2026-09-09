#pragma once

#include <QEvent>
#include <QGuiApplication>
#include <QMatrix4x4>
#include <QQuickItem>
#include <QQuickWindow>
#include <QVariant>
#include <cmath>

namespace lunchbox {

// Render-only translation: never replace QML x/y bindings or resize glyphs.
class TextPixelTranslation final : public QQuickTransform {
public:
    explicit TextPixelTranslation(QQuickItem* item) : QQuickTransform(item) { appendToItem(item); }
    void applyTo(QMatrix4x4* matrix) const override { matrix->translate(offset.x(), offset.y()); }
    void align(QQuickItem* item, qreal dpr) {
        const QPointF baseline(0, item->baselineOffset());
        const auto scene = item->mapToScene(baseline);
        const QPointF snapped(std::round(scene.x() * dpr) / dpr,
                              std::round(scene.y() * dpr) / dpr);
        const auto next = offset + item->mapFromScene(snapped) - baseline;
        if (std::abs(next.x() - offset.x()) > 0.00001 || std::abs(next.y() - offset.y()) > 0.00001) {
            offset = next;
            update();
        }
    }
private:
    QPointF offset;
};

inline void alignWindowText(QQuickItem* item, qreal dpr) {
    if (!item || !item->isVisible() || dpr <= 0) return;
    // Qt's styled Label/TextField/TextArea implementations inherit these types.
    if (item->inherits("QQuickText") || item->inherits("QQuickTextInput") || item->inherits("QQuickTextEdit")) {
        auto* translation = static_cast<TextPixelTranslation*>(
            item->property("_lunchboxTextPixelTranslation").value<QObject*>());
        if (!translation) {
            translation = new TextPixelTranslation(item);
            item->setProperty("_lunchboxTextPixelTranslation", QVariant::fromValue(static_cast<QObject*>(translation)));
        }
        translation->align(item, dpr);
    }
    for (auto* child : item->childItems()) alignWindowText(child, dpr);
}

class TextPixelAlignment final : public QObject {
public:
    explicit TextPixelAlignment(QObject* parent) : QObject(parent) {}
    bool eventFilter(QObject* watched, QEvent* event) override {
        if (event->type() == QEvent::Show) {
            auto* window = qobject_cast<QQuickWindow*>(watched);
            if (window && !window->property("_lunchboxTextPixelAlignment").toBool()) {
                window->setProperty("_lunchboxTextPixelAlignment", true);
                // GUI-thread signal, after animations and before scene graph sync.
                connect(window, &QQuickWindow::afterAnimating, window, [window] {
                    alignWindowText(window->contentItem(), window->effectiveDevicePixelRatio());
                });
            }
        }
        return false;
    }
};

inline void installTextPixelAlignment() {
    auto* application = QGuiApplication::instance();
    if (!application || application->property("_lunchboxTextPixelAlignment").toBool()) return;
    application->setProperty("_lunchboxTextPixelAlignment", true);
    application->installEventFilter(new TextPixelAlignment(application));
}
} // namespace lunchbox
