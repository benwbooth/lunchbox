# Save/State & Firmware Adapter Roadmap

## Already implemented (CI-green, 25 independent adapters + 93/94 RetroArch cores)

bsnes, Stella, VICE, Hatari, DeSmuME, openMSX, Mesen2, BlastEm, xemu, ScummVM,
Mednafen (firmware bind), DuckStation, melonDS, mGBA, Snes9x, FCEUX, SameBoy,
PCSX2, RPCS3, Flycast, MAME, BizHawk, ares.

## Capture records (data-only, adapter not yet built)

These have `emulator_details/records/<slug>.json` with evidence-captured paths,
naming, and firmware pointers. Follow the ScummVM/DeSmuME pattern:
contract module → session probe → native_command → guided wiring → gates → CI.

| Candidate | Record | Config syntax | Notes |
|---|---|---|---|
| Kronos | kronos.json | ~/.config/kronos/qt/kronos.ini | bkram.bin whole-image |
| Yaba Sanshiro 2 | yaba-sanshiro-2.json | ~/.config/YabaSanshiro/qt/yabause.ini | same whole-image |
| simple64 | (none yet) | mupen64plus.cfg + simple64-gui.ini | archived upstream |
| RMG | (none yet) | ~/.config/RMG/mupen64plus.cfg | input profiles as sections |
| Gearcoleco | gearcoleco.json | Geardome/Gearcoleco/config.ini | [InputA]/[InputB] scancode |
| puNES | punes.json | ~/.config/puNES/puNES.cfg + input.cfg | FDS bios optional |
| Nestopia UE | nestopia-ue.json | XDG nestopia/nestopia.conf + input.conf | FDS disksys.rom |
| Emulicious | emulicious.json | Emulicious.ini beside exe | binary-only, docs-based |
| DOSBox Staging | dosbox-staging.json | XDG dosbox/dosbox-staging.conf | saves inside mounted images |
| Altirra | altirra.json | registry or Altirra.ini | saves inside ATR/CAS images |
| Citron Neo | citron-neo.json | yuzu-fork qt-config.ini | Switch keys/firmware gap |
| Eden | eden.json | yuzu-fork qt-config.ini | Switch keys/firmware gap |
| jgenesis | jgenesis.json | XDG jgenesis/jgenesis-config.toml | TOML per-console input tables |

## Deferred

- FS-UAE: config format lives in bundled Python layer not in the repository.

## Pattern reference

Follow ScummVM/DeSmuME (compact single-controller) or Mesen2/VICE (full
contract writer + calibration translation) patterns. Each adapter needs:
contract module, settings.rs storage, session probe, native_command launch,
guided target registration, coverage entry, settings-model invokables,
QML editor entry, and `#[cfg(target_os = "linux")]` gating on session and
native_command modules.
