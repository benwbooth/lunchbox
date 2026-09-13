# OpenBOR, OSX-KVM and OVCC controller boundary

## OpenBOR

The source oracle is `DCurrent/openbor@9d81480f8481fbb9e76b0b5f2a5dfa408376761a`.
The SDL desktop engine stores settings and key matrices as a packed
`s_savedata` structure in `Saves/default.cfg` or `Saves/<pak>.cfg`. Player 1
starts with the source's SDL scancode defaults; the current `clearbuttons`
implementation initializes players 2-4 to `CONTROL_NONE` even though
joystick virtual-code constants are defined. Progress, high scores and script
variables use `.sav`, `.hi`, and `.s00` through `.s99` files; recorded input
uses a script-supplied path and filename (often `.inp`). The structure is
build-conditional (`SDL`, `ANDROID`, and compatible-version fields), and joystick values are runtime
SDL virtual codes rather than stable host identities. No binary writer is
implemented until the exact target executable ABI is verified.

## OSX-KVM

The source oracle is `kholia/OSX-KVM@4c378a4b5e0b219783683012bec680325eb40719`.
Its documented Linux host workflow is a collection of QEMU shell scripts:
`usb-kbd`/`usb-tablet` are generic runtime devices, while optional USB or
evdev passthrough is launch topology. `mac_hdd_ng.img` is the guest disk;
`OVMF_CODE*.fd`, `OVMF_VARS*.fd`, and `OpenCore/OpenCore.qcow2` are boot media.
`snapshot=on` on the OpenCore disk is a temporary overlay, not a resumable
VM save state. The repository's `notes.md` documents invoking QEMU monitor
`savevm`, but defines no state-slot/path grammar for an adapter. No persistent
controller profile or device selector exists, so the adapter refuses to write
one. The AppleSMC OSK remains inline source key material and is not copied
into adapter output.

## OVCC

The source oracle is `WallyZambotti/OVCC@cc936b25be3da2c03b21a9bf1cc2ff4a494a5f09`.
`CoCo/config.c` writes the source-defined `Vcc.ini` entries: `Misc/KeyMapIndex`
and the `LeftJoyStick`/`RightJoyStick` `UseMouse`, directional, fire,
`DiDevice`, and `HiResDevice` values. The native writer in
`controller_ovcc_standalone.rs` patches only those fields in a copied INI,
preserves unrelated text, and requires caller-measured SDL device indices.
`UseMouse` follows the source's four input modes (audio, joystick, mouse, and
keyboard), while `HiResDevice` follows its three emulation values (standard,
Tandy hi-res, and CC-MAX).
Required CoCo ROMs and user-selected DSK/VHD images remain outside controller
serialization. OVCC does not define a desktop save-state format.

All three records keep native runtime, package activation, and effective
gameplay input as separate verification states. No Flatpak package contract
is asserted for these source trees.
