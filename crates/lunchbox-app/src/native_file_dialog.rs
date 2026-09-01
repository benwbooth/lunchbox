use cxx_qt_lib::{QString, QUrl};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qurl.h");

        type QString = cxx_qt_lib::QString;
        type QUrl = cxx_qt_lib::QUrl;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type NativeFileDialog = super::NativeFileDialogRust;

        #[qinvokable]
        fn pick_open_file(
            self: &NativeFileDialog,
            title: QString,
            filter_name: QString,
            extensions: QString,
        ) -> QUrl;

        #[qinvokable]
        fn pick_save_file(
            self: &NativeFileDialog,
            title: QString,
            filter_name: QString,
            extensions: QString,
            default_file_name: QString,
        ) -> QUrl;
    }
}

#[derive(Default)]
pub struct NativeFileDialogRust;

impl qobject::NativeFileDialog {
    pub fn pick_open_file(
        &self,
        title: QString,
        filter_name: QString,
        extensions: QString,
    ) -> QUrl {
        let extensions = parse_extensions(&extensions.to_string());
        let extension_refs = extensions.iter().map(String::as_str).collect::<Vec<_>>();
        let dialog = filtered_dialog(
            rfd::FileDialog::new().set_title(title.to_string()),
            &filter_name.to_string(),
            &extension_refs,
        );
        dialog
            .pick_file()
            .map_or_else(QUrl::default, local_file_url)
    }

    pub fn pick_save_file(
        &self,
        title: QString,
        filter_name: QString,
        extensions: QString,
        default_file_name: QString,
    ) -> QUrl {
        let extensions = parse_extensions(&extensions.to_string());
        let extension_refs = extensions.iter().map(String::as_str).collect::<Vec<_>>();
        let mut dialog = filtered_dialog(
            rfd::FileDialog::new().set_title(title.to_string()),
            &filter_name.to_string(),
            &extension_refs,
        );
        if let Some(file_name) = safe_file_name(&default_file_name.to_string()) {
            dialog = dialog.set_file_name(file_name);
        }
        dialog
            .save_file()
            .map_or_else(QUrl::default, local_file_url)
    }
}

fn filtered_dialog<'a>(
    dialog: rfd::FileDialog,
    filter_name: &str,
    extensions: &'a [&'a str],
) -> rfd::FileDialog {
    if filter_name.trim().is_empty() || extensions.is_empty() {
        dialog
    } else {
        dialog.add_filter(filter_name, extensions)
    }
}

fn local_file_url(path: std::path::PathBuf) -> QUrl {
    QUrl::from_local_file(&QString::from(path.to_string_lossy().as_ref()))
}

fn parse_extensions(value: &str) -> Vec<String> {
    let mut extensions = value
        .split([',', ';', ' '])
        .map(str::trim)
        .map(|extension| extension.trim_start_matches('.'))
        .filter(|extension| {
            !extension.is_empty()
                && extension.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    extensions.sort_unstable();
    extensions.dedup();
    extensions
}

fn safe_file_name(value: &str) -> Option<&str> {
    let file_name = value
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    (!file_name.is_empty() && file_name != "." && file_name != "..").then_some(file_name)
}

#[cfg(test)]
mod tests {
    use super::{parse_extensions, safe_file_name};

    #[test]
    fn parses_only_safe_unique_extensions() {
        assert_eq!(
            parse_extensions(".ZIP, keys;zip  lunchbox-profile *.exe png/jpeg"),
            ["keys", "lunchbox-profile", "zip"]
        );
    }

    #[test]
    fn keeps_only_the_default_file_basename() {
        assert_eq!(
            safe_file_name("/tmp/Arcade Classics.lunchbox-collection.json"),
            Some("Arcade Classics.lunchbox-collection.json")
        );
        assert_eq!(
            safe_file_name(r"C:\temp\profile.lunchbox-profile"),
            Some("profile.lunchbox-profile")
        );
        assert_eq!(safe_file_name(".."), None);
    }
}
