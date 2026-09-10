{ lib, fetchurl, libretro }:

# Opt-in core artifact only. Does not select a runtime, forward clicks, or
# establish isolation from saved native UI sequences.
libretro.mame.overrideAttrs (old: {
  pname = "lunchbox-mame-game-mouse-only";
  version = "unstable-4fc9a931-mouse2";
  src = fetchurl {
    url = "https://codeload.github.com/libretro/mame/tar.gz/4fc9a9312baaf34963847f884961ad9793fbbc1d";
    hash = "sha256-DrOAm6fdUAcE/ooOhWTM27DklMKTbvoCi4gmJXKfFbc=";
    name = "mame-4fc9a931.tar.gz";
  };
  patches = (old.patches or [ ]) ++ [ ./mame-game-mouse-only.patch ];
  postPatch = (old.postPatch or "") + ''
    substituteInPlace src/osd/modules/input/input_retro.cpp \
      --replace-fail '#define LUNCHBOX_MAME_GAME_MOUSE_ONLY 0' \
                     '#define LUNCHBOX_MAME_GAME_MOUSE_ONLY 1'
  '';
  # The inherited wrapper selects stock retroarch-bare. Install only the core:
  # the owned relative launch path must select its compatible frontend itself.
  installPhase = ''
    runHook preInstall
    install -Dm755 mame_libretro.so "$out/lib/retroarch/cores/mame_libretro.so"
    runHook postInstall
  '';
  passthru = builtins.removeAttrs (old.passthru or { }) [ "tests" "updateScript" ] // {
    gameMouseContract = {
      sourceRevision = "4fc9a9312baaf34963847f884961ad9793fbbc1d";
      suppressGeneratedUiPointerEvents = true;
      suppressMouseUiDefaults = true;
      # Declarative package intent, not evidence about a loaded core.
      savedUiSequenceIsolation = false;
    };
  };
  meta = builtins.removeAttrs old.meta [ "mainProgram" ] // {
    description = "Opt-in MAME libretro core with game-only mouse UI defaults and events";
    platforms = lib.platforms.linux;
  };
})
