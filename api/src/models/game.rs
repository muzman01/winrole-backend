use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameResult {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub game_id: String,
    pub winner_id: String,
    pub players: Vec<Player>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    pub player_id: String,
    pub rolls: Vec<i32>,
}
