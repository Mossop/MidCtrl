pub mod controls;

use midir::MidiOutputConnection;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value as JsonValue;
use std::{collections::HashMap, error::Error, path::Path};

use crate::midi::controls::ContinuousLayer;
use crate::midi::controls::KeyLayer;
use crate::profile::controls::ContinuousSource;
use crate::{
    midi::{
        controls::{Control, LayerControl},
        device::Device,
    },
    state::{State, Value},
    utils::iter_json,
};

use self::controls::ContinuousAction;
use self::controls::ContinuousProfile;
use self::controls::ControlLayerInfo;
use self::controls::ControlProfile;
use self::controls::KeyAction;
use self::controls::KeyProfile;
use self::controls::KeySource;

pub enum Action {
    SetParameter { name: String, value: Value },
    Sequence(Vec<Action>),
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    controls: HashMap<ControlLayerInfo, ControlProfile>,
}

#[derive(Deserialize, Debug)]
struct ProfileConfig {
    controls: Vec<Map<String, JsonValue>>,
}

fn get_control(devices: &Vec<Device>, info: &ControlLayerInfo) -> Option<LayerControl> {
    for device in devices {
        if device.name != info.device {
            continue;
        }

        for control in &device.controls {
            if control.name() == info.control {
                return control.layer(&info.layer);
            }
        }
    }

    None
}

impl ProfileConfig {
    pub fn info_profile(
        self,
        name: &str,
        devices: &Vec<Device>,
    ) -> Result<Profile, Box<dyn Error>> {
        let mut map = HashMap::new();

        for control in self.controls {
            let info: ControlLayerInfo =
                match serde_json::from_value(JsonValue::Object(control.clone())) {
                    Ok(info) => info,
                    Err(e) => {
                        log::error!("Failed to decode control profile: {}", e);
                        continue;
                    }
                };

            let action = match get_control(devices, &info) {
                Some(layer_control) => match layer_control {
                    LayerControl::Continuous(_) => {
                        match serde_json::from_value::<ContinuousProfile>(JsonValue::Object(
                            control,
                        )) {
                            Ok(control_profile) => ControlProfile::Continuous(control_profile),
                            Err(e) => {
                                log::error!("Failed to decode control profile: {}", e);
                                continue;
                            }
                        }
                    }
                    _ => {
                        log::warn!(
                            "Control {} in layer {} on device {} is not yet supported",
                            info.control,
                            info.layer,
                            info.device
                        );
                        continue;
                    }
                },
                None => {
                    log::warn!(
                        "Unknown control {} in layer {} on device {}",
                        info.control,
                        info.layer,
                        info.device
                    );
                    continue;
                }
            };

            map.insert(info, action);
        }

        Ok(Profile {
            name: String::from(name),
            controls: map,
        })
    }
}

fn perform_continuous_update(
    connection: &mut MidiOutputConnection,
    state: &State,
    control: &ContinuousLayer,
    control_profile: &ContinuousProfile,
    force: bool,
) {
    let source = match &control_profile.source {
        Some(source) => source.resolve(state),
        None => match control_profile.action.resolve(state) {
            Some(ContinuousAction::Parameter(parameter)) => {
                Some(ContinuousSource::Parameter(parameter))
            }
            _ => None,
        },
    };

    if let Some(source) = source {
        let value = match source {
            ContinuousSource::Constant(value) => value,
            ContinuousSource::Parameter(parameter) => {
                if let Some((_, Some(Value::Float(value)))) = state.get(&parameter) {
                    *value
                } else {
                    return;
                }
            }
        };

        control.update(connection, control.state_from_value(value), force);
    }
}

fn perform_key_update(
    connection: &mut MidiOutputConnection,
    state: &State,
    control: &KeyLayer,
    control_profile: &KeyProfile,
    force: bool,
) {
    let source = match &control_profile.source {
        Some(source) => source.resolve(state),
        None => match control_profile
            .action
            .resolve(state)
            .and_then(|actions| actions.single_action())
        {
            Some(KeyAction::Parameter(parameter)) => Some(KeySource::Parameter(parameter)),
            _ => None,
        },
    };

    if let Some(source) = source {
        let value = match source {
            KeySource::Constant(value) => value,
            KeySource::Parameter(parameter) => {
                if let Some((_, Some(Value::Boolean(value)))) = state.get(&parameter) {
                    *value
                } else {
                    return;
                }
            }
        };

        control.update(connection, value.into(), force);
    }
}

