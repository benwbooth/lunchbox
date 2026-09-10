# Stella standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

stella-emu/stella `c65c845c8686c81698ffbd2fc9dfc5ccea5b32a1` (7.x line; nixpkgs
ships 7.0c — the saved executable SHA-256 pins the actual binary):

- `PJoystickHandler.cxx` — mappings persist in the `joymap` settings key as a
  JSON array, one object per joystick: `name`, `port` (`Left`/`Right`/`Auto`)
  and six event-mode arrays (`kMenuMode`, `kJoystickMode`, `kPaddlesMode`,
  `kKeyboardMode`, `kDrivingMode`, `kCommonMode`; exactly six keys are
  required by `PhysicalJoystick::setMap`). Sticks are matched by name;
  same-name sticks are renamed `<name> #N` in enumeration order.
- `JoyMap.cxx` — entries carry `event` plus `button`, or `axis` (`x`,`y`,`z`,
  `a3`..`a7`) with `axisDirection` (`negative`/`positive`), or `hat` with
  `hatDirection` (`up`/`down`/`left`/`right`). A missing `event_ver` (Event
  VERSION = 9) discards every saved mapping.
- `PJoystickHandler::handleRegularAxisEvent` — SDL axis indices cast directly
  to JoyAxis numbers (X=0, Y=1, Z=2, A3..A7=3..7); digital engagement uses the
  dead zone `3200 + joydeadzone*1000` with the default `joydeadzone` 13 →
  16200 in SDL axis units.
- `main.cxx` — `-basedir <dir>` relocates every configuration file; a bare
  existing path argument is the ROM.
- `OSystemStandalone`/`StellaDb`/`KeyValueRepositorySqlite`/`SqliteDatabase` —
  settings live in `<basedir>/stella.sqlite3`, table
  `settings(setting TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID`.
- The observation probe and the child launch both run with
  `SDL_JOYSTICK_LINUX_CLASSIC=1`, pinning the verified SDL 3.2.20 classic
  backend whose joystick numbering equals the kernel joydev order — the same
  semantics the DuckStation contract established.

## Integration

- `controller_stella_native.rs` renders the joymap JSON (entry grammar,
  six-mode stick objects, Stella's `#N` same-name suffixing) with unit tests;
  `controller_stella_native/settings.rs` holds the saved launch setups;
  `controller_stella_native/session.rs` probes the pinned SDL3 runtime per
  player, translates calibrated controls through the classic joystick map,
  applies dead-zone rest/travel checks, and writes only the `event_ver` and
  `joymap` rows into the private `stella.sqlite3`; `native_command.rs` owns
  the `-basedir` launch.
- Guided target `stella:standalone-atari2600-stella-panel` (two players,
  Atari 2600 platform) consumes guided player order; saved per-game setups
  are reused for another game of the same emulator. Console switches
  (Select/Reset/difficulty/Color/B-W) map on player one's stick; player two
  gets the joystick events expanded to `JoystickOne*`.
- The persistent `base_directory` keeps native saves/states across launches
  while never touching the user's own Stella database.

## Limits

Paddles, driving controllers, keypads, SaveKey/AtariVox, Stelladaptor and
2600-daptor passthrough, CompuMate, mouse events and the UI/menu modes are
not covered. Runtime behavior with a real Stella process is unverified; the
`joydeadzone` setting is assumed at its default (13). Sticks whose SDL name
is absent cannot be mapped.
