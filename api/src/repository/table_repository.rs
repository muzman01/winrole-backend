// src/repository/table_repository.rs
use mongodb::{Client, Collection};
use mongodb::error::Result;
use mongodb::bson::{doc, to_bson}; // to_bson'u içe aktardık
use crate::models::table::Table; // Table yapısını içe aktar
use crate::models::salon::Salon; // Salon ile ilişkilendirmek için

pub struct TableRepository {
    collection: Collection<Salon>,
}

impl TableRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("salons");
        let collection = db.collection::<Salon>("salons");
        TableRepository { collection }
    }

    // Belirli bir salona oda eklemek için fonksiyon
    pub async fn add_table_to_salon(&self, salon_id: i32, table: Table) -> Result<()> {
        let filter = doc! { "salon_id": salon_id };

        // Table'ı BSON'a dönüştürüyoruz
        let table_bson = to_bson(&table).unwrap(); // BSON dönüşümünü gerçekleştir

        let update = doc! { "$push": { "tables": table_bson }};
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }
}
