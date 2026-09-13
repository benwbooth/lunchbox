# MasterGear standalone controller boundary

MasterGear 4.9 has a built-in configuration menu and Windows joystick support,
but the official distribution publishes stripped binaries only and explicitly
says that source is not publicly available. The Linux artifact is
`MG49-Ubuntu-x86-bin.tgz` and the Windows artifact is `MG49-Windows-bin.zip`,
both linked from <https://fms.komkon.org/MG/>.

The adapter therefore refuses to invent a config path, joystick profile, or
device identity. The binaries document `.sav` cartridge saves and
`DEFAULT.STA`/`.STA` state snapshots, but those are not controller settings.
