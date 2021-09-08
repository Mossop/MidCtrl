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
    device::{devices, get_layer_control, Device},
};
use profile::{Action, Profiles};
use state::State;

use crate::lightroom::OutgoingMessage;

use self::state::{Module, Value};

#[derive(Debug)]
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
    devices: HashMap<String, Device>,
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

        profile.update_devices(&mut self.devices, &self.state, false);
    }

    fn reset_state(&mut self) {
        log::trace!("Resetting state");
        self.state.clear();
        self.update_profile();
    }

    fn update_state(&mut self, module: Module, state: HashMap<String, Option<Value>>) {
        log::trace!("Updating state");

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
            Action::Sequence(actions) => {
                for action in actions {
                    self.perform_action(action);
                }
            }
        }
    }

    fn continuous_change(&mut self, device: String, control: String, layer: String, value: f64) {
        log::trace!(
            "Continuous control {} in layer {} on device {} changed to {}",
            control,
            layer,
            device,
            value
        );
        if let Some(profile) = self.profiles.current_profile() {
            if let Some(action) =
                profile.continuous_action(&self.state, &device, &control, &layer, value)
            {
                self.perform_action(action);
            }
        }
    }

    fn key_change(&mut self, device: String, control: String, layer: String, key_state: KeyState) {
        log::trace!(
            "Key control {} in layer {} on device {} changed to {}",
            control,
            layer,
            device,
            key_state
        );
        if let Some(profile) = self.profiles.current_profile() {
            if key_state == KeyState::Off {
                if let Some(layer_control) =
                    get_layer_control(&self.devices, &device, &control, &layer)
                {
                    if let Some(device) = self.devices.get_mut(&device) {
                        if let Some(ref mut connection) = device.output {
                            profile.update_layer_control(
                                connection,
                                &self.state,
                                &device.name,
                                &control,
                                &layer,
                                &layer_control,
                                false,
                            )
                        }
                    }
                }

                return;
            }

            if let Some(action) = profile.key_action(&self.state, &device, &control, &layer) {
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
