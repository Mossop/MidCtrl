use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Integer {
    value: i64,
    min: i64,
    max: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Float {
    value: f64,
    min: f64,
    max: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Integer(Integer),
    Float(Float),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub enum Module {
    Internal,
    Lightroom,
}

pub type State = HashMap<String, (Module, Value)>;
