#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("lunchpail-app/window_icon.h");

        #[namespace = "lunchpail"]
        fn configureTextRendering();

        #[namespace = "lunchpail"]
        #[rust_name = "set_application_window_icon"]
        fn setApplicationWindowIcon(resource_path: &QString);
    }
}

pub fn install() {
    ffi::configureTextRendering();
    ffi::set_application_window_icon(&cxx_qt_lib::QString::from(
        ":/qt/qml/Lunchpail/qml/icons/lunchpail.svg",
    ));
}
