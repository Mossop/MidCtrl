use midir::MidiOutputConnection;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

use crate::{midi::controls::Control, state::State, utils::iter_json};

#[derive(Deserialize, Debug)]
pub struct ProfileControl {
    device: String,
    layer: Option<String>,
    name: String,
}

#[derive(Deserialize, Debug)]
pub struct Profile {
    #[serde(skip)]
    pub name: String,
    controls: Vec<ProfileControl>,
}

impl Profile {
    pub fn update_controls(&self, connection: &mut MidiOutputConnection, controls: &Vec<Control>) {}
}

pub struct Profiles {
    current_profile: Option<String>,
    profiles: HashMap<String, Profile>,
}

fn read_profiles(root: &Path) -> HashMap<String, Profile> {
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
            Ok((name, mut profile)) => {
                profile.name = name.clone();
                profiles.insert(name, profile);
            }
            Err(e) => log::error!("Failed to parse profile: {}", e),
        }
    }

    profiles
}

impl Profiles {
    pub fn new(root: &Path) -> Profiles {
        let profile_list = read_profiles(root);

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
