fn main() {
    println!("cargo:rerun-if-changed=src/retro_log_shim.c");
    cc::Build::new()
        .file("src/retro_log_shim.c")
        .compile("lunchbox_retro_log_shim");
}
