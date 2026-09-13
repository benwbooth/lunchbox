# Magnavox Odyssey DS controller boundary

The preserved `Magnavox Odyssey DS` artifact is a 2010 Nintendo DS homebrew
program by Bobbyloujo. GameBrew documents chips 1–4 and only two controls:
`R/L` changes which player is controlled and `A` resets the ball. The linked
archive is
`https://dlhb.gamebrew.org/dshomebrew2/magnavoxodysseyds.rar`; no source,
desktop executable, or desktop configuration format is published, and the
linked author page is currently unavailable.

No Linux, Flatpak, Windows, or macOS host has a source-backed config/input,
save, save-state, BIOS/firmware, or key path. Nintendo DS input and storage
cannot be converted into a desktop controller profile without inventing an
emulator or reverse-engineering an unpinned binary.

Lunchbox therefore adds no native controller writer. All four host cells are
explicitly unsupported in the platform record; the GameBrew control summary
is preserved as artifact documentation only, not as a desktop runtime claim.
