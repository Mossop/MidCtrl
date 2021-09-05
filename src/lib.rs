mod lightroom;
mod midi;
mod profile;
mod state;

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
    ControlChange,
    StateChange(Module, HashMap<String, Value>),
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

        let profiles = Profiles::new(root);
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
            lightroom: Lightroom::new(sender.clone(), 61327, 61328),
            devices: devices(sender, root),
            profiles,
            state,
        }
    }

    pub fn control_updated(&mut self, control: &Control) {
        // Generate action from profile
        // Maybe dispatch to lightroom
    }

    pub fn update_states(&mut self, module: Module, state: HashMap<String, Value>) {
        // Update our state.
        for (k, v) in state {
            self.state.insert(k, (module.clone(), v));
        }

        // Select the new profile.
        if let Some(profile) = self.profiles.select_profile(&self.state) {
            self.state.insert(
                String::from("profile"),
                (Module::Internal, Value::String(profile.name.clone())),
            );

            // Update our MIDI device displays.
            for device in self.devices.iter_mut() {
                match device.controls.lock() {
                    Ok(controls) => {
                        if let Some(ref mut output) = device.output {
                            profile.update_controls(output, &controls);
                        }
                    }
                    Err(e) => log::error!("Failed to lock controls for update: {}", e),
                };
            }
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let message = self.receiver.recv()?;
            match message {
                ControlMessage::StateChange(module, state) => self.update_states(module, state),
                ControlMessage::ControlChange => (),
            }
        }
    }
}
