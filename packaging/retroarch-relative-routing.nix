{ lib, fetchurl, retroarch-bare }:

# Opt-in only. This does not replace Lunchbox's selected emulator or enable a
# command listener. Launch ownership and effective-state checks remain required.
retroarch-bare.overrideAttrs (old: {
  pname = "lunchbox-retroarch-relative-routing";
  version = "unstable-69a4f0e-routing1";
  src = fetchurl {
    url = "https://codeload.github.com/libretro/RetroArch/tar.gz/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576";
    hash = "sha256-tL+kbqXOCQCElAmblngW3o2noUbt4vfRpS1YQqaq4hU=";
    name = "retroarch-69a4f0ea.tar.gz";
  };
  patches = (old.patches or [ ]) ++ [
    ./retroarch-relative-routing.patch
    ./retroarch-relative-route-apply.patch
  ];
  configureFlags = (old.configureFlags or [ ]) ++ [
    "--enable-command"
    "--enable-udev"
  ];
  # The stock wrapper closes over the unmodified retroarch-bare package. Do not
  # expose it as a wrapper for this runtime, or inherit unrelated test claims.
  passthru = builtins.removeAttrs (old.passthru or { }) [
    "wrapper"
    "tests"
    "updateScript"
  ] // {
    relativeRoutingContract = {
      sourceRevision = "69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576";
      query = "GET_CONFIG_PARAM lunchbox_mouse_routes_v1";
      players = 8;
    };
  };
  meta = old.meta // {
    description = "Opt-in RetroArch with effective relative-input routing telemetry";
    platforms = lib.platforms.linux;
    changelog = "https://github.com/libretro/RetroArch/blob/69a4f0ea1e8aaf442ae4858f2e7f2b31a1776576/CHANGES.md";
  };
})
