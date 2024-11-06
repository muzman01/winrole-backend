use mongodb::{Client, Collection};
use mongodb::bson::doc;
use mongodb::bson::to_bson;
use mongodb::error::Result;
use futures_util::TryStreamExt; // try_collect için gerekli import
use crate::models::salon::Salon;

#[derive(Clone)] // Clone trait'ini ekliyoruz
pub struct SalonRepository {
    collection: Collection<Salon>,
}

impl SalonRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("salons");
        let collection = db.collection::<Salon>("salons");
        SalonRepository { collection }
    }

    pub async fn find_salon_by_id(&self, salon_id: i32) -> mongodb::error::Result<Option<Salon>> {
        let filter = doc! { "salon_id": salon_id };
        let salon = self.collection.find_one(filter, None).await?;
        Ok(salon)
    }

    pub async fn add_salon(&self, salon: Salon) -> mongodb::error::Result<()> {
        self.collection.insert_one(salon, None).await?;
        Ok(())
    }

    pub async fn update_salon(&self, salon: Salon) -> mongodb::error::Result<()> {
        let filter = doc! { "salon_id": salon.salon_id };
        let salon_bson = to_bson(&salon)?;  // `Salon` yapısını BSON'a dönüştürüyoruz
        let update = doc! { "$set": salon_bson };
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    // Tüm salonları getiren fonksiyon
    pub async fn get_all_salons(&self) -> mongodb::error::Result<Vec<Salon>> {
        let cursor = self.collection.find(None, None).await?; // Tüm kayıtları al
        let salons: Vec<Salon> = cursor.try_collect().await?; // Cursor'u Vec<Salon>'a dönüştür
        Ok(salons)
    }
    pub fn get_collection(&self) -> &Collection<Salon> {
        &self.collection
    }
    
}
