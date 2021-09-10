use serde::Deserialize;

use crate::state::{
    params::{BoolParam, FloatParam},
    Condition, State,
};

use super::Action;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ContinuousAction {
    Parameter(FloatParam),
}

impl ContinuousAction {
    pub fn actions(&self, _state: &State, value: f64) -> Option<Vec<Action>> {
        match self {
            ContinuousAction::Parameter(parameter) => Some(vec![Action::SetFloatParameter {
                parameter: parameter.clone(),
                value,
            }]),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeyAction {
    Parameter(BoolParam),
    Toggle { toggle: BoolParam },
    Action(Action),
    Sequence { sequence: Vec<KeyAction> },
}

impl KeyAction {
    pub fn action(&self, state: &State) -> Vec<Action> {
        let mut actions = Vec::new();

        match self {
            KeyAction::Parameter(parameter) => actions.push(Action::SetBoolParameter {
                parameter: parameter.clone(),
                value: true,
            }),
            KeyAction::Toggle { toggle: parameter } => {
                if let Some(val) = state.bools.get(parameter) {
                    actions.push(Action::SetBoolParameter {
                        parameter: parameter.clone(),
                        value: !val,
                    });
                }
            }
            KeyAction::Action(action) => actions.push(action.clone()),
            KeyAction::Sequence { sequence } => {
                for action in sequence {
                    actions.append(&mut action.action(state));
                }
            }
        }

        actions
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum KeySource {
    Parameter(BoolParam),
    InvertedParameter {
        parameter: BoolParam,
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
    Parameter(FloatParam),
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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Choices<T>
where
    T: Clone,
{
    Single(Choice<T>),
    Many(Vec<Choice<T>>),
}

impl<T> Choices<T>
where
    T: Clone,
{
    pub fn resolve(&self, state: &State) -> Option<T> {
        match self {
            Choices::Single(choice) => choice.resolve(state),
            Choices::Many(choices) => {
                for choice in choices {
                    if let Some(result) = choice.resolve(state) {
                        return Some(result);
                    }
                }

                None
            }
        }
    }
}

#[derive(Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct ControlLayerInfo {
    #[serde(rename = "device")]
    pub device_id: String,
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
    pub action: Choices<KeyAction>,
    #[serde(default)]
    pub source: Option<Choices<KeySource>>,
}

impl KeyProfile {
    pub fn actions(&self, state: &State) -> Option<Vec<Action>> {
        self.action
            .resolve(state)
            .map(|action| action.action(state))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub enum ControlProfile {
    Continuous(ContinuousProfile),
    Key(KeyProfile),
}
