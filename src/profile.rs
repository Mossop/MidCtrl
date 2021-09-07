use midir::MidiOutputConnection;
use serde::Deserialize;
use serde_json::Map;
use std::{collections::HashMap, error::Error, path::Path};

use crate::{
    midi::{
        controls::{Control, LayerControl},
        device::Device,
    },
    state::{State, Value},
    utils::iter_json,
};

pub enum Action {
    SetParameter { name: String, value: Value },
}

#[derive(Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ControlLayerInfo {
    device: String,
    control: String,
    layer: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ContinuousLayerAction {
    #[serde(flatten)]
    pub info: ControlLayerInfo,
    pub parameter: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyLayerAction {
    #[serde(flatten)]
    pub info: ControlLayerInfo,
    #[serde(default)]
    pub display: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub enum ControlLayerAction {
    Continuous(ContinuousLayerAction),
    Key(KeyLayerAction),
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    controls: HashMap<ControlLayerInfo, ControlLayerAction>,
}

#[derive(Deserialize, Debug)]
struct ProfileConfig {
    controls: Vec<Map<String, serde_json::Value>>,
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

        for action in self.controls {
            let info: ControlLayerInfo =
                match serde_json::from_value(serde_json::Value::Object(action.clone())) {
                    Ok(info) => info,
                    Err(e) => {
                        log::error!("Failed to decode control action: {}", e);
                        continue;
                    }
                };

            let action = match get_control(devices, &info) {
                Some(layer_control) => match layer_control {
                    LayerControl::Continuous(_) => {
                        match serde_json::from_value::<ContinuousLayerAction>(
                            serde_json::Value::Object(action),
                        ) {
                            Ok(action) => ControlLayerAction::Continuous(action),
                            Err(e) => {
                                log::error!("Failed to decode control action: {}", e);
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

fn perform_update(
    connection: &mut MidiOutputConnection,
    state: &State,
    layer_control: LayerControl,
    action: ControlLayerAction,
    force: bool,
) {
    match (layer_control.clone(), action.clone()) {
        (LayerControl::Continuous(control), ControlLayerAction::Continuous(action)) => {
            if let Some((_, Some(Value::Float(value)))) = state.get(&action.parameter) {
                control.update(connection, control.state_from_value(*value), force);
            }
        }
        _ => log::warn!(
            "Unable to apply action {:?} to control {:?}",
            action,
            layer_control
        ),
    }
}

impl Profile {
    fn get_action<'a>(
        &'a self,
        device: &str,
        name: &str,
        layer: &str,
    ) -> Option<&'a ControlLayerAction> {
        let info = ControlLayerInfo {
            device: String::from(device),
            control: String::from(name),
            layer: String::from(layer),
        };

        self.controls.get(&info)
    }

    pub fn continuous_action(
        &self,
        device: &str,
        name: &str,
        layer: &str,
        value: f64,
    ) -> Option<Action> {
        let control_action = self.get_action(device, name, layer)?;

        match control_action.clone() {
            ControlLayerAction::Continuous(action) => Some(Action::SetParameter {
                name: action.parameter.clone(),
                value: Value::Float(value),
            }),
            _ => None,
        }
    }

    pub fn key_action(&self, device: &str, name: &str, layer: &str) -> Option<Action> {
        let control_action = self.get_action(device, name, layer)?;

        None
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
                        if let Some(action) = self.get_action(device, &continuous.name, layer) {
                            perform_update(
                                connection,
                                state,
                                LayerControl::Continuous(layer_control.clone()),
                                action.clone(),
                                force,
                            );
                        }
                    }
                }
                Control::Key(key) => {
                    for (layer, layer_control) in &key.layers {
                        if let Some(action) = self.get_action(device, &key.name, layer) {
                            perform_update(
                                connection,
                                state,
                                LayerControl::Key(layer_control.clone()),
                                action.clone(),
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

    fn choose_profile(&mut self, state: Option<&State>) -> Option<String> {
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
