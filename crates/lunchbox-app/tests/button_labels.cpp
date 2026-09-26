#include "text_pixel_alignment.h"
#include <QtQuickTest/quicktest.h>

// qmltestrunner alone does not install the application's render-time pixel
// alignment. Exercise that integration when checking label positioning.
class ButtonLabelTestSetup : public QObject {
    Q_OBJECT
public slots:
    void applicationAvailable() {
        QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
        lunchbox::installTextPixelAlignment();
    }
};

QUICK_TEST_MAIN_WITH_SETUP(button_labels, ButtonLabelTestSetup)
#include "button_labels.moc"
