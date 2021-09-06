mod lightroom;
mod midi;
mod profile;
mod state;
pub mod utils;

use std::{
    collections::HashMap,
    error::Error,
    path::Path,
    sync::mpsc::{channel, Receiver},
};

use lightroom::Lightroom;
use midi::{
    controls::ButtonState,
    device::{devices, Device},
};
use profile::Profiles;
use state::State;

use crate::lightroom::OutgoingMessage;

use self::state::{Module, Value};

pub enum ControlMessage {
    Reset,
    ContinuousChange {
        device: String,
        control: String,
        layer: String,
        value: f64,
    },
    NoteChange {
        device: String,
        control: String,
        layer: String,
        state: ButtonState,
    },
    StateChange {
        module: Module,
        state: HashMap<String, Option<Value>>,
    },
}

pub struct Controller {
    receiver: Receiver<ControlMessage>,
    lightroom: Lightroom,
    devices: Vec<Device>,
    profiles: Profiles,
    state: State,
}

impl Controller {
    pub fn new(root: &Path) -> Controller {
        let (sender, receiver) = channel();

        let devices = devices(sender.clone(), root);
        let profiles = Profiles::new(root, &devices);
        let mut state = State::new();

        if let Some(profile) = profiles.current_profile() {
            state.insert(
                String::from("profile"),
                (Module::Internal, Value::String(profile.name.clone())),
            );
        }

        // We expect that the first thing Lightroom will do is send a state update which will
        // trigger updates to the device.

        Controller {
            receiver,
            lightroom: Lightroom::new(sender, 61327, 61328),
            devices,
            profiles,
            state,
        }
    }

    fn update_profile(&mut self) {
        // Select the new profile.
        let profile = match self.profiles.select_new_profile(&self.state) {
            Some(profile) => {
                self.state.insert(
                    String::from("profile"),
                    (Module::Internal, Value::String(profile.name.clone())),
                );

                self.lightroom.send(OutgoingMessage::Notification {
                    message: format!("Changed to profile {}", profile.name),
                });
                profile
            }
            None => match self.profiles.current_profile() {
                Some(profile) => profile,
                None => return,
            },
        };

        log::trace!("New state: {:?}", self.state);

        for device in self.devices.iter_mut() {
            match device.controls.lock() {
                Ok(ref mut controls) => {
                    if let Some(ref mut output) = device.output {
                        profile.update_controls(output, &self.state, &device.name, controls, false);
                    }
                }
                Err(e) => log::error!("Failed to lock controls for update: {}", e),
            };
        }
    }

    fn reset_state(&mut self) {
        self.state.clear();
        self.update_profile();
    }

    fn update_state(&mut self, module: Module, state: HashMap<String, Option<Value>>) {
        // Update our state.
        for (k, v) in state {
            match v {
                Some(val) => self.state.insert(k, (module.clone(), val)),
                None => self.state.remove(&k),
            };
        }

        self.update_profile();
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let message = self.receiver.recv()?;
            match message {
                ControlMessage::Reset => self.reset_state(),
                ControlMessage::StateChange { module, state } => self.update_state(module, state),
                _ => (),
            }
        }
    }
}
