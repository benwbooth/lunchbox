# Hypseus Singe native controller contract

This contract is pinned to DirtBagXon/hypseus-singe commit
`a16e2521ee3233ef20c44e562008c47f4b4293df`.

When SDL_Gamepad mode is enabled with `-gamepad` or `GAMEPAD=TRUE`, Hypseus
reads `hypinput_gamepad.ini` from its home directory (or an alternate `.ini`
selected by `-keymapfile`/`-config`). `[KEYBOARD]` lines have two SDL keyboard
keys, a required first gamepad-button value, and up to three optional values
for the first gamepad axis and the second gamepad's button/axis. The official
file contains both five- and six-value lines. Lunchbox accepts the source
parser's three-through-six-value grammar and normalizes each patched line to
all six values.
`controller_hypseus_singe_native.rs` patches only those four controller
columns for explicitly named source switches, validates the source SDL button
and axis macros with their source-required uppercase spelling, emits all six
input fields on each patched line, and retains comments and unrelated settings.

The Linux native launch adapter is connected only for an explicit saved setup
whose emulator identity and framefile match the generated launch. It hashes the
executable, SDL3 library, mapping database, probe, baseline input file,
framefile, runtime dependencies, and staged copies; uses the target SDL3 build
to resolve each selected physical device's exact `SDL_GetGamepads` position;
and supplies a complete eight-index `-gamepad_reorder` permutation. It writes
the patched keymap and `gamecontrollerdb.txt` into a private temporary home,
while `-ramdir` keeps the selected writable NVRAM directory outside that home.
The adapter rejects custom launch environments, pre-existing input/reorder
overrides, topology or hash changes, and any command-line token longer than 80
bytes because the pinned parser copies every argument through `char s[81]`.

On Unix the default home is `$HOME/.hypseus`; Windows uses the current
directory unless `-homedir` is supplied. The source does not persist a stable
physical controller identity: SDL enumeration and any `gamecontrollerdb.txt`
mapping must be verified immediately before launch. NVRAM, where a game
driver has it, is compressed under `<home>/ram/<short-game-name>.gz`. The
reviewed source has no general savestate file contract and no universal BIOS;
game ROM/laserdisc data is driver-specific under the documented home folders.
No first-party Flatpak artifact was found.

On 2026-09-12, the pinned executable was built and run on native Linux against
an isolated home containing the official input file under Lunchbox's alternate
name and the pinned SDL mapping database. Its real parser accepted `-homedir`,
`-ramdir`, `-keymapfile`, and the full eight-value `-gamepad_reorder`; input
initialization applied `GAMEPAD=TRUE`, loaded the mapping database, enumerated
three real SDL gamepads, and reached the expected missing-ROM boundary. This
verifies parser/input initialization, not button behavior during gameplay or
NVRAM save/load persistence. Those broader runtime claims remain unverified,
so support remains partial.
