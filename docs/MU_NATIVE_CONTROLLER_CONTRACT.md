# Mu native controller contract

Source: `meepingsnesroms/Mu@4ac406874ccdc33ca3282299fda412f15ec544ad`.
The Qt desktop frontend creates `QSettings(QDir::homePath() + "/MuCfg.txt",
QSettings::IniFormat)`. `SettingsManager` assigns eleven Palm hardware-button
bindings through the exact keys `palmButton0Key` through `palmButton10Key`,
whose values are Qt integer key codes captured from `QKeyEvent::key()`.

`controller_mu_native::patch_config` patches those keys in a copied complete
INI baseline and preserves unrelated settings. It does not claim a gamepad
profile or physical-device identity. ROM, RAM, SD, and state roots remain
owned by Mu; runtime file selection and input behavior are unverified.
