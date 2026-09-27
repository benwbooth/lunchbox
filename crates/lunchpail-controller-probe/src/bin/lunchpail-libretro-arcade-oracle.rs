#[path = "../libretro_arcade_oracle.rs"]
mod libretro_arcade_oracle;

fn main() {
    if let Err(error) = libretro_arcade_oracle::main_entry() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
