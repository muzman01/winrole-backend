use mongodb::{Client, Collection};
use mongodb::bson::doc;

pub struct LiveGameRepository {
    collection: Collection<LiveGame>,
}

impl LiveGameRepository {
    pub fn new(client: &Client) -> Self {
        let collection = client.database("live_games").collection("live_games");
        Self { collection }
    }

    pub async fn create_game(&self, game: LiveGame) -> Result<(), Box<dyn std::error::Error>> {
        self.collection.insert_one(game, None).await?;
        Ok(())
    }

    pub async fn update_game(&self, game_id: &str, rolls: Vec<Vec<i32>>, winner_id: Option<i64>) -> Result<(), Box<dyn std::error::Error>> {
        self.collection.update_one(
            doc! { "game_id": game_id },
            doc! { "$set": { "rolls": rolls, "winner_id": winner_id } },
            None
        ).await?;
        Ok(())
    }
}
