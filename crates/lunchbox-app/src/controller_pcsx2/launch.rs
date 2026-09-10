//! Native Qt launch-plan preparation; runtime/content identities are caller-owned.
use super::{data_tree::PreparedData, prepared::Controller, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub(crate) struct PreparedLaunch {
    plan: LaunchPlan,
    data: PreparedData,
    executable_hash: String,
    content: PathBuf,
    content_hash: String,
    log: tempfile::NamedTempFile,
}

fn reject_portable(executable: &Path) -> Result<()> {
    let root = executable
        .parent()
        .ok_or_else(|| anyhow::anyhow!("PCSX2 executable has no directory"))?;
    for name in ["portable.ini", "portable.txt"] {
        ensure!(
            !root.join(name).try_exists()?,
            "PCSX2 portable marker overrides private -datapath selection"
        );
    }
    Ok(())
}

impl PreparedLaunch {
    pub(crate) fn create(
        setup: &SavedSetup,
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        calibrations: &HashMap<String, Calibration>,
        controllers: &[Controller<'_>],
    ) -> Result<Self> {
        setup.validate()?;
        let native = setup.native.as_ref().ok_or_else(|| {
            anyhow::anyhow!("PCSX2 launch needs native data_root, serial and disc CRC")
        })?;
        ensure!(
            setup.emulator_id == option.emulator_id,
            "PCSX2 selected emulator differs from saved setup"
        );
        let EmulatorExecutable::Native(selected) = &option.executable else {
            anyhow::bail!("PCSX2 mapping requires native Qt executable");
        };
        let executable = selected.canonicalize()?;
        ensure!(
            original.program.canonicalize()? == executable
                && original.arguments.len() == 1
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "PCSX2 mapped launch requires a plain native content argument without custom environment"
        );
        reject_portable(&executable)?;
        let executable_hash = file_hash(&executable)?;
        ensure!(
            executable_hash.eq_ignore_ascii_case(&setup.executable_sha256),
            "PCSX2 executable differs from trusted hash"
        );
        let content = setup.content.canonicalize()?;
        let argument = PathBuf::from(&original.arguments[0]);
        let argument = if argument.is_absolute() {
            argument
        } else {
            original.current_directory.join(argument)
        };
        ensure!(
            argument.canonicalize()? == content && content.is_file(),
            "PCSX2 selected content differs from saved setup"
        );
        let content_hash = file_hash(&content)?;
        let profile = super::prepared::controller_profile(setup, calibrations, controllers)?;
        let data = PreparedData::create(
            &setup.source_config,
            &native.data_root,
            &native.serial,
            native.crc,
            &profile,
        )?;
        let mut plan = original.clone();
        let log = tempfile::Builder::new()
            .prefix("lunchbox-pcsx2-startup-")
            .tempfile()?;
        plan.program = executable;
        plan.environment
            .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        plan.arguments = vec![
            "-datapath".into(),
            data.directory().as_os_str().to_owned(),
            "-logfile".into(),
            log.path().as_os_str().to_owned(),
            "--".into(),
            content.as_os_str().to_owned(),
        ];
        let launch = Self {
            plan,
            data,
            executable_hash,
            content,
            content_hash,
            log,
        };
        launch.verify_before_launch()?;
        Ok(launch)
    }

    pub(crate) fn plan(&self) -> &LaunchPlan {
        &self.plan
    }

    pub(crate) fn startup_log(&self) -> Result<String> {
        use std::io::Read;
        let file = std::fs::File::open(self.log.path())?;
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 4 * 1024 * 1024,
            "PCSX2 startup log exceeds limit"
        );
        let end = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        Ok(std::str::from_utf8(&bytes[..end])?.to_owned())
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        reject_portable(&self.plan.program)?;
        ensure!(
            file_hash(&self.plan.program)? == self.executable_hash
                && file_hash(&self.content)? == self.content_hash,
            "PCSX2 executable or content changed during preparation"
        );
        self.data.verify_before_launch()
    }
}
