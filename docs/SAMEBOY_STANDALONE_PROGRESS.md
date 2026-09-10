# SameBoy SDL standalone mapping

Current status: native Linux saved-setup launch dispatch is connected, including
private config preparation, process startup and health checks. Coverage is now
93/94 RetroArch profiles (98.9%) and 8/249 standalone candidates with partial
dispatch (3.2%). This is untested partial support: ABI/DATA_DIR are declared by
the user; accelerometer games can consume directional axes as tilt. Historical
implementation checkpoints below do not reflect the current dispatch count.

Source pin: v1.0.3 commit 208ba4afabffab9edde416f2dbb8ae459e34adb8
(peeled annotated tag, not the separate libretro tag).

Implemented native button-array generation in the exact configuration.h order:
right, left, up, down, A, B, Select, Start. It rejects duplicate/out-of-range
buttons and clears menu, turbo, rewind, slow-motion, hotkey and rapid-fire slots.
The native lookup compares bytes directly, so disabling with 255 requires proving
the device has no button or axis at that index. Hats remain frontend-hardwired.

Directional calibration now accepts buttons, fixed cardinal hats, and paired
native axes. Axis mappings require right/down positive polarity, strict activation
beyond +/-0x4000, and rest strictly inside +/-0x3800. Opposite directions must
share an axis; X/Y cannot share one. Native hats are not index-filtered. The
future writer must disable faux analog mode; accelerometer games need separate
handling because native axis events become tilt instead of directional input.

Binary config replacement now patches only the 32-byte button array, two axis
bytes and faux-analog flag, preserving every other byte. The v1.0.3 standard C
ABI layout is explicitly selected: 32-bit enums, 8-bit bool, 4-byte alignment,
and the packed/aligned final struct. Complete 4460-byte preferences are required;
older truncated files are not zero-filled. Launch must establish the trusted
executable ABI; length alone is not evidence. No compiler/layout test was run.

Pending: runtime ABI validation,
native controller selection, visual catalog/setup UI and launch integration.
The native SDL profile is now in the catalog using the existing Game Boy visual
layout and a distinct validated transport. Physical calibration translation now
resolves raw SDL buttons, axis measurements and hats into the native binding
arrays, checking captured released state and complete eight-control ownership.
Dedicated saved-setup UI and launch integration still remain.
Native routing now verifies the selected physical path is SDL device zero,
matching connect_joypad rather than inventing a GUID-selection setting. Private
binary preference staging and a file-only writable overlay are implemented,
retaining source bytes and canonical identity for rechecks without redirecting
save directories. The launch owner still needs to resolve the native preference
path and connect the saved setup, runtime capture and process handoff.
Saved setups now persist emulator/content identity, binary preference path,
trusted executable hash, helper/runtime paths and a single physical controller.
They are integrated into settings defaults and validation. The no-I/O review
model supplies source/destination layout and complete native-calibration mapping
rows, explicitly reporting launch integration pending rather than readiness.
The shared setup UI now loads, reviews and stages SameBoy records through bounded
JSON methods, displays the existing source/destination mapping views, and explains
the single-player/device-zero and binary-preference requirements. Staging validates
the full list before changing settings. Launch integration remains pending.
The native Linux session now connects cancellable SDL2 capture, stable physical
identity resolution, measured control translation, device-zero routing and private
binary preference staging. It retains source/runtime inventory and kernel topology
for rechecks. The caller must supply the established runtime ABI. No capture was
executed. Native command construction must also resolve preference precedence:
main.c uses writable resource_path("prefs.bin") first, otherwise SDL_GetPrefPath
(empty organization, "SameBoy"). Saved source_config alone is not proof of selection.
Read-only preference resolution now mirrors native access-based precedence:
resource-local prefs first, compiled DATA_DIR fallback only when the local file
does not exist, then SDL's XDG_DATA_HOME or HOME/.local/share/SameBoy path when
the resource candidate is not readable/writable. It creates no directories.
Resource roots and compiled DATA_DIR presence still must be established from the
selected runtime; the resolver does not treat an unknown build setting as absent.
Saved setups now require an explicit runtime declaration tied to the trusted
executable hash: the supported binary ABI and either an explicit compiled data
directory or an explicit not-compiled variant. The UI documents both. These are
user-declared build facts, not automatic ABI verification; no compatibility claim
is inferred from a 4460-byte file. Native Linux resource_folder uses SDL_GetBasePath
(the executable directory), with cwd fallback only if SDL cannot determine it.
Native command preparation now checks saved emulator/content identity, native
executable hash, declared resource-path precedence, and runtime/content hashes,
then builds a private preference-file overlay. Child startup ownership checks
the intended executable, mapped SDL library, private config inode and selected
controller descriptor, with bounded cancellation and cleanup. These paths have
not been executed. Application dispatcher integration remains pending; runtime
ABI and DATA_DIR declarations remain user-supplied, not automatically proven.
Source inspection confirms no startup rewrite of the generated gameplay slots;
the legacy defaults repair only affects shortcut slots already cleared by us.
Axis preparation now rejects MBC7 cartridge type 0x22 with an actionable message,
because Core/gb.c GB_has_accelerometer routes those axes to tilt instead of the
requested directions. Button/hat mappings remain usable. This prevents silent
misrouting; it does not implement tilt coverage or claim all-mode completion.
No tests, builds, device probes or emulator launches have run. SameBoy is not
counted as a launch adapter. Coverage remains 93/94 RetroArch profiles (98.9%)
and 7/249 standalone candidates with partial dispatch (2.8%).
