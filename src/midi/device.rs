use std::{
    error::Error,
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};

use midi_control::MidiMessage;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Deserialize;

use crate::{utils::iter_json, ControlMessage};

use super::controls::{Control, KeyState};

#[derive(Deserialize, Clone, Debug)]
pub struct DeviceConfig {
    pub name: String,
    pub controls: Vec<Control>,
}

pub struct Device {
    pub name: String,
    // Required to keep the input connection open.
    _connection: MidiInputConnection<()>,
    pub output: Option<MidiOutputConnection>,
    pub controls: Arc<Mutex<Vec<Control>>>,
}

impl Device {
    pub fn new(
        name: String,
        control_sender: Sender<ControlMessage>,
        mut config: DeviceConfig,
    ) -> Result<Option<Device>, Box<dyn Error>> {
        let midi_input = MidiInput::new("MidiCtrl")?;
        let midi_output = MidiOutput::new("MidiCtrl")?;

        let mut output = None;
        for port in midi_output.ports() {
            let port_name = midi_output.port_name(&port)?;
            if config.name == port_name {
                output = midi_output.connect(&port, "MidiCtrl").ok();
                break;
            }
        }

        for port in midi_input.ports() {
            let port_name = midi_input.port_name(&port)?;
            if config.name == port_name {
                if let Some(ref mut output) = output {
                    for control in config.controls.iter_mut() {
                        match control {
                            Control::Continuous(ref mut control) => control.update(output, 0),
                            Control::Key(ref mut control) => control.update(output, KeyState::Off),
                        }
                    }
                }

                let controls = Arc::new(Mutex::new(config.controls));

                let receiver_controls = controls.clone();
                let connection = midi_input.connect(
                    &port,
                    "MidiCtrl",
                    move |_, buffer, _| {
                        let message = MidiMessage::from(buffer);
                        Device::handle_message(message, &control_sender, &receiver_controls);
                    },
                    (),
                )?;

                let device = Device {
                    name,
                    _connection: connection,
                    output,
                    controls: controls,
                };

                return Ok(Some(device));
            }
        }

        Ok(None)
    }

    fn update_control(&mut self) {}

    fn handle_message(
        message: MidiMessage,
        sender: &Sender<ControlMessage>,
        controls: &Arc<Mutex<Vec<Control>>>,
    ) {
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
}

pub fn devices(sender: Sender<ControlMessage>, root: &Path) -> Vec<Device> {
    let mut devices = Vec::new();

    let dir = root.join("devices");
    let entries = match iter_json::<DeviceConfig>(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Failed to read devices directory: {}", e);
            return devices;
        }
    };

    for entry in entries {
        match entry {
            Ok((name, config)) => match Device::new(name, sender.clone(), config) {
                Ok(Some(device)) => {
                    log::debug!("Connected to MIDI device {}", device.name);
                    devices.push(device);
                }
                Ok(None) => continue,
                Err(e) => log::error!("Failed to connect to device: {}", e),
            },
            Err(e) => log::error!("Failed to parse device config: {}", e),
        }
    }

    if devices.is_empty() {
        log::warn!("Found no MIDI devices to connect to");
    } else {
        log::info!("Found {} MIDI devices", devices.len());
    }

    devices
}
