#[path = "../src/controller_punes_flatpak/configuration.rs"]
mod configuration;

use configuration::{Pad, jsc_file_name, render_input, render_jsc, render_main};

#[test]
fn exact_one_player_private_copy_is_complete() {
    let pad = Pad {
        player: 1,
        guid: "{FE12FF9C-1209-1141-4C51-4B250001FE71}".into(),
    };
    let input = render_input(b"[port 1]\nP1K A=S\n", &[pad.clone()]).unwrap();
    assert!(input.contains("controller 1=standard"));
    assert!(input.contains("controller 2=disable"));
    assert!(input.contains("P1K A=S"));
    assert!(input.contains(&format!("P1J GUID={}", pad.guid)));
    assert_eq!(
        jsc_file_name(&pad.guid).unwrap(),
        "FE12FF9C120911414C514B250001FE71.jsc"
    );
    assert!(render_jsc().contains("Right=BTN19"));
    let main = render_main(
        b"[system]\ncheat mode=gamegenie\ngame genie rom file=/tmp/gamegenie.rom\n\
[GUI]\nallow multiple instances of the emulator=no\n",
    )
    .unwrap();
    assert!(main.contains("allow multiple instances of the emulator=yes"));
    assert!(main.contains("cheat mode=disabled"));
    assert!(main.contains("game genie rom file=\n"));
    assert!(!main.contains("/tmp/gamegenie.rom"));
}
