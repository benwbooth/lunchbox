# Kronos, Yaba Sanshiro 2, and FinalBurn Neo controller contract

This document records the source-backed boundaries for this adapter batch.
The FinalBurn Neo module currently covers the recorded libretro/inspection
transport; a separate native SDL frontend profile is not claimed here.

## Pinned upstream contracts

- Kronos native Qt input uses the recorded `FCare/Kronos@d451a55253e2e75bcef704ec8ade2085d298212c` contract. `kronos.ini` stores SDL
  `Button`, `Axis positive|negative`, and cardinal `Hat` values under the
  `[1.0]` settings group. Default Saturn backup RAM is the whole-image
  `bkram.bin` (32 KiB or extended 8 MiB), with per-game saves inside it;
  default states are `<gameid>_<slot>.yss`. Saturn BIOS is an explicit path or
  the built-in emulated BIOS; ST-V requires `stvbios.zip` in the ROM folder.
- Yaba Sanshiro 2 uses the official v1.20.37 source tarball contract. Its
  `~/.config/YabaSanshiro/qt/yabause.ini` stores `[0.9.11]` keys under
  `Input/Port/<port>/Id/<id>/Controller/<peripheral>/Key/<pad>`, with u32 host
  codes and `0xffffffff` for unbound values. `bkram.bin` is the single backup
  image (64 KiB or 8 MiB) and `.yss` states use `<gameid>_<slot>.yss`; BIOS may
  be a user-selected Saturn image or built-in emulation.
- FinalBurn Neo's recorded native frontend facts are not represented by a
  standalone-native adapter in this tree. `controller_fbneo.rs` remains the
  libretro/inspection contract: native addresses map to RetroArch channels,
  while actual FBNeo SDL input, generated INI/config, NVRAM, memory cards, and
  `.fs` state files require a separate source-backed native implementation.

## Hardening covered here

Yaba Sanshiro now rejects duplicate pad-key entries instead of relying on the
INI reader's last-value behavior. Kronos's existing tests cover native binding
spellings and the complete twelve-control table. FBNeo tests cover the existing
transport address/part mapping and reject unknown addresses without guessing.

## Root and runtime boundary

Kronos/Yaba backup images and state files must retain their configured native
roots; no staged configuration is evidence that firmware or save data exists.
The focused tests cover pure mapping/serialization only. They do not prove
device discovery, native process startup, effective configuration, firmware
availability, gameplay input, or save/state round trips. Those claims require
launch-time logs and artifacts from the selected runtime.
