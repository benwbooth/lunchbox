//! MAME libretro looks up the driver (including screen rotation) using the
//! FIRST command-file argument, before MAME parses its ordinary CLI options.
//! A leading `mame` happens to launch, but loses that driver metadata.

use anyhow::{Context, Result, ensure};

pub(crate) fn options<'a>(command: &'a str, machine: &str) -> Result<&'a str> {
    validate_machine(machine)?;
    let command = command.trim();
    ensure!(!command.contains(['\n', '\r']), "Invalid MAME command line");
    if let Some(legacy) = command.strip_prefix("mame ") {
        // Accept previously generated sessions without rewriting their files.
        return legacy
            .strip_suffix(machine)
            .filter(|prefix| prefix.ends_with(char::is_whitespace))
            .map(str::trim)
            .context("MAME command belongs to a different machine");
    }
    command
        .strip_prefix(machine)
        .filter(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
        .map(str::trim)
        .context("Unsupported MAME command for this machine")
}

pub(crate) fn build(machine: &str, options: &str) -> Result<String> {
    validate_machine(machine)?;
    ensure!(
        !options.contains(['\n', '\r']),
        "Invalid MAME command options"
    );
    let command = format!("{machine} {}\n", options.trim());
    ensure!(
        command.len() <= 4095,
        "MAME command exceeds the core's command limit"
    );
    Ok(command)
}

fn validate_machine(machine: &str) -> Result<()> {
    ensure!(
        !machine.is_empty()
            && machine.len() <= 64
            && machine
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
        "Invalid MAME machine name"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_is_first_for_all_orientations_and_session_options_survive() {
        for machine in ["ddonpach", "1942", "pacman", "mslug", "simpsons2p"] {
            let opts = "-state \"/saves/resume.sta\" -rompath \"/ROMs (non-merged)\" -noautosave";
            let legacy = format!("mame {opts} {machine}\n");
            let normalized = build(machine, options(&legacy, machine).unwrap()).unwrap();
            assert_eq!(normalized.split_whitespace().next(), Some(machine));
            assert_eq!(options(&normalized, machine).unwrap(), opts);
            assert_eq!(
                build(machine, options(&normalized, machine).unwrap()).unwrap(),
                normalized
            );
        }
    }

    #[test]
    fn mismatched_and_multiline_commands_are_rejected() {
        for command in [
            "mame -noautosave other",
            "other -noautosave",
            "mslug2 -noautosave",
            "mslug\n-reset",
        ] {
            assert!(options(command, "mslug").is_err());
        }
        assert!(build("bad machine", "-noautosave").is_err());
        assert!(build("mslug", "-noautosave\n-reset").is_err());
    }
}
