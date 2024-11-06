use mongodb::{Client, Collection};
use mongodb::error::Result;
use mongodb::bson::{doc, to_bson};
use futures::stream::TryStreamExt;
use crate::models::salon::Salon;
use crate::models::table::{Player}; // Gerekli yapıları içe aktar
use crate::models::table::Table; // Table yapısını içe aktarın



pub struct SalonRepository {
    collection: Collection<Salon>,
}

impl SalonRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("salons");
        let collection = db.collection::<Salon>("salons");
        SalonRepository { collection }
    }

    // Yeni bir salon eklemek için fonksiyon
    pub async fn add_salon(&self, salon: Salon) -> Result<()> {
        self.collection.insert_one(&salon, None).await?;
        Ok(())
    }

    // Tüm salonları almak için fonksiyon
    pub async fn get_all_salons(&self) -> Result<Vec<Salon>> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut salons = Vec::new();
        while let Some(salon) = cursor.try_next().await? {
            salons.push(salon);
        }
        Ok(salons)
    }

    // Belirli bir salonu bulmak için fonksiyon
    pub async fn find_salon_by_id(&self, salon_id: i32) -> Result<Option<Salon>> {
        let filter = doc! { "salon_id": salon_id };
        let salon = self.collection.find_one(filter, None).await?;
        Ok(salon)
    }
 

    pub async fn add_player_to_table(
        &self,
        salon_id: i32,
        table_id: i32,
        telegram_id: i64,
    ) -> Result<bool> {
        // İlk olarak tüm salonları kontrol ederek oyuncunun zaten bir masada olup olmadığını kontrol ediyoruz
        let existing_player = self.is_player_in_any_table(telegram_id).await?;
        if existing_player {
            println!("Oyuncu zaten bir masada oturuyor, işlem yapılmadı.");
            return Ok(false);
        }

        // Oyuncu başka bir masada değilse, belirli salona ve masaya ekleme yapalım
        if let Some(mut salon) = self.find_salon_by_id(salon_id).await? {
            if let Some(table) = salon.tables.iter_mut().find(|t| t.table_id == table_id) {
                let new_player = Player {
                    player_id: telegram_id,
                    is_active: true,
                    has_paid: false,
                    dice_rolls: Vec::new(),
                };

                // Player yapısını BSON'a dönüştür
                let bson_player = to_bson(&new_player).unwrap();

                // MongoDB güncelleme işlemi
                self.collection.update_one(
                    doc! { "salon_id": salon_id, "tables.table_id": table_id },
                    doc! { "$push": { "tables.$.players": bson_player } }, // BSON formatında oyuncuyu ekle
                    None,
                ).await?;

                println!("Oyuncu {} masaya eklendi: salon_id: {}, table_id: {}", telegram_id, salon_id, table_id);
                return Ok(true);
            }
        }
        println!("Salon veya masa bulunamadı.");
        Ok(false)
    }

    // Oyuncunun herhangi bir salondaki herhangi bir masada olup olmadığını kontrol etmek için fonksiyon
    pub async fn is_player_in_any_table(&self, telegram_id: i64) -> Result<bool> {
        let filter = doc! { "tables.players.player_id": telegram_id };
        let count = self.collection.count_documents(filter, None).await?;
        Ok(count > 0)
    }

    pub async fn update_salon(&self, salon: Salon) -> Result<()> {
        let filter = doc! { "salon_id": salon.salon_id };

        // Burada "tables" alanını BSON'a çeviriyoruz
        let bson_tables = to_bson(&salon.tables).unwrap_or_else(|_| panic!("Table BSON dönüşümü başarısız oldu."));

        let update = doc! { "$set": { "tables": bson_tables }};
        self.collection.update_one(filter, update, None).await?;
        Ok(())
    }
    
}
