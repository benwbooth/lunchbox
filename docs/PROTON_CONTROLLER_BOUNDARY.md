# Proton controller boundary

Proton is Valve's Wine-based compatibility layer, pinned here to
`ValveSoftware/Proton@5b89db940e0ebe3a137a6009a3589232fe084c09`.
Proton translates Windows games and their own input/configuration systems; it
does not define one portable controller profile, save/state grammar, BIOS, or
key contract across games. Lunchbox therefore refuses to synthesize a Proton
controller file. Game-specific runtime, prefix, and Steam Input behavior must
be inspected separately.
