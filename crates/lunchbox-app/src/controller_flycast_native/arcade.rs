//! Standard Naomi/JVS panel routing from pinned maple_jvs.cpp.
//! Geometry is generic; no cabinet-specific controls are inspected or invented.
use super::Input;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Panel {
    #[default]
    Six,
    Eight,
}

impl Panel {
    pub(crate) fn layout(self) -> &'static str {
        match self {
            Self::Six => "arcade-six-button",
            Self::Eight => "arcade-eight-button",
        }
    }

    pub(crate) fn routes(self) -> BTreeMap<String, String> {
        let mut routes: BTreeMap<_, _> = [
            ("up", "btn_dpad1_up"),
            ("down", "btn_dpad1_down"),
            ("left", "btn_dpad1_left"),
            ("right", "btn_dpad1_right"),
            ("start", "btn_start"),
            ("coin", "btn_d"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
        let actions = [
            "btn_a",
            "btn_b",
            "btn_c",
            "btn_x",
            "btn_y",
            "btn_z",
            "btn_dpad2_left",
            "btn_dpad2_right",
        ];
        for (index, output) in actions
            .iter()
            .take(if self == Self::Six { 6 } else { 8 })
            .enumerate()
        {
            routes.insert(format!("button{}", index + 1), (*output).to_owned());
        }
        routes
    }

    pub(crate) fn native_controls(
        self,
        physical: &BTreeMap<String, Input>,
    ) -> Result<BTreeMap<String, Input>> {
        let routes = self.routes();
        ensure!(
            physical.len() == routes.len() && routes.keys().all(|key| physical.contains_key(key)),
            "Flycast panel requires directions, Start, Coin and every selected action button"
        );
        Ok(routes
            .into_iter()
            .map(|(target, output)| (output, physical[&target]))
            .collect())
    }
}
