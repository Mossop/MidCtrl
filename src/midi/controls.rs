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
pub struct ControlInfo {
    name: String,
    layer: Option<String>,
}

impl PartialEq for ControlInfo {
    fn eq(&self, other: &ControlInfo) -> bool {
        self.name.eq(&other.name) && self.layer.eq(&other.layer)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContinuousControl {
    #[serde(flatten)]
    info: ControlInfo,
    #[serde(deserialize_with = "deserialize_channel")]
    channel: Channel,
    control: u8,
    min: u8,
    max: u8,
    #[serde(skip)]
    state: u8,
}

impl ContinuousControl {
    pub fn update(&mut self, connection: &mut MidiOutputConnection, state: u8) {
        let message = midi_control::control_change(self.channel, self.control, state);

        match connection.send_message(message) {
            Ok(()) => self.state = state,
            Err(e) => log::error!("Failed to send MIDI message: {}", e),
        }
    }
}

impl PartialEq for ContinuousControl {
    fn eq(&self, other: &ContinuousControl) -> bool {
        self.channel.eq(&other.channel) && self.control.eq(&other.control)
    }
}

#[derive(Clone, Debug)]
pub enum KeyState {
    Off,
    On,
}

impl Default for KeyState {
    fn default() -> Self {
        KeyState::Off
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct KeyControl {
    #[serde(flatten)]
    info: ControlInfo,
    #[serde(deserialize_with = "deserialize_channel")]
    channel: Channel,
    note: MidiNote,
    off: u8,
    on: u8,
    #[serde(default)]
    display: bool,
    #[serde(skip)]
    state: KeyState,
}

impl KeyControl {
    pub fn update(&mut self, connection: &mut MidiOutputConnection, state: KeyState) {
        let message = match state {
            KeyState::On => midi_control::note_on(self.channel, self.note, self.on),
            KeyState::Off => midi_control::note_off(self.channel, self.note, self.off),
        };

        match connection.send_message(message) {
            Ok(()) => self.state = state,
            Err(e) => log::error!("Failed to send MIDI message: {}", e),
        }
    }
}

impl PartialEq for KeyControl {
    fn eq(&self, other: &KeyControl) -> bool {
        self.channel.eq(&other.channel) && self.note.eq(&other.note)
    }
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
#[serde(tag = "type")]
pub enum Control {
    #[serde(rename = "cc")]
    Continuous(ContinuousControl),
    #[serde(rename = "key")]
    Key(KeyControl),
}
