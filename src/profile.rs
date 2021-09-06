use midir::MidiOutputConnection;
use serde::{Deserialize, Deserializer};
use std::{collections::HashMap, path::Path};

use crate::{
    midi::{controls::Hardware, device::Device},
    state::State,
    utils::iter_json,
};

#[derive(Deserialize, Debug)]
pub struct ProfileControl {
    device: String,
    name: String,
    layer: String,
}

fn deserialize_profile_controls<'de, D>(
    de: D,
) -> Result<HashMap<(String, String, String), ProfileControl>, D::Error>
where
    D: Deserializer<'de>,
{
    let list = Vec::<ProfileControl>::deserialize(de)?;

    let mut map = HashMap::new();

    for control in list {
        map.insert(
            (
                control.device.clone(),
                control.name.clone(),
                control.layer.clone(),
            ),
            control,
        );
    }

    Ok(map)
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    #[serde(skip)]
    pub name: String,
    #[serde(deserialize_with = "deserialize_profile_controls")]
    controls: HashMap<(String, String, String), ProfileControl>,
}

impl Profile {
    fn get_control<'a>(
        &'a self,
        device: &str,
        name: &str,
        layer: &str,
    ) -> Option<&'a ProfileControl> {
        self.controls
            .get(&(device.to_string(), name.to_string(), layer.to_string()))
    }

    pub fn verify_controls(&self, devices: &Vec<Device>) -> Result<(), String> {
        for device in devices {
            match device.controls.lock() {
                Ok(controls) => {
                    for hw in controls.iter() {
                        match hw {
                            Hardware::Continuous(hw) => {
                                for (layer, control) in &hw.layers {
                                    if let Some(profile_control) =
                                        self.get_control(&device.name, &hw.name, &layer)
                                    {
                                        ()
                                    }
                                }
                            }
                            Hardware::Key(hw) => {
                                for (layer, control) in &hw.layers {
                                    if let Some(profile_control) =
                                        self.get_control(&device.name, &hw.name, &layer)
                                    {
                                        ()
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => return Err(format!("Unable to lock device controls: {}", e)),
            };
        }

        Ok(())
    }

    pub fn update_controls(
        &self,
        connection: &mut MidiOutputConnection,
        state: &State,
        device: &str,
        controls: &mut Vec<Hardware>,
        force: bool,
    ) {
        for hw in controls.iter_mut() {
            match hw {
                Hardware::Continuous(hw) => {
                    for (layer, control) in hw.layers.iter_mut() {
                        if let Some(profile_control) = self.get_control(device, &hw.name, layer) {}
                    }
                }
                Hardware::Key(hw) => {
                    for (layer, control) in hw.layers.iter_mut() {
                        if let Some(profile_control) = self.get_control(device, &hw.name, layer) {}
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
    let entries = match iter_json::<Profile>(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Error reading profiles directory: {}", e);
            return profiles;
        }
    };

    for entry in entries {
        match entry {
            Ok((name, mut profile)) => match profile.verify_controls(devices) {
                Ok(()) => {
                    profile.name = name.clone();
                    profiles.insert(name, profile);
                }
                Err(e) => log::error!("Profile contains invalid controls: {}", e),
            },
            Err(e) => log::error!("Failed to parse profile: {}", e),
        }
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

    pub fn select_new_profile(&mut self, state: &State) -> Option<&Profile> {
        let new_profile = self.choose_profile(Some(state));

        if new_profile == self.current_profile {
            None
        } else {
            if let Some(ref name) = new_profile {
                log::info!("Switched to profile {}", name);
                self.profiles.get(name)
            } else {
                log::info!("There are no longer any valid profiles");
                None
            }
        }
    }

    pub fn current_profile(&self) -> Option<&Profile> {
        self.current_profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
    }
}
