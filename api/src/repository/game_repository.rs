use mongodb::{Client, Collection};
use mongodb::bson::{doc, Binary};
use mongodb::error::Result;
use futures::stream::TryStreamExt;
use crate::models::game::GameResult;

pub struct GameRepository {
    collection: Collection<GameResult>,
}



impl GameRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("games"); // Veritabanı adı
        let collection = db.collection::<GameResult>("game_results"); // Koleksiyon adı
        GameRepository { collection }
    }


    pub async fn get_all_games(&self) -> Result<Vec<GameResult>> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut users = Vec::new();
        while let Some(user) = cursor.try_next().await? {
            users.push(user);
        }
        Ok(users)
    }

}
