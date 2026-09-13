# Emma 02 standalone controller boundary

The adapter intentionally refuses to serialize a generic controller profile.
Emma 02's native input facilities are keyboard-oriented and vary by emulated
machine; no stable gamepad profile or physical-device identity is exposed by
the verified source.

## Verified source contract

Pinned source: `etxmato/emma_02@6cef5f299a32206d939afaaabb56d4c5efc6c4fa`.
The official project lists native Windows, macOS, and Linux support. On Linux,
the wxWidgets frontend stores its main `wxFileConfig` at
`~/.emma_02/emma_02.ini` (portable mode instead places it beside the
executable). Windows uses `%LOCALAPPDATA%/Emma 02 Config/emma_02.ini`; macOS
uses `~/Library/Application Support/Emma 02/emma_02.ini` according to the
platform branches in `src/main.cpp`.

The main config stores function/hot keys as numeric wx keycodes, including
`/Main/Exit_Key`, `/Main/Help_Key`, `/Main/Start_Reset_Key`, and
`/Main/Ctrlv_Key`. The keyboard-layout menu selects a data-directory INI such
as `us.ini`, `dutch.ini`, or `user_defined.ini`; these define emulated hex and
arcade keypad positions (`pad a1`, `pad a2`, `pad b1`, `pad b2`, and related
`game keys`). They are not host gamepad profiles. Separately, a `keyfile`
configuration points to a byte-stream playback file for scripted keyboard
input; it is not a mapping file.

## Refusal boundary

The adapter does not invent a joystick/gamepad mapping, host-device selector,
or cross-machine keymap. Editing numeric function-key settings or the
machine-specific keypad INI without a selected emulated machine would be
ambiguous and could break unrelated keyboard behavior. This boundary does not
claim save/state/ROM/BIOS completeness, runtime input behavior, or platform
parity.
