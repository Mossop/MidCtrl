use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
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

pub type State = HashMap<String, (Module, Option<Value>)>;
