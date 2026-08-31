use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

pub fn is_flatpak() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").is_file())
}

pub fn host_command(program: impl AsRef<OsStr>) -> Command {
    if is_flatpak() {
        let mut command = Command::new("flatpak-spawn");
        command.arg("--host").arg(program);
        command
    } else {
        Command::new(program)
    }
}

pub fn host_program_available(program: &str) -> bool {
    if !is_flatpak() {
        return false;
    }
    host_command(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_processes_are_not_rewritten() {
        if is_flatpak() {
            return;
        }
        let command = host_command("lunchbox-test-command");
        assert_eq!(command.get_program(), "lunchbox-test-command");
        assert_eq!(command.get_args().count(), 0);
    }
}
