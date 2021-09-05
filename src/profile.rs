use serde::Deserialize;
use std::{
    collections::HashMap,
    error::Error,
    fs::{read_dir, DirEntry, File},
    io,
    path::Path,
};

use crate::state::State;

#[derive(Deserialize, Debug)]
pub struct Profile {
    #[serde(skip)]
    pub name: String,
}

pub struct Profiles {
    current_profile: Option<String>,
    profiles: HashMap<String, Profile>,
}

fn read_profile(entry: Result<DirEntry, io::Error>) -> Result<Option<Profile>, Box<dyn Error>> {
    let entry = entry?;

    let file_type = entry.file_type()?;
    if !file_type.is_file() {
        return Ok(None);
    }

    let file = File::open(entry.path())?;
    Ok(Some(serde_json::from_reader(file)?))
}

fn read_profiles(root: &Path) -> HashMap<String, Profile> {
    let mut profiles = HashMap::new();

    let dir = root.join("profiles");
    let entries = match read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("Failed to read profiles directory: {}", e);
            return profiles;
        }
    };

    for entry in entries {
        match read_profile(entry) {
            Ok(Some(profile)) => profiles.insert(profile.name.clone(), profile),
            Ok(None) => continue,
            Err(e) => {
                log::error!("Failed to read profiles directory: {}", e);
                continue;
            }
        };
    }

    if profiles.is_empty() {
        log::warn!("Found no profiles.");
    }

    profiles
}

impl Profiles {
    pub fn new(root: &Path) -> Profiles {
        let profiles = read_profiles(root);

        if profiles.len() > 0 {
            log::info!("Loaded {} profiles", profiles.len());

            if profiles.contains_key("default") {
                return Profiles {
                    current_profile: Some(String::from("default")),
                    profiles,
                };
            } else {
                log::warn!("No default profile found");
            }
        } else {
            log::warn!("Found no profiles");
        }

        Profiles {
            current_profile: None,
            profiles,
        }
    }

    pub fn select_profile(&mut self, state: &State) -> Option<&Profile> {
        self.current_profile()
    }

    pub fn current_profile(&self) -> Option<&Profile> {
        self.current_profile
            .as_ref()
            .and_then(|name| self.profiles.get(name))
    }
}
