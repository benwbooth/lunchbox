# RetroArch core feature records

Each JSON file in this directory owns one canonical RetroArch core identifier
from the Lunchbox database. The host-specific frontend paths remain in
`../records/retroarch.json`; these files capture the distinctions that cannot
be truthfully inherited from the frontend alone:

- the exact Lunchbox controller contracts for the core;
- per-core availability on native Linux, the RetroArch Flatpak, macOS, and
  Windows (frontend availability alone is insufficient);
- required and optional firmware paths, including a checksum or an explicit
  reason why upstream publishes no canonical digest;
- whether the core writes persistent saves and their extensions/naming; and
- whether the core implements serialized save states.

Files conform to `../retroarch-core.schema.json`. Their filenames are the
canonical core identifier plus `.json`. Core-info facts are pinned by
`../../sources/libretro-core-info.json`; additional evidence should point to a
revision-pinned upstream source file or the official Libretro core manual.
When the pinned core-info snapshot no longer contains a catalog core, use a
null `core_info_file` and cite that absence in every affected feature instead
of inventing a filename.

`not_required`, `not_supported`, and `unknown` are evidence-bearing outcomes,
not empty placeholders. Never infer a successful runtime result from a source
record.
