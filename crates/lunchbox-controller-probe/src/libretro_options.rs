//! Owned legacy core-option registry for isolated inspection frontends.
//! FBNeo's version-zero registration places each option's default value first.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString, c_char, c_void};

#[repr(C)]
struct NativeVariable {
    key: *const c_char,
    value: *const c_char,
}

/// Option callback state for a fresh, isolated inspection process. This does
/// not replace the historical diagnostic frontend's fixed option responses.
pub struct OptionEnvironment {
    registry: OptionRegistry,
    failure: Option<String>,
}

impl OptionEnvironment {
    pub fn new(overrides: BTreeMap<String, String>) -> Result<Self> {
        Ok(Self {
            registry: OptionRegistry::new(overrides)?,
            failure: None,
        })
    }

    /// Handle only legacy option commands. `None` leaves other commands to
    /// the containing frontend. Once registration fails, reporting fails too.
    ///
    /// # Safety
    /// The trusted core must supply correctly aligned, readable/writable ABI
    /// objects and valid NUL-terminated strings for the entire call. Bounds
    /// limit traversal, but cannot make arbitrary native pointers safe. Keep
    /// this object alive until after core deinitialization.
    pub unsafe fn handle(&mut self, command: u32, data: *mut c_void) -> Option<bool> {
        let command = command & !0x10000;
        if !matches!(command, 15 | 16 | 17 | 52) {
            return None;
        }
        if data.is_null() {
            self.failure
                .get_or_insert_with(|| "Null core-option callback data".into());
            return Some(false);
        }
        let result: Result<bool> = (|| match command {
            15 => {
                let variable = unsafe { &mut *data.cast::<NativeVariable>() };
                variable.value = std::ptr::null();
                let key = unsafe { copy_native_string(variable.key, 512) }?;
                validate_key(&key)?;
                if let Some(value) = self.registry.value(&key) {
                    variable.value = value.as_ptr();
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            16 => {
                let mut definitions = Vec::new();
                for index in 0..=4096 {
                    let variable = unsafe { &*data.cast::<NativeVariable>().add(index) };
                    if variable.key.is_null() {
                        self.registry.register(&definitions)?;
                        return Ok(true);
                    }
                    ensure!(
                        index < 4096,
                        "Core-option list is not terminated within limit"
                    );
                    definitions.push((unsafe { copy_native_string(variable.key, 512) }?, unsafe {
                        copy_native_string(variable.value, 65536)
                    }?));
                }
                unreachable!("Bounded registration returns at terminator or limit")
            }
            17 => {
                // Overrides are fixed before initialization. Registration is
                // not a frontend value edit and does not signal an update.
                unsafe { data.cast::<bool>().write(false) };
                Ok(true)
            }
            52 => {
                // Explicitly request the legacy registration API supported by
                // this frontend, including FBNeo's default-first conversion.
                unsafe { data.cast::<u32>().write(0) };
                Ok(true)
            }
            _ => unreachable!(),
        })();
        Some(match result {
            Ok(handled) => handled,
            Err(error) => {
                self.failure.get_or_insert_with(|| error.to_string());
                false
            }
        })
    }

    pub fn effective_values(&self) -> Result<BTreeMap<String, String>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("Core-option callback failed: {error}");
        }
        self.registry.effective_values()
    }
}

unsafe fn copy_native_string(pointer: *const c_char, limit: usize) -> Result<String> {
    ensure!(!pointer.is_null(), "Null core-option string");
    let mut bytes = Vec::new();
    for offset in 0..=limit {
        let byte = unsafe { *pointer.add(offset) } as u8;
        if byte == 0 {
            return Ok(String::from_utf8(bytes)?);
        }
        ensure!(offset < limit, "Core-option string exceeds capture limit");
        bytes.push(byte);
    }
    unreachable!("Bounded string copy returns at terminator or limit")
}

pub struct OptionRegistry {
    overrides: BTreeMap<String, String>,
    values: BTreeMap<String, CString>,
    // Keep previously returned C strings alive across re-registration.
    retired: Vec<CString>,
}

impl OptionRegistry {
    pub fn new(overrides: BTreeMap<String, String>) -> Result<Self> {
        ensure!(overrides.len() <= 4096, "Too many core-option overrides");
        for (key, value) in &overrides {
            validate_key(key)?;
            ensure!(
                !value.is_empty() && value.len() <= 4096 && !value.contains('\0'),
                "Invalid core-option override value"
            );
        }
        Ok(Self {
            overrides,
            values: BTreeMap::new(),
            retired: Vec::new(),
        })
    }

    /// Replace the complete current definition list atomically. Unknown
    /// overrides are checked separately after driver-specific registration.
    pub fn register(&mut self, definitions: &[(String, String)]) -> Result<()> {
        ensure!(
            definitions.len() <= 4096,
            "Too many core-option definitions"
        );
        let mut values = BTreeMap::new();
        for (key, definition) in definitions {
            validate_key(key)?;
            ensure!(
                definition.len() <= 65536 && !definition.contains('\0'),
                "Invalid core-option definition"
            );
            let (_, options) = definition
                .split_once(';')
                .ok_or_else(|| anyhow::anyhow!("Core option has no description/value delimiter"))?;
            let options = options.strip_prefix(' ').unwrap_or(options);
            let choices = options.split('|').collect::<Vec<_>>();
            ensure!(
                !choices.is_empty()
                    && choices.len() <= 256
                    && choices
                        .iter()
                        .all(|choice| !choice.is_empty() && choice.len() <= 4096),
                "Invalid core-option choices"
            );
            ensure!(
                choices.iter().copied().collect::<BTreeSet<_>>().len() == choices.len(),
                "Duplicate core-option choice"
            );
            let selected = self
                .overrides
                .get(key)
                .map(String::as_str)
                .unwrap_or(choices[0]);
            ensure!(
                choices.contains(&selected),
                "Unsupported core-option override for {key}"
            );
            ensure!(
                values
                    .insert(key.clone(), CString::new(selected)?)
                    .is_none(),
                "Duplicate core-option definition: {key}"
            );
        }
        let retired_bytes = self
            .retired
            .iter()
            .chain(self.values.values())
            .map(|value| value.as_bytes_with_nul().len())
            .sum::<usize>();
        ensure!(
            retired_bytes <= 32 * 1024 * 1024,
            "Core-option re-registration exceeds retained-string limit"
        );
        self.retired
            .extend(std::mem::replace(&mut self.values, values).into_values());
        Ok(())
    }

    pub fn value(&self, key: &str) -> Option<&CStr> {
        self.values.get(key).map(CString::as_c_str)
    }

    pub fn effective_values(&self) -> Result<BTreeMap<String, String>> {
        for key in self.overrides.keys() {
            ensure!(
                self.values.contains_key(key),
                "Requested core option was not registered: {key}"
            );
        }
        self.values
            .iter()
            .map(|(key, value)| Ok((key.clone(), value.to_str()?.to_owned())))
            .collect()
    }
}

fn validate_key(key: &str) -> Result<()> {
    ensure!(
        !key.is_empty()
            && key.len() <= 512
            && !key
                .chars()
                .any(|character| character.is_control() || character.is_whitespace()),
        "Invalid core-option key"
    );
    Ok(())
}
