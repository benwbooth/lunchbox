# Infuse standalone controller boundary

Infuse is pinned to Tuxality/Infuse commit
`c29fb7dce336259b3453a8c1ac03e86be8a2361f`. That upstream tree contains only
the README, license, and ignore file. The README describes a multiplatform
Qualcomm BREW/Zeebo HLE project, but provides no executable, build system,
configuration grammar, input backend, firmware/BIOS loader, save/state path,
or key mapping implementation.

Accordingly, this project has no Infuse controller writer and must not invent
one. Linux, Flatpak, Windows, and macOS remain unresolved until an exact
upstream implementation or executable artifact supplies those semantics.
