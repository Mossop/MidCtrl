use std::{
    error::Error,
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};

use midi_control::MidiMessage;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde::Deserialize;

use crate::{utils::iter_json, ControlMessage};

use super::controls::{ButtonState, Hardware};

#[derive(Deserialize, Clone, Debug)]
pub struct DeviceConfig {
    pub name: String,
    pub controls: Vec<Hardware>,
}

pub struct Device {
    pub name: String,
    // Required to keep the input connection open.
    _connection: MidiInputConnection<()>,
    pub output: Option<MidiOutputConnection>,
    pub controls: Arc<Mutex<Vec<Hardware>>>,
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
                            Hardware::Continuous(ref mut hw) => {
                                for control in hw.layers.values_mut() {
                                    control.update(output, 0, true);
                                }
                            }
                            Hardware::Key(ref mut hw) => {
                                for control in hw.layers.values_mut() {
                                    control.update(output, ButtonState::Off, true);
                                }
                            }
                        }
                    }
                }

                let controls = Arc::new(Mutex::new(config.controls));

                let receiver_controls = controls.clone();
                let device_name = name.clone();
                let connection = midi_input.connect(
                    &port,
                    "MidiCtrl",
                    move |_, buffer, _| {
                        let message = MidiMessage::from(buffer);
                        if let Err(e) = Device::handle_message(
                            device_name.clone(),
                            message,
                            &control_sender,
                            &receiver_controls,
                        ) {
                            log::error!("Failed handling MIDI message: {}", e);
                        }
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

    fn handle_message<'a>(
        device: String,
        message: MidiMessage,
        sender: &Sender<ControlMessage>,
        controls: &'a Arc<Mutex<Vec<Hardware>>>,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let mut controls = controls.lock()?;

        match message {
            MidiMessage::ControlChange(channel, event) => {
                for hw in controls.iter_mut() {
                    if let Hardware::Continuous(hw) = hw {
                        for (layer, control) in hw.layers.iter_mut() {
                            if control.channel == channel && control.control == event.control {
                                control.state = event.value;
                                let value: f64 = (event.value - control.min).into();
                                let range: f64 = (control.max - control.min).into();

                                sender.send(ControlMessage::ContinuousChange {
                                    device,
                                    control: hw.name.clone(),
                                    layer: String::from(layer),
                                    value: value / range,
                                })?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            MidiMessage::NoteOn(channel, event) => {
                for hw in controls.iter_mut() {
                    if let Hardware::Key(hw) = hw {
                        for (layer, control) in hw.layers.iter_mut() {
                            if control.channel == channel && control.note == event.key {
                                control.state = ButtonState::On;

                                sender.send(ControlMessage::NoteChange {
                                    device,
                                    control: hw.name.clone(),
                                    layer: String::from(layer),
                                    state: ButtonState::On,
                                })?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            MidiMessage::NoteOff(channel, event) => {
                for hw in controls.iter_mut() {
                    if let Hardware::Key(hw) = hw {
                        for (layer, control) in hw.layers.iter_mut() {
                            if control.channel == channel && control.note == event.key {
                                control.state = ButtonState::Off;

                                sender.send(ControlMessage::NoteChange {
                                    device,
                                    control: hw.name.clone(),
                                    layer: String::from(layer),
                                    state: ButtonState::Off,
                                })?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(())
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
