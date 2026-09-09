//! Model recognition is separate from physical-instance identity and calibration.
use crate::controllers::ControllerDevice;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize)]
pub(crate) struct Model {
    pub id: String,
    pub name: String,
    pub device_name: String,
    pub source: String,
    pub os: String,
    pub driver: String,
    pub vendor: Option<u16>,
    pub product: Option<u16>,
    pub bus: Option<u16>,
    pub version: Option<u16>,
    pub bindings: BTreeMap<String, String>,
}
#[derive(Deserialize)]
struct Database {
    models: Vec<Model>,
}
pub(crate) fn models() -> &'static [Model] {
    static DB: OnceLock<Database> = OnceLock::new();
    &DB.get_or_init(|| {
        serde_json::from_str(include_str!("../data/controller-models/models.json"))
            .expect("validated controller model database")
    })
    .models
}
pub(crate) fn model(id: &str) -> Option<&'static Model> {
    models().iter().find(|model| model.id == id)
}
fn normalized(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn hex(value: Option<&String>) -> Option<u16> {
    u16::from_str_radix(value?, 16).ok()
}
fn hardware_matches(model: &Model, device: &ControllerDevice) -> bool {
    model.vendor.is_some()
        && model.product.is_some()
        && model.vendor == hex(device.vendor_id.as_ref())
        && model.product == hex(device.product_id.as_ref())
        && model
            .bus
            .is_none_or(|value| Some(value) == hex(device.bus_type.as_ref()))
        && model
            .version
            .is_none_or(|value| Some(value) == hex(device.version.as_ref()))
}
pub(crate) fn detected<'a>(
    rows: &'a [Model],
    device: &ControllerDevice,
    os: &str,
) -> Option<&'a Model> {
    if device.is_virtual {
        return None;
    }
    let name = normalized(&device.name);
    // Common protocol aliases are not proof of the controller's physical model.
    if name.contains("xbox") || name.contains("xinput") || name.contains("generic") {
        return None;
    }
    let exact: Vec<_> = rows
        .iter()
        .filter(|model| {
            model.os == os
                && hardware_matches(model, device)
                && normalized(&model.device_name) == name
        })
        .collect();
    let sdl: Vec<_> = exact
        .iter()
        .copied()
        .filter(|model| model.source == "SDL")
        .collect();
    if sdl.len() == 1 {
        return Some(sdl[0]);
    }
    if exact.len() == 1 {
        Some(exact[0])
    } else {
        None
    }
}
pub(crate) fn summary(model: &Model) -> Value {
    json!({"id":model.id,"name":model.name,"device_name":model.device_name,"source":model.source,"os":model.os,"driver":model.driver,"mapping_entries":model.bindings.len()})
}
pub(crate) fn review(
    device: &ControllerDevice,
    saved: Option<&str>,
    query: &str,
    os: &str,
) -> Value {
    let selected = saved.and_then(model);
    let automatic = detected(models(), device, os);
    let query = normalized(query);
    let mut rows: Vec<_> = models()
        .iter()
        .filter(|model| {
            normalized(&model.name).contains(&query)
                || normalized(&model.device_name).contains(&query)
        })
        .collect();
    rows.sort_by_cached_key(|model| (normalized(&model.name), model.os.clone(), model.id.clone()));
    let unique_id = device
        .unique_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    json!({"selected":selected.map(summary),"detected":automatic.map(summary),"candidates":rows.iter().map(|m| summary(m)).collect::<Vec<_>>(),"total":rows.len(),"device_name":device.name,
        "hardware_unique_id":unique_id,"connection_id":device.stable_id,
        "message": if selected.is_some() { "Model selected by you" } else if automatic.is_some() { "Detected from hardware identity and reported name" } else { "Choose your model; this device identity is not conclusive" }})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device() -> ControllerDevice {
        ControllerDevice {
            stable_id: "unit-one".into(),
            name: "8BitDo Test".into(),
            device_path: "/dev/input/js0".into(),
            event_paths: vec![],
            vendor_id: Some("2dc8".into()),
            product_id: Some("5006".into()),
            version: Some("0100".into()),
            bus_type: Some("0003".into()),
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        }
    }
    fn row(id: &str) -> Model {
        Model {
            id: id.into(),
            name: "8BitDo Test".into(),
            device_name: "8BitDo Test".into(),
            source: "SDL".into(),
            os: "linux".into(),
            driver: "sdl".into(),
            vendor: Some(0x2dc8),
            product: Some(0x5006),
            bus: Some(3),
            version: Some(0x100),
            bindings: BTreeMap::new(),
        }
    }
    #[test]
    fn only_unambiguous_hardware_and_platform_identity_autodetects() {
        let rows = vec![row("one")];
        let mut device = device();
        assert_eq!(detected(&rows, &device, "linux").unwrap().id, "one");
        assert!(detected(&rows, &device, "windows").is_none());
        device.bus_type = Some("0005".into());
        assert!(detected(&rows, &device, "linux").is_none());
        device.bus_type = Some("0003".into());
        device.is_virtual = true;
        assert!(detected(&rows, &device, "linux").is_none());
        assert!(detected(&[row("one"), row("two")], &self::device(), "linux").is_none());
    }
    #[test]
    fn imported_sources_have_unique_ids_and_retro_models() {
        let rows = models();
        let ids: std::collections::HashSet<_> = rows.iter().map(|m| &m.id).collect();
        assert_eq!(ids.len(), rows.len());
        for source in ["SDL", "RetroArch"] {
            assert!(
                rows.iter()
                    .any(|m| m.source == source && m.name.to_lowercase().contains("8bitdo"))
            );
        }
        assert!(
            rows.iter()
                .any(|m| m.name.to_lowercase().contains("brawler"))
        );
    }

    #[test]
    fn manual_search_includes_models_with_only_foreign_platform_profiles() {
        let result = review(&device(), None, "brawler64", "linux");
        assert!(!result["candidates"].as_array().unwrap().is_empty());
        assert!(result["detected"].is_null());
    }

    #[test]
    fn empty_search_lists_every_profile_and_ids_are_not_confused() {
        let mut device = device();
        let result = review(&device, None, "", "linux");
        assert_eq!(
            result["candidates"].as_array().unwrap().len(),
            models().len()
        );
        assert!(result["hardware_unique_id"].is_null());
        assert_eq!(result["connection_id"], "unit-one");
        device.unique_id = Some("serial-123".into());
        assert_eq!(
            review(&device, None, "8bitdo", "linux")["hardware_unique_id"],
            "serial-123"
        );
    }
}
