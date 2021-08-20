use specs::{Component, storage::BTreeStorage};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player;

impl Component for Player {
    type Storage = BTreeStorage<Self>;
}
