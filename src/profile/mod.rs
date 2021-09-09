pub mod controls;

use midir::MidiOutputConnection;
use serde::Deserialize;
use serde_json::Map;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::fs::File;
use std::{collections::HashMap, path::Path};

use crate::actions::InternalAction;
use crate::lightroom::LightroomAction;
use crate::midi::controls::ContinuousLayer;
use crate::midi::controls::KeyLayer;
use crate::midi::device::get_layer_control;
use crate::profile::controls::ContinuousSource;
use crate::state::params::BoolParam;
use crate::state::params::FloatParam;
use crate::state::params::StringParam;
use crate::state::Condition;
use crate::{
    midi::{controls::LayerControl, device::Device},
    state::State,
    utils::iter_json,
};

use self::controls::ContinuousAction;
use self::controls::ContinuousProfile;
use self::controls::ControlLayerInfo;
use self::controls::ControlProfile;
use self::controls::KeyAction;
use self::controls::KeyProfile;
use self::controls::KeySource;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Action {
    SetFloatParameter {
        parameter: FloatParam,
        value: f64,
    },
    SetBoolParameter {
        parameter: BoolParam,
        value: bool,
    },
    SetStringParameter {
        parameter: StringParam,
        value: String,
    },
    LightroomAction(LightroomAction),
    InternalAction(InternalAction),
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    when: Option<Condition>,
    controls: HashMap<ControlLayerInfo, ControlProfile>,
}

#[derive(Deserialize, Debug)]
struct ProfileConfig {
    #[serde(rename = "if")]
    #[serde(default)]
    when: Option<Condition>,
    controls: Vec<Map<String, JsonValue>>,
}

