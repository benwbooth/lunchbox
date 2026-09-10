//! Native launch preparation; the session must verify runtime directory/routing.
use super::{isolation::PreparedProfile, prepared::Controller, settings::SavedSetup};
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
    profile: PreparedProfile,
    executable_hash: String,
    content: PathBuf,
    content_hash: String,
    directory: super::paths::ConfigDirectory,
}

impl PreparedLaunch {
    /// Resolve the native Linux input directory; caller still verifies native selection.
    pub(crate) fn create(
        setup: &SavedSetup,
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        calibrations: &HashMap<String, Calibration>,
        controllers: &[Controller<'_>],
    ) -> Result<Self> {
        setup.validate()?;
        ensure!(
            setup.emulator_id == option.emulator_id,
            "RPCS3 selected emulator differs from saved setup"
        );
        let EmulatorExecutable::Native(selected) = &option.executable else {
            anyhow::bail!("RPCS3 mapping requires a native executable");
        };
        let executable = selected.canonicalize()?;
        ensure!(
            original.program.canonicalize()? == executable
                && original.arguments.len() == 1
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "RPCS3 mapped launch requires one native boot path without custom environment"
        );
        let executable_hash = file_hash(&executable)?;
        ensure!(
            executable_hash.eq_ignore_ascii_case(&setup.executable_sha256),
            "RPCS3 executable differs from trusted hash"
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
            "RPCS3 preparation currently requires an exact file boot target; directory boot preparation is pending"
        );
        let content_hash = file_hash(&content)?;
        let yaml = super::prepared::controller_profile(setup, calibrations, controllers)?;
        let directory =
            super::paths::ConfigDirectory::capture(&executable, &original.current_directory)?;
        let profile = PreparedProfile::create(&directory.input_directory(), yaml)?;
        let mut plan = original.clone();
        plan.program = executable;
        plan.environment
            .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        plan.arguments = profile.arguments().into_iter().collect();
        plan.arguments.push("--".into());
        plan.arguments.push(content.as_os_str().to_owned());
        let prepared = Self {
            plan,
            profile,
            executable_hash,
            content,
            content_hash,
            directory,
        };
        prepared.verify_before_launch()?;
        Ok(prepared)
    }

    pub(crate) fn plan(&self) -> &LaunchPlan {
        &self.plan
    }

    pub(crate) fn profile_path(&self) -> &Path {
        self.profile.path()
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        self.directory.verify()?;
        ensure!(
            file_hash(&self.plan.program)? == self.executable_hash
                && file_hash(&self.content)? == self.content_hash,
            "RPCS3 executable or content changed during preparation"
        );
        self.profile.verify()
    }
}
