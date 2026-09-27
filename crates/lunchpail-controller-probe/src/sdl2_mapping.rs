//! Strict, ordered SDL2 mapping decoding for calibration translation.
//! SDL 3eba0b6f8a21392f47b1b53a476e7633048de9b1, SDL_gamecontroller.c
//! SDL_PrivateGameControllerParseElement. Unknown syntax is not guessed.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable interpretation of a measured logical binding. Device identity is
/// supplied separately by the app; GUID is a backend/model check, not identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingContext {
    pub library: std::path::PathBuf,
    pub library_sha256: String,
    #[serde(default)]
    pub input_environment_sha256: String,
    pub version: [u8; 3],
    pub guid: String,
    pub mapping: String,
    pub counts: crate::sdl2::ControlCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_classic: Option<crate::linux_classic::ClassicMap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_evdev: Option<crate::sdl2_evdev::EvdevMap>,
}

impl MappingContext {
    pub fn capture(snapshot: &crate::sdl2::Snapshot, path: &str) -> Result<Self> {
        let device = snapshot.device_at_path(path)?;
        ensure!(
            device.is_game_controller,
            "Logical mapping requires an SDL2 GameController"
        );
        let context = Self {
            library: snapshot.library.clone(),
            library_sha256: snapshot.library_sha256.clone(),
            input_environment_sha256: snapshot.input_environment_sha256.clone(),
            version: snapshot.version,
            guid: device.guid.clone(),
            mapping: device.mapping.clone().context("SDL2 mapping is absent")?,
            counts: device
                .controls
                .clone()
                .context("SDL2 control counts are absent")?,
            linux_classic: device.linux_classic.clone(),
            linux_evdev: device.linux_evdev.clone(),
        };
        context.validate()?;
        Ok(context)
    }

    pub fn validate(&self) -> Result<()> {
        // Empty is accepted only for loading old records; exact current-context
        // matching rejects them until a fresh capture supplies provenance.
        ensure!(
            self.input_environment_sha256.is_empty()
                || (self.input_environment_sha256.len() == 64
                    && self
                        .input_environment_sha256
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit())),
            "Invalid saved SDL2 environment fingerprint"
        );
        ensure!(
            self.linux_classic.is_none() || self.linux_evdev.is_none(),
            "Ambiguous saved SDL2 backend metadata"
        );
        if let Some(map) = &self.linux_classic {
            crate::sdl2_physical::PhysicalMap::Classic(map).validate_counts(&self.counts)?;
        }
        if let Some(map) = &self.linux_evdev {
            crate::sdl2_physical::PhysicalMap::Evdev(map).validate_counts(&self.counts)?;
        }
        ensure!(
            self.library.is_absolute()
                && self.library.as_os_str().len() <= 4096
                && self.library_sha256.len() == 64
                && self.library_sha256.bytes().all(|b| b.is_ascii_hexdigit())
                && self.guid.len() == 32
                && self.guid.bytes().all(|b| b.is_ascii_hexdigit())
                && self.version[0] == 2
                && self.version[1] >= 24,
            "Invalid saved SDL2 runtime context"
        );
        ensure!(
            [self.counts.buttons, self.counts.axes, self.counts.hats]
                .iter()
                .all(|count| *count <= 1024),
            "Invalid saved SDL2 control counts"
        );
        for binding in parse(&self.mapping)? {
            ensure!(
                match binding.input {
                    Input::Button(index) => index < self.counts.buttons,
                    Input::Axis { index, .. } => index < self.counts.axes,
                    Input::Hat { index, .. } => index < self.counts.hats,
                },
                "SDL2 mapping references an unavailable physical control"
            );
        }
        Ok(())
    }

    pub fn ensure_current(&self, snapshot: &crate::sdl2::Snapshot, path: &str) -> Result<()> {
        self.validate()?;
        ensure!(
            *self == Self::capture(snapshot, path)?,
            "SDL2 runtime or effective mapping changed; recapture the logical bindings"
        );
        Ok(())
    }
}

/// Measured SDL joystick values, after the selected backend's conversion.
/// Missing entries are unknown, not released/centered. Keep this separate from
/// stable device identity: state changes must not invalidate routing snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputState {
    pub buttons: BTreeMap<u32, bool>,
    pub axes: BTreeMap<u32, i16>,
    pub hats: BTreeMap<u32, u8>,
}

impl InputState {
    pub fn validate(&self, counts: &crate::sdl2::ControlCounts) -> Result<()> {
        ensure!(
            self.buttons.keys().all(|index| *index < counts.buttons)
                && self.axes.keys().all(|index| *index < counts.axes)
                && self
                    .hats
                    .iter()
                    .all(|(index, mask)| *index < counts.hats && *mask <= 15),
            "Measured SDL2 state contains an unavailable control or invalid hat value"
        );
        Ok(())
    }

