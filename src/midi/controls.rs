use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use midi_control::note::MidiNote;
use midi_control::transport::MidiMessageSend;
use midi_control::Channel;
use midir::MidiOutputConnection;
use serde::{de, de::Visitor, Deserialize, Deserializer};

fn deserialize_channel<'de, D: Deserializer<'de>>(de: D) -> Result<Channel, D::Error> {
    struct ChannelVisitor;

    impl<'de> Visitor<'de> for ChannelVisitor {
        type Value = Channel;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a number between 1 and 16")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            match v {
                1 => Ok(Channel::Ch1),
                2 => Ok(Channel::Ch2),
                3 => Ok(Channel::Ch3),
                4 => Ok(Channel::Ch4),
                5 => Ok(Channel::Ch5),
                6 => Ok(Channel::Ch6),
                7 => Ok(Channel::Ch7),
                8 => Ok(Channel::Ch8),
                9 => Ok(Channel::Ch9),
                10 => Ok(Channel::Ch10),
                11 => Ok(Channel::Ch11),
                12 => Ok(Channel::Ch12),
                13 => Ok(Channel::Ch13),
                14 => Ok(Channel::Ch14),
                15 => Ok(Channel::Ch15),
                16 => Ok(Channel::Ch16),
                _ => Err(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self)),
            }
        }
    }

    de.deserialize_any(ChannelVisitor {})
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContinuousLayer {
    #[serde(deserialize_with = "deserialize_channel")]
    pub channel: Channel,
    pub control: u8,
    min: u8,
    max: u8,
    #[serde(skip)]
    pub state: Arc<Mutex<u8>>,
}

impl ContinuousLayer {
    pub fn value_from_state(&self, state: u8) -> f64 {
        let value: f64 = (state - self.min).into();
        let range: f64 = (self.max - self.min).into();
        value / range
    }

    pub fn state_from_value(&self, value: f64) -> u8 {
        if value >= 1.0 {
            self.max
        } else if value <= 0.0 {
            self.min
        } else {
            let range: f64 = (self.max - self.min).into();
            (value * range).round() as u8 + self.min
        }
    }

    pub fn set_value(&self, state: u8) {
        let mut guard = match self.state.lock() {
            Ok(state) => state,
            Err(e) => {
                log::warn!("Failed to lock state for update: {}", e);
                return;
            }
        };

        *guard = state;
    }

    pub fn update(&self, connection: &mut MidiOutputConnection, state: u8, force: bool) {
        let mut guard = match self.state.lock() {
            Ok(state) => state,
            Err(e) => {
                log::warn!("Failed to lock state for update: {}", e);
                return;
            }
        };

        if !force && *guard == state {
            return;
        }

        let message = midi_control::control_change(self.channel, self.control, state);

        match connection.send_message(message) {
            Ok(()) => *guard = state,
            Err(e) => log::error!("Failed to send MIDI message: {}", e),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContinuousControl {
    pub name: String,
    pub layers: HashMap<String, ContinuousLayer>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(from = "bool")]
pub enum KeyState {
    Off,
    On,
}

impl From<bool> for KeyState {
    fn from(val: bool) -> Self {
        match val {
            true => KeyState::On,
            false => KeyState::Off,
        }
    }
}

impl Default for KeyState {
    fn default() -> Self {
        KeyState::Off
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct KeyLayer {
    #[serde(deserialize_with = "deserialize_channel")]
    pub channel: Channel,
    pub note: MidiNote,
    pub off: u8,
    pub on: u8,
    #[serde(skip)]
    pub state: Arc<Mutex<KeyState>>,
}

impl KeyLayer {
    pub fn set_value(&self, state: KeyState) {
        let mut guard = match self.state.lock() {
            Ok(state) => state,
            Err(e) => {
                log::warn!("Failed to lock state for update: {}", e);
                return;
            }
        };

        *guard = state;
    }

    pub fn update(&self, connection: &mut MidiOutputConnection, state: KeyState, force: bool) {
        let mut guard = match self.state.lock() {
            Ok(state) => state,
            Err(e) => {
                log::warn!("Failed to lock state for update: {}", e);
                return;
            }
        };

        if !force && state == *guard {
            return;
        }

        let message = match state {
            KeyState::On => midi_control::note_on(self.channel, self.note, self.on),
            KeyState::Off => midi_control::note_off(self.channel, self.note, self.off),
        };

        match connection.send_message(message) {
            Ok(()) => *guard = state,
            Err(e) => log::error!("Failed to send MIDI message: {}", e),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct KeyControl {
    pub name: String,
    #[serde(default)]
    pub display: bool,
    pub layers: HashMap<String, KeyLayer>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Control {
    #[serde(rename = "cc")]
    Continuous(ContinuousControl),
    #[serde(rename = "key")]
    Key(KeyControl),
}

#[derive(Clone, Debug)]
pub enum LayerControl {
    Continuous(ContinuousLayer),
    Key(KeyLayer),
}

impl Control {
    pub fn name(&self) -> &str {
        match self {
            Control::Continuous(control) => &control.name,
            Control::Key(control) => &control.name,
        }
    }

    pub fn layer(&self, layer: &str) -> Option<LayerControl> {
        match self {
            Control::Continuous(control) => control
                .layers
                .get(layer)
                .map(|layer_control| LayerControl::Continuous(layer_control.clone())),
            Control::Key(control) => control
                .layers
                .get(layer)
                .map(|layer_control| LayerControl::Key(layer_control.clone())),
        }
    }
}
