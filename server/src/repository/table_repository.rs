use mongodb::{Client, Collection};
use mongodb::bson::doc;
use futures::stream::StreamExt; // futures kütüphanesinden stream desteği için
use crate::models::table::Table;

#[derive(Clone)]
pub struct TableRepository {
    collection: mongodb::Collection<Table>,
}

impl TableRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("tables_db");
        let collection = db.collection::<Table>("tables");
        TableRepository { collection }
    }

    pub async fn get_table_by_id(&self, salon_id: i32, table_id: i32) -> mongodb::error::Result<Option<Table>> {
        let filter = doc! { "salon_id": salon_id, "table_id": table_id };
        let table = self.collection.find_one(filter, None).await?;
        Ok(table)
    }

    pub async fn update_table(&self, salon_id: i32, table: Table) -> mongodb::error::Result<()> {
        let filter = doc! { "salon_id": salon_id, "table_id": table.table_id };
        let players_bson = mongodb::bson::to_bson(&table.players).unwrap();
        let update = doc! { "$set": { "players": players_bson } };
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }

    // Bütün tabloları getiren fonksiyon
    pub async fn get_all_tables(&self) -> mongodb::error::Result<Vec<Table>> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut tables: Vec<Table> = Vec::new();

        // Cursor'daki dökümanları toplayalım
        while let Some(result) = cursor.next().await {
            match result {
                Ok(table) => tables.push(table),
                Err(e) => return Err(e.into()), // Hata varsa geri döndür
            }
        }

        Ok(tables)
    }
}
