use std::{
    error::Error,
    fs::{read_dir, DirEntry, File},
    io,
    path::Path,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use midi_control::MidiMessage;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Deserialize;

use crate::ControlMessage;

use super::controls::Control;

#[derive(Deserialize, Clone, Debug)]
pub struct DeviceConfig {
    pub name: String,
    pub controls: Vec<Control>,
}

pub struct Device {
    pub name: String,
    // Required to keep the input connection open.
    connection: MidiInputConnection<()>,
    pub output: Option<MidiOutputConnection>,
    pub controls: Arc<Mutex<Vec<Control>>>,
}

impl Device {
    pub fn new(
        control_sender: Sender<ControlMessage>,
        config: DeviceConfig,
    ) -> Result<Option<Device>, Box<dyn Error>> {
        let midi_input = MidiInput::new("MidiCtrl")?;
        let midi_output = MidiOutput::new("MidiCtrl")?;

        let mut output = None;
        for port in midi_output.ports() {
            let port_name = midi_output.port_name(&port)?;
            if config.name == port_name {
                output = Some(port);
                break;
            }
        }

        for port in midi_input.ports() {
            let port_name = midi_input.port_name(&port)?;
            if config.name == port_name {
                let (sender, receiver) = channel();

                let controls = Arc::new(Mutex::new(config.controls));

                let connection = midi_input.connect(
                    &port,
                    "MidiCtrl",
                    move |_, buffer, _| {
                        let message = MidiMessage::from(buffer);
                        sender.send(message).unwrap();
                    },
                    (),
                )?;

                Device::listen(receiver, control_sender, controls.clone());

                let device = Device {
                    name: config.name,
                    connection,
                    output: output.and_then(|port| midi_output.connect(&port, "MidiCtrl").ok()),
                    controls: controls,
                };

                return Ok(Some(device));
            }
        }

        Ok(None)
    }

    fn update_control(&mut self) {}

    fn listen(
        receiver: Receiver<MidiMessage>,
        sender: Sender<ControlMessage>,
        controls: Arc<Mutex<Vec<Control>>>,
    ) {
        thread::spawn(move || {
            for message in receiver {
                match controls.lock() {
                    Ok(mut controls) => match message {
                        MidiMessage::ControlChange(channel, event) => {
                            // Modify controls.
                            // Send ControlMessage::ControlChange
                        }
                        MidiMessage::NoteOn(channel, event) => {
                            // Modify controls.
                            // Send ControlMessage::ControlChange
                        }
                        MidiMessage::NoteOff(channel, event) => {
                            // Modify controls.
                            // Send ControlMessage::ControlChange
                        }
                        _ => (),
                    },
                    Err(e) => log::error!("Failed to lock controls: {}", e),
                }
            }
        });
    }
}

fn read_device_config(
    entry: Result<DirEntry, io::Error>,
) -> Result<Option<DeviceConfig>, Box<dyn Error>> {
    let entry = entry?;

    let file_type = entry.file_type()?;
    if !file_type.is_file() {
        return Ok(None);
    }

    let file = File::open(entry.path())?;
    Ok(Some(serde_json::from_reader(file)?))
}

pub fn devices(sender: Sender<ControlMessage>, root: &Path) -> Vec<Device> {
    let mut devices = Vec::new();

    let dir = root.join("devices");
    let entries = match read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Failed to read devices directory: {}", e);
            return devices;
        }
    };

    for entry in entries {
        let config = match read_device_config(entry) {
            Ok(Some(config)) => config,
            Ok(None) => continue,
            Err(e) => {
                log::error!("Failed to read devices directory: {}", e);
                continue;
            }
        };

        match Device::new(sender.clone(), config) {
            Ok(Some(device)) => {
                log::debug!("Connected to MIDI device {}", device.name);
                devices.push(device);
            }
            Ok(None) => continue,
            Err(e) => log::error!("Failed to connect to device: {}", e),
        }
    }

    if devices.is_empty() {
        log::warn!("Found no MIDI devices to connect to");
    } else {
        log::info!("Found {} MIDI devices", devices.len());
    }

    devices
}
