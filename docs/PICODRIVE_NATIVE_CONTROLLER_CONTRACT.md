# PicoDrive native controller contract

Pinned upstream: `notaz/picodrive@26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee`.

The source writes a line-oriented `config2.cfg`: `platform/common/config_file.c` emits `binddev = <device>` and `bind <host-key> = playerN <action>` (lines 68-114 and 170-184). The action table in `platform/common/menu_pico.c` (lines 341-380) supplies the accepted player actions. `platform/common/main.c` documents `-config <file>` and `-loadstate <num>` (lines 27-69), while `platform/common/emu.c` resolves the default config under the platform root (lines 648-658).

`controller_picodrive_native` renders only this proven bind grammar, validates player/action/device atoms, escapes the source-reserved `#` and `=` key names, and never invents save/state filenames or physical-device identity. Runtime/package activation remains unverified.
