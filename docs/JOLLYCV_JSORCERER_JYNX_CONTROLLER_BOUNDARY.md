# JollyCV, JSorcerer, and Jynx controller boundary

These three records have explicit refusal modules because no source-backed
native controller writer is available.

* **JollyCV** — `jgemu/jollycv@7141b3438c9f6d21e619df56cea76a8d998f38c5`
  is a Jolly Good API module. `jg.c` consumes `jg_inputstate` button/relative
  values and exposes settings, but does not write host keyboard/gamepad
  mappings. The host frontend therefore must own the mapping and provide the
  same-launch input oracle.
* **JSorcerer 1.30** — source archive SHA-256
  `2d8e625ff94b1d5c9f39aa00e2763fe2a120149ef14943b4dd283a2ef29d199e` and
  `sorcerer.jar` SHA-256
  `60af6401eb8cace30dad0836fd6fec7eb2c86dc026a1a6c435cdeba2ccde3524`.
  The Swing frame forwards AWT `KeyEvent` values to a compiled keyboard
  matrix; `hardKeyMap` only toggles physical-key versus character handling.
  No persistent mapping grammar or joystick backend is present.
* **Jynx** — `jonathan-markland/Jynx@a02560be8a9b6a13b3a5a33de7e032d8f86dc658`
  has fixed GDK hardware-keycode and Win32 virtual-key matrices. Its
  `JynxEmulatorSettings.config` serializes display/emulation settings, not
  host key bindings; the macOS directory is only a placeholder and no
  Flatpak package is published.

The Rust modules `controller_jollycv_standalone`,
`controller_jsorcerer_standalone`, and `controller_jynx_standalone` are
registered in `lunchbox-app/src/lib.rs`. Each returns an error rather than
writing guessed files or claiming effective gameplay input. Their unit tests
pin the source/artifact identity and assert the refusal.
