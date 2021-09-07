use std::{error::Error, path::Path, sync::mpsc::Sender};

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
    pub controls: Vec<Control>,
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
                            Control::Continuous(ref mut continuous) => {
                                for continuous_layer in continuous.layers.values_mut() {
                                    continuous_layer.update(output, 0, true);
                                }
                            }
                            Control::Key(ref mut key) => {
                                for key_layer in key.layers.values_mut() {
                                    key_layer.update(output, KeyState::Off, true);
                                }
                            }
                        }
                    }
                }

                let receiver_controls = config.controls.clone();
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
                    controls: config.controls,
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
        controls: &'a Vec<Control>,
    ) -> Result<(), Box<dyn Error + 'a>> {
        match message {
            MidiMessage::ControlChange(channel, event) => {
                for control in controls {
                    if let Control::Continuous(continuous) = control {
                        for (layer, continuous_layer) in &continuous.layers {
                            if continuous_layer.channel == channel
                                && continuous_layer.control == event.control
                            {
                                continuous_layer.set_value(event.value);
                                sender.send(ControlMessage::ContinuousChange {
                                    device,
                                    control: continuous.name.clone(),
                                    layer: String::from(layer),
                                    value: continuous_layer.value_from_state(event.value),
                                })?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            MidiMessage::NoteOn(channel, event) => {
                for control in controls {
                    if let Control::Key(key) = control {
                        for (layer, key_layer) in &key.layers {
                            if key_layer.channel == channel && key_layer.note == event.key {
                                key_layer.set_value(KeyState::On);

                                sender.send(ControlMessage::KeyChange {
                                    device,
                                    control: key.name.clone(),
                                    layer: String::from(layer),
                                    state: KeyState::On,
                                })?;
                                return Ok(());
                            }
                        }
                    }
                }
            }
            MidiMessage::NoteOff(channel, event) => {
                for control in controls {
                    if let Control::Key(key) = control {
                        for (layer, key_layer) in &key.layers {
                            if key_layer.channel == channel && key_layer.note == event.key {
                                key_layer.set_value(KeyState::Off);

                                sender.send(ControlMessage::KeyChange {
                                    device,
                                    control: key.name.clone(),
                                    layer: String::from(layer),
                                    state: KeyState::Off,
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
