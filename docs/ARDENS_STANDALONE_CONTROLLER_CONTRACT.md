# Ardens standalone controller boundary

Source oracle: `tiberiusbrown/Ardens@661a7dd4febc8d00e790a4ecde44b936295adf97`.

The SDL desktop frontend polls the first recognized SDL gamepad and exposes
its standard south/east/d-pad buttons as ImGui keys. The six Arduboy inputs
then read fixed keyboard keys: arrows for directions, A/Z for Arduboy A, and
B/S/X for Arduboy B. The persisted `Ardens.ini` contains display, debugger,
and window settings only; it has no controller profile or physical-device
selector.

The adapter therefore refuses to serialize a controller profile. This avoids
confusing Ardens's UI settings with an input schema or pretending that SDL's
runtime first-gamepad enumeration is a stable identity. The source does
provide separate EEPROM/FX save data (`absim_<game_hash>.save`) and compressed
timestamped `.snapshot` files, but those are outside controller serialization.
