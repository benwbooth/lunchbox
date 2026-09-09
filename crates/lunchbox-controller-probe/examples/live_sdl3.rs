fn main() -> anyhow::Result<()> {
    let library = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("SDL3 library path required"))?;
    lunchbox_controller_probe::live_sdl3::stream(std::path::Path::new(&library), true)
}
