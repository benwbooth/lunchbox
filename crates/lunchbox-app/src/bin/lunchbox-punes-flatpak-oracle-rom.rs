//! Generate the battery, controller-register and save-state puNES oracle ROM.

use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
};

const HEADER: usize = 16;
const PRG: usize = 16 * 1024;
const CHR: usize = 8 * 1024;

fn branch(program: &mut Vec<u8>, opcode: u8) -> usize {
    program.extend([opcode, 0]);
    program.len() - 1
}

fn patch_branch(program: &mut [u8], operand: usize, target: usize) {
    let displacement = isize::try_from(target).unwrap() - isize::try_from(operand + 1).unwrap();
    assert!((-128..=127).contains(&displacement));
    program[operand] = displacement as i8 as u8;
}

fn absolute(program: &mut Vec<u8>, opcode: u8) -> usize {
    program.extend([opcode, 0, 0]);
    program.len() - 2
}

fn patch_absolute(program: &mut [u8], operand: usize, target: usize) {
    let address = 0x8000u16 + u16::try_from(target).unwrap();
    program[operand..operand + 2].copy_from_slice(&address.to_le_bytes());
}

/// SRAM begins `LBPU`, boot generation, event count and stable state color.
/// From `$6010`, each nonzero transition is an active-high `$4016/$4017`
/// pair. A/B set stable red/green state respectively; native save/load must
/// roll that color and the log back together before the next clean battery save.
fn oracle_rom() -> Vec<u8> {
    let mut program = vec![
        0x78, 0xd8, 0xa2, 0xff, 0x9a, // sei; cld; ldx #$ff; txs
        0xa9, 0x00, 0x8d, 0x00, 0x20, 0x8d, 0x01, 0x20, // disable PPU
    ];
    let mut initialize_branches = Vec::new();
    for (address, expected) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'P'),
        (0x6003, b'U'),
    ] {
        program.extend([0xad, address as u8, (address >> 8) as u8, 0xc9, expected]);
        initialize_branches.push(branch(&mut program, 0xd0));
    }
    program.extend([0xee, 0x04, 0x60]); // inc boot generation
    let setup_jump = absolute(&mut program, 0x4c);

    let initialize = program.len();
    for (address, value) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'P'),
        (0x6003, b'U'),
        (0x6004, 1),
    ] {
        program.extend([0xa9, value, 0x8d, address as u8, (address >> 8) as u8]);
    }
    program.extend([0xa9, 0x00, 0x8d, 0x05, 0x60, 0xa2, 0x00]); // count=0; x=0
    let clear = program.len();
    program.extend([0x9d, 0x10, 0x60, 0xe8, 0xe0, 0x80]); // clear 128 log bytes
    let clear_branch = branch(&mut program, 0xd0);

    let setup = program.len();
    program.extend([
        0xa9, 0x16, 0x85, 0x04, 0x8d, 0x06, 0x60, // stable red state color
        0xa9, 0x00, 0x85, 0x02, 0x85, 0x03, // previous P1/P2 states
    ]);
    let wait_vblank = program.len();
    program.extend([0x2c, 0x02, 0x20]); // bit $2002
    let vblank_branch = branch(&mut program, 0x10); // bpl wait
    program.extend([
        0xa9, 0x3f, 0x8d, 0x06, 0x20, 0xa9, 0x00, 0x8d, 0x06, 0x20, // PPUADDR $3f00
        0xa5, 0x04, 0x8d, 0x07, 0x20, // palette universal background
        0xa9, 0x00, 0x8d, 0x05, 0x20, 0x8d, 0x05, 0x20, // PPUSCROLL
        0xa9, 0x80, 0x8d, 0x00, 0x20, // enable NMI
        0xa9, 0x08, 0x8d, 0x01, 0x20, // enable background
    ]);

    let poll = program.len();
    program.extend([
        0xa9, 0x01, 0x8d, 0x16, 0x40, // controller latch high
        0xa9, 0x00, 0x8d, 0x16, 0x40, 0x85, 0x00, 0x85, 0x01, // latch low; samples=0
        0xa2, 0x00,
    ]);
    let read1 = program.len();
    program.extend([0xad, 0x16, 0x40, 0x4a, 0x66, 0x00, 0xe8, 0xe0, 0x08]);
    let read1_branch = branch(&mut program, 0xd0);
    program.extend([0xa2, 0x00]);
    let read2 = program.len();
    program.extend([0xad, 0x17, 0x40, 0x4a, 0x66, 0x01, 0xe8, 0xe0, 0x08]);
    let read2_branch = branch(&mut program, 0xd0);

    // Stable state marker: A means red, B means green. Save/load must restore it.
    program.extend([0xa5, 0x00, 0x29, 0x01]);
    let no_a = branch(&mut program, 0xf0);
    program.extend([0xa9, 0x16, 0x85, 0x04]);
    let after_a = program.len();
    program.extend([0xa5, 0x00, 0x29, 0x02]);
    let no_b = branch(&mut program, 0xf0);
    program.extend([0xa9, 0x19, 0x85, 0x04]);
    let after_b = program.len();
    program.extend([0xa5, 0x04, 0x8d, 0x06, 0x60]); // mirror state into battery RAM

    program.extend([0xa5, 0x00, 0xc5, 0x02]);
    let changed = branch(&mut program, 0xd0);
    program.extend([0xa5, 0x01, 0xc5, 0x03]);
    let unchanged = branch(&mut program, 0xf0);
    let changed_target = program.len();
    program.extend([
        0xa5, 0x00, 0x85, 0x02, // previous P1
        0xa5, 0x01, 0x85, 0x03, // previous P2
        0x05, 0x00, // ora P1
    ]);
    let released = branch(&mut program, 0xf0);
    program.extend([0xae, 0x05, 0x60, 0xe0, 0x40]); // count < 64
    let full = branch(&mut program, 0xb0);
    program.extend([
        0x8a, 0x0a, 0xaa, 0xa5, 0x00, 0x9d, 0x10, 0x60, 0xe8, 0xa5, 0x01, 0x9d, 0x10, 0x60, 0xee,
        0x05, 0x60,
    ]);
    let loop_jump = absolute(&mut program, 0x4c);

    let nmi = program.len();
    program.extend([
        0x48, 0x8a, 0x48, 0x98, 0x48, // save A/X/Y
        0xa9, 0x3f, 0x8d, 0x06, 0x20, 0xa9, 0x00, 0x8d, 0x06, 0x20, 0xa5, 0x04, 0x8d, 0x07, 0x20,
        0xa9, 0x00, 0x8d, 0x05, 0x20, 0x8d, 0x05, 0x20, 0x68, 0xa8, 0x68, 0xaa, 0x68,
        0x40, // restore; rti
    ]);

    for operand in initialize_branches {
        patch_branch(&mut program, operand, initialize);
    }
    patch_absolute(&mut program, setup_jump, setup);
    patch_branch(&mut program, clear_branch, clear);
    patch_branch(&mut program, vblank_branch, wait_vblank);
    patch_branch(&mut program, read1_branch, read1);
    patch_branch(&mut program, read2_branch, read2);
    patch_branch(&mut program, no_a, after_a);
    patch_branch(&mut program, no_b, after_b);
    patch_branch(&mut program, changed, changed_target);
    for operand in [unchanged, released, full] {
        patch_branch(&mut program, operand, poll);
    }
    patch_absolute(&mut program, loop_jump, poll);

    let mut rom = vec![0; HEADER + PRG + CHR];
    rom[..HEADER].copy_from_slice(&[
        b'N', b'E', b'S', 0x1a, 1, 1, 0x02, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    ]);
    rom[HEADER..HEADER + program.len()].copy_from_slice(&program);
    rom[HEADER + 0x3ffa..HEADER + 0x3ffc]
        .copy_from_slice(&(0x8000u16 + u16::try_from(nmi).unwrap()).to_le_bytes());
    for vector in [0x3ffc, 0x3ffe] {
        rom[HEADER + vector..HEADER + vector + 2].copy_from_slice(&0x8000u16.to_le_bytes());
    }
    rom
}

fn main() -> io::Result<()> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let output = PathBuf::from(arguments.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: lunchbox-punes-flatpak-oracle-rom OUTPUT.nes",
        )
    })?);
    if arguments.next().is_some()
        || output.extension().and_then(|value| value.to_str()) != Some("nes")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one .nes output path",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&oracle_rom())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_is_reproducible_battery_nrom_with_distinct_nmi() {
        let rom = oracle_rom();
        assert_eq!(rom, oracle_rom());
        assert_eq!(rom.len(), HEADER + PRG + CHR);
        assert_eq!(&rom[..9], &[b'N', b'E', b'S', 0x1a, 1, 1, 0x02, 0, 1]);
        assert_eq!(
            &rom[HEADER + 0x3ffc..HEADER + 0x3ffe],
            &0x8000u16.to_le_bytes()
        );
        assert_ne!(
            &rom[HEADER + 0x3ffa..HEADER + 0x3ffc],
            &0x8000u16.to_le_bytes()
        );
    }
}
