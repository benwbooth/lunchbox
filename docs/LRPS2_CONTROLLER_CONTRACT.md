# LRPS2 controller implementation

## Pinned source

Repository: https://github.com/libretro/ps2

Revision: `de0da87d20617d711aed1cbca975f8a51c998d36`.

The inspected checkout is `/tmp/lunchbox-lrps2-contract.cKPhfD`. This is the
libretro LRPS2 implementation, not the standalone PCSX2 configuration API.
The inventory's `pcsx2` identity must be matched to this core deliberately; a
matching filename alone is not a runtime compatibility check.

`pcsx2/PAD/PAD.cpp` 206..223 defines the sixteen digital button mappings.
317..340 reads analog-button pressure first and uses a digital full-press
fallback only when the analog value is zero. 407..458 polls two frontend ports,
with twelve pressure-sensitive controls (D-pad, face buttons and L1/R1/L2/R2),
four digital controls (Select, Start, L3, R3) and four bipolar stick axes.
The pressure values are unsigned positive input magnitudes, converted to 8-bit
levels after the configured deadzone. They must not receive signed negative
half-axis values. Stick deadzone, scaling and inversion are separate transforms.

378..387 advertises DualShock 2 as device 1 plus USB keyboard, mouse and combined
keyboard/mouse devices. 475..520 selects the native pad and USB attachments.
The combined device takes both USB ports and cannot be treated as an independent
per-port peripheral. The poll loop does not establish eight-player multitap
input support despite the internal eight-slot PAD structures.

`libretro/main.cpp` 1439..1508 reads per-port stick/pressure settings;
1907..1908 reports library name `LRPS2` and content extensions
`elf|iso|ciso|cue|gz|chd|cso|zso|m3u`.

## Step 149: pressure signals independent of control position

Catalog controls now have an optional explicit `pressure` signal flag. Existing
analog shoulders retain their previous interpretation. Explicit pressure is
allowed for analog face, D-pad, shoulder and rear controls, not sticks or pointer
coordinates. Calibration validation, measured capability promotion, normalized
transport selection and positive RetroArch axis binding use this signal instead
of relying exclusively on shoulder placement. Assignment policy version 5
rejects pressure-to-bipolar substitutions for analog targets.

Physical buttons are promoted only when their existing calibration includes a
valid measured proportional evdev gesture for the requested role. Face and D-pad
controls keep their ergonomic group; they are not relabeled as shoulders. No
proportional input is synthesized from a digital button.

The LRPS2 catalog profile, per-port options and complete launch constraints are
still pending. This change alone adds no enabled core or profile. USB peripherals,
multitaps and force feedback remain separate requirements. No builds, tests or
runtime execution were performed.

## Step 150: explicit two-port pressure profile

`retroarch:pcsx2:dualshock2-pressure` now enables the source-defined DualShock 2
device 1 on two frontend ports through the generic normalized RetroPad launch
writer. The new `dualshock2-pressure` layout has twelve independently required
pressure controls, four digital buttons and eight stick half-directions (four
bipolar axes). Calibrated digital-only face/D-pad inputs cannot satisfy it.

The private options for both ports are `pcsx2_axis_scaleN = 100%`,
`pcsx2_axis_deadzoneN = 0%`, `pcsx2_button_deadzoneN = 0%`, both stick-inversion
options disabled, `pcsx2_analog_modeN = enabled`, and
`pcsx2_enable_rumbleN = disabled`. With zero deadzone and unit scale the inspected
`process_analog` takes its signed-clamp fast path. The initial analog-mode option
is a one-time promotion of an unlocked digital pad after reset, not an override
of later game negotiations. The mode requires a fresh start.

Pressure bindings use normalized positive axis channels; inherited keyboard,
mouse-button and ordinary joypad-button fallback channels are cleared for these
controls. The catalog adapter does not provide USB keyboard/mouse, multitap,
native PCSX2 standalone settings or physical rumble forwarding. Runtime/core
revision compatibility, media and BIOS remain independent requirements.

Read-only catalog metadata now contains 91 enabled core identities, 289 enabled
profiles and 148 layouts. No builds, tests or runtime verification were run.

## Step 151: executable contract constraints

Catalog validation now calls `controller_lrps2::validate_profile` for enabled
`pcsx2` profiles. It enforces the exact LRPS2 mode/library, explicit fresh start,
two device-1 ports without topology or per-port overrides, all fourteen pinned
input options, and the complete semantic/native binding table. Required controls
cannot be marked optional, repeated, or changed between pressure, digital and
bipolar signals. New USB/multitap modes need their own implementation before the
guard can admit them. The guard is added to application code, but has not been
executed: formatting and source inspection are not tests or runtime acceptance.

## Step 152: pressure calibration event normalization

The calibration wizard now recognizes the evdev transport when GilRs labels a
physical axis event as a button. It records that event as a logical positive
axis gesture, matching `PendingCalibration`'s measured release result while
keeping the raw physical polarity in `native.direction`. Previously the initial
event could be rejected for an analog target, or fail the release binding match
after the Rust capture converted it to an axis.

Pressure targets require physical axis evidence at capture time and get full-
press/release instructions rather than stick-direction instructions. The wizard
also rejects a shared axis if either control is pressure-sensitive; two halves
of one axis cannot represent independent pressure values. Rust calibration and
launch checks remain authoritative. No UI, tests or builds were executed.
