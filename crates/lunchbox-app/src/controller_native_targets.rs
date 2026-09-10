//! Guided targets backed by the existing standalone writers, never RetroPad
//! aliases. Keep native vocabulary in the writer's single route table.
use crate::controller_catalog::{Catalog, EmulatorProfile};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) fn add_profiles(db: &mut Catalog) -> Result<()> {
    let mut ds = db
        .layout("nds-stylus-controls")
        .context("Missing DS geometry")?
        .clone();
    ds.id = "nds-native-buttons".into();
    ds.name = "Nintendo DS — buttons (touchscreen uses mouse)".into();
    ds.notes = "Native melonDS button controls. Touchscreen, microphone and lid are not mapped by this contract.".into();
    let routes = crate::controller_melonds::visual_routes();
    ds.controls
        .retain(|control| routes.contains_key(control.id.as_str()));
    db.layouts.push(ds);
    for (core, layout, players, platforms, source) in [
        (
            "pcsx2",
            "dualshock",
            8,
            vec!["Sony Playstation 2"],
            "https://github.com/PCSX2/pcsx2/tree/98697735f1bb1a1452d975251269abd1019876d1/pcsx2/SIO/Pad",
        ),
        (
            "rpcs3",
            "dualshock",
            7,
            vec!["Sony Playstation 3"],
            "https://github.com/RPCS3/rpcs3/tree/54014a7de4b2ccec98c9c0cb7dbebec0606c5cd6/rpcs3/Input",
        ),
        (
            "melonds",
            "nds-native-buttons",
            1,
            vec!["Nintendo DS"],
            "https://github.com/melonDS-emu/melonDS/tree/906e9ebb27da8c6a715cd7abab4abfe8a8d29427/src/frontend/qt_sdl",
        ),
    ] {
        add(db, core, layout, players, &platforms, source)?;
    }
    for layout in ["arcade-six-button", "arcade-eight-button"] {
        add(
            db,
            "flycast",
            layout,
            4,
            &["Arcade", "Sega Naomi", "Sega Naomi 2", "Sammy Atomiswave"],
            "https://github.com/flyinghead/flycast/tree/fb286f777ce690ef8acf3359a75ab84b61566ad9/core/input",
        )?;
        add(
            db,
            "mame",
            layout,
            8,
            &["Arcade"],
            "https://github.com/mamedev/mame/blob/ec9abd86c6c9029f67e9cf4908ef5426b78d3eab/docs/source/advanced/ctrlr_config.rst",
        )?;
    }
    crate::controller_bizhawk::guided::add_profiles(db)?;
    Ok(())
}

pub(crate) fn add(
    db: &mut Catalog,
    core: &str,
    layout: &str,
    players: usize,
    platforms: &[&str],
    source: &str,
) -> Result<()> {
    let bindings = routes(core, layout).context("Unknown native target contract")?;
    let name = db
        .layout(layout)
        .context("Missing native target layout")?
        .name
        .clone();
    db.emulator_profiles.push(serde_json::from_value(serde_json::json!({
        "id": format!("{core}:standalone-{layout}"),
        "name": format!("{core} · {name}"), "core": core, "target_layout": layout,
        "transport": format!("{core}-native-settings"), "status": "documented",
        "source": source, "native_launch": {"platforms": platforms, "max_players": players},
        "conditions": ["Guided players and button choices feed the native writer. A matching Linux runtime setup and physical calibration are required. Source implementation only; runtime compatibility is unverified."],
        "bindings": bindings
    }))?);
    Ok(())
}

fn routes(core: &str, layout: &str) -> Option<BTreeMap<String, String>> {
    if core == "bizhawk" {
        return crate::controller_bizhawk::guided::routes(layout);
    }
    if core == "mame" {
        let panel = match layout {
            "arcade-six-button" => crate::controller_mame_native::Panel::Six,
            "arcade-eight-button" => crate::controller_mame_native::Panel::Eight,
            _ => return None,
        };
        // Preview shows the P1 vocabulary; the writer expands the same routes
        // with the actual assigned player number for every native port.
        return Some(
            panel
                .routes(1)
                .into_iter()
                .map(|(target, output)| {
                    (
                        if target == "coin" {
                            "select".into()
                        } else {
                            target
                        },
                        output,
                    )
                })
                .collect(),
        );
    }
    if core == "flycast" {
        let panel = match layout {
            "arcade-six-button" => crate::controller_flycast_native::arcade::Panel::Six,
            "arcade-eight-button" => crate::controller_flycast_native::arcade::Panel::Eight,
            _ => return None,
        };
        return Some(
            panel
                .routes()
                .into_iter()
                .map(|(target, output)| {
                    (
                        if target == "coin" {
                            "select".into()
                        } else {
                            target
                        },
                        output,
                    )
                })
                .collect(),
        );
    }
    let routes = match (core, layout) {
        ("pcsx2", "dualshock") => crate::controller_pcsx2::visual_routes(),
        ("rpcs3", "dualshock") => crate::controller_rpcs3::visual_routes(),
        ("melonds", "nds-native-buttons") => crate::controller_melonds::visual_routes(),
        _ => return None,
    };
    Some(
        routes
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect(),
    )
}

pub(crate) fn validate(profile: &EmulatorProfile) -> Result<()> {
    ensure!(
        profile.transport == format!("{}-native-settings", profile.core)
            && profile.native_launch.is_some()
            && profile.retroarch_launch.is_none(),
        "Native target has the wrong transport"
    );
    ensure!(
        routes(&profile.core, &profile.target_layout).as_ref() == Some(&profile.bindings),
        "Native target must exactly match its writer's control routes"
    );
    Ok(())
}

pub(crate) fn valid_output(profile: &EmulatorProfile, target: &str, output: &str) -> bool {
    routes(&profile.core, &profile.target_layout)
        .and_then(|routes| routes.get(target).cloned())
        .as_deref()
        == Some(output)
}
