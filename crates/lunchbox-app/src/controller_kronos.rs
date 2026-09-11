//! Kronos native `kronos.ini` input bindings for the SDL2 Qt frontend.
//! Config: `~/.config/kronos/qt/kronos.ini` with `[Input]` section keys
//! `Port1<Button>` / `Port2<Button>` mapping SDL button/axis/hat strings.
//! Backup RAM: `bkram.bin` (32 KiB internal, 8 MiB cart) — whole-image
//! archive, per-game saves live inside it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { hat: u32, direction: u8 },
}

pub(crate) fn binding_str(b: &Binding) -> Result<String, ()> {
    match *b {
        Binding::Button(i) => Ok(format!("Button {}", i)),
        Binding::Axis { index, positive } => Ok(format!(
            "Axis {} {}",
            index,
            if positive { "positive" } else { "negative" }
        )),
        Binding::Hat { hat, direction } => {
            let dir = match direction {
                1 => "up",
                2 => "right",
                4 => "down",
                8 => "left",
                _ => return Err(()),
            };
            Ok(format!("Hat {} {}", hat, dir))
        }
    }
}

pub(crate) const CONTROLS: [(&str, &str); 12] = [
    ("a", "A"),
    ("b", "B"),
    ("c", "C"),
    ("x", "X"),
    ("y", "Y"),
    ("z", "Z"),
    ("start", "Start"),
    ("mode", "Mode"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_strings_are_correct() {
        assert_eq!(binding_str(&Binding::Button(5)).unwrap(), "Button 5");
        assert_eq!(
            binding_str(&Binding::Axis {
                index: 0,
                positive: true
            })
            .unwrap(),
            "Axis 0 positive"
        );
        assert_eq!(
            binding_str(&Binding::Axis {
                index: 0,
                positive: false
            })
            .unwrap(),
            "Axis 0 negative"
        );
        assert_eq!(
            binding_str(&Binding::Hat {
                hat: 0,
                direction: 1
            })
            .unwrap(),
            "Hat 0 up"
        );
        assert_eq!(
            binding_str(&Binding::Hat {
                hat: 0,
                direction: 8
            })
            .unwrap(),
            "Hat 0 left"
        );
        assert!(
            binding_str(&Binding::Hat {
                hat: 0,
                direction: 3
            })
            .is_err()
        );
    }

    #[test]
    fn controls_cover_all_twelve_genesis_buttons() {
        assert_eq!(CONTROLS.len(), 12);
    }
}