    pub fn read(&self, input: &Input) -> Result<i32> {
        match input {
            Input::Button(index) => self
                .buttons
                .get(index)
                .copied()
                .map(i32::from)
                .with_context(|| format!("SDL2 button {index} has no measured state")),
            Input::Axis { index, .. } => self
                .axes
                .get(index)
                .copied()
                .map(i32::from)
                .with_context(|| format!("SDL2 axis {index} has no measured state")),
            Input::Hat { index, .. } => self
                .hats
                .get(index)
                .copied()
                .map(i32::from)
                .with_context(|| format!("SDL2 hat {index} has no measured state")),
        }
    }

    /// Resolve every exposed logical output from one explicit measured state.
    /// Require all referenced inputs even when an earlier mapping would win:
    /// incomplete capture must not silently appear usable for calibration.
    pub fn outputs(
        &self,
        bindings: &[Binding],
        counts: &crate::sdl2::ControlCounts,
    ) -> Result<BTreeMap<String, i32>> {
        self.validate(counts)?;
        ensure!(!bindings.is_empty(), "SDL2 mapping has no control bindings");
        for binding in bindings {
            self.read(&binding.input)?;
        }
        let mut outputs = BTreeMap::new();
        for binding in bindings {
            if !outputs.contains_key(&binding.output) {
                outputs.insert(
                    binding.output.clone(),
                    output_value(bindings, &binding.output, |input| self.read(input))?,
                );
            }
        }
        Ok(outputs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    Button(u32),
    Axis {
        index: u32,
        minimum: i32,
        maximum: i32,
    },
    Hat {
        index: u32,
        mask: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub output: String,
    /// None for buttons; signed destination endpoints for analog outputs.
    pub output_range: Option<(i32, i32)>,
    pub input: Input,
}

/// Logical values observed across a caller-confirmed released/pressed pair.
/// Multiple changes are retained: selecting the intended control is a separate
/// calibration decision, not an arbitrary first match in mapping order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputChange {
    pub output: String,
    pub analog: bool,
    pub released: i32,
    pub pressed: i32,
}

pub fn changed_outputs(
    bindings: &[Binding],
    counts: &crate::sdl2::ControlCounts,
    released: &InputState,
    pressed: &InputState,
) -> Result<Vec<OutputChange>> {
    let before = released.outputs(bindings, counts)?;
    let after = pressed.outputs(bindings, counts)?;
    let mut changes = Vec::new();
    for (output, released) in before {
        let pressed = *after
            .get(&output)
            .context("SDL2 output disappeared between samples")?;
        if released != pressed {
            let binding = bindings
                .iter()
                .find(|binding| binding.output == output)
                .context("Changed output has no SDL2 mapping")?;
            changes.push(OutputChange {
                output,
                analog: binding.output_range.is_some(),
                released,
                pressed,
            });
        }
    }
    Ok(changes)
}

/// Evaluate SDL2's polled GameController state from backend-normalized inputs.
/// Contract: SDL_GameControllerGetAxis/GetButton in the pinned SDL source.
/// Buttons combine with OR; axes use the first nonzero in-range result in
/// mapping order. The reader must supply the full input state, not assume that
/// unrelated physical axes rest at zero (notably triggers).
pub fn output_value(
    bindings: &[Binding],
    output: &str,
    mut read: impl FnMut(&Input) -> Result<i32>,
) -> Result<i32> {
    let selected: Vec<_> = bindings
        .iter()
        .filter(|binding| binding.output == output)
        .collect();
    let first = selected
        .first()
        .context("SDL2 mapping does not expose the requested output")?;
    let analog = first.output_range.is_some();
    ensure!(
        selected
            .iter()
            .all(|binding| binding.output_range.is_some() == analog),
        "Inconsistent SDL2 output kinds"
    );
    let within = |value: i32, a: i32, b: i32| value >= a.min(b) && value <= a.max(b);
    let mut button = 0;
    for binding in selected {
        let raw = read(&binding.input)?;
        let (active, axis) = match binding.input {
            Input::Button(_) => {
                ensure!((0..=1).contains(&raw), "Invalid physical button state");
                (raw != 0, None)
            }
            Input::Hat { mask, .. } => {
                ensure!(
                    (0..=15).contains(&raw) && mask > 0 && mask <= 15,
                    "Invalid physical hat state or mapping mask"
                );
                (raw & i32::from(mask) != 0, None)
            }
            Input::Axis {
                minimum, maximum, ..
            } => {
                ensure!(
                    (-32768..=32767).contains(&raw)
                        && (-32768..=32767).contains(&minimum)
                        && (-32768..=32767).contains(&maximum)
                        && minimum != maximum,
                    "Invalid SDL2 axis state or mapping endpoints"
                );
                let valid = within(raw, minimum, maximum);
                let threshold = minimum + (maximum - minimum) / 2;
                (
                    valid
                        && if minimum < maximum {
                            raw >= threshold
                        } else {
                            raw <= threshold
                        },
                    Some((minimum, maximum, valid)),
                )
            }
        };
        if let Some((out_min, out_max)) = binding.output_range {
            ensure!(
                (-32768..=32767).contains(&out_min)
                    && (-32768..=32767).contains(&out_max)
                    && out_min != out_max,
                "Invalid SDL2 output endpoints"
            );
            let value = match axis {
                Some((_, _, false)) => 0,
                Some((minimum, maximum, true)) if minimum == out_min && maximum == out_max => raw,
                Some((minimum, maximum, true)) => {
                    // Match SDL's float normalization and truncation toward zero.
                    let normalized = (raw - minimum) as f32 / (maximum - minimum) as f32;
                    out_min + (normalized * (out_max - out_min) as f32) as i32
                }
                None => {
                    if active {
                        out_max
                    } else {
                        0
                    }
                }
            };
            if value != 0 && within(value, out_min, out_max) {
                return Ok(value);
            }
        } else {
            button |= i32::from(active);
        }
    }
    Ok(button)
}

fn half(text: &str) -> (&str, Option<char>) {
    if let Some(tail) = text.strip_prefix('+') {
        (tail, Some('+'))
    } else if let Some(tail) = text.strip_prefix('-') {
        (tail, Some('-'))
    } else {
        (text, None)
    }
}

fn range(sign: Option<char>) -> (i32, i32) {
    match sign {
        Some('+') => (0, 32767),
        Some('-') => (0, -32768),
        _ => (-32768, 32767),
    }
}

fn number(text: &str) -> Result<u32> {
    ensure!(
        !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()),
        "Invalid SDL2 control index"
    );
    text.parse().context("SDL2 control index overflow")
}

pub fn parse(mapping: &str) -> Result<Vec<Binding>> {
    ensure!(mapping.len() <= 64 * 1024, "Oversized SDL2 mapping");
    let mut fields = mapping.split(',');
    let guid = fields.next().context("Missing SDL2 mapping GUID")?;
    ensure!(
        guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid SDL2 mapping GUID"
    );
    fields.next().context("Missing SDL2 mapping name")?;
    let mut result = Vec::new();
    for field in fields {
        if field.is_empty() {
            continue;
        }
        let field = field.replace(' ', "");
        let (destination, source) = field
            .split_once(':')
            .context("Malformed SDL2 mapping field")?;
        if matches!(
            destination,
            "platform" | "crc" | "type" | "hint" | "sdk>=" | "sdk<="
        ) {
            continue;
        }
        let (output, output_sign) = half(destination);
        let output_range = match output {
            "lefttrigger" | "righttrigger" => Some((0, 32767)),
            "leftx" | "lefty" | "rightx" | "righty" => Some(range(output_sign)),
            "a" | "b" | "x" | "y" | "back" | "guide" | "start" | "leftstick" | "rightstick"
            | "leftshoulder" | "rightshoulder" | "dpup" | "dpdown" | "dpleft" | "dpright"
            | "misc1" | "paddle1" | "paddle2" | "paddle3" | "paddle4" | "touchpad" => {
                ensure!(output_sign.is_none(), "Unexpected half-button SDL2 output");
                None
            }
            _ => bail!("Unresolved SDL2 mapping output: {output}"),
        };
        let (source, sign) = half(source);
        let inverted = source.ends_with('~');
        let source = source.strip_suffix('~').unwrap_or(source);
        let input = if let Some(index) = source.strip_prefix('a') {
            let (mut minimum, mut maximum) = range(sign);
            if inverted {
                std::mem::swap(&mut minimum, &mut maximum);
            }
            Input::Axis {
                index: number(index)?,
                minimum,
                maximum,
            }
        } else {
            ensure!(
                sign.is_none() && !inverted,
                "Unexpected SDL2 non-axis modifier"
            );
            if let Some(index) = source.strip_prefix('b') {
                Input::Button(number(index)?)
            } else if let Some(hat) = source.strip_prefix('h') {
                let (index, mask) = hat.split_once('.').context("Malformed SDL2 hat binding")?;
                // This SDL2 parser accepts only one digit before the dot.
                ensure!(index.len() == 1, "Unsupported SDL2 hat index syntax");
                let mask = number(mask)?;
                ensure!((1..=15).contains(&mask), "Invalid SDL2 hat mask");
                Input::Hat {
                    index: number(index)?,
                    mask: mask as u8,
                }
            } else {
                bail!("Unresolved SDL2 physical binding: {source}");
            }
        };
        result.push(Binding {
            output: output.into(),
            output_range,
            input,
        });
    }
    ensure!(!result.is_empty(), "SDL2 mapping has no control bindings");
    Ok(result)
}
