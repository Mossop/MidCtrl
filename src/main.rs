use std::{env::current_dir, error::Error};

use midi_ctrl::Controller;

fn run() -> Result<(), Box<dyn Error>> {
    let dir = current_dir()?;
    let controller = Controller::new(&dir);
    Ok(())
}

fn main() {
    pretty_env_logger::init();

    if let Err(e) = run() {
        log::error!("Error starting: {}", e);
    }
}
