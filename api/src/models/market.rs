use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, Binary, Bson};
use mongodb::bson::oid::ObjectId;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    pub id: Binary, // Accept BSON binary type directly
    pub item_name: String,
    pub item_slug: String,
    pub reputation_points: i32,
    pub price: i32,
    pub seller: i64, // telegram ID as seller identifier
}