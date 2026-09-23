use cxx_qt_lib::{QByteArray, QGuiApplication, QVector};

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;
        include!("cxx-qt-lib/core/qvector/qvector_QByteArray.h");
        type QVector_QByteArray = cxx_qt_lib::QVector<QByteArray>;
        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;

        include!("lunchbox-app/desktop_application.h");
        #[namespace = "lunchbox"]
        #[rust_name = "new_desktop_application"]
        fn newDesktopApplication(args: &QVector_QByteArray) -> UniquePtr<QGuiApplication>;
    }
}

pub fn new() -> cxx::UniquePtr<QGuiApplication> {
    let mut args = QVector::<QByteArray>::default();
    for arg in std::env::args_os() {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStrExt;
        #[cfg(windows)]
        let arg = arg.to_string_lossy();

        args.append(QByteArray::from(arg.as_bytes()));
    }
    ffi::new_desktop_application(&args)
}
