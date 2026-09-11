//! Reuse an explicit native runtime declaration for a newly selected game.
//! Only launch-local copies are produced. Exact per-game setups take precedence.
//! Unknown future fields remain part of the identity comparison by default.
use crate::{emulator::LaunchPlan, settings::ControllerMappingSettings};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::path::PathBuf;

fn template<T: Clone + Serialize>(
    setups: &[T],
    emulator: &str,
    player_fields: &[&str],
) -> Result<Option<T>> {
    let mut selected: Option<(serde_json::Value, T)> = None;
    for setup in setups {
        let mut identity = serde_json::to_value(setup)?;
        if identity["emulator_id"].as_str() != Some(emulator) {
            continue;
        }
        let fields = identity
            .as_object_mut()
            .context("Native runtime declaration must be an object")?;
        fields.remove("content");
        for key in player_fields {
            fields.remove(*key);
        }
        if let Some((previous, _)) = &selected {
            ensure!(
                previous == &identity,
                "This emulator has conflicting runtime configurations. Choose a runtime setup for this game before launching; no existing setup was changed."
            );
        } else {
            selected = Some((identity, setup.clone()));
        }
    }
    Ok(selected.map(|(_, setup)| setup))
}

fn content(plan: &LaunchPlan, flags: &[&str]) -> Result<PathBuf> {
    let arguments: Vec<_> = plan
        .arguments
        .iter()
        .filter(|arg| !arg.to_str().is_some_and(|arg| flags.contains(&arg)))
        .collect();
    ensure!(
        arguments.len() == 1,
        "Automatic native runtime reuse requires one game file; custom launch arguments need an explicit setup"
    );
    let path = PathBuf::from(arguments[0]);
    ensure!(
        path.is_absolute() && path.is_file(),
        "Native runtime reuse requires the selected game's absolute file path"
    );
    Ok(path)
}

