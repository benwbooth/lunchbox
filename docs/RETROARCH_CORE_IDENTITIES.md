# RetroArch core identities

Catalog labels are not always library basenames. Discovery canonicalizes the
following explicit aliases before deduplication, so launch options and controller
contract lookup use the same identity as the installed library. Filename lookup
also accepts these aliases. Unknown names are preserved without prefix rewriting.

| Catalog alias | Library basename (before `_libretro`) | Upstream build definition |
| --- | --- | --- |
| `beetle_cygne` | `mednafen_wswan` | [WonderSwan Makefile](https://github.com/libretro/beetle-wswan-libretro/blob/master/Makefile) |
| `beetle_lynx` | `mednafen_lynx` | [Lynx Makefile](https://github.com/libretro/beetle-lynx-libretro/blob/master/Makefile) |
| `beetle_ngp` | `mednafen_ngp` | [NGP Makefile](https://github.com/libretro/beetle-ngp-libretro/blob/master/Makefile) |
| `beetle_pce_fast` | `mednafen_pce_fast` | [PCE Fast Makefile](https://github.com/libretro/beetle-pce-fast-libretro/blob/master/Makefile) |
| `beetle_supergrafx` | `mednafen_supergrafx` | [SuperGrafx Makefile](https://github.com/libretro/beetle-supergrafx-libretro/blob/master/Makefile) |
| `beetle_psx` | `mednafen_psx` | [PSX Makefile](https://github.com/libretro/beetle-psx-libretro/blob/master/Makefile) |
| `beetle_psx_hw` | `mednafen_psx_hw` | [PSX Makefile, hardware build](https://github.com/libretro/beetle-psx-libretro/blob/master/Makefile) |
| `beetle_vb` | `mednafen_vb` | [Virtual Boy Makefile](https://github.com/libretro/beetle-vb-libretro/blob/master/Makefile) |

These names were checked against upstream `TARGET_NAME` assignments on 2026-09-05.
The references track upstream branches, not pinned build inputs. This table defines
identity only: recognizing a library does not establish its controller contract or
claim runtime validation. Tests cover canonicalization, duplicate catalog rows,
and discovery filenames for Linux, Windows, and macOS; filename fixtures are never
loaded as libraries.

Firmware rule lookup compares canonical identities within the selected platform
and runtime, preserving BIOS requirements stored under older aliases. Saved
emulator preferences and manager selection also accept either spelling. Launch
customization lookup prefers an exact saved name, then an equivalent identity;
clearing a customization removes both spellings within that exact scope and
emulator. Existing saved rows are not rewritten during discovery.

Emulator Manager deduplicates and detects canonical core identities, including
platform compatibility, and uses canonical buildbot filenames. If a matching
existing ownership receipt uses the legacy alias, that receipt key is retained
for update/uninstall; a newly installed core uses the canonical key. Merely finding
a different installation of the same core does not establish ownership of it.
