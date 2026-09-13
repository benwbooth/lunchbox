# Nuance Resurrection native controller contract

Source: `andkrau/NuanceResurrection@700d28553f3a06b16385ed9e48f194377e131b03`.
The source ships `nuance.cfg`; its `[Controller1Mappings]` section contains
the Nuon actions `CPAD_*`, `A`, `B`, `L`, `R`, `DPAD_*`, `NUON`, and accepts
`KEY_<vkey>_0`, `JOYBUT_<n>_0`, `JOYAXIS_<axis>_<direction>`, and
`JOYPOV_<pov>_<direction>` values. `controller_nuance_resurrection_native`
patches a complete copied section, preserving BIOS, game-media, and other
configuration roots. Runtime device identity and gameplay are unverified.
