# BlastEm standalone controller contract

Source-level integration; no runtime/device verification has been performed.

## Pinned functional contract

BlastEm sources as mirrored at libretro/blastem
b4d75247ebad8852fd9bc385b423df704c6c5af5 (canonical upstream is Mercurial):

- `config.c serialize_config` — the configuration is the tern tree
  serialized as nested `name { ... }` blocks of `key value` lines.
- `bindings.c get_binding_node_for_pad` — pad bindings live under
  `bindings pads <idx>` where idx is the SDL device index (GUID type-id and
  controller-type fallbacks follow, then `default`).
- `bindings.c handle_joy_added`/`process_pad_axis` — children are
  `dpads <n> up|down|left|right <target>` for host hat directions,
  `buttons <n> <target>` for host buttons and
  `axes <n>.positive|.negative <target>` for host axis halves.
- `bindings.c parse_binding_target`/`get_pad_buttons` — targets are
  `gamepads.<1-8>.<button>` with buttons up/down/left/right, a, b, c, x, y,
  z, start and mode.
- `paths.c get_config_dir` — the configuration directory is `$HOME/blastem`,
  so a private HOME isolates every read and write; `blastem.c main` takes
  the ROM as a bare argument.

## Integration

- `controller_blastem_native.rs` renders the per-device tern pad blocks
  (dpads/buttons/axes with escaped `gamepads.<port>.<button>` targets) with
  unit tests; `controller_blastem_native/settings.rs` holds the saved launch
  setups; `controller_blastem_native/session.rs` probes the trusted SDL2
  runtime per player through the established observation path and translates
  the calibrated controls with rest/travel checks; `controller_blastem_native/
  native_command.rs` owns the HOME-isolated launch with the bare ROM argument.
- Guided target `blastem:standalone-genesis-6` (two players) reuses the
  existing genesis-6 layout for the Sega Genesis/CD/32X platforms. Saved
  per-game setups are reused for another game of the same emulator.

## Limits

The six-button Genesis pad only. Mice, the tee-input adapter, hotkeys,
analog stick translation (BlastEm consumes SDL joystick axes directly),
Saturn-style pads, keyboard targets and the UI bindings are not covered.
Same-model pads sharing an SDL device slot order are disambiguated only by
the device index. Runtime behavior is unverified.
