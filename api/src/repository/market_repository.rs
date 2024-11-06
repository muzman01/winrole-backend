use mongodb::{Client, Collection};
use mongodb::bson::{doc, Binary};
use mongodb::error::Result;
use futures::stream::TryStreamExt;
use crate::models::market::Market;

pub struct MarketRepository {
    collection: Collection<Market>,
}

impl MarketRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("market"); // Veritabanı adı
        let collection = db.collection::<Market>("market"); // Koleksiyon adı
        MarketRepository { collection }
    }
    pub async fn find_item_by_id(&self, item_id: Binary) -> Result<Option<Market>> {
        let filter = doc! { "id": item_id };
        self.collection.find_one(filter, None).await
    }

    // Tüm market öğelerini getirme
    pub async fn get_all_market(&self) -> Result<Vec<Market>> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut markets = Vec::new();
        while let Some(market) = cursor.try_next().await? {
            markets.push(market);
        }
        Ok(markets)
    }

    // Yeni bir item ekleme
    pub async fn add_item(&self, item: Market) -> Result<()> {
        self.collection.insert_one(item, None).await.map(|_| ())
    }

    // Belirli bir item_id ile item silme
    pub async fn delete_item_from_market(&self, item_id: Binary) -> Result<()> {
        let filter = doc! { "id": item_id };
        self.collection.delete_one(filter, None).await.map(|_| ())
    }
}
