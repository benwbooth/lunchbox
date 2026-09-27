# Emulators and launching games

Lunchpail starts emulators; it is not an emulator itself. A game needs a
compatible installed emulator, the correct game files, and sometimes BIOS
or firmware.

## Choose a default

Select a local game and choose an emulator in Game details. You can save the
choice for that game or for its system.

For RetroArch, choose the core as well. Two cores for the same console can
differ in accuracy, performance, save-state format, and controller support.

A per-game choice overrides the system default. Remove the override to use
the system default again.

## Install and update emulators

Open Settings and use the emulator manager to see the options available for
your operating system. You can also use an emulator you installed yourself.

Use the update check to review available updates. Lunchpail does not need to
replace a working emulator every time you start a game. It only removes
installations it manages, not an unrelated installation you maintain yourself.

On Linux, native and Flatpak installations are separate choices. They can
have different permissions, settings, cores, and save locations.

## BIOS and firmware

Read the requirement shown for the selected emulator. Import your own file or
use an offered source where appropriate. Some firmware must be supplied manually.

A BIOS for one emulator is not necessarily in the right format or folder for
another. For systems requiring user-owned keys, Lunchpail's import workflow
does not obtain those keys for you.

## What “preparing” means

Before starting, Lunchpail may need to extract an archive, apply a changed patch,
prepare a multi-file game, restore a save backup, or check the selected setup.
A first launch can take longer than later launches that reuse prepared files.

If preparation fails, read the specific message in the game's launch status.
Repeatedly pressing Play will not supply a missing BIOS or fix an incompatible patch.

Live translation is separate and [opt-in per game](translation.md).

## Stop and resume

Use **Stop** while a game is running, or quit from the emulator. Lunchpail
supports one active game session at a time, including a session it recognizes
after Lunchpail restarts.

Automatic state loading is available only where the emulator integration
supports it. A save state is tied closely to its emulator, core, and game version;
do not assume it will survive a core change.

## Controller support varies

Being listed as an emulator does not mean every controller, peripheral, or
automatic setting is supported. Standard gamepads, multitaps, mice, light guns,
and wheels are different input modes.

Review the target and any warning in controller setup. If automatic mapping
is unavailable, configure input in the emulator itself. Navigating Lunchpail
successfully is not proof that the emulator has received a mapping.

## Advanced launch options

Game details and the Launch Commands manager let you customize supported
launch profiles for a game, system, or emulator.

Start with the built-in command and change only what you need. Arguments are
passed directly to the program, not through a shell: shell pipes, redirects,
and command substitutions will not work.

If a customized launch fails, remove that override and try the default before
reinstalling the emulator.

