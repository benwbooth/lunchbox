# SimCoupe controller contract

Source: libretro/libretro-simcoupe commit
`c28046241ac6a4d79e55326b6e354dc02f92fa34`, inspected without building or running.
This is the `simcp` libretro fork, not current standalone SimCoupe.

## Core-side input gaps

`libretro/simcp-mapper.c:update_input` reads frontend port zero. In joystick
mode it packs D-pad directions and A/B into `MXjoy0`, then calls `retro_joy0`.
However, `SimCoupe/Retro/Input.cpp:retro_joy0` only calls
`Joystick::SetPosition(0, ...)`; both fire bits are discarded. The emulated
Sinclair/Kempston fire lines instead depend on the separate button state in
`SimCoupe/Base/Joystick.cpp`. Adding A/B labels to a launcher profile cannot
repair this missing connection.

Start toggles joystick/mouse mode on release. In mouse mode D-pad moves the
GUI cursor and A/B call the left/right mouse wrappers. The left wrapper forwards
GUI button events; the right wrapper is empty. Mouse motion updates a clamped
640-by-480 GUI position, not a proven emulated SAM mouse. R calls an empty reset
wrapper. These must not be labeled as working emulated right-click or reset.

L toggles the virtual keyboard on release; D-pad navigates and A selects on
release. Key wrappers do call `Keyboard_SetKey`, unlike the empty reset wrapper.
Y requests the menu and X opens disk-one browsing. Select toggles `RVSYNC`,
L2 toggles status display, and R2 toggles a sound flag; downstream effects still
need tracing before these become supported actions.

## Implementation boundary

An opt-in Nix package, `simcoupe-controller`, now pins the above source revision
and archive SHA-256 and applies `packaging/simcoupe-controller-input.patch`.
The patch forwards both host fire bits into the existing joystick button state
and clears directions/buttons when entering virtual-keyboard or mouse mode.
Both host buttons still represent the SAM interface's single fire line, not two
independent emulated buttons. No installed core or default launcher selection is
changed. The package has not been evaluated, built or runtime-validated; launch
integration must identify this patched artifact before relying on its behavior.
The reset, GUI right-click and special-key issues are not fixed by this patch.

Startup tracing also found that upstream `retro_load_game` only copies `RPATH`
and its first-frame `loadfirst` wrapper is empty: initialization opens saved
media, not the requested path. The maintained patch now calls the native disk-one
insertion API with autoload enabled and returns failure when insertion fails or
drive one is not configured as a floppy drive. It does not silently fall back to
the saved disk. This new content-loading path remains unbuilt and untested.

The patched core reports version `v1-lunchbox-controller1`. Its opt-in package
also emits `simcp_libretro.so.controller.json` beside the library, containing
the contract ID, pinned source revision and built library SHA-256. This is
build provenance/integrity metadata, not a runtime-success certificate or a
signature. The launcher still needs to consume it and reject an upstream or
changed artifact before enabling the patched joystick contract.

`controller_simcp.rs` now implements that read-only artifact parser: strict
receipt fields and contract identifiers, bounded regular-file reads, streamed
SHA-256 matching, canonical-path snapshots and later identity/content rechecks.
It does not dynamically load an unknown core to inspect it. Calibrated launch
preparation now checks the exact prepared core path, retains this snapshot and
rechecks it with the other launch inputs. The path requires an explicit fresh
native RetroArch launch without a custom environment. Native configuration
resolution and profile activation remain outstanding.

Native configuration parsing is now implemented separately for
`HOME/.simcoupe/SimCoupe.cfg`: the guard requires version 4, an explicit JoyType1
matching SAM joystick 1, SAM joystick 2 or Kempston, and JoyType2 disabled. The
core reads that file before initialization and resets options for incompatible
versions. The guard rejects duplicate input keys, ambiguous text and oversized
lines, preserves the file, and snapshots its contents and HOME identity. This
configuration helper is now connected through dedicated SAM-one, SAM-two and
Kempston content guards. Launch preparation requires both artifact and native
configuration snapshots, and retains/rechecks both before launch. Catalog
validation requires the patched one-player disk layout, exact library name,
fresh explicit selection and SAD/DSK/MGT media. Three guarded patched-core
profiles are now enabled, one for each interface. Drive1 must explicitly be 1
(floppy). The alternate B fire button is optional because it asserts the same
emulated fire line as A.

The catalog now contains a preview-only virtual-keyboard layout and output
profile: D-pad navigation, A selection and L visibility toggle. It has no launch
adapter and is excluded from enabled-core coverage. All six UI actions trigger
on release. A selected ordinary key is released at the start of the next input
update, so this cannot replace held gameplay keys or arbitrary key chords.

Physical keyboard callbacks pass the original key code, character and modifiers
to `Keyboard_SetKey`; the C++ wrapper transforms modifiers using `16 - nMods`.
The virtual keyboard instead passes modifier value 16 and translates keys at
or above 200 through a legacy table. Full special-key/modifier equivalence is
not established. The menu request currently sets `MMENU`, with no reader found
in the inspected source, so Y is not included as a working menu action.

Next work must complete keyboard translation and actual GUI action handling.
The enabled joystick profiles depend on the maintained input patch and matching
native settings; they do not silently modify or replace the installed core.

Coverage is 87/95 cores (91.6%) with an enabled mapping mode. Builds,
tests and runtime validation remain deferred.

The database's canonical platform name is `SAM Coupé`, which is included in all
three patched profile aliases. The launch branch now explicitly requires Linux,
matching both the package and the `HOME/.simcoupe` configuration resolver; merely
having a native RetroArch executable on another OS is not sufficient.
