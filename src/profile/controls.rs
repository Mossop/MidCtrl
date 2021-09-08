use serde::{de::DeserializeOwned, Deserialize};
use serde_with::{serde_as, OneOrMany};
use std::convert::TryFrom;

use crate::state::{State, Value};

use super::Action;

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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
#[serde(rename = "camelCase")]
pub enum Condition {
    Any {
        when_any: Vec<Condition>,
    },
    All {
        when_all: Vec<Condition>,
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
            Condition::Any { when_any } => {
                for condition in when_any {
                    if condition.matches(state) {
                        return true;
                    }
                }

                false
            }
            Condition::All { when_all } => {
                for condition in when_all {
                    if !condition.matches(state) {
                        return false;
                    }
                }

                true
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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ContinuousAction {
    Parameter(String),
}

impl ContinuousAction {
    pub fn action(&self, _state: &State, value: f64) -> Option<Action> {
        match self {
            ContinuousAction::Parameter(parameter) => Some(Action::SetParameter {
                name: parameter.clone(),
                value: Value::Float(value),
            }),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeyAction {
    Parameter(String),
    Toggle { toggle: String },
    SetParameter { parameter: String, value: Value },
}

impl KeyAction {
    pub fn action(&self, state: &State) -> Option<Action> {
        match self {
            KeyAction::Parameter(parameter) => Some(Action::SetParameter {
                name: parameter.clone(),
                value: Value::Boolean(true),
            }),
            KeyAction::Toggle { toggle: parameter } => {
                if let Some((_, Some(Value::Boolean(val)))) = state.get(parameter) {
                    Some(Action::SetParameter {
                        name: parameter.clone(),
                        value: Value::Boolean(!val),
                    })
                } else {
                    None
                }
            }
            KeyAction::SetParameter { parameter, value } => Some(Action::SetParameter {
                name: parameter.clone(),
                value: value.clone(),
            }),
        }
    }
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct KeyActions {
    #[serde_as(deserialize_as = "OneOrMany<_>")]
    actions: Vec<KeyAction>,
}

impl KeyActions {
    pub fn single_action(&self) -> Option<KeyAction> {
        if self.actions.len() == 1 {
            self.actions.get(0).cloned()
        } else {
            None
        }
    }

    pub fn action(&self, state: &State) -> Option<Action> {
        match self.actions.len() {
            0 => None,
            1 => self.actions.get(0).and_then(|action| action.action(state)),
            _ => Some(Action::Sequence(
                self.actions
                    .iter()
                    .filter_map(|action| action.action(state))
                    .collect(),
            )),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeySource {
    Parameter(String),
    InvertedParameter {
        parameter: String,
        #[serde(default)]
        invert: bool,
    },
    Constant(bool),
    Condition {
        condition: Condition,
        #[serde(default)]
        invert: bool,
    },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ContinuousSource {
    Parameter(String),
    Constant(f64),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Choice<T>
where
    T: Clone,
{
    Conditional { when: Condition, result: T },
    Simple(T),
}

impl<T> Choice<T>
where
    T: Clone,
{
    pub fn resolve(&self, state: &State) -> Option<T> {
        match self {
            Choice::Conditional { when, result } => {
                if when.matches(state) {
                    Some(result.clone())
                } else {
                    None
                }
            }
            Choice::Simple(result) => Some(result.clone()),
        }
    }
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Choices<T>
where
    T: Clone + DeserializeOwned,
{
    #[serde_as(deserialize_as = "OneOrMany<_>")]
    choices: Vec<Choice<T>>,
}

impl<T> Choices<T>
where
    T: Clone + DeserializeOwned,
{
    pub fn resolve(&self, state: &State) -> Option<T> {
        for choice in &self.choices {
            if let Some(result) = choice.resolve(state) {
                return Some(result);
            }
        }

        None
    }
}

#[derive(Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ControlLayerInfo {
    pub device: String,
    pub control: String,
    pub layer: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ContinuousProfile {
    #[serde(flatten)]
    pub info: ControlLayerInfo,
    pub action: Choices<ContinuousAction>,
    #[serde(default)]
    pub source: Option<Choices<ContinuousSource>>,
}

impl ContinuousProfile {
    pub fn action(&self, state: &State, value: f64) -> Option<Action> {
        self.action
            .resolve(state)
            .and_then(|action| action.action(state, value))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyProfile {
    #[serde(flatten)]
    pub info: ControlLayerInfo,
    pub action: Choices<KeyActions>,
    #[serde(default)]
    pub source: Option<Choices<KeySource>>,
}

impl KeyProfile {
    pub fn action(&self, state: &State) -> Option<Action> {
        self.action
            .resolve(state)
            .and_then(|action| action.action(state))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum ControlProfile {
    Continuous(ContinuousProfile),
    Key(KeyProfile),
}
