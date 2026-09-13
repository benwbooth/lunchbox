//! Bounded capture of Libretro's experimental memory-map callback.
//!
//! Some cores expose useful emulated RAM only through
//! `RETRO_ENVIRONMENT_SET_MEMORY_MAPS`, not `RETRO_MEMORY_SYSTEM_RAM`. The
//! diagnostic processes load explicitly trusted native cores, but still copy
//! and validate descriptor metadata immediately instead of retaining pointers
//! to the core-owned descriptor array itself.

use anyhow::{Result, ensure};
use std::ffi::{c_char, c_void};

const RETRO_MEMDESC_CONST: u64 = 1;
const MAX_DESCRIPTORS: usize = 4096;

#[repr(C)]
struct NativeMemoryDescriptor {
    flags: u64,
    pointer: *mut c_void,
    offset: usize,
    start: usize,
    select: usize,
    disconnect: usize,
    len: usize,
    addrspace: *const c_char,
}

#[repr(C)]
struct NativeMemoryMap {
    descriptors: *const NativeMemoryDescriptor,
    num_descriptors: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Descriptor {
    flags: u64,
    pointer: usize,
    offset: usize,
    start: usize,
    select: usize,
    disconnect: usize,
    len: usize,
    has_addrspace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryMapSnapshot {
    descriptors: Vec<Descriptor>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactMapping {
    pub address: usize,
    pub bytes: usize,
    pub select: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryRegion {
    pointer: usize,
    bytes: usize,
}

impl MemoryMapSnapshot {
    /// Copy a core-owned `retro_memory_map` during the callback. Native pointers
    /// inside individual descriptors remain core-owned and are used only while
    /// that core is loaded in the same diagnostic process.
    ///
    /// # Safety
    ///
    /// `data` must point to a valid `retro_memory_map` for the duration of this
    /// call, as required by `RETRO_ENVIRONMENT_SET_MEMORY_MAPS`. The captured
    /// region pointers must not be dereferenced after the publishing core is
    /// unloaded.
    pub unsafe fn capture(data: *const c_void) -> Result<Self> {
        ensure!(!data.is_null(), "Core supplied a null memory map");
        let map = unsafe { &*data.cast::<NativeMemoryMap>() };
        let count = usize::try_from(map.num_descriptors)?;
        ensure!(
            count > 0 && count <= MAX_DESCRIPTORS,
            "Core memory map has an invalid descriptor count"
        );
        ensure!(
            !map.descriptors.is_null(),
            "Core memory map has no descriptor array"
        );
        let native = unsafe { std::slice::from_raw_parts(map.descriptors, count) };
        let descriptors = native
            .iter()
            .map(|descriptor| Descriptor {
                flags: descriptor.flags,
                pointer: descriptor.pointer as usize,
                offset: descriptor.offset,
                start: descriptor.start,
                select: descriptor.select,
                disconnect: descriptor.disconnect,
                len: descriptor.len,
                has_addrspace: !descriptor.addrspace.is_null(),
            })
            .collect();
        Ok(Self { descriptors })
    }

    /// Resolve one exact, writable, unnamed mapping. Diagnostics intentionally
    /// refuse general mirror/bank translation: each supported core contract
    /// pins the descriptor shape reviewed in that core's source.
    pub fn exact_writable_region(&self, expected: ExactMapping) -> Result<MemoryRegion> {
        ensure!(expected.bytes > 0, "Expected memory-map region is empty");
        let mut matches = self.descriptors.iter().filter(|descriptor| {
            descriptor.pointer != 0
                && descriptor.flags & RETRO_MEMDESC_CONST == 0
                && descriptor.offset == 0
                && descriptor.start == expected.address
                && descriptor.select == expected.select
                && descriptor.disconnect == 0
                && descriptor.len == expected.bytes
                && !descriptor.has_addrspace
        });
        let descriptor = matches
            .next()
            .ok_or_else(|| anyhow::anyhow!("Exact writable memory-map region was not published"))?;
        ensure!(
            matches.next().is_none(),
            "Exact writable memory-map region is ambiguous"
        );
        Ok(MemoryRegion {
            pointer: descriptor.pointer,
            bytes: descriptor.len,
        })
    }
}

impl MemoryRegion {
    pub fn pointer(self) -> *mut u8 {
        self.pointer as *mut u8
    }

    pub fn bytes(self) -> usize {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_mapping_rejects_const_mirrored_and_ambiguous_regions() {
        let base = Descriptor {
            flags: 0,
            pointer: 0x1000,
            offset: 0,
            start: 0x0200_0000,
            select: 0xff00_0000,
            disconnect: 0,
            len: 256 * 1024,
            has_addrspace: false,
        };
        let expected = ExactMapping {
            address: 0x0200_0000,
            bytes: 256 * 1024,
            select: 0xff00_0000,
        };
        let snapshot = MemoryMapSnapshot {
            descriptors: vec![base],
        };
        assert_eq!(
            snapshot.exact_writable_region(expected).unwrap().bytes(),
            256 * 1024
        );

        for changed in [
            Descriptor { flags: 1, ..base },
            Descriptor {
                disconnect: 1,
                ..base
            },
            Descriptor {
                has_addrspace: true,
                ..base
            },
        ] {
            assert!(
                MemoryMapSnapshot {
                    descriptors: vec![changed]
                }
                .exact_writable_region(expected)
                .is_err()
            );
        }
        assert!(
            MemoryMapSnapshot {
                descriptors: vec![base, base]
            }
            .exact_writable_region(expected)
            .is_err()
        );
    }
}
