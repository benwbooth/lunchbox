//! Linux driver directory choice and version-sorted joystick names.
use anyhow::{Result, ensure};
use std::{
    ffi::CString,
    path::{Path, PathBuf},
};

pub(crate) fn names(directory: &Path, prefix: &str) -> Result<Option<Vec<String>>> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut names = Vec::new();
    for entry in entries {
        let name = entry?.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(suffix) = name.strip_prefix(prefix) else {
            continue;
        };
        if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        ensure!(
            names.len() < 1024,
            "Mednafen joystick directory exceeds limit"
        );
        names.push(name.to_owned());
    }
    // Linux versionsort delegates to strverscmp; retain leading-zero behavior.
    names.sort_by(|a, b| {
        let a = CString::new(a.as_str()).expect("directory name contains no NUL");
        let b = CString::new(b.as_str()).expect("directory name contains no NUL");
        unsafe extern "C" {
            fn strverscmp(a: *const libc::c_char, b: *const libc::c_char) -> libc::c_int;
        }
        unsafe { strverscmp(a.as_ptr(), b.as_ptr()) }.cmp(&0)
    });
    Ok(Some(names))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Enumeration {
    input: Option<Vec<String>>,
    legacy: Option<Vec<String>>,
    pub(crate) paths: Vec<PathBuf>,
}

impl Enumeration {
    pub(crate) fn capture() -> Result<Self> {
        let input = names(Path::new("/dev/input"), "js")?;
        let legacy = names(Path::new("/dev"), "js")?;
        let count =
            |names: &Option<Vec<String>>| names.as_ref().map_or(-1, |names| names.len() as i32);
        // Native chooses /dev only when it has strictly more matches.
        let (root, selected) = if count(&legacy) > count(&input) {
            (Path::new("/dev"), &legacy)
        } else {
            (Path::new("/dev/input"), &input)
        };
        let paths = selected
            .iter()
            .flatten()
            .map(|name| root.join(name))
            .collect();
        Ok(Self {
            input,
            legacy,
            paths,
        })
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            Self::capture()? == *self,
            "Mednafen joystick enumeration changed during preparation"
        );
        Ok(())
    }
}
