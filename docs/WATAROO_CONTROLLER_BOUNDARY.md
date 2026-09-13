# Wataroo controller boundary

Wataroo is a closed-source Windows-only emulator. The official distribution page is <http://tailchao.com/Wataroo/> and the reviewed archive is `Wataroo_v0-8-0-0.zip` (version 0.8.0.0, dated 2021-05-28). The executable strings and bundled help identify adjacent `Wataroo.ini`, `Wataroo.joy`, user-selected `.sav` snapshots, and `.cdf`/`.bin` content. The help documents `[general]` autosave/load settings and `[input]` settings; the `.joy` artifact has device-specific `.NAME`, `.KEYS`, `.GROUP`, and `.JOY` sections.

Although the `.joy` text is inspectable, DirectInput/XInput port selection and physical device identity are runtime and binary-specific. Lunchbox does not infer a portable profile from those artifacts. BIOS and cryptographic key files are not required by the reviewed contract. Save snapshots are user-selected `.sav` files and autosave is controlled by the INI; no cross-platform runtime exists.

The controller refusal is implemented in `crates/lunchbox-app/src/controller_wataroo_standalone.rs`. Windows paths are captured in `emulator_details/records/wataroo.json`; Linux, Linux Flatpak, and macOS are explicitly unsupported.
