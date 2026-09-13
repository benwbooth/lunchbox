# Waydroid controller and persistence boundary

Waydroid is pinned to `waydroid/waydroid@5a51271131bfca8b7ee75ed067d09b26460f3a7b`.
The source initializes `/var/lib/waydroid/waydroid.cfg` as an INI file and
uses its `images_path` and `properties` sections for Android image and
property configuration. The default image root is `/var/lib/waydroid/images`.

Waydroid is a Linux Android container/session, not a console emulator. The
source exposes session/container lifecycle commands but no save-state slot
or physical-controller profile grammar. Android app input and compositor
forwarding are runtime- and frontend-owned. Lunchbox therefore fails closed
and does not write `waydroid.cfg` as if it were a gamepad profile. Runtime
container launch and effective Android gamepad input remain unverified.
