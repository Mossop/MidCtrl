use std::{collections::HashMap, error::Error, path::Path, sync::mpsc::Sender};

use midi_control::MidiMessage;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, MidiOutputPort};
use serde::Deserialize;

use crate::{utils::iter_json, ControlMessage};

use super::controls::{Control, KeyState, LayerControl};

#[derive(Deserialize, Clone, Debug)]
pub struct DeviceConfig {
    pub port: String,
    pub controls: Vec<Control>,
}

pub struct Device {
    connection: Option<MidiInputConnection<()>>,
    pub output: Option<MidiOutputConnection>,
    pub controls: HashMap<String, Control>,
}

fn input_port(
    midi_input: MidiInput,
    device_id: &str,
    port: &str,
    controls: &Vec<Control>,
    sender: Sender<ControlMessage>,
) -> Result<Option<MidiInputConnection<()>>, String> {
    for input_port in midi_input.ports() {
        let port_name = midi_input
            .port_name(&input_port)
            .map_err(|e| format!("Failed to get MIDI port name: {}", e))?;
        if port == port_name {
            let device_id = device_id.to_string();
            let receiver_controls = controls.clone();
            return Ok(Some(
                midi_input
                    .connect(
                        &input_port,
                        "MidiCtrl",
                        move |_, buffer, _| {
                            let message = MidiMessage::from(buffer);
                            if let Err(e) = Device::handle_message(
                                device_id.clone(),
                                message,
                                &sender,
                                &receiver_controls,
                            ) {
                                log::error!("Failed handling MIDI message: {}", e);
                            }
                        },
                        (),
                    )
                    .map_err(|e| {
                        format!("Failed to connect to MIDI device {}: {}", port_name, e)
                    })?,
            ));
        }
    }

    Ok(None)
}

fn output_port(midi_output: &MidiOutput, name: &str) -> Result<Option<MidiOutputPort>, String> {
    for port in midi_output.ports() {
        let port_name = midi_output
            .port_name(&port)
            .map_err(|e| format!("Failed to get MIDI port name: {}", e))?;
        if name == port_name {
            return Ok(Some(port));
        }
    }

    Ok(None)
}

impl Device {
    pub fn new(
        id: String,
        sender: Sender<ControlMessage>,
        mut config: DeviceConfig,
    ) -> Result<Device, String> {
        let midi_input =
            MidiInput::new("MidiCtrl").map_err(|e| format!("Failed to open MIDI input: {}", e))?;
        let midi_output = MidiOutput::new("MidiCtrl")
            .map_err(|e| format!("Failed to open MIDI output: {}", e))?;

        let mut output = output_port(&midi_output, &config.port)?
            .and_then(|port| midi_output.connect(&port, "MidiCtrl").ok());

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

        let connection: Option<MidiInputConnection<()>> =
            input_port(midi_input, &id, &config.port, &config.controls, sender)?;
        if connection.is_some() {
            log::debug!("Connected to MIDI device {}", id);
        }

        Ok(Device {
            connection,
            output,
            controls: config
                .controls
                .into_iter()
                .map(|control| (String::from(control.name()), control))
                .collect(),
        })
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    fn handle_message<'a>(
        device_id: String,
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
                                    device_id,
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
                                    device_id,
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
                                    device_id,
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

pub fn devices(sender: Sender<ControlMessage>, root: &Path) -> HashMap<String, Device> {
    let mut devices = HashMap::new();

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
            Ok((id, config)) => match Device::new(id.clone(), sender.clone(), config) {
                Ok(device) => {
                    devices.insert(id, device);
                }
                Err(e) => log::error!("Failed to connect to device: {}", e),
            },
            Err(e) => log::error!("Failed to parse device config: {}", e),
        }
    }

    devices
}

pub fn get_layer_control(
    devices: &HashMap<String, Device>,
    device: &str,
    control: &str,
    layer: &str,
) -> Option<LayerControl> {
    if let Some(device) = devices.get(device) {
        if let Some(control) = device.controls.get(control) {
            return control.layer(layer);
        }
    }

    None
}
