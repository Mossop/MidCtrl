use std::{
    fs::{read_dir, File, ReadDir},
    io,
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
    type Item = Result<(String, T), String>;

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

            let file_name = match entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };

            if !file_name.ends_with(".json") {
                continue;
            }

            let reader = match File::open(entry.path()) {
                Ok(reader) => reader,
                Err(_) => continue,
            };

            let mut name = file_name.clone();
            name.truncate(name.len() - 5);

            match serde_json::from_reader(reader) {
                Ok(data) => return Some(Ok((name, data))),
                Err(e) => {
                    return Some(Err(format!(
                        "Failed to parse {} at line {}, column {}: {}",
                        file_name,
                        e.line(),
                        e.column(),
                        e
                    )))
                }
            }
        }
    }
}

pub fn iter_json<T>(path: &Path) -> Result<IterJson<T>, io::Error>
where
    T: DeserializeOwned,
{
    let dir_reader = read_dir(path)?;
    Ok(IterJson {
        data_type: PhantomData,
        dir_reader,
    })
}
