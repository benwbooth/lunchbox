//! Generate the original battery-backed NES input ROM for the Nestopia oracle.

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

/// The SRAM transaction is `LBNP`, boot generation, log count, then pairs of
/// active-high P1/P2 register bytes at `$6010`. A fresh run records each
/// nonzero state transition; a second process increments generation without
/// erasing the first run's log, proving battery reload as well as routing.
fn oracle_rom() -> Vec<u8> {
    let mut program = vec![
        0x78, 0xd8, 0xa2, 0xff, 0x9a, // sei; cld; ldx #$ff; txs
        0xa9, 0x00, 0x8d, 0x00, 0x20, 0x8d, 0x01, 0x20, // disable PPU
    ];
    let mut init_branches = Vec::new();
    for (address, expected) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'N'),
        (0x6003, b'P'),
    ] {
        program.extend([0xad, address as u8, (address >> 8) as u8, 0xc9, expected]);
        init_branches.push(branch(&mut program, 0xd0)); // bne initialize
    }
    program.extend([0xee, 0x04, 0x60]); // inc generation
    let setup_jump = absolute(&mut program, 0x4c);

    let initialize = program.len();
    for (address, value) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'N'),
        (0x6003, b'P'),
        (0x6004, 1),
    ] {
        program.extend([0xa9, value, 0x8d, address as u8, (address >> 8) as u8]);
    }
    program.extend([0xa9, 0x00, 0x8d, 0x05, 0x60, 0xa2, 0x00]); // count=0; x=0
    let clear = program.len();
    program.extend([0x9d, 0x10, 0x60, 0xe8, 0xe0, 0x40]); // sta $6010,x; inx; cpx #64
    let clear_branch = branch(&mut program, 0xd0);

    let setup = program.len();
    program.extend([0xa9, 0x00, 0x85, 0x02, 0x85, 0x03]); // previous states
    let poll = program.len();
    program.extend([
        0xa9, 0x01, 0x8d, 0x16, 0x40, // latch high
        0xa9, 0x00, 0x8d, 0x16, 0x40, 0x85, 0x00, 0x85, 0x01, // latch low, clear samples
        0xa2, 0x00,
    ]);
    let read1 = program.len();
    program.extend([0xad, 0x16, 0x40, 0x4a, 0x66, 0x00, 0xe8, 0xe0, 0x08]);
    let read1_branch = branch(&mut program, 0xd0);
    program.extend([0xa2, 0x00]);
    let read2 = program.len();
    program.extend([0xad, 0x17, 0x40, 0x4a, 0x66, 0x01, 0xe8, 0xe0, 0x08]);
    let read2_branch = branch(&mut program, 0xd0);

    program.extend([0xa5, 0x00, 0xc5, 0x02]); // lda p1; cmp prev1
    let changed = branch(&mut program, 0xd0);
    program.extend([0xa5, 0x01, 0xc5, 0x03]); // lda p2; cmp prev2
    let unchanged = branch(&mut program, 0xf0); // beq poll
    let changed_target = program.len();
    program.extend([
        0xa5, 0x00, 0x85, 0x02, // prev1=p1
        0xa5, 0x01, 0x85, 0x03, // prev2=p2
        0x05, 0x00, // ora p1
    ]);
    let released = branch(&mut program, 0xf0); // beq poll
    program.extend([0xae, 0x05, 0x60, 0xe0, 0x20]); // ldx count; cpx #32
    let full = branch(&mut program, 0xb0); // bcs poll
    program.extend([
        0x8a, 0x0a, 0xaa, // txa; asl; tax
        0xa5, 0x00, 0x9d, 0x10, 0x60, 0xe8, // log p1
        0xa5, 0x01, 0x9d, 0x10, 0x60, // log p2
        0xee, 0x05, 0x60, // inc count
    ]);
    let loop_jump = absolute(&mut program, 0x4c);

    for operand in init_branches {
        patch_branch(&mut program, operand, initialize);
    }
    patch_absolute(&mut program, setup_jump, setup);
    patch_branch(&mut program, clear_branch, clear);
    patch_branch(&mut program, read1_branch, read1);
    patch_branch(&mut program, read2_branch, read2);
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
    for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
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
            "usage: lunchbox-nestopia-flatpak-oracle-rom OUTPUT.nes",
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
    let bytes = oracle_rom();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_is_reproducible_battery_nrom_with_vectors() {
        let rom = oracle_rom();
        assert_eq!(rom, oracle_rom());
        assert_eq!(rom.len(), HEADER + PRG + CHR);
        assert_eq!(&rom[..9], &[b'N', b'E', b'S', 0x1a, 1, 1, 0x02, 0, 1]);
        for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
            assert_eq!(
                &rom[HEADER + vector..HEADER + vector + 2],
                &0x8000u16.to_le_bytes()
            );
        }
    }
}
