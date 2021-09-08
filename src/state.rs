use std::{collections::HashMap, convert::TryFrom, fmt::Display};

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
pub enum Module {
    Internal,
    Lightroom,
}

pub type State = HashMap<String, (Module, Option<Value>)>;

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub enum Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
}

impl Default for Comparison {
    fn default() -> Self {
        Comparison::Equal
    }
}

impl TryFrom<String> for Comparison {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "==" => Ok(Comparison::Equal),
            "!=" => Ok(Comparison::NotEqual),
            "<" => Ok(Comparison::LessThan),
            "<=" => Ok(Comparison::LessThanEqual),
            ">" => Ok(Comparison::GreaterThan),
            ">=" => Ok(Comparison::GreaterThanEqual),
            _ => Err(format!("Unknown comparison: {}", value)),
        }
    }
}

impl Into<String> for Comparison {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Comparison::Equal => "==",
            Comparison::NotEqual => "!=",
            Comparison::LessThan => "<",
            Comparison::LessThanEqual => "<=",
            Comparison::GreaterThan => ">",
            Comparison::GreaterThanEqual => ">=",
        };

        write!(f, "{}", str)
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
    Comparison {
        parameter: String,
        #[serde(default)]
        comparison: Comparison,
        value: Option<Value>,
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
            Condition::Comparison {
                parameter,
                comparison,
                value,
            } => {
                if let Some((_, state_value)) = state.get(parameter) {
                    match comparison {
                        Comparison::Equal => state_value == value,
                        Comparison::NotEqual => state_value != value,
                        float_comparison => match (state_value, value) {
                            (Some(Value::Float(state_value)), Some(Value::Float(value))) => {
                                match float_comparison {
                                    Comparison::Equal => state_value == value,
                                    Comparison::NotEqual => state_value != value,
                                    Comparison::LessThan => state_value < value,
                                    Comparison::LessThanEqual => state_value <= value,
                                    Comparison::GreaterThan => state_value > value,
                                    Comparison::GreaterThanEqual => state_value >= value,
                                }
                            }
                            _ => false,
                        },
                    }
                } else {
                    false
                }
            }
        }
    }
}
