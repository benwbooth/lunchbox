#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("lunchbox-app/window_icon.h");

        #[namespace = "lunchbox"]
        #[rust_name = "set_application_window_icon"]
        fn setApplicationWindowIcon(resource_path: &QString);
    }
}

pub fn install() {
    ffi::set_application_window_icon(&cxx_qt_lib::QString::from(
        ":/qt/qml/Lunchbox/qml/icons/lunchbox.svg",
    ));
}
