use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum LightroomAction {
    NextPhoto,
    PreviousPhoto,
}
