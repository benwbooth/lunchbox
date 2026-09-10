# PUAE input configuration boundary

Source: libretro/libretro-uae revision
`6536174a80d74e6c325aaa5390ff091fac8761d0`, `libretro-core.c`,
`libretro-core.h`, `libretro-mapper.c` and `libretro-glue.c`.
RetroArch save-directory routing was inspected at revision
`9a4726b05089ea339a53a313ece920bd8748d006`, `runloop.c`.

PUAE appends three custom configuration layers: the selected model's
`puae_libretro_MODEL.uae`, `puae_libretro_global.uae`, and the content basename
with its extension replaced by `.uae`. All are read from the effective core
save directory. The core removes a final one-byte directory component from
that path before lookup. These layers can supersede generated machine/input
settings and cannot be ignored when publishing a fixed input contract.

The Rust snapshot helper reads bounded regular files and currently admits only
absent or blank/comment-only overrides. It rechecks both absence and bytes
immediately before launch. Substantive custom configurations are preserved and
reported as requiring further effective-input resolution.

Launch integration resolves native RetroArch's explicit save settings, including
content-directory and core-name sorting. It requires an existing final directory
to avoid RetroArch's creation-failure fallback, then pins that same resolved
location in the private launch config without a second sorting pass. This does
not move save files or edit the user's RetroArch config. Platform-default save
paths, missing explicit sorting flags, Flatpak namespaces and custom launch
environments still require adapters.

Ten explicit computer-model profiles now use this guard: A500OG, A500,
A500PLUS, A600, A1200OG, A1200, A2000OG, A2000, A4030 and A4040. Fixed models
bypass the filename-tag machine selection path, which only runs in Automatic
mode. Device 257 explicitly selects ordinary RetroPad behavior instead of the
default device's content-driven CD32/Arcadia substitution.

Each profile has two two-button joysticks. Player one also owns Select for the
virtual keyboard and Start for Return; player two has only directions and fire
buttons. Physical mouse input, turbo and mapper substitutions are disabled.
Content is limited to direct floppy formats; firmware and format decoder
requirements remain separate from input mapping. Other peripherals, custom
configurations and content entrypoints still need implementation. Builds, tests
and runtime verification remain deferred.

The CD32 and CD32FR profiles use device 517 and a separate seven-button target:
B red, A blue, Y green, X yellow, L rewind, R fast forward and Start play/pause.
The computer profile's Return binding is cleared from Start. Both players receive
the hardware controls; only player one receives Select for the virtual keyboard.
These profiles admit direct CD-image formats and use the same configuration
snapshot boundary. The launcher permits this analog-class device only for the
guarded CD32 target; its hardware controls still use digital RetroPad inputs.

Digital and native-left-stick mouse profiles are available for the ten computer
models and CDTV. Mouse mode fixes B/A as left/right click, Y as middle click,
L2 as slower and R2 as faster. Digital speed is 6; analog modes use the left
stick, 15% radial deadzone and speed 1.0. Right-stick mouse input is disabled.
Digital modes require no native stick. Physical mouse input is disabled to keep
the mapped sources deterministic. Two pads can drive the two separate mouse
ports, subject to guest software support; only player one owns keyboard
shortcuts. CDTV mouse coverage is not full CDTV remote-control coverage.
