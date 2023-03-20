use std::{
    env::{consts::EXE_EXTENSION, current_exe},
    error::Error,
    process::Command,
};

fn run() -> Result<(), Box<dyn Error>> {
    let mut path = match current_exe()?.parent() {
        Some(dir) => dir.to_path_buf(),
        None => {
            eprintln!("Unknown directory.");
            return Ok(());
        }
    };

    path.push("midi-ctrl");
    path.set_extension(EXE_EXTENSION);

    Command::new(path).arg("embedded").spawn()?;

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Failed to start: {e}");
    }
}
