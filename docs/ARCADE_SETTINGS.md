# Native arcade settings

Game details → Settings & mappings → Arcade settings → Blood offers:

- **Use game setting** (default): Lunchbox does not override blood.
- **Red blood**: force the game's native Blood option on for this session.
- **Censored blood**: force the native option off.

The choice is saved automatically per catalog game, applies on the next launch,
and is reapplied when an older state is loaded or the emulated machine resets.
“Use game setting” leaves whatever value the game/save state supplies; it does
not rewrite an existing state to factory defaults. Use “Censored blood” to
explicitly turn red blood back off.

## Support

Currently supported through **RetroArch MAME with an MVS arcade BIOS**:
Metal Slug, Metal Slug 2, Metal Slug X, Metal Slug 3, Metal Slug 4 and Metal Slug 5,
including the explicitly listed MAME cartridge revisions in `arcade_settings.rs`.
The game archive must use its MAME set name. Native RetroArch and RetroArch
Flatpak launch plans use the same adapter. Other cores keep the preference but
display that it cannot be applied with that core.

Metal Slug 6 (Atomiswave), console ports, AES BIOS mode, and other cores are not
handled by this Neo Geo adapter. Metal Slug 6 needs a separate, verified adapter;
it must not receive Neo Geo RAM writes or another user's replacement NVRAM.

## Implementation

A private MAME Lua autoboot script reads the decoded cartridge's English soft-DIP
tables to find **Blood**, **On** and **Off**. The option byte is not hard-coded
per game. US/European metadata must agree, and unknown or ambiguous tables are
rejected with an emulator message. The header version byte is not assumed zero:
the tested Metal Slug 2 cartridge uses `0x10`.

The script changes only that option in the BIOS's documented `BIOS_GAME_DIP`
working-RAM block while the cartridge is running in MVS mode. It does not write
ROMs, battery RAM, high scores, region/language settings or save files. Emulator
saves still happen normally. Startup RAM tests and the service menu are left
alone. A lightweight frame callback reapplies the choice after state loads;
reset replaces the callback subscription instead of adding duplicates.

Launch staging retains controller files and the scheduled MAME resume command,
keeps the original content basename and Lunchbox's save routes, and retains the
private script until emulator exit. An existing custom autoboot script causes a
clear preparation error rather than being replaced. No model download, external
process, cheat database or launch delay is added. Lua is supplied by MAME itself.

References:

- [MAME Metal Slug FAQ](https://wiki.mamedev.org/index.php/FAQ:Games#Metal_Slug_(Series))
- [Neo Geo soft-DIP format](https://www.ajworld.net/neogeodev/beginner/)
- [Neo Geo BIOS RAM ABI](https://wiki.neogeodev.org/index.php/BIOS_RAM_locations)
- [MAME Lua memory API](https://docs.mamedev.org/luascript/ref-mem.html)
- [Flycast developer's clarification of Metal Slug 6 debug settings](https://github.com/flyinghead/flycast/discussions/1146)

## Focused verification

```sh
cargo test -p lunchbox-app --lib arcade_settings
cargo test -p lunchbox-app --lib arcade_blood
lua5.4 crates/lunchbox-app/tests/arcade_blood.lua
qmltestrunner -input crates/lunchbox-app/tests/qml/tst_game_play_hero.qml
```

The Lua fixtures exercise varying option positions, reversed choices, ambiguous
metadata, cartridge header variants, RAM-test protection, old-state reapplication,
reset subscriptions, explicit on/off and refusal of AES/non-Neo-Geo hardware.
Isolated installed-core probes with MAME 0.287 verified Metal Slug and Metal Slug 2,
including loading a pre-override Metal Slug state; the battery-RAM Blood value
remained unchanged while the live soft-DIP byte changed. Other supported cartridge
releases use the same validated metadata path but were not locally play-tested.
