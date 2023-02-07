extern crate log;
extern crate mysql;
extern crate rand;
use hydra::HydraClient;

fn init_logger() {
    let _ = env_logger::builder()
        // Include all events in tests
        .filter_level(log::LevelFilter::Warn)
        //.filter_level(log::LevelFilter::Error)
        // Ensure events are captured by `cargo test`
        .is_test(true)
        // Ignore errors initializing the logger if tests race to configure it
        .try_init();
}

fn main() {
    init_logger();
    let dbname = "pseudotester";
}
