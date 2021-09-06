use std::{
    error::Error,
    fs::{read_dir, File, ReadDir},
    marker::PhantomData,
    path::Path,
};

use serde::de::DeserializeOwned;

pub struct IterJson<T>
where
    T: DeserializeOwned,
{
    data_type: PhantomData<T>,
    dir_reader: ReadDir,
}

impl<T> Iterator for IterJson<T>
where
    T: DeserializeOwned,
{
    type Item = (String, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.dir_reader.next() {
                None => return None,
                Some(Ok(entry)) => entry,
                Some(Err(_)) => continue,
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };

            if !file_type.is_file() {
                continue;
            }

            let mut name = match entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };

            if !name.ends_with(".json") {
                continue;
            }

            let reader = match File::open(entry.path()) {
                Ok(reader) => reader,
                Err(_) => continue,
            };

            name.truncate(name.len() - 5);

            match serde_json::from_reader(reader) {
                Ok(data) => return Some((name, data)),
                Err(e) => (),
            }
        }
    }
}

pub fn iter_json<T>(path: &Path) -> Result<IterJson<T>, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let dir_reader = read_dir(path)?;
    Ok(IterJson {
        data_type: PhantomData,
        dir_reader: dir_reader.into_iter(),
    })
}
