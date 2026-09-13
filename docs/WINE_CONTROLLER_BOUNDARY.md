# Wine controller boundary

Wine is pinned to upstream commit
`788d90c4e1d628fab6672623f0c8094b984ea2fa`. Its README documents a Unix
compatibility layer, `WINEPREFIX`/`~/.wine`, and `drive_c`; the source's
DirectInput implementation uses prefix registry state for device and action
mappings. Those mappings are application- and device-specific, not a stable
Wine-wide profile.

Lunchbox therefore fails closed and does not mutate `user.reg`,
`system.reg`, or DirectInput registry keys. Windows application saves and
states remain title/frontend-owned, and Wine runtime behavior is not claimed
from the source inspection alone.
