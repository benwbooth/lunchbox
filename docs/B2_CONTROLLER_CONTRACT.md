# b2 controller contracts

Source revision: `9793f6a8040c416d147003cada29363b955b54e7` from
[zoltanvb/b2-libretro](https://github.com/zoltanvb/b2-libretro).
Implementation is source-derived; tests, builds and runtime checks are deferred.

## Analog joysticks

Nine explicit profiles cover all entries in `src/libretro/core.h`'s active
`machine_types` array (`MACHINE_TYPES_COUNT = 9`): B/Acorn 1770, B/Watford
1770 DDB2 and DDB3, B/Opus 1770, B/Opus CHALLENGER 256K and 512K, B+,
B+128, and Master 128 MOS 3.20. The option values preserve exact source spelling.
Definitions after the array are commented-out future work, not selectable models.

`src/beeb/src/type.cpp:1164` excludes only Master Compact from ADC support.
None of the nine active entries is Compact. Non-Master ADC registers begin at
0xfec0; Master uses 0xfe18. `core.cpp:866` polls left X/Y and RetroPad A on two
frontend ports, delivering ADC pairs 0/1 and 2/3 and their corresponding fire.
Axes use `(32767 - value) >> 6`. The core only delivers changed axes and initializes
its previous-value cache to zero, so users must move and recenter each stick after
startup. Reset/model-change behavior is not claimed to preserve initialized axes.

All 24 `b2_joypad_*` options are privately set to `None`, including digital
buttons and eight analog-direction key assignments. This prevents simultaneous
keyboard actions. Native physical keyboard input remains available. The two
analog ports do not imply that a game supports two players.

## Remaining work

QAOP and AZOP keyboard presets are now implemented for each of the nine active
models (18 profiles). All 24 options are fixed, with 15 unique guest keys on the
first pad and both analog-to-key groups disabled. RetroPad A stays unbound because
the ADC path also consumes it as fire. The preset does not disable ADC hardware.
The layouts show Return/Escape, modifiers and the auxiliary keys; games must be
configured to use the selected direction keys. Physical keyboard remains needed
for other input. Duplicate keys are avoided because the core's per-control
transition tracking does not aggregate multiple controls holding the same key.

The live key-assignment interface can map the first joypad and both stick
direction thresholds to BBC keys. Additional presets, analog-direction key
profiles and mixed key/ADC profiles remain to be implemented. The digital-joystick block in `core.cpp:807` and the
controller-device remapping body at 1328 are commented out; neither is a working
device mode. No controller-operated virtual keyboard is established by this code.
Only direct SSD/DSD entrypoints are covered. Model selection is not proof of
firmware, disk or game compatibility.
