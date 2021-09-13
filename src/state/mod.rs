pub mod params;

use std::hash::Hash;
use std::{collections::HashMap, convert::TryFrom, fmt::Display};

use serde::{Deserialize, Deserializer, Serialize};

use self::params::{BoolParam, FloatParam, StringParam};

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SerializedStringParam {
    Param(StringParam),
    Custom(String),
}

pub fn deserialize_string_param<'de, D>(deserializer: D) -> Result<StringParam, D::Error>
where
    D: Deserializer<'de>,
{
    match SerializedStringParam::deserialize(deserializer)? {
        SerializedStringParam::Param(string_param) => Ok(string_param),
        SerializedStringParam::Custom(str) => Ok(StringParam::Custom(str)),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Module {
    Internal,
    Lightroom,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Param {
    Float(FloatParam),
    Bool(BoolParam),
    String(StringParam),
}

impl Into<Param> for FloatParam {
    fn into(self) -> Param {
        Param::Float(self)
    }
}

impl Into<Param> for BoolParam {
    fn into(self) -> Param {
        Param::Bool(self)
    }
}

impl Into<Param> for StringParam {
    fn into(self) -> Param {
        Param::String(self)
    }
}

pub fn param_module<P>(param: &P) -> Module
where
    P: Clone + Into<Param>,
{
    let param: Param = param.clone().into();
    match param {
        Param::String(StringParam::Profile) => Module::Internal,
        Param::String(StringParam::Custom(_)) => Module::Internal,
        _ => Module::Lightroom,
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum StateValue {
    Float {
        parameter: FloatParam,
        value: Option<f64>,
    },
    String {
        parameter: StringParam,
        value: Option<String>,
    },
    Bool {
        parameter: BoolParam,
        value: Option<bool>,
    },
}

pub trait SetMapEntry {
    type Key;
    type Value;

    fn set(&mut self, param: Self::Key, value: Option<Self::Value>) -> ();
}

impl<P, V> SetMapEntry for HashMap<P, V>
where
    P: Eq + Hash,
{
    type Key = P;
    type Value = V;

    fn set(&mut self, param: P, value: Option<V>) {
        match value {
            Some(v) => self.insert(param, v),
            None => self.remove(&param),
        };

        ()
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Float(f64),
    Boolean(bool),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(val) => write!(f, "\"{}\"", val),
            Value::Boolean(val) => write!(f, "{}", val),
            Value::Float(val) => write!(f, "{}", val),
        }
    }
}

#[derive(Default)]
pub struct State {
    pub bools: HashMap<BoolParam, bool>,
    pub floats: HashMap<FloatParam, f64>,
    pub strings: HashMap<StringParam, String>,
}

impl State {
    pub fn new() -> State {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.bools.clear();
        self.floats.clear();
        self.strings.clear();
    }

    pub fn update(&mut self, values: Vec<StateValue>) {
        for value in values {
            match value {
                StateValue::Float { parameter, value } => self.floats.set(parameter, value),
                StateValue::String { parameter, value } => self.strings.set(parameter, value),
                StateValue::Bool { parameter, value } => self.bools.set(parameter, value),
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub enum GeneralComparison {
    Equal,
    NotEqual,
}

impl Default for GeneralComparison {
    fn default() -> Self {
        GeneralComparison::Equal
    }
}

impl TryFrom<String> for GeneralComparison {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "==" => Ok(GeneralComparison::Equal),
            "!=" => Ok(GeneralComparison::NotEqual),
            _ => Err(format!("Unknown comparison: {}", value)),
        }
    }
}

#[derive(Deserialize, PartialEq, Debug, Clone)]
#[serde(try_from = "String")]
pub enum NumericComparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
}

impl Default for NumericComparison {
    fn default() -> Self {
        NumericComparison::Equal
    }
}

impl TryFrom<String> for NumericComparison {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "==" => Ok(NumericComparison::Equal),
            "!=" => Ok(NumericComparison::NotEqual),
            "<" => Ok(NumericComparison::LessThan),
            "<=" => Ok(NumericComparison::LessThanEqual),
            ">" => Ok(NumericComparison::GreaterThan),
            ">=" => Ok(NumericComparison::GreaterThanEqual),
            _ => Err(format!("Unknown comparison: {}", value)),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
#[serde(rename = "camelCase")]
pub enum Condition {
    Any {
        any: Vec<Condition>,
        #[serde(default)]
        invert: bool,
    },
    All {
        all: Vec<Condition>,
        #[serde(default)]
        invert: bool,
    },
    NumericComparison {
        parameter: FloatParam,
        #[serde(default)]
        comparison: NumericComparison,
        value: Option<f64>,
    },
    BoolComparison {
        parameter: BoolParam,
        #[serde(default)]
        comparison: GeneralComparison,
        value: Option<bool>,
    },
    StringComparison {
        #[serde(deserialize_with = "deserialize_string_param")]
        parameter: StringParam,
        #[serde(default)]
        comparison: GeneralComparison,
        value: Option<String>,
    },
}

impl Condition {
    pub fn matches(&self, state: &State) -> bool {
        match self {
            Condition::Any { any, invert } => {
                for condition in any {
                    if condition.matches(state) {
                        return !invert;
                    }
                }

                *invert
            }
            Condition::All { all, invert } => {
                for condition in all {
                    if !condition.matches(state) {
                        return *invert;
                    }
                }

                !invert
            }
            Condition::NumericComparison {
                parameter,
                comparison,
                value,
            } => {
                let state_value = match state.floats.get(parameter) {
                    Some(val) => *val,
                    None => {
                        if value.is_some() {
                            return comparison == &NumericComparison::NotEqual;
                        } else {
                            return comparison == &NumericComparison::Equal;
                        }
                    }
                };

                let value = match value {
                    Some(val) => *val,
                    None => return comparison == &NumericComparison::NotEqual,
                };

                match comparison {
                    NumericComparison::Equal => state_value == value,
                    NumericComparison::NotEqual => state_value != value,
                    NumericComparison::LessThan => state_value < value,
                    NumericComparison::LessThanEqual => state_value <= value,
                    NumericComparison::GreaterThan => state_value > value,
                    NumericComparison::GreaterThanEqual => state_value >= value,
                }
            }
            Condition::BoolComparison {
                parameter,
                comparison,
                value,
            } => {
                let state_value = state.bools.get(parameter).copied();

                match comparison {
                    GeneralComparison::Equal => &state_value == value,
                    GeneralComparison::NotEqual => &state_value != value,
                }
            }
            Condition::StringComparison {
                parameter,
                comparison,
                value,
            } => {
                let state_value = state.strings.get(parameter).cloned();

                match comparison {
                    GeneralComparison::Equal => &state_value == value,
                    GeneralComparison::NotEqual => &state_value != value,
                }
            }
        }
    }
}
