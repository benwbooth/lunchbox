# Steem SSE native controller contract

The native adapter is pinned to the SourceForge SVN-origin source tree at
`d2c604006c94686ba98faff252d25586089fd099` (repository UUID
`dd3fbd3b-ef5d-4b78-98e8-da144fa02e5b`). This is distinct from the existing
RetroArch/libretro route.

`ConfigStoreFile` persists controller settings in the Steem INI format. The
active setup is `[Joysticks] Setup=0..2`; each `[Joystick 1]` through
`[Joystick 8]` section contains `Type`, `ToggleKey`, `AnyFireOnJoy`,
`DeadZone`, `AutoFireSpeed`, and `DirID0..5`. The native writer emits the
source's normal joystick type (`Type=0`), two measured axes, a measured fire
button, and the blank autofire sentinel (`65535`). For setup 1 or 2 the source
uses the corresponding `1_` or `2_` key prefix.

Steem's `DirID` high byte is `10 + 10 * device_index` plus one for a negative
axis half; the low byte is axis+1 or 100+button. Thus the device index is an
enumeration slot, not a stable physical identity. The caller must probe and
recheck it immediately before launch and must preserve the baseline INI while
patching the selected sections.

This contract does not claim a packaged native Linux executable, startup
success, effective gameplay input, firmware availability, save/state behavior,
or cross-platform parity. The adapter must preserve the declared SteemSSE
RunDir, TOS/EmuTOS selection and override rules, mounted disk images, and
guest-save roots.
