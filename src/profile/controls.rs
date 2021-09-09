use serde::{de::DeserializeOwned, Deserialize};
use serde_with::{serde_as, OneOrMany};

use crate::state::{Condition, State, Value};

use super::Action;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ContinuousAction {
    Parameter(String),
}

impl ContinuousAction {
    pub fn actions(&self, _state: &State, value: f64) -> Option<Vec<Action>> {
        match self {
            ContinuousAction::Parameter(parameter) => Some(vec![Action::SetParameter {
                parameter: parameter.clone(),
                value: Value::Float(value),
            }]),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeyAction {
    Parameter(String),
    Toggle { toggle: String },
    Action(Action),
}

impl KeyAction {
    pub fn action(&self, state: &State) -> Option<Action> {
        match self {
            KeyAction::Parameter(parameter) => Some(Action::SetParameter {
                parameter: parameter.clone(),
                value: Value::Boolean(true),
            }),
            KeyAction::Toggle { toggle: parameter } => {
                if let Some((_, Some(Value::Boolean(val)))) = state.get(parameter) {
                    Some(Action::SetParameter {
                        parameter: parameter.clone(),
                        value: Value::Boolean(!val),
                    })
                } else {
                    None
                }
            }
            KeyAction::Action(action) => Some(action.clone()),
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

    pub fn actions(&self, state: &State) -> Option<Vec<Action>> {
        match self.actions.len() {
            0 => None,
            _ => Some(
                self.actions
                    .iter()
                    .filter_map(|action| action.action(state))
                    .collect(),
            ),
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
    Conditional {
        #[serde(rename = "if")]
        when: Condition,
        then: T,
    },
    Simple(T),
}

impl<T> Choice<T>
where
    T: Clone,
{
    pub fn resolve(&self, state: &State) -> Option<T> {
        match self {
            Choice::Conditional { when, then } => {
                if when.matches(state) {
                    Some(then.clone())
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
    pub fn actions(&self, state: &State, value: f64) -> Option<Vec<Action>> {
        self.action
            .resolve(state)
            .and_then(|action| action.actions(state, value))
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
    pub fn actions(&self, state: &State) -> Option<Vec<Action>> {
        self.action
            .resolve(state)
            .and_then(|action| action.actions(state))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum ControlProfile {
    Continuous(ContinuousProfile),
    Key(KeyProfile),
}
