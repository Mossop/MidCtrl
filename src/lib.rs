pub mod actions;
mod lightroom;
mod midi;
mod profile;
mod state;
pub mod utils;

use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    fs::metadata,
    io::ErrorKind,
    path::Path,
    sync::mpsc::{channel, Receiver},
};

use actions::InternalAction;
use lightroom::Lightroom;
use midi::{
    controls::KeyState,
    device::{devices, get_layer_control, Device},
};
use profile::{Action, Profile, Profiles};
use state::{
    param_module,
    params::{BoolParam, FloatParam, StringParam},
    Param, SetMapEntry, State, StateValue,
};

use crate::lightroom::OutgoingMessage;

use self::state::Module;

#[derive(Debug)]
pub enum ControlMessage {
    Disconnect,
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
        values: Vec<StateValue>,
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
    pub fn new(root: &Path) -> Result<Controller, String> {
        match metadata(root) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(format!("{} is not a directory", root.display()));
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Err(format!("Path {} not found", root.display()));
                }
                return Err(format!(
                    "Error accessing settings path {}: {}",
                    root.display(),
                    e
                ));
            }
        }

        let (sender, receiver) = channel();

        let devices = devices(sender.clone(), root);
        // if devices.is_empty() {
        //     return Err(String::from("No MIDI devices found"));
        // }

        let profiles = Profiles::new(root, &devices);
        let mut state = State::new();

        if let Some(profile) = profiles.current_profile() {
            state
                .strings
                .insert(StringParam::Profile, profile.name.clone());
        }

        // We expect that the first thing Lightroom will do is send a state update which will
        // trigger updates to the device.

        Ok(Controller {
            receiver,
            lightroom: Lightroom::new(sender, 61327, 61328),
            devices,
            profiles,
            state,
        })
    }

    fn profile_changed(&mut self, profile: &Option<Profile>) {
        self.state.strings.set(
            StringParam::Profile,
            profile.as_ref().map(|profile| profile.name.clone()),
        );

        if let Some(profile) = profile {
            self.lightroom.send(OutgoingMessage::Notification {
                message: format!("Changed to profile {}", profile.name),
            });
        } else {
            self.lightroom.send(OutgoingMessage::Notification {
                message: format!("Lost profile"),
            });
        }
    }

    fn update_profile(&mut self) {
        // Select the new profile.
        let previous_profile = self.profiles.current_profile();
        let new_profile = self.profiles.state_update(&self.state);

        if previous_profile != new_profile {
            self.profile_changed(&new_profile);
        }

        if let Some(profile) = new_profile {
            profile.update_devices(&mut self.devices, &self.state, false);
        }
    }

    fn reset_state(&mut self) {
        log::trace!("Resetting state");
        self.state.clear();
        self.update_profile();
    }

    fn update_state(&mut self, values: Vec<StateValue>) {
        log::trace!("Updating state");

        self.state.update(values);
        self.update_profile();
    }

    fn set_internal_string_parameter(&mut self, param: StringParam, value: String) {
        match param {
            StringParam::Profile => {
                if let Some(current_profile) = self.profiles.current_profile() {
                    if current_profile.name == value {
                        return;
                    }
                }

                let new_profile = self.profiles.set_profile(&value, &self.state);
                if new_profile.is_some() {
                    self.profile_changed(&new_profile);
                }

                if let Some(profile) = new_profile {
                    profile.update_devices(&mut self.devices, &self.state, false)
                };
            }
            _ => log::warn!("Attempting to set unknown parameter {:?}", param),
        }
    }

    fn set_internal_bool_parameter(&mut self, param: BoolParam, _: bool) {
        match param {
            _ => log::warn!("Attempting to set unknown parameter {:?}", param),
        }
    }

    fn set_internal_float_parameter(&mut self, param: FloatParam, _: f64) {
        match param {
            _ => log::warn!("Attempting to set unknown parameter {:?}", param),
        }
    }

    fn perform_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::SetBoolParameter { parameter, value } => match param_module(&parameter) {
                    Module::Lightroom => self.lightroom.send(OutgoingMessage::SetValue {
                        parameter: Param::Bool(parameter),
                        value: JsonValue::from(value),
                    }),
                    Module::Internal => self.set_internal_bool_parameter(parameter, value),
                },
                Action::SetFloatParameter { parameter, value } => match param_module(&parameter) {
                    Module::Lightroom => self.lightroom.send(OutgoingMessage::SetValue {
                        parameter: Param::Float(parameter),
                        value: JsonValue::from(value),
                    }),
                    Module::Internal => self.set_internal_float_parameter(parameter, value),
                },
                Action::SetStringParameter { parameter, value } => match param_module(&parameter) {
                    Module::Lightroom => self.lightroom.send(OutgoingMessage::SetValue {
                        parameter: Param::String(parameter),
                        value: JsonValue::from(value),
                    }),
                    Module::Internal => self.set_internal_string_parameter(parameter, value),
                },
                Action::LightroomAction(action) => {
                    self.lightroom.send(OutgoingMessage::Action(action));
                }
                Action::InternalAction(InternalAction::RefreshController) => {
                    if let Some(profile) = self.profiles.current_profile() {
                        profile.update_devices(&mut self.devices, &self.state, true);
                    };
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
                profile.continuous_actions(&self.state, &device, &control, &layer, value)
            {
                self.perform_actions(action);
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

            if let Some(action) = profile.key_actions(&self.state, &device, &control, &layer) {
                self.perform_actions(action);
            }
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let message = self
                .receiver
                .recv()
                .map_err(|e| format!("Control message channel failed: {}", e))?;
            match message {
                ControlMessage::Reset => self.reset_state(),
                ControlMessage::Disconnect => {
                    log::info!("Service disconnecting");
                    return Ok(());
                }
                ControlMessage::StateChange { values } => self.update_state(values),
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
