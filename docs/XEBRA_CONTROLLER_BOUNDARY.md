# XEBRA controller boundary

XEBRA is pinned to the official Windows archive
[`xebra221106.zip`](https://drhell.web.fc2.com/ps1/xebra221106.zip), SHA-256
`5f522e51a0cad7395bdcb070e7c6955327f4bd3449a9c04dc7ed35c6693c1bd2`.
The official [PC instructions](https://drhell.web.fc2.com/ps1/xebra/index2.html)
configure the controller interactively: a PS1 port and controller type are
selected, then host buttons and X/Y/Z/R/U/V axes are sampled and associated.
The bundled documentation also explains that host axis ranges must be
measured at runtime.

Lunchbox therefore fails closed and does not invent a controller profile or
device identity writer. The documented `OSROM`, memory-card images, and
`XEBRA.RUN`/`.RI0`/`.RI1` state names are persistence evidence, not proof of
runtime compatibility or mapping round-trip behavior.
