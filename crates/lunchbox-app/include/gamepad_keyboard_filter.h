#pragma once

#include <QEvent>
#include <QGuiApplication>
#include <QKeyEvent>
#include <QObject>
#include <chrono>

namespace lunchbox {

// The SC2 can expose a keyboard alongside its native gamepad interface. Qt
// receives the keyboard key through the normal window path, while SDL3 emits
// the same navigation action independently. Observe, but never consume, keys
// so the gamepad router can coalesce matching actions from those two paths.
inline int navigationActionForKey(int key) {
    switch (key) {
    case Qt::Key_Up: return 1;
    case Qt::Key_Down: return 2;
    case Qt::Key_Left: return 3;
    case Qt::Key_Right: return 4;
    case Qt::Key_Return:
    case Qt::Key_Enter:
    case Qt::Key_Space: return 5;
    case Qt::Key_Escape:
    case Qt::Key_Backspace: return 6;
    default: return 0;
    }
}

struct KeyboardNavigationObservation {
    int action = 0;
    std::chrono::steady_clock::time_point when;
};

inline KeyboardNavigationObservation& lastKeyboardNavigation() {
    static KeyboardNavigationObservation observation;
    return observation;
}

class GamepadKeyboardObserver final : public QObject {
public:
    explicit GamepadKeyboardObserver(QObject* parent) : QObject(parent) {}
    bool eventFilter(QObject*, QEvent* event) override {
        if (event->type() == QEvent::KeyPress) {
            const auto action = navigationActionForKey(static_cast<QKeyEvent*>(event)->key());
            if (action != 0) lastKeyboardNavigation() = {action, std::chrono::steady_clock::now()};
        }
        return false;
    }
};

inline void installGamepadKeyboardObserver() {
    auto* application = QGuiApplication::instance();
    if (!application || application->property("_lunchboxGamepadKeyboardObserver").toBool()) return;
    application->setProperty("_lunchboxGamepadKeyboardObserver", true);
    application->installEventFilter(new GamepadKeyboardObserver(application));
}

inline bool keyboardNavigationSeenRecently(int action, int maxAgeMs) {
    const auto& observation = lastKeyboardNavigation();
    return action != 0 && observation.action == action && maxAgeMs > 0
        && std::chrono::steady_clock::now() - observation.when
               <= std::chrono::milliseconds(maxAgeMs);
}

} // namespace lunchbox
