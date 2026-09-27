fn main() {
    if let Err(error) = lunchpail_controller_probe::libretro_persistence::run_cli() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
