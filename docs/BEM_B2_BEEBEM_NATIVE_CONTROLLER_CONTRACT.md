# B-em, b2 and BeebEm native controller contract

This contract is grounded in the pinned upstream records. B-em and BeebEm
remain source-shaped low-level writers only. b2 additionally has a native
Linux launch adapter; that adapter has compiled and its focused pure writer
tests pass, but it has not yet been exercised with the emulator and attached
hardware.

| Emulator | Config | Input identity | State behavior |
| --- | --- | --- |
| B-em | Linux `$XDG_CONFIG_HOME/b-em/b-em.cfg`; Windows Allegro roaming `b-em.exe/b-em.cfg` | `key_actions`, `user_keyboard`, and `[joymap <Allegro name>]` persist; physical joystick order is runtime-only | User-selected `.snp`, `BEMSNAP` revision 1–3; no numbered slots |
| b2 | Linux `$XDG_CONFIG_HOME/b2/b2.json`; Windows `%LOCALAPPDATA%\\b2\\b2.json`; macOS Application Support `com.tom-seddon.b2/b2.json` | JSON `joysticks.device_names` has exactly three SDL names plus `swap_joysticks_when_shared`; names are runtime matched | No persistent save-state file is implemented |
| BeebEm | Windows `%USERPROFILE%\\Documents\\BeebEm\\Preferences.cfg` unless UserDataFolder/`-Data`/`-CustomData` overrides it | `Sticks` mode persists in Preferences.cfg; WinMM `JOYSTICKID1` remains runtime-only | `BeebState\\quicksave.uefstate` and `quicksaveN.uefstate` |

## Adapter rules

1. Use an isolated private data root for probes and launches. Do not rewrite a
   user's live configuration merely to discover its grammar.
2. Preserve unknown INI/JSON/key-value fields. For B-em, replace only the
   measured device's `stickNaxisN{adc,scale,nkey,pkey}` and
   `buttonN{btn,key}` fields inside its `joymap` section. For b2, patch only
   the `joysticks` object: an empty name means `(none)`, while repeating a
   name intentionally shares one controller between logical inputs.
3. Do not convert a runtime index or display name into a claimed stable
   physical-device identity. Multiple connected SDL/Allegro devices with the
   same runtime name require an explicit ambiguity result.
4. Treat attached BBC disk images as the guest save medium. None of these
   three adapters has a generic libretro SRAM path.
5. Keep state slots emulator-specific: B-em has chooser-selected snapshots;
   b2 has no persistent state file; BeebEm has quicksave and numbered
   quicksave files.

## b2 executable integration

The Settings controller page accepts exact b2 setup JSON. A setup pins the
emulator/content identity, source `b2.json`, controller-probe executable,
target SDL2 library, executable SHA-256, and one to three destination slots.
At launch Lunchbox resolves each selected physical Linux joystick into the
target SDL2 inventory, opens it through that exact runtime to require a valid
GameController mapping, and rechecks the device topology, SDL ordering,
library, executable, content, source config, and generated config immediately
before spawning b2.

The adapter rejects two attached devices with the same SDL controller name
because b2 persists only the name. Reusing one exact controller in both
analogue slots remains allowed and preserves b2's shared-stick semantics. The
patched file lives at `XDG_CONFIG_HOME/b2/b2.json` in a launch-owned temporary
directory. No original config, disk image, resource ROM, or state path is
rewritten. This is an executable partial Linux integration, not runtime
acceptance evidence or a claim about Windows, macOS, or a Flatpak.

See `emulator_details/records/b-em.json`, `b2.json`, and `beebem.json` for
platform-specific paths and pinned source evidence.
