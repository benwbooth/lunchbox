//! Native Linux launch composition; does not spawn a child or infer SDL IDs.
use super::{
    prepared::{Controller, PreparedPanel},
    settings::SavedSetup,
};
use crate::{
    controller_catalog::Calibration,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{collections::HashMap, path::PathBuf};

pub(crate) struct PreparedLaunch {
    plan: LaunchPlan,
    panel: PreparedPanel,
    executable_hash: String,
    content: PathBuf,
    content_hash: String,
}

impl PreparedLaunch {
    pub(crate) fn create(
        setup: &SavedSetup,
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        calibrations: &HashMap<String, Calibration>,
        controllers: &[Controller<'_>],
        observed_native_instances: &[i32],
    ) -> Result<Self> {
        setup.validate()?;
        ensure!(
            setup.emulator_id == option.emulator_id,
            "Flycast emulator differs from saved setup"
        );
        let EmulatorExecutable::Native(selected) = &option.executable else {
            anyhow::bail!("Flycast mapped launch requires a native executable");
        };
        let executable = selected.canonicalize()?;
        ensure!(
            original.program.canonicalize()? == executable,
            "Flycast launch executable differs from selected emulator"
        );
        ensure!(
            original.arguments.len() == 1
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Flycast mapped preparation requires a plain native content launch without custom arguments or environment"
        );
        let content = setup.content.canonicalize()?;
        let argument = PathBuf::from(&original.arguments[0]);
        let resolved = if argument.is_absolute() {
            argument
        } else {
            original.current_directory.join(argument)
        };
        ensure!(
            resolved.canonicalize()? == content && content.is_file(),
            "Flycast launch content differs from saved setup"
        );
        let executable_hash = file_hash(&executable)?;
        ensure!(
            executable_hash.eq_ignore_ascii_case(&setup.executable_sha256),
            "Flycast executable differs from trusted hash"
        );
        let content_hash = file_hash(&content)?;
        let panel =
            PreparedPanel::create(setup, calibrations, controllers, observed_native_instances)?;
        let mut plan = original.clone();
        plan.program = executable;
        plan.arguments = vec![content.as_os_str().to_owned()];
        panel.config.select_for_launch(&mut plan)?;
        let launch = Self {
            plan,
            panel,
            executable_hash,
            content,
            content_hash,
        };
        launch.verify_before_launch()?;
        Ok(launch)
    }

    pub(crate) fn plan(&self) -> &LaunchPlan {
        &self.plan
    }

    pub(crate) fn verify_before_launch(&self) -> Result<()> {
        ensure!(
            file_hash(&self.plan.program)? == self.executable_hash,
            "Flycast executable changed during preparation"
        );
        ensure!(
            file_hash(&self.content)? == self.content_hash,
            "Flycast content changed during preparation"
        );
        self.panel.verify_before_launch()
    }
}
