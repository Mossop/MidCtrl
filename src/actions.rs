use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub enum InternalAction {
    RefreshController,
}
