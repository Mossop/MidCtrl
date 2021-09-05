mod lightroom;
mod midi;
mod profile;
mod state;

use std::{collections::HashMap, path::Path};

use lightroom::Lightroom;
use midi::{
    controls::Control,
    device::{devices, Device},
};
use profile::Profiles;
use state::State;

use self::state::{Module, Value};

pub struct Controller {
    lightroom: Lightroom,
    devices: Vec<Device>,
    profiles: Profiles,
    state: State,
}

impl Controller {
    pub fn new(root: &Path) -> Controller {
        Controller {
            lightroom: Lightroom::new(61327, 61328),
            devices: devices(root),
            profiles: Profiles::new(root),
            state: State::new(),
        }
    }

    pub fn control_updated(&mut self, control: &Control) {
        // Generate action from profile
    }

    pub fn update_states(&mut self, module: Module, state: HashMap<String, Value>) {
        for (k, v) in state {
            self.state.insert(k, (module.clone(), v));
        }

        if let Some(profile) = self.profiles.select_profile(&self.state) {
            // Apply profile to devices
        }
    }
}
