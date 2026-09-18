//! Build identity for the running binary: the short git revision and the time
//! it was compiled. Shown quietly in the main window so a local dev build is
//! identifiable at a glance; the values are embedded by `build.rs`.

use cxx_qt_lib::QString;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, build_hash)]
        #[qproperty(QString, built_unix)]
        type BuildInfo = super::BuildInfoRust;
    }
}

pub struct BuildInfoRust {
    build_hash: QString,
    built_unix: QString,
}

impl Default for BuildInfoRust {
    fn default() -> Self {
        Self {
            build_hash: QString::from(env!("LUNCHBOX_BUILD_HASH")),
            // Seconds since the Unix epoch; the QML side formats it locally.
            built_unix: QString::from(env!("LUNCHBOX_BUILT_UNIX")),
        }
    }
}
