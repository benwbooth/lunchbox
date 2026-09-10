//! Prepared launch composition. Native inventory capture/child handoff remain
//! caller-owned; construction does not run an emulator or any probe.
use super::{config_copy::PreparedConfigs, configuration::PreparedProfile, settings::SavedSetup};
use crate::{
    controller_catalog::Calibration,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{file_hash, sdl2::Device};
use std::{collections::HashMap, path::PathBuf};

pub(crate) struct PreparedLaunch {
    pub plan: LaunchPlan,
    profile: PreparedProfile,
    configs: PreparedConfigs,
    executable: PathBuf,
    executable_hash: String,
}

impl PreparedLaunch {
    pub(crate) fn create(
        setup: &SavedSetup,
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        calibrations: &HashMap<String, Calibration>,
        physical_paths: &HashMap<String, String>,
        devices: &[Device],
    ) -> Result<Self> {
        setup.validate()?;
        ensure!(
            setup.emulator_id == option.emulator_id,
            "MAME selected emulator differs from saved setup"
        );
        let EmulatorExecutable::Native(selected) = &option.executable else {
            anyhow::bail!("MAME raw SDL preparation requires a native executable");
        };
        let executable = setup.executable.canonicalize()?;
        ensure!(
            selected.canonicalize()? == executable
                && original.program.canonicalize()? == executable,
            "MAME executable differs from saved setup"
        );
        ensure!(
            original.environment.is_empty()
                && original.arguments.len() == 1
                && original.retroarch_content.is_none(),
            "MAME preparation currently requires a plain native machine argument with no custom environment"
        );
        let machine = original.arguments[0]
            .to_str()
            .context("MAME machine argument must be UTF-8")?;
        let executable_hash = file_hash(&executable)?;
        ensure!(
            executable_hash.eq_ignore_ascii_case(&setup.executable_sha256),
            "MAME executable differs from trusted hash"
        );
        let source_cfg = setup
            .cfg_directory
            .as_ref()
            .context("Declare the original MAME cfg_directory before preparing a mapped launch")?;
        let xml = super::prepared::controller_xml(setup, calibrations, physical_paths, devices)?;
        let profile = PreparedProfile::create(setup, xml)?;
        let configs = PreparedConfigs::create(setup, source_cfg, machine)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.arguments.extend(profile.arguments());
        plan.arguments.extend(configs.arguments());
        let launch = Self {
            plan,
            profile,
            configs,
            executable,
            executable_hash,
        };
        launch.verify_before_launch()?;
        Ok(launch)
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        ensure!(
            self.plan.program == self.executable
                && file_hash(&self.executable)? == self.executable_hash,
            "MAME runtime changed during preparation"
        );
        self.profile.verify()?;
        self.configs.verify_before_launch()
    }

    pub(crate) fn controller_profile(&self) -> &std::path::Path {
        self.profile.path()
    }
}
