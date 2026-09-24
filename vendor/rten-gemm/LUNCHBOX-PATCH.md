This is `rten-gemm` 0.24.0 from crates.io, by Robert Knight, under
`MIT OR Apache-2.0` (https://github.com/robertknight/rten).

The only code change is in `src/i8dot.rs`: use the same guarded AVX-512 VNNI
instruction via inline assembly that upstream already uses in
`src/kernels/x86_64.rs`. This avoids an intrinsic signature mismatch in the
LLVM 21 compiler shipped by Lunchbox's pinned Nix toolchain. No detection or
instruction dispatch behavior was changed.
