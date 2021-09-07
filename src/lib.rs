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
    controls::KeyState,
    device::{devices, Device},
};
use profile::{Action, Profiles};
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
    KeyChange {
        device: String,
        control: String,
        layer: String,
        state: KeyState,
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
                (Module::Internal, Some(Value::String(profile.name.clone()))),
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

    fn profile_changed(&mut self, profile: &str) {
        self.state.insert(
            String::from("profile"),
            (Module::Internal, Some(Value::String(String::from(profile)))),
        );

        self.lightroom.send(OutgoingMessage::Notification {
            message: format!("Changed to profile {}", profile),
        });
    }

    fn update_profile(&mut self) {
        // Select the new profile.
        let profile = match self.profiles.select_new_profile(&self.state) {
            Some(profile) => {
                self.profile_changed(&profile.name);
                profile
            }
            None => match self.profiles.current_profile() {
                Some(profile) => profile,
                None => return,
            },
        };

        log::trace!("New state: {:?}", self.state);

        for device in self.devices.iter_mut() {
            if let Some(ref mut output) = device.output {
                profile.update_controls(output, &self.state, &device.name, &device.controls, false);
            }
        }
    }

    fn reset_state(&mut self) {
        self.state.clear();
        self.update_profile();
    }

    fn update_state(&mut self, module: Module, state: HashMap<String, Option<Value>>) {
        // Update our state.
        for (k, v) in state {
            self.state.insert(k, (module.clone(), v));
        }

        self.update_profile();
    }

    fn set_internal_parameter(&mut self, name: String, value: Value) {
        match (name.as_str(), value) {
            ("profile", Value::String(val)) => {
                if let Some(profile) = self.profiles.set_profile(&val) {
                    self.profile_changed(&profile.name);
                };
            }
            _ => log::warn!("Attempting to set unknown parameter {}", name),
        }
    }

    fn perform_action(&mut self, action: Action) {
        match action {
            Action::SetParameter { name, value } => {
                if let Some((module, _)) = self.state.get(&name) {
                    match module {
                        Module::Internal => self.set_internal_parameter(name, value),
                        Module::Lightroom => self
                            .lightroom
                            .send(OutgoingMessage::SetValue { name, value }),
                    }
                } else {
                    log::warn!("Attempting to set unknown parameter {}", name);
                }
            }
        }
    }

    fn continuous_change(&mut self, device: String, control: String, layer: String, value: f64) {
        if let Some(profile) = self.profiles.current_profile() {
            if let Some(action) = profile.continuous_action(&device, &control, &layer, value) {
                self.perform_action(action);
            }
        }
    }

    fn key_change(&mut self, device: String, control: String, layer: String, state: KeyState) {
        if state == KeyState::Off {
            return;
        }

        if let Some(profile) = self.profiles.current_profile() {
            if let Some(action) = profile.key_action(&device, &control, &layer) {
                self.perform_action(action);
            }
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let message = self.receiver.recv()?;
            match message {
                ControlMessage::Reset => self.reset_state(),
                ControlMessage::StateChange { module, state } => self.update_state(module, state),
                ControlMessage::ContinuousChange {
                    device,
                    control,
                    layer,
                    value,
                } => self.continuous_change(device, control, layer, value),
                ControlMessage::KeyChange {
                    device,
                    control,
                    layer,
                    state,
                } => self.key_change(device, control, layer, state),
            }
        }
    }
}
