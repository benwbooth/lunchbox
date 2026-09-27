# RetroAchievements

Lunchpail can configure RetroAchievements for supported RetroArch games.
RetroArch handles recognizing the game, tracking achievements, and displaying
unlock notifications.

## Sign in

Open **Settings → RetroAchievements** and sign in. Lunchpail stores the returned
login token in your operating system's credential store, not your password.

Choose a default:

- **Casual** allows a more flexible play session.
- **Hardcore** restricts features such as loading save states and rewind.
- **Keep RetroArch's own settings** leaves account and mode choices to RetroArch.

In a game's **Settings & mappings → RetroAchievements**, you can inherit that
default or choose a different mode. Changes apply on the next launch.

## See your progress

Open RetroArch's **Quick Menu → Achievements** while playing, or open your
RetroAchievements profile from Lunchpail.

Lunchpail does not currently show a synchronized achievement list of its own
or configure standalone emulators' achievement accounts.

## Why isn't this game recognized?

The exact ROM revision and core matter. A patch or translation changes the
game's contents and may require its own supported achievement set.

Do not change the reported identity to make a patched game look like an
unmodified one. Use a supported version instead.

## Hardcore and saves

Lunchpail blocks enabled cheats and disables automatic state loading and rewind
for its managed Hardcore sessions. In-game saves remain separate from loading
an emulator state. RetroArch enforces the rest of the mode's requirements.

If login stops working, sign in again. If only one game fails, check its core
and exact version before resetting your account.

