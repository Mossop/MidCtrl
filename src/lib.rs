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
    controls::Control,
    device::{devices, Device},
};
use profile::Profiles;
use state::State;

use self::state::{Module, Value};

pub enum ControlMessage {
    Reset,
    ControlChange {
        device: String,
        control: Control,
    },
    StateChange {
        module: Module,
        state: HashMap<String, Value>,
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

    fn control_updated(&mut self, device: String, control: Control) {
        // Generate action from profile
        // Maybe dispatch to lightroom
    }

    fn update_profile(&mut self) {
        // Select the new profile.
        let profile = match self.profiles.select_new_profile(&self.state) {
            Some(profile) => {
                self.state.insert(
                    String::from("profile"),
                    (Module::Internal, Value::String(profile.name.clone())),
                );

                // Send message about changed profile.
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

    fn update_state(&mut self, module: Module, state: HashMap<String, Value>) {
        // Update our state.
        for (k, v) in state {
            self.state.insert(k, (module.clone(), v));
        }

        self.update_profile();
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let message = self.receiver.recv()?;
            match message {
                ControlMessage::Reset => self.reset_state(),
                ControlMessage::StateChange { module, state } => self.update_state(module, state),
                ControlMessage::ControlChange { device, control } => {
                    log::trace!("Saw control change {:?} on device {}", control, device);
                    self.control_updated(device, control)
                }
            }
        }
    }
}
