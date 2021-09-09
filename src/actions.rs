use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "action")]
pub enum InternalAction {
    RefreshController,
}
