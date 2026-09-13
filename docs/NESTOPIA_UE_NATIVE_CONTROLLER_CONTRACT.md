# Nestopia UE standalone controller contract

Source oracle: `0ldsk00l/nestopia` tag `1.53.2`, commit
`4470a2e99199d8010322eef4bf680fb3760f6eda`, especially
`source/fltkui/inputmanager.cpp`, `source/fltkui/jg/jg_nes.h`,
`source/fltkui/jg.cpp`, and `source/fltkui/setmanager.cpp`.

The FLTK/SDL2 frontend stores the standard NES controls in `input.conf`.
Sections are `nespad1j` through `nespad4j`; joystick values are
`j<player>b<N>` for buttons, `j<player>h<N>` for hats, and
`j<player>a<N>` for axis halves, where an axis half is `axis * 2 + polarity`
and hat directions are 0=up, 1=down, 2=left, 3=right.  The `[nestopia]`
section selects standard controller hardware with `port1` through `port4 =
1`.  `crates/lunchbox-app/src/controller_nestopia_ue_native.rs` renders only
these source-backed fragments and rejects malformed or duplicate bindings.

The SDL frontend assigns player indices dynamically with
`SDL_JoystickSetPlayerIndex` as devices are connected.  A player index is not
a persistent physical identity.  A caller must therefore resolve and verify
the selected host controller immediately before launch; the fragment writer
alone does not establish device persistence or runtime input behavior.

Preserve the XDG config root containing `nestopia.conf` and `input.conf`, the
XDG data `nestopia/save/` battery/FDS files, `nestopia/state/` `.nst` states,
the `disksys.rom` FDS BIOS/user-asset path, and the installed core data such as
`NstDatabase.xml`.  Native Linux, the `ca._0ldsk00l.Nestopia` Flatpak, macOS
source builds and Windows XML configuration are separate contracts; this
writer covers only the FLTK/SDL2 grammar and does not imply libretro-core
compatibility or runtime verification.
