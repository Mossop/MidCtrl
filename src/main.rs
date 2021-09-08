use std::{
    env::{self, current_dir},
    error::Error,
    fs::canonicalize,
    path::PathBuf,
};

use daemonize::Daemonize;
use midi_ctrl::Controller;

fn run(dir: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut controller = Controller::new(&dir)?;
    controller.run()
}

fn main() {
    let mut args: Vec<String> = env::args().collect();

    if !args.is_empty() {
        args.remove(0);
    }

    let mut embedded = false;

    if let Some(arg) = args.get(0) {
        if arg == "embedded" {
            embedded = true;
            args.remove(0);
        }
    }

    let dir = if let Some(arg) = args.get(0) {
        match canonicalize(PathBuf::from(arg)) {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Failed to find settings directory: {}", e);
                return;
            }
        }
    } else {
        match dirs::config_dir() {
            Some(mut dir) => {
                dir.push("midi-ctrl");
                dir
            }
            None => match current_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    eprintln!("Failed to find settings directory: {}", e);
                    return;
                }
            },
        }
    };

    if embedded {
        let daemonize = Daemonize::new();

        match daemonize.start() {
            Ok(_) => {
                pretty_env_logger::init();
                if let Err(e) = run(dir) {
                    log::error!("Error starting: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error starting: {}", e);
            }
        }
    } else {
        pretty_env_logger::init();

        if let Err(e) = run(dir) {
            log::error!("Error starting: {}", e);
        }
    }
}