fn add_controls(
    devices: &HashMap<String, Device>,
    map: &mut HashMap<ControlLayerInfo, ControlProfile>,
    path: &Path,
    controls: Vec<Map<String, JsonValue>>,
) -> Result<(), String> {
    for control in controls {
        if let Some(JsonValue::String(include)) = control.get("include") {
            let new_path = path.join(include);
            let file = File::open(&new_path).map_err(|e| {
                format!("Failed to open included file {}: {}", new_path.display(), e)
            })?;
            let included: Vec<Map<String, JsonValue>> =
                serde_json::from_reader(file).map_err(|e| {
                    format!(
                        "Failed to parse included file {}: {}",
                        new_path.display(),
                        e
                    )
                })?;
            add_controls(devices, map, new_path.parent().unwrap(), included)?;
            continue;
        }

        let info: ControlLayerInfo =
            match serde_json::from_value(JsonValue::Object(control.clone())) {
                Ok(info) => info,
                Err(e) => {
                    log::error!("Failed to decode control profile: {}", e);
                    continue;
                }
            };

        let action = match get_layer_control(devices, &info.device, &info.control, &info.layer) {
            Some(layer_control) => match layer_control {
                LayerControl::Continuous(_) => {
                    match serde_json::from_value::<ContinuousProfile>(JsonValue::Object(control)) {
                        Ok(control_profile) => ControlProfile::Continuous(control_profile),
                        Err(e) => {
                            log::error!("Failed to decode control profile: {}", e);
                            continue;
                        }
                    }
                }
                LayerControl::Key(_) => {
                    match serde_json::from_value::<KeyProfile>(JsonValue::Object(control)) {
                        Ok(control_profile) => ControlProfile::Key(control_profile),
                        Err(e) => {
                            log::error!("Failed to decode control profile: {}", e);
                            continue;
                        }
                    }
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

    Ok(())
}

impl ProfileConfig {
    pub fn into_profile(
        self,
        path: &Path,
        name: &str,
        devices: &HashMap<String, Device>,
    ) -> Result<Profile, String> {
        let mut map = HashMap::new();

        add_controls(devices, &mut map, path, self.controls)?;

        Ok(Profile {
            name: String::from(name),
            when: self.when,
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
                if let Some(value) = state.floats.get(&parameter) {
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
            Some(KeyAction::Toggle { toggle: parameter }) => Some(KeySource::Parameter(parameter)),
            _ => None,
        },
    };

    if let Some(source) = source {
        let value = match source {
            KeySource::Constant(value) => value,
            KeySource::Parameter(parameter) => {
                if let Some(value) = state.bools.get(&parameter) {
                    *value
                } else {
                    false
                }
            }
            KeySource::InvertedParameter { parameter, invert } => {
                if let Some(value) = state.bools.get(&parameter) {
                    if invert {
                        !*value
                    } else {
                        *value
                    }
                } else {
                    false
                }
            }
            KeySource::Condition { condition, invert } => {
                let result = condition.matches(state);
                if invert {
                    !result
                } else {
                    result
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

    pub fn continuous_actions(
        &self,
        state: &State,
        device: &str,
        name: &str,
        layer: &str,
        value: f64,
    ) -> Option<Vec<Action>> {
        let control_profile = self.get_control_profile(device, name, layer)?;

        match control_profile {
            ControlProfile::Continuous(control_profile) => control_profile.actions(state, value),
            _ => None,
        }
    }

    pub fn key_actions(
        &self,
        state: &State,
        device: &str,
        name: &str,
        layer: &str,
    ) -> Option<Vec<Action>> {
        let control_profile = self.get_control_profile(device, name, layer)?;

        match control_profile {
            ControlProfile::Key(control_profile) => control_profile.actions(state),
            _ => None,
        }
    }

    pub fn update_layer_control(
        &self,
        connection: &mut MidiOutputConnection,
        state: &State,
        device: &str,
        control: &str,
        layer: &str,
        layer_control: &LayerControl,
        force: bool,
    ) {
        match layer_control {
            LayerControl::Continuous(layer_control) => {
                if let Some(ControlProfile::Continuous(control_profile)) =
                    self.get_control_profile(device, control, layer)
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
            LayerControl::Key(layer_control) => {
                if let Some(ControlProfile::Key(control_profile)) =
                    self.get_control_profile(device, control, layer)
                {
                    perform_key_update(connection, state, layer_control, control_profile, force);
                }
            }
        }
    }

    pub fn update_devices(
        &self,
        devices: &mut HashMap<String, Device>,
        state: &State,
        force: bool,
    ) {
        for device in devices.values_mut() {
            if let Some(ref mut output) = device.output {
                for control in device.controls.values() {
                    for (layer, layer_control) in control.layers() {
                        self.update_layer_control(
                            output,
                            state,
                            &device.name,
                            control.name(),
                            &layer,
                            &layer_control,
                            force,
                        );
                    }
                }
            }
        }
    }

    pub fn is_enabled(&self, state: &State) -> bool {
        match &self.when {
            Some(condition) => condition.matches(state),
            None => true,
        }
    }
}

impl PartialEq for Profile {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

pub struct Profiles {
    current_profile: Option<String>,
    profiles: BTreeMap<String, Profile>,
}

fn read_profiles(root: &Path, devices: &HashMap<String, Device>) -> BTreeMap<String, Profile> {
    let mut profiles = BTreeMap::new();

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
            Ok((name, config)) => match config.into_profile(&dir, &name, devices) {
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
    pub fn new(root: &Path, devices: &HashMap<String, Device>) -> Profiles {
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

        profiles.state_update(&State::new());
        profiles
    }

    pub fn set_profile(&mut self, name: &str, state: &State) -> Option<Profile> {
        if let Some(profile) = self.profiles.get(name) {
            if profile.is_enabled(state) {
                self.current_profile = Some(String::from(name));
                Some(profile.clone())
            } else {
                log::warn!(
                    "Attempted to select profile {} but it is not available",
                    name
                );
                None
            }
        } else {
            None
        }
    }

    pub fn state_update(&mut self, state: &State) -> Option<Profile> {
        if let Some(profile) = self
            .current_profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
        {
            if profile.is_enabled(state) {
                return Some(profile.clone());
            }
        }

        for (name, profile) in &self.profiles {
            if profile.is_enabled(state) {
                log::info!("Switched to profile {}", name);
                self.current_profile = Some(name.clone());
                return Some(profile.clone());
            }
        }

        log::info!("There are no valid profiles");
        self.current_profile = None;
        None
    }

    pub fn current_profile(&self) -> Option<Profile> {
        self.current_profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
            .cloned()
    }
}
