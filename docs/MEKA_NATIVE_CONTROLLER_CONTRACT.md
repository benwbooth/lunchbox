# MEKA native controller boundary

MEKA has a source-backed native input file. The adapter emits its joypad
sections when the caller supplies an explicit Allegro runtime connection
ordinal; that ordinal is deliberately not presented as a stable physical
device id.

## Verified native format and roots

Pinned source: `ocornut/meka@3bf4a519ab7ee52f53d7a739c638475a8a68ab50`.
`meka/srcs/file.cpp` resolves writable resources beside the executable on
Windows and Unix-like builds (with platform-specific writable resources on
macOS).  It sets `meka.inp` as the input file and `meka.cfg` on Unix-like
hosts (`mekaw.cfg` on Windows).  Save RAM is `<ROM basename>.sav` in the
`Saves` directory.  Native savestates are `<ROM basename>.S%02d`, with slots
0 through 99, also in `Saves`.  MEKA's bundled data includes machine/BIOS
resources; BIOS use is machine-dependent rather than a universal external
file contract.

`meka/srcs/inputs_f.cpp` defines the writable INP grammar.  A source section
uses `type = keyboard | joypad | mouse`, `player = 1 | 2`, and joypad
`connection = <number>`.  Digital mappings serialize as `joy_button <n>` or
`joy stick <stick> axis <axis> dir <dir>`; the writer rewrites the whole file
on exit.  `meka/srcs/inputs_i.cpp` initializes Allegro and marks a source ready
when its zero-based `Connection_Port` is below `al_get_num_joysticks()`;
`inputs_u.cpp` then reads that ordinal with `al_get_joystick()`.

## Writer boundary

The INP syntax is known, but `connection` is one-based text for an Allegro
enumeration index.  No GUID, stable name, or other persisted physical identity
is loaded or written. The writer therefore requires a caller-resolved ordinal,
validates all eight digital controls, and emits the exact `joy_button` and
`joy stick ... axis ... dir ...` grammar. This does not claim startup,
effective input, BIOS behavior, save/state round trips, or full parity across
Windows, Linux, and macOS.
