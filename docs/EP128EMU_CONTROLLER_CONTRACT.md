# ep128emu controller contract prerequisites

Source inspected: [libretro/ep128emu-core](https://github.com/libretro/ep128emu-core),
revision `6af5d03de220e6effcd0ffcd99e0fb8e8c77cc9e`.
This is a source-derived implementation prerequisite, not runtime verification.
Default-controller contracts are launch-enabled for selected CPC, Enterprise,
ZX Spectrum and TVC media routes under the restrictions below. Custom
configuration and remaining machine/media variants are incomplete.

## Implemented CPC default launch

`controller_ep128emu.rs` requires a CPC disk header in DSK content or the nine-byte
`ZXTape!`/0x1a/version-1 signature in CDT content, and an explicit existing
native RetroArch system directory. The system-wide CPC configuration, per-game
sidecar and all four content-directory machine markers must be absent. It checks
those paths and the header again before launch, and pins the system directory in
the private append configuration. User files are never rewritten. This initial
resolver rejects custom configurations rather than interpreting them as defaults;
custom mapping support remains required work.

Both ports use default devices, external joystick 1/2, X/A fire, with player-one
Enter/0–3/information/zoom shortcuts. Private options fix information=L3,
zoom=R3 and autofire=None. Only fresh direct DSK/CDT entrypoints are covered.
The core's TZX signature branch selects CPC only with the CDT extension; renaming
the same bytes to TZX instead selects ZX Spectrum. The launch guard preserves
this distinction. Tape loading uses the core's startup sequence, not a new
controller-operated transport UI.

The shared automatic per-port mode route excludes explicit-selection profiles;
this contract therefore stays on the launch path that retains its configuration
snapshot. Header reads require regular media of at least 64 bytes, matching the
core's mandatory first header read and rejecting special files before opening.

## Configuration determines the machine and topology

An additional Enterprise default profile covers native ep128emu TAP files with
the eight-byte `02 75 cd 72 1c 44 51 26` signature, selecting Enterprise 128 tape
mode. Its six default ports are internal joystick then external 1–5; port one
also has the keyboard/system shortcuts. This uses the same absence snapshot,
with `enterprise.ep128cfg` selected in the system configuration directory.
Rare external adapters do not establish six-player software support. Custom
assignments, external 6, disk/direct-file routes and other tape formats remain
incomplete.

EPT was initially routed from its detection branch, but `retro_get_system_info`
does not advertise that extension. The profile now uses advertised TAP content;
the native signature also excludes the earlier ZX TAP heuristic. No file is
renamed to work around frontend support.

`src/dave.cpp:617` confirms external joystick multiplexing. External 3 and 5
fire share external 1's second/third fire lines; external 4 shares external 2's
second fire line (external 6 shares its third). This profile maps one fire per
adapter and explicitly retains the hardware-alias limitation. Directions for
additional adapters are carried in columns K/L; EnterMice can also share K.

`core/main.cpp:867` derives the content-specific `.ep128cfg` path by replacing
the content filename's last extension. Content headers, extension and filename
also participate in detection; a platform label alone does not select the VM.

`core/core.cpp:48` adds another input beyond the sample configuration's documented
system-wide and per-game files: machine-named files in the content directory.
Their **existence** forces the initial machine family. Checks occur in this order:
`tvc.ep128cfg`, `cpc.ep128cfg`, `zx.ep128cfg`, `enterprise.ep128cfg`.
These are independent checks, so the last existing file wins.

The constructor then selects the matching family configuration filename and loads
settings in increasing precedence: system directory `ep128emu/config/`, content
directory, content-specific sidecar (`core/core.cpp:160`). The detailed machine
setting is interpreted before subsequent machine initialization. Merely checking
the sidecar or system directory is therefore insufficient for a launch guard.

## Device callbacks do not preserve a full topology

`core/main.cpp:1316` creates a fresh six-element DEFAULT array for each controller
device callback, changes only the requested port, then rebuilds every port.
`core/core.cpp:905` resolves DEFAULT entries from `joypad.user1` through
`joypad.user6`. Later callbacks can consequently replace earlier explicit device
choices with configuration defaults. Core-option updates also rebuild mappings.
Independent device IDs are not a durable six-port configuration mechanism.

`core/core.cpp:797` establishes machine defaults, then player-one button overrides,
special core-option controls and finally configured/explicit joystick adapters.
The final adapter pass must be considered when predicting effective button maps.

## Default topology inventory

The ZX route also accepts TAP headers matching the exact `zx_header_match`
heuristic at `core/main.cpp:823`, selecting ZX128 direct-file mode. Nonmatching
TAP files are rejected, not assumed to be ZX. TVC additionally accepts MOPS CRT
cartridges; `core/main.cpp:1136` installs them into ROM segment 1. Neither media
extension alone establishes machine identity or software compatibility.

TVC now has a TVCWAV RIFF/WAVE tape profile covering five usable default
mappings. `src/tvc64vm.cpp:42` maps internal directions and external 1 directions
to the same matrix contacts, but internal fire is Space. External 1/2 fire
contacts are distinct. `ioPortReadCallback` at 766 implements GameCard external
3/4 at ports 0x32/0x33 with active-low direction/fire bits 0–4. External 5 has
neither a matrix conversion nor a GameCard read path, so it is not offered.
The sample configuration's two-external-joystick description is incomplete for
this revision. Software must support GameCard reads; five mappings are not five
independent players. The profile guards `tvc.ep128cfg` and the common inputs.

The ZX default-adapter profile now covers TZX v1 content in ZX128 tape mode,
checking `zx.ep128cfg` plus the common sidecar/marker inputs. Three dedicated
layouts label Sinclair 1, Sinclair 2 and Protek key aliases, with Kempston and
shortcuts on port one. These are four adapters, **not four independent players**.
Use the adapter required by guest software; simultaneous overlapping adapters or
number-key shortcuts can interfere due to per-port source-key transitions.
`src/zx128vm.cpp:35` confirms Kempston codes convert to bits 0–4, while the
number keys and Enter retain their documented meanings.

For CPC, `src/cpc464vm.cpp:39` converts the Enterprise-format key codes to
the CPC matrix. Joystick two maps up/down/left/right to matrix 48/49/50/51
(keys 6/5/R/T), with fire 1/2 at 52/53 (G/F). Its dedicated target layout
displays these aliases. `convertKeyboardState` starts from all released and
combines active source keys, so distinct source codes sharing a CPC contact are
combined at this conversion stage. This differs from multiple ports directly
writing the same source key code discussed below.

Player-one Enter and 0/1/2/3 convert to CPC matrix 18/32/64/65/57,
confirming the shortcut labels. Joystick one uses matrix 72–77.

| Machine | Source default ports | Joystick controls per port |
| --- | --- | --- |
| CPC | External 1, external 2 | Four directions and two fire buttons |
| ZX | Kempston, Sinclair 1, Sinclair 2, Protek | Four directions and one fire button |
| Enterprise | Internal, external 1 through 5 | Four directions and one fire button |
| TVC | Internal, external 1/2, GameCard external 3/4 | Four directions and one fire button; internal/external 1 directions alias |

Player one additionally defaults to Y=Enter, L=0, R=1, L2=2, R2=3,
L3=information and R3=zoom. These are overrideable, not unconditional labels.
`update_joystick_map` (`core/core.cpp:960`) assigns primary fire to RetroPad X,
and CPC secondary fire to A. No current caller requests the third-fire Y path.
Four catalog target layouts encode one-/two-fire variants with and without
the player-one shortcuts. The two-fire layouts are used by the CPC profile;
the one-fire layouts await other machine contracts. CPC player two uses a
dedicated layout showing its keyboard-matrix aliases.

Configuration layers are loaded again after machine defaults (`core/core.cpp:457`),
so the earlier load is not merely a one-time machine-selection hint. Input updates
send joystick key codes directly to guest keyboard state, bypassing the physical
keyboard conversion function. Multiple ports sharing a guest key do not aggregate
held state: an individual port release can release that key. Shared-adapter
topologies therefore need an explicit limitation or conflict policy.

The sample describes TVC internal plus only two external joysticks; the shared
six-port initialization branch does not prove six usable TVC hardware devices.

## Required implementation boundary

A deterministic launch must own or snapshot every effective configuration input,
including all four content-directory machine markers, not only the selected
family file. It must preserve user files, establish the complete port topology,
and account for per-button and special-option precedence. A private content
wrapper would need explicit launch-path and relative-media semantics; copying a
sidecar into an unrelated directory would not make the core load it.

Before enabling profiles, trace final button assignment and guest key conversion,
resolve supported machine/media routes, and carry the effective configuration
identity into launch validation. Do not count these prerequisites as implemented
controller coverage.