impl Profile {
    fn get_control_profile<'a>(
        &'a self,
        device: &str,
        name: &str,
        layer: &str,
    ) -> Option<&'a ControlProfile> {
        let info = ControlLayerInfo {
            device: String::from(device),
            control: String::from(name),
            layer: String::from(layer),
        };

        self.controls.get(&info)
    }

    pub fn continuous_action(
        &self,
        state: &State,
        device: &str,
        name: &str,
        layer: &str,
        value: f64,
    ) -> Option<Action> {
        let control_profile = self.get_control_profile(device, name, layer)?;

        match control_profile {
            ControlProfile::Continuous(control_profile) => control_profile.action(state, value),
            _ => None,
        }
    }

    pub fn key_action(
        &self,
        state: &State,
        device: &str,
        name: &str,
        layer: &str,
    ) -> Option<Action> {
        let control_profile = self.get_control_profile(device, name, layer)?;

        match control_profile {
            ControlProfile::Key(control_profile) => control_profile.action(state),
            _ => None,
        }
    }

    pub fn update_controls(
        &self,
        connection: &mut MidiOutputConnection,
        state: &State,
        device: &str,
        controls: &Vec<Control>,
        force: bool,
    ) {
        for control in controls {
            match control {
                Control::Continuous(continuous) => {
                    for (layer, layer_control) in &continuous.layers {
                        if let Some(ControlProfile::Continuous(control_profile)) =
                            self.get_control_profile(device, &continuous.name, layer)
                        {
                            perform_continuous_update(
                                connection,
                                state,
                                layer_control,
                                control_profile,
                                force,
                            );
                        }
                    }
                }
                Control::Key(key) => {
                    for (layer, layer_control) in &key.layers {
                        if let Some(ControlProfile::Key(control_profile)) =
                            self.get_control_profile(device, &key.name, layer)
                        {
                            perform_key_update(
                                connection,
                                state,
                                layer_control,
                                control_profile,
                                force,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub struct Profiles {
    current_profile: Option<String>,
    profiles: HashMap<String, Profile>,
}

fn read_profiles(root: &Path, devices: &Vec<Device>) -> HashMap<String, Profile> {
    let mut profiles = HashMap::new();

    let dir = root.join("profiles");
    let entries = match iter_json::<ProfileConfig>(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Error reading profiles directory: {}", e);
            return profiles;
        }
    };

    for entry in entries {
        match entry {
            Ok((name, config)) => match config.info_profile(&name, devices) {
                Ok(profile) => {
                    profiles.insert(name, profile);
                }
                Err(e) => log::error!("Profile {} contained invalid controls: {}", name, e),
            },
            Err(e) => log::error!("Failed to parse profile: {}", e),
        };
    }

    profiles
}

impl Profiles {
    pub fn new(root: &Path, devices: &Vec<Device>) -> Profiles {
        let profile_list = read_profiles(root, devices);

        if profile_list.len() > 0 {
            log::info!("Loaded {} profiles", profile_list.len());
        } else {
            log::warn!("Found no profiles");
        }

        let mut profiles = Profiles {
            current_profile: None,
            profiles: profile_list,
        };

        profiles.current_profile = profiles.choose_profile(None);
        match profiles.current_profile {
            Some(ref name) => log::info!("Selected profile {}", name),
            None => log::warn!("No default profile found"),
        }

        profiles
    }

    fn choose_profile(&mut self, _state: Option<&State>) -> Option<String> {
        if self.profiles.contains_key("default") {
            return Some(String::from("default"));
        }

        None
    }

    pub fn set_profile(&mut self, name: &str) -> Option<Profile> {
        if let Some(profile) = self.profiles.get(name) {
            self.current_profile = Some(String::from(name));
            return Some(profile.clone());
        }

        None
    }

    pub fn select_new_profile(&mut self, state: &State) -> Option<Profile> {
        let new_profile = self.choose_profile(Some(state));

        if new_profile == self.current_profile {
            None
        } else {
            if let Some(ref name) = new_profile {
                log::info!("Switched to profile {}", name);
                self.profiles.get(name).cloned()
            } else {
                log::info!("There are no longer any valid profiles");
                None
            }
        }
    }

    pub fn current_profile(&self) -> Option<Profile> {
        self.current_profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
            .cloned()
    }
}
