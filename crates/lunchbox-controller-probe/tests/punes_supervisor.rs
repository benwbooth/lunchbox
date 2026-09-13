use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

pub fn file_hash(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[path = "../src/punes_supervisor.rs"]
mod punes_supervisor;

#[test]
fn live_inventory_is_exact_or_well_formed_when_targets_are_absent() {
    let inventory = punes_supervisor::inspect_targets().unwrap();
    assert_eq!(inventory.schema_version, 1);
    assert_eq!(inventory.sha256.len(), 64);
    match inventory.devices.len() {
        0 => assert!(inventory.validate(1).is_err()),
        players @ (1 | 2) => inventory.validate(players).unwrap(),
        players => panic!("unexpected live puNES target count: {players}"),
    }
}
