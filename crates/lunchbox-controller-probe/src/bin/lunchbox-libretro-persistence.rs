#[path = "../libretro_persistence.rs"]
mod libretro_persistence;

fn main() {
    if let Err(error) = libretro_persistence::run_cli() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