pub(super) fn reuse(
    mapping: &mut ControllerMappingSettings,
    core: &str,
    emulator: &str,
    plan: &LaunchPlan,
) -> Result<()> {
    let matches = |id: &str, path: &std::path::Path| {
        id == emulator && plan.arguments.iter().any(|arg| arg == path.as_os_str())
    };
    // Every field excluded below is explicitly overwritten by the guided
    // player/target bridge. Runtime/config paths, dependencies, ABI and hashes
    // are never excluded, guessed, or silently selected from conflicting setups.
    match core {
        "mgba"
            if !mapping
                .mgba_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.mgba_launches,
                emulator,
                &["controller_id", "handheld"],
            )? {
                setup.content = content(plan, &["-f", "--fullscreen"])?;
                mapping.mgba_launches.push(setup);
            }
        }
        "fceux"
            if !mapping
                .fceux_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.fceux_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.fceux_launches.push(setup);
            }
        }
        "scummvm"
            if !mapping
                .scummvm_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.scummvm_native_launches,
                emulator,
                &["controller_id"],
            )? {
                setup.content = content(plan, &[])?;
                mapping.scummvm_native_launches.push(setup);
            }
        }
        "jgenesis"
            if !mapping
                .jgenesis_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.jgenesis_native_launches,
                emulator,
                &["controller_id"],
            )? {
                setup.content = content(plan, &[])?;
                mapping.jgenesis_native_launches.push(setup);
            }
        }
        "xemu"
            if !mapping
                .xemu_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.xemu_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.xemu_native_launches.push(setup);
            }
        }
        "blastem"
            if !mapping
                .blastem_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.blastem_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.blastem_native_launches.push(setup);
            }
        }
        "mesen2"
            if !mapping
                .mesen2_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.mesen2_native_launches,
                emulator,
                &["controller_id"],
            )? {
                setup.content = content(plan, &[])?;
                mapping.mesen2_native_launches.push(setup);
            }
        }
        "openmsx"
            if !mapping
                .openmsx_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.openmsx_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.openmsx_native_launches.push(setup);
            }
        }
        "desmume"
            if !mapping
                .desmume_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.desmume_native_launches,
                emulator,
                &["controller_id"],
            )? {
                setup.content = content(plan, &[])?;
                mapping.desmume_native_launches.push(setup);
            }
        }
        "hatari"
            if !mapping
                .hatari_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.hatari_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.hatari_native_launches.push(setup);
            }
        }
        "vice"
            if !mapping
                .vice_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.vice_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.vice_native_launches.push(setup);
            }
        }
        "stella"
            if !mapping
                .stella_native_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) =
                template(&mapping.stella_native_launches, emulator, &["players"])?
            {
                setup.content = content(plan, &[])?;
                mapping.stella_native_launches.push(setup);
            }
        }
        "bsnes"
            if !mapping
                .bsnes_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.bsnes_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.bsnes_launches.push(setup);
            }
        }
        "snes9x"
            if !mapping
                .snes9x_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.snes9x_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.snes9x_launches.push(setup);
            }
        }
        "sameboy"
            if !mapping
                .sameboy_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.sameboy_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.sameboy_launches.push(setup);
            }
        }
        "mednafen"
            if !mapping
                .mednafen_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.mednafen_launches,
                emulator,
                &[
                    "players",
                    "gamepad",
                    "md_tap",
                    "psx_multitaps",
                    "saturn_multitaps",
                ],
            )? {
                setup.content = content(plan, &[])?;
                mapping.mednafen_launches.push(setup);
            }
        }
        "rpcs3"
            if !mapping
                .rpcs3_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.rpcs3_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.rpcs3_launches.push(setup);
            }
        }
        "melonds"
            if !mapping
                .melonds_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(&mapping.melonds_launches, emulator, &["players"])? {
                setup.content = content(plan, &[])?;
                mapping.melonds_launches.push(setup);
            }
        }
        "dolphin"
            if !mapping
                .dolphin_launches
                .iter()
                .any(|s| matches(&s.emulator_id, &s.content)) =>
        {
            if let Some(mut setup) = template(
                &mapping.dolphin_launches,
                emulator,
                &[
                    "players",
                    "game_id",
                    "revision",
                    "resolve_devices_at_launch",
                ],
            )? {
                setup.content = content(plan, &["-e", "--exec", "-b", "--batch"])?;
                // Use the actual new disc header. Never copy another game's
                // identity or derive it from the library title or filename.
                let disc = crate::controller_dolphin::prepare_raw_content(&setup.content)?;
                let (game_id, revision) = disc.game_identity();
                setup.game_id = std::str::from_utf8(&game_id)?.into();
                setup.revision = revision;
                mapping.dolphin_launches.push(setup);
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_snes9x::settings::Player as Snes9xPlayer;
    use crate::controller_snes9x::settings::SavedSetup as Snes9xSetup;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;

    fn plan(arguments: Vec<OsString>) -> LaunchPlan {
        LaunchPlan {
            emulator_name: "Test".into(),
            program: PathBuf::from("/usr/bin/emulator"),
            arguments,
            current_directory: PathBuf::from("/tmp"),
            environment: Vec::new(),
            cleanup_paths: Vec::new(),
            retroarch_content: None,
        }
    }

    fn game_file(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, b"content").unwrap();
        path
    }

    fn snes9x_setup(emulator_id: &str, content: &Path, hash: &str) -> Snes9xSetup {
        Snes9xSetup {
            emulator_id: emulator_id.into(),
            content: content.to_path_buf(),
            source_config: PathBuf::from("/etc/snes9x/snes9x.conf"),
            probe_program: PathBuf::from("/usr/bin/lunchbox-probe"),
            sdl_library: PathBuf::from("/usr/lib/libSDL2-2.0.so.0"),
            bubblewrap_program: PathBuf::from("/usr/bin/bwrap"),
            executable_sha256: hash.into(),
            players: vec![Snes9xPlayer {
                player: 1,
                controller_id: "controller-a".into(),
            }],
        }
    }

    #[test]
    fn reuses_runtime_for_another_game_of_the_same_emulator() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        let launch = plan(vec![second.clone().into_os_string()]);
        reuse(&mut mapping, "snes9x", "snes9x", &launch).unwrap();
        assert_eq!(mapping.snes9x_launches.len(), 2);
        assert_eq!(mapping.snes9x_launches[0].content, first);
        let reused = &mapping.snes9x_launches[1];
        assert_eq!(reused.content, second);
        assert_eq!(
            reused.source_config,
            PathBuf::from("/etc/snes9x/snes9x.conf")
        );
        assert_eq!(reused.executable_sha256, "a");
    }

    #[test]
    fn exact_per_game_setup_takes_precedence_over_reuse() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &second, "b"));
        let launch = plan(vec![second.clone().into_os_string()]);
        reuse(&mut mapping, "snes9x", "snes9x", &launch).unwrap();
        assert_eq!(mapping.snes9x_launches.len(), 2);
        assert_eq!(mapping.snes9x_launches[1].executable_sha256, "b");
    }

    #[test]
    fn another_emulators_setup_is_not_reused() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        let launch = plan(vec![second.into_os_string()]);
        reuse(&mut mapping, "snes9x", "other-snes9x", &launch).unwrap();
        assert_eq!(mapping.snes9x_launches.len(), 1);
    }

    #[test]
    fn conflicting_runtime_details_require_an_explicit_choice() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let third = game_file(directory.path(), "third.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &second, "b"));
        let launch = plan(vec![third.into_os_string()]);
        let error = reuse(&mut mapping, "snes9x", "snes9x", &launch)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("conflicting runtime configurations"),
            "unexpected error: {error}"
        );
        assert_eq!(mapping.snes9x_launches.len(), 2);
    }

    #[test]
    fn differences_only_in_replaced_fields_do_not_conflict() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let third = game_file(directory.path(), "third.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        let mut other = snes9x_setup("snes9x", &second, "a");
        other.players = vec![Snes9xPlayer {
            player: 2,
            controller_id: "controller-b".into(),
        }];
        mapping.snes9x_launches.push(other);
        let launch = plan(vec![third.clone().into_os_string()]);
        reuse(&mut mapping, "snes9x", "snes9x", &launch).unwrap();
        assert_eq!(mapping.snes9x_launches.len(), 3);
        assert_eq!(mapping.snes9x_launches[2].content, third);
    }

    #[test]
    fn mgba_launch_flags_are_not_mistaken_for_content() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.gba");
        let second = game_file(directory.path(), "second.gba");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .mgba_launches
            .push(crate::controller_mgba::settings::SavedSetup {
                emulator_id: "mgba".into(),
                content: first,
                handheld: crate::controller_mgba::settings::Handheld::Gba,
                controller_id: "controller-a".into(),
                source_config: PathBuf::from("/etc/mgba/config.ini"),
                probe_program: PathBuf::from("/usr/bin/lunchbox-probe"),
                sdl_library: PathBuf::from("/usr/lib/libSDL2-2.0.so.0"),
                bubblewrap_program: PathBuf::from("/usr/bin/bwrap"),
                executable_sha256: "a".into(),
            });
        let launch = plan(vec!["-f".into(), second.clone().into_os_string()]);
        reuse(&mut mapping, "mgba", "mgba", &launch).unwrap();
        assert_eq!(mapping.mgba_launches.len(), 2);
        assert_eq!(mapping.mgba_launches[1].content, second);
    }

    #[test]
    fn custom_arguments_need_an_explicit_setup() {
        let directory = tempfile::tempdir().unwrap();
        let first = game_file(directory.path(), "first.sfc");
        let second = game_file(directory.path(), "second.sfc");
        let third = game_file(directory.path(), "third.sfc");
        let mut mapping = ControllerMappingSettings::default();
        mapping
            .snes9x_launches
            .push(snes9x_setup("snes9x", &first, "a"));
        let launch = plan(vec![second.into_os_string(), third.into_os_string()]);
        let error = reuse(&mut mapping, "snes9x", "snes9x", &launch)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("requires one game file"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn dolphin_identity_is_recomputed_from_the_new_disc() {
        let directory = tempfile::tempdir().unwrap();
        let mut disc = vec![0u8; 0x448];
        disc[0..6].copy_from_slice(b"GM4E01");
        disc[7] = 0x2a;
        disc[0x1c..0x20].copy_from_slice(&0xc2339f3du32.to_be_bytes());
        disc[0x424..0x428].copy_from_slice(&0x430u32.to_be_bytes());
        disc[0x428..0x42c].copy_from_slice(&24u32.to_be_bytes());
        // Minimal filesystem table: one root directory, empty string table.
        disc[0x430] = 1;
        disc[0x438..0x43c].copy_from_slice(&1u32.to_be_bytes());
        let second = directory.path().join("second.iso");
        fs::write(&second, disc).unwrap();
        let first = game_file(directory.path(), "first.iso");
        let mut mapping = ControllerMappingSettings::default();
        mapping.dolphin_launches.push(
            crate::controller_dolphin::standalone::settings::SavedSetup {
                resolve_devices_at_launch: true,
                emulator_id: "dolphin".into(),
                content: first,
                game_id: "GALE01".into(),
                revision: 0,
                user_directory: PathBuf::from("/home/ben/.local/share/dolphin"),
                system_directory: PathBuf::from("/usr/share/dolphin"),
                bubblewrap_program: PathBuf::from("/usr/bin/bwrap"),
                executable_sha256: "a".into(),
                players: vec![crate::controller_dolphin::standalone::settings::Player {
                    port: 1,
                    controller_id: "controller-a".into(),
                    device_qualifier: String::new(),
                }],
            },
        );
        let launch = plan(vec!["-b".into(), second.clone().into_os_string()]);
        reuse(&mut mapping, "dolphin", "dolphin", &launch).unwrap();
        assert_eq!(mapping.dolphin_launches.len(), 2);
        let reused = &mapping.dolphin_launches[1];
        assert_eq!(reused.content, second);
        assert_eq!(reused.game_id, "GM4E01");
        assert_eq!(reused.revision, 0x2a);
        assert_eq!(reused.executable_sha256, "a");
    }
}
