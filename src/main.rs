use flexi_logger::{FileSpec, Logger};
use std::{
    env::{self, current_dir},
    fs::canonicalize,
    path::PathBuf,
};

use midi_ctrl::Controller;

fn run() -> Result<(), String> {
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
                return Err(format!("Failed to find settings directory: {}", e));
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
                    return Err(format!("Failed to find settings directory: {}", e));
                }
            },
        }
    };

    let logger = Logger::try_with_env_or_str("info")
        .map_err(|e| format!("Failed to initialize logging: {}", e))?;

    let logger = if embedded {
        let mut filename = dir.clone();
        filename.push("midi-ctrl.log");
        let spec = FileSpec::try_from(filename).unwrap();

        logger.log_to_file(spec)
    } else {
        logger
    };

    let _log_handle = logger
        .start()
        .map_err(|e| format!("Failed to start logging: {}", e))?;

    let mut controller = Controller::new(&dir)?;
    controller.run()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error starting midi-ctrl: {}", e);
    }
}
