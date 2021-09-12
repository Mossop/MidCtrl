use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(tag = "action")]
pub enum LightroomAction {
    NextPhoto,
    PreviousPhoto,
    Undo,
    Redo,
}
