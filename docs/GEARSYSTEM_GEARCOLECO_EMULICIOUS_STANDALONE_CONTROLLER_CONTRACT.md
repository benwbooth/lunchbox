# Gearsystem, Gearcoleco, and Emulicious standalone controller contracts

Gearsystem and Gearcoleco now have source-backed SDL3 mapping writers.
Emulicious remains a separately justified closed-source boundary.

## Gearsystem

The pinned source is `drhelius/Gearsystem@253752954d5237a30b60c40789117ded345bcd48`.
`config_definitions.inc.h` verifies the mINI `[InputA]`/`[InputB]` fields for
the SDL logical direction mode, axes, inversion, buttons 1/2, Start, and Reset.
`events.cpp` consumes those values as SDL gamepad enums. Crucially,
`gamepad.cpp` assigns the first two currently open SDL gamepads to player slots;
there is no persisted physical-device index in `config.ini`. The writer now
patches the complete logical mapping while preserving keyboard, ROM,
configurable save/state roots and optional SMS/GG boot-ROM paths. The launch
layer must constrain and verify SDL device order immediately before startup.

## Gearcoleco

The pinned source is `drhelius/Gearcoleco@8ad5f92c45e7ca616535a057495557c2352a9115`.
It verifies `config.ini` `[InputA]`/`[InputB]` direction, fire, color-button and
full keypad fields, including the frontend's trigger-axis-as-button encoding.
As in Gearsystem, `gamepad.cpp` assigns the first two live SDL gamepads and the
file stores only logical SDL controls. The writer patches all 16 non-directional
controls plus D-pad/analog selection and preserves `SaveFilesDirOption`,
`SaveStatesDirOption`, five `.stateN` slots, and the 8192-byte size-only
ColecoVision BIOS requirement. Physical identity remains a launch-time probe,
not a reason to refuse the authorable mapping.

## Emulicious

Emulicious has no source repository (the recorded repository is empty) and is a
closed-source Java application. The official archive is pinned in the record
(`Emulicious.jar` release archive SHA-256
`6e1c6d511014033bbc2668360a0194389a5bad2bf6c5ffd0fe093b84da33c0fc`). Its
`platform.Emulicious$243` and `$50` bytecode establishes the Java-properties
writer implemented by `controller_emulicious_standalone`: selectors
`GBGamepad`, `SMSGamepadA/B`, `SMSbuttonsGamepad`, and `MSXGamepadA/B`, their
keyboard-union and threshold properties, and action mappings named
`Gamepad<N>Key<Action>` / `_1` or `Key<Action>` / `_1`. Unknown settings and
comments are preserved. JInput controller numbers are still runtime-environment
order, not stable physical identity, so launch must enumerate and verify the
selected controller immediately before startup.

## Runtime boundary

The Gearsystem and Gearcoleco modules are writer-only and do not yet prove
device discovery, executable startup, effective mappings, firmware availability,
gameplay input, or save/state round trips. A launch adapter must use the same
SDL3 inventory as the child, verify that selected physical identities occupy
the first two slots, isolate a copied config root, and check the consumed file.
The Emulicious module is a writer-only contract: it makes no claim about
executable startup, JInput availability, effective gameplay mappings, BIOS
availability, or save/state round trips. Keep a copied writable install root,
verify the consumed `Emulicious.ini`, and use a version-pinned executable and
runtime controller oracle before promoting launch support.
