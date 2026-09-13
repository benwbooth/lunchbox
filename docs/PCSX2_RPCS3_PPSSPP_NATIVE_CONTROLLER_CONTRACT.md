# PCSX2, RPCS3, and PPSSPP native controller contract

These three standalone adapters are source-pinned and remain runtime-gated.

* **PCSX2** uses the native Qt/SIO pad sections `Pad1` through `Pad8`. The
  adapter keeps DualShock triggers as half-axis destinations and preserves the
  configured BIOS, memory-card, savestate, and other data folders while
  staging controller sections in a private data tree. Multitap UI numbering is
  separate from native pad-slot numbering.
* **RPCS3** uses the pinned SDL pad handler's positional names (`South`, `LS
  X-`, `LT`, and so on), not raw SDL ordinals. Profiles are written below
  `input_configs/global`; the existing `dev_hdd0`, `dev_flash*`, and savestate
  roots are not replaced. Seven native player entries are supported by the
  source contract. PS/pressure/limiter controls are explicitly blanked unless
  supplied.
* **PPSSPP** uses the SDL frontend's `controls.ini` `[ControlMapping]` keys and
  `<device-id>-<key-code>` values. SDL joypad IDs are 10 through 19, and the
  adapter requires complete, independent bipolar analog pairs. It overlays
  only `PSP/SYSTEM`; `SAVEDATA`, `PPSSPP_STATE`, and the native memory-stick
  root remain persistent.

Review and staging do not establish compatibility. Each launch still needs
the exact executable, SDL dependency, device identity/order, configuration
search path, and child-runtime confirmation captured by the existing session
code. BIOS/firmware and save data are never synthesized by these adapters.
