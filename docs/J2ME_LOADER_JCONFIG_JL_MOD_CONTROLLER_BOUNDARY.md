# J2ME Loader, JConfig, and JL-Mod controller boundaries

This batch deliberately adds no controller writer. The three products do not
provide a shared, source-backed desktop controller contract for Lunchbox's four
target hosts (Linux, Linux Flatpak, macOS, and Windows).

## J2ME Loader

The pinned upstream source is `nikita36078/J2ME-Loader@9b0fa48a0a0d1e61376c0b9af28b3d2caec0a4cc`.
Its README identifies the project as a J2ME emulator for Android and requires
Android 4.0+. `app/src/main/java/ru/playsoftware/j2meloader/config/Config.java`
derives an Android emulator directory and defines `/configs/`, `/data/`, and
per-MIDlet `config.json` paths. `ProfileModel.java` defines the JSON fields
`Layout`, `KeyCodeMap`, and `KeyMappings`; `ProfilesManager.java` writes the
model with Gson. `KeyMapper.java` maps Android `KeyEvent` values to MIDP Canvas
codes, including default, Siemens, Motorola, and custom layouts.

The custom virtual-keyboard layout is separate from JSON. `VirtualKeyboard.java`
reads and writes `VirtualKeyboardLayout` as a binary signature/version/block
format. RMS data is per converted MIDlet under `/data/<app>/`;
`AndroidRecordStoreManager.java` uses `<record-store>.rsh` headers and
`<record-store>.<record-id>.rsr` records. The source establishes no save-state
or snapshot file. It requires no external BIOS/firmware or cryptographic key
file; “key” in this source means input mapping.

Those facts are useful Android provenance, not a desktop adapter contract. The
Android framework dependencies, APK-only distribution, and absence of a
desktop executable or same-launch input oracle make a four-host controller
writer unjustified. The record therefore marks all four target hosts
unsupported and records the Android boundary as `native_writer:
not_implemented`.

## JL-Mod

The pinned upstream fork is `woesss/JL-Mod@f723a190c0bdb44b31c3bc0ead6f8665c7ea517d`.
Its README calls JL-Mod an unofficial J2ME Loader fork for Android. The fork
retains the same Android per-MIDlet `/configs/<app>/config.json`,
`KeyMappings`, and binary `VirtualKeyboardLayout` model, with additional
`CustomKeys` and `SoundBank` profile fields. Its `Config.java` documents the
`/soundbanks/` and `/skins/` resource directories. RMS data uses the same
`<record-store>.rsh`/`<record-store>.<record-id>.rsr` files under the per-app
`/data/<app>/` directory. No save-state format, BIOS/firmware file, or external
cryptographic key contract is established; DLS/SF2 soundbanks are audio
resources, not firmware.

The upstream warning that JL-Mod settings are changed relative to J2ME Loader
also makes cross-app config reuse unsafe. As with J2ME Loader, Android-only
framework code and no desktop runtime/input oracle require refusal for all four
target hosts; no desktop controller writer is added.

## JConfig

No upstream source repository or official standalone download was found. The
community-maintained [JConfig Universe documentation](https://jconfig-universe.fandom.com/wiki/JVS/NESiCA_Config_(or_JConfig))
identifies JConfig as a Windows arcade configurator and documents DInput,
XInput, Winmm, GetAsyncKeyState, joystick order/reversal, POV/analog/deadzone
options, and P1/P2 keyboard commands. It says individual game settings and
Windows registry settings can be loaded/saved, but publishes no stable file
name, section grammar, encoding, or physical-device identity contract.

The same documentation says Save Patch *attempts* to place game save data in
`SV` under the game directory, with title-specific exceptions. It documents
CryptServer `*.key` emulation and says a required key file may remain in the
game directory. Neither statement establishes a universal save/state or key
format. JConfig documentation does not establish emulator snapshots or a BIOS
directory; those are game/interface-emulator specific. An independent
Hybrid Analysis report pins one PE32 `JConfig.exe` sample
(`3c475df3211bde59acd3bcfec30845e17829b50d16d6ced564bab83f95d75805`), but
that sample is not treated as an official distribution or as evidence for
undocumented field semantics.

JConfig is therefore captured only as an unresolved Windows per-game/registry
boundary. Linux, Linux Flatpak, and macOS are unsupported. No writer is added:
rewriting guessed registry/file values or game patches would not prove
effective JVS/Fast IO gameplay input and could damage title-specific save or
key material.

## Verification boundary

The three records contain six-purpose dispositions (config, input, saves,
states, BIOS/firmware, and keys) and evidence pointers. They intentionally do
not imply executable startup, runtime input behavior, save/state round trips,
firmware availability, or parity on any target host.
