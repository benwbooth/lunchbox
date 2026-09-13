# OdyWeb controller boundary

OdyWeb is distributed by the [official project page](https://prehistoricgaming.com/en/odyweb-the-html-magnavox-odyssey-simulator/)
as `OdyWeb_1.0.zip` (SHA-256
`f1951bbe832c88938e4855f2282833ecdbafcfab98fa83b8a7ff3b045a82fc65`).  The
archive contains only `OdyWeb.html` and `Odyweblogo.png`.

The HTML's JavaScript keeps keyboard state in an in-memory object, reads the
optional overlay with `FileReader`, and exposes gameplay/settings sliders in
the page.  It has no local-storage or server-backed persistence, native
configuration file, save-game/state format, BIOS, key file, or stable
controller identity.  The keyboard and browser APIs are therefore runtime
behavior rather than a serializable host-controller profile.

Lunchbox refuses to invent a native writer or claim a desktop emulator
integration.  Browser execution and effective gameplay input remain
unverified on each host platform.
