pub mod controls;

use midir::MidiOutputConnection;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::{collections::HashMap, path::Path};

use crate::actions::InternalAction;
use crate::lightroom::LightroomAction;
use crate::midi::controls::KeyLayer;
use crate::midi::controls::{ContinuousLayer, KeyState};
use crate::midi::device::get_layer_control;
use crate::profile::controls::ContinuousSource;
use crate::state::deserialize_string_param;
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
        #[serde(deserialize_with = "deserialize_string_param")]
        parameter: StringParam,
        value: String,
    },
    LightroomAction(LightroomAction),
    InternalAction(InternalAction),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ControlConfig {
    Control(ControlProfile),
    Include { include: String },
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub id: String,
    name: Option<String>,
    when: Option<Condition>,
    controls: HashMap<ControlLayerInfo, ControlProfile>,
}

#[derive(Deserialize, Debug)]
struct ProfileConfig {
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "if")]
    #[serde(default)]
    when: Option<Condition>,
    controls: Vec<ControlConfig>,
}

fn add_controls(
    profile: &str,
    devices: &HashMap<String, Device>,
    map: &mut HashMap<ControlLayerInfo, ControlProfile>,
    path: &Path,
    controls: Vec<ControlConfig>,
) -> Result<(), String> {
    for control in controls {
        match control {
            ControlConfig::Include { include } => {
                let new_path = path.join(include);
                let file = File::open(&new_path).map_err(|e| {
                    format!("Failed to open included file {}: {}", new_path.display(), e)
                })?;
                let included: Vec<ControlConfig> = serde_json::from_reader(file).map_err(|e| {
                    format!(
                        "Failed to parse included file {} at line {}, column {}: {}",
                        new_path.display(),
                        e.line(),
                        e.column(),
                        e
                    )
                })?;
                add_controls(profile, devices, map, new_path.parent().unwrap(), included)?;
            }
            ControlConfig::Control(control) => {
                let info = control.info();
                if map.contains_key(&info) {
                    log::warn!("Found duplicate definition for control {} in layer {} on device {} in profile {}", info.control, info.layer, info.device_id, profile);
                }

                match (
                    control,
                    get_layer_control(devices, &info.device_id, &info.control, &info.layer),
                ) {
                    (ControlProfile::Continuous(control), Some(LayerControl::Continuous(_))) => {
                        map.insert(info, ControlProfile::Continuous(control));
                    }
                    (ControlProfile::Key(control), Some(LayerControl::Key(_))) => {
                        map.insert(info, ControlProfile::Key(control));
                    }
                    (control_profile, Some(device_control)) => {
                        return Err(format!("Profile {} configuration for control {} in device {}, layer {} did not match the control type from the device, {:?} {:?}", profile, info.control, info.device_id, info.layer, control_profile, device_control));
                    }
                    (_, _) => {
                        return Err(format!("Profile {} configuration contained control {} in layer {}that does not exist in device {}", profile, info.control, info.layer, info.device_id));
                    }
                }
            }
        }
    }

    Ok(())
}

impl ProfileConfig {
    pub fn into_profile(
        self,
        path: &Path,
        id: &str,
        devices: &HashMap<String, Device>,
    ) -> Result<Profile, String> {
        let mut map = HashMap::new();

        add_controls(id, devices, &mut map, path, self.controls)?;

        Ok(Profile {
            id: String::from(id),
            name: self.name,
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
    let source = match &control_profile.value_source {
        Some(source) => source.resolve(state),
        None => match control_profile.on_change.resolve(state) {
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
    let source = match &control_profile.note_source {
        Some(source) => source.resolve(state),
        None => match &control_profile.on_press.resolve(state) {
            Some(KeyAction::Parameter(parameter)) => Some(KeySource::Parameter(parameter.clone())),
            Some(KeyAction::Toggle { toggle: parameter }) => {
                Some(KeySource::Parameter(parameter.clone()))
            }
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
    pub fn name(&self) -> String {
        match &self.name {
            Some(name) => name.clone(),
            None => self.id.clone(),
        }
    }

    fn get_control_profile<'a>(
        &'a self,
        device_id: &str,
        control_name: &str,
        layer: &str,
    ) -> Option<&'a ControlProfile> {
        let info = ControlLayerInfo {
            device_id: String::from(device_id),
            control: String::from(control_name),
            layer: String::from(layer),
        };

        self.controls.get(&info)
    }

    pub fn continuous_actions(
        &self,
        state: &State,
        device_id: &str,
        control_name: &str,
        layer: &str,
        value: f64,
    ) -> Option<Vec<Action>> {
        let control_profile = self.get_control_profile(device_id, control_name, layer)?;

        match control_profile {
            ControlProfile::Continuous(control_profile) => {
                control_profile.change_action(state, value)
            }
            _ => None,
        }
    }

    pub fn key_actions(
        &self,
        state: &State,
        device_id: &str,
        control_name: &str,
        layer: &str,
        key_state: KeyState,
    ) -> Option<Vec<Action>> {
        let control_profile = self.get_control_profile(device_id, control_name, layer)?;

        match (control_profile, key_state) {
            (ControlProfile::Key(control_profile), KeyState::On) => {
                control_profile.press_actions(state)
            }
            (ControlProfile::Key(control_profile), KeyState::Off) => {
                control_profile.release_actions(state)
            }
            _ => None,
        }
    }

    pub fn update_layer_control(
        &self,
        connection: &mut MidiOutputConnection,
        state: &State,
        device_id: &str,
        control_name: &str,
        layer: &str,
        layer_control: &LayerControl,
        force: bool,
    ) {
        match layer_control {
            LayerControl::Continuous(layer_control) => {
                if let Some(ControlProfile::Continuous(control_profile)) =
                    self.get_control_profile(device_id, control_name, layer)
                {
                    perform_continuous_update(
                        connection,
                        state,
                        layer_control,
                        control_profile,
                        force,
                    );
                } else {
                    layer_control.update(connection, layer_control.min, force);
                }
            }
            LayerControl::Key(layer_control) => {
                if let Some(ControlProfile::Key(control_profile)) =
                    self.get_control_profile(device_id, control_name, layer)
                {
                    perform_key_update(connection, state, layer_control, control_profile, force);
                } else {
                    layer_control.update(connection, KeyState::Off, true);
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
        for (id, device) in devices {
            if let Some(ref mut output) = device.output {
                for control in device.controls.values() {
                    for (layer, layer_control) in control.layers() {
                        self.update_layer_control(
                            output,
                            state,
                            id,
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
        self.id == other.id
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
            Ok((id, config)) => match config.into_profile(&dir, &id, devices) {
                Ok(profile) => {
                    profiles.insert(id, profile);
                }
                Err(e) => log::error!("{}", e),
            },
            Err(e) => log::error!("{}", e),
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

    pub fn set_profile(&mut self, id: &str, state: &State) -> Option<Profile> {
        if let Some(profile) = self.profiles.get(id) {
            if profile.is_enabled(state) {
                self.current_profile = Some(String::from(id));
                Some(profile.clone())
            } else {
                log::warn!("Attempted to select profile {} but it is not available", id);
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
            .and_then(|id| self.profiles.get(id))
        {
            if profile.is_enabled(state) {
                return Some(profile.clone());
            }
        }

        for (id, profile) in &self.profiles {
            if profile.is_enabled(state) {
                log::info!("Switched to profile {}", id);
                self.current_profile = Some(id.clone());
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
            .and_then(|id| self.profiles.get(id))
            .cloned()
    }
}
