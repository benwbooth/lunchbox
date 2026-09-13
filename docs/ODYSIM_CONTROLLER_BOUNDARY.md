# OdySim controller boundary

OdySim is preserved at the [Internet Archive OdySim item](https://archive.org/details/OdySim).
The item description identifies the games as LÖVE/Lua programs; the published
game archives are bundled Windows executables and DLLs.  For example,
`OdySim-Dodgeball0-10-1.zip` has SHA-256
`af5a2ebc90bd0843c345433098baa2223d4646c87b83a81ab31b43129ff35643` and its
`OdySim-Dodgeball-ReadMe.txt` documents fixed keyboard controls (WASD/R/T/V
for player 1 and arrows/U/I/M for player 2).

The archive does not publish source, a writable controller-profile grammar, a
stable physical-device selector, or a save/state serialization contract.  The
readme's keyboard table is runtime input documentation, not a file format.
Lunchbox therefore refuses to synthesize an SDL, RetroPad, or guessed LÖVE
configuration.  The archive and its platform behavior remain unverified at
runtime; no external BIOS or cryptographic-key file is established.
