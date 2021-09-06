use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub enum Module {
    Internal,
    Lightroom,
}

pub type State = HashMap<String, (Module, Value)>;
