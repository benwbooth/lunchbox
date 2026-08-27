fn main() {
    lunchbox_app::mark_process_started();
    std::process::exit(lunchbox_app::run());
}
