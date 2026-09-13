# WASM-4 controller boundary

WASM-4 is recorded from pinned upstream `aduros/wasm4@9d6c962785cfe3719d0245fd279ecbe98a4dbb63`. The native runtime supports Linux, Windows, and macOS. Its native backend loads and saves cartridge data in an adjacent `<cart-basename>.disk` file, with a documented 1024-byte limit. The native keyboard backends provide fixed bindings; they do not expose a host profile file or configuration grammar.

The web runtime uses browser `localStorage`, while the libretro backend has frontend-owned input, save-RAM, and serialization callbacks. Those are separate ownership models and are not silently treated as native filesystem profiles. Native BIOS and cryptographic keys are not required. No native filesystem savestate contract was established.

Lunchbox refuses to synthesize a native WASM-4 controller profile. The refusal is implemented in `crates/lunchbox-app/src/controller_wasm4_w4_standalone.rs`. Native paths and the libretro distinction are captured in `emulator_details/records/wasm4-w4.json`; no verified Linux Flatpak package is recorded.
