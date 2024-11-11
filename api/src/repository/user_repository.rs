use std::default;

use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
// src/repository/user_repository.rs
use std::time::{SystemTime, UNIX_EPOCH};
use mongodb::{Client, Collection};
use mongodb::error::Result;
use mongodb::bson::{doc, Binary, Bson}; // BSON içe aktarımı
use futures::stream::TryStreamExt;
use mongodb::bson::spec::BinarySubtype;
use uuid::Uuid;
use mongodb::bson;
use crate::models::user::{Boost, User}; // User yapısını içe aktar
use crate::models::user::Item;



pub struct UserRepository {
    collection: Collection<User>,
}

impl UserRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("users"); // Veritabanı adını değiştir
        let collection = db.collection::<User>("users"); // Koleksiyon adı
        UserRepository { collection }
    }
    pub async fn create_user(&self, mut user: User) -> Result<Option<User>> {
        // Eğer kullanıcı zaten mevcutsa, onu döndür
        if let Some(existing_user) = self.find_user_by_telegram_id(user.telegram_id).await? {
            return Ok(Some(existing_user));
        }
    
        // Varsayılan değerler atama
        user.click_score = Some(user.click_score.unwrap_or(0)); // Varsayılan olarak 0 click_score
        user.click_power = Some(user.click_power.unwrap_or(1)); // Varsayılan olarak 1 click_power
        user.game_pass = Some(user.game_pass.unwrap_or(0)); // Varsayılan olarak 1 click_power
        user.ton_amount = Some(user.ton_amount.unwrap_or(0.0));
        user.reputation_points = Some(user.reputation_points.unwrap_or(0));
        // Kullanıcıyı veritabanına ekle
        self.collection.insert_one(&user, None).await?;
        
        // Kullanıcı nesnesini döndür
        Ok(Some(user))
    }
    
    pub async fn find_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
        let user = self.collection.find_one(filter, None).await?;
        Ok(user)
    }

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let mut cursor = self.collection.find(None, None).await?;
        let mut users = Vec::new();
        while let Some(user) = cursor.try_next().await? {
            users.push(user);
        }
        Ok(users)
    }

     // update_user metodu
     pub async fn update_user_hp(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "hp": user.hp,
                "click_score": user.click_score
            }
        };

        self.collection.update_one(filter, update, None).await.map(|_| ())
    }

     // update_user metodu
     pub async fn update_user_game_pass(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "game_pass": user.game_pass,
              
            }
        };

        self.collection.update_one(filter, update, None).await.map(|_| ())
    }
    pub async fn update_user_hp_and_gamepass(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "hp": user.hp,
                "game_pass": user.game_pass,
            }
        };

        self.collection.update_one(filter, update, None).await.map(|_| ())
    }
    pub async fn update_user_hp_and_gamepasston(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "ton_amount": user.ton_amount,
                "game_pass": user.game_pass,
            }
        };

        self.collection.update_one(filter, update, None).await.map(|_| ())
    }
    pub async fn add_item_to_user(
        &self,
        telegram_id: i64,
        new_item: Item,
        hp_cost: i32,
    ) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
    
        // Kullanıcıyı bul
        let user = self.collection.find_one(filter.clone(), None).await?;
    
        // Eğer kullanıcı yoksa, hatayı döndür
        if user.is_none() {
            return Ok(None);
        }
    
        // Eğer `items` alanı null ise, onu boş bir dizi olarak başlatıyoruz.
        if user.as_ref().unwrap().items.is_none() {
            let initialize_items_if_null = doc! {
                "$set": { "items": Bson::Array(vec![]) }
            };
    
            // Kullanıcıyı güncelleyip, `items` alanını boş dizi olarak başlatıyoruz
            self.collection.update_one(filter.clone(), initialize_items_if_null, None).await?;
        }
    
        // `items` alanına yeni itemi ekle ve hp güncelle
        let update = doc! {
            "$push": { "items": {
                "id": new_item.id.clone(), // UUID olarak `Binary` kullanıldı
                "item_name": new_item.item_name,
                "item_slug": new_item.item_slug,
                "reputation_points": new_item.reputation_points,
            }},
            "$inc": { "hp": -hp_cost } // HP'yi azalt
        };
    
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
    
        match self.collection.find_one_and_update(filter, update, options).await {
            Ok(updated_user) => Ok(updated_user), // Güncellenmiş kullanıcıyı döndür
            Err(e) => {
                eprintln!("Failed to update user: {:?}", e);
                Err(e) // Hata varsa döndür
            }
        }
    }
    pub async fn remove_boost(&self, telegram_id: i64) -> Result<()> {
        let filter = doc! { "telegram_id": telegram_id };
        let update = doc! {
            "$set": {
                "boost": Bson::Null // Boost'u kaldırmak için null yapıyoruz
            }
        };
    
        self.collection.update_one(filter, update, None).await.map(|_| ())
    }
    pub async fn add_item_to_user_market(
        &self,
        telegram_id: i64,
        new_item: Item,
    ) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
    
        // Kullanıcıyı bul
        let user = self.collection.find_one(filter.clone(), None).await?;
    
        // Eğer kullanıcı yoksa, hatayı döndür
        if user.is_none() {
            return Ok(None);
        }
    
        // Eğer `items` alanı null ise, onu boş bir dizi olarak başlatıyoruz.
        if user.as_ref().unwrap().items.is_none() {
            let initialize_items_if_null = doc! {
                "$set": { "items": Bson::Array(vec![]) }
            };
    
            // Kullanıcıyı güncelleyip, `items` alanını boş dizi olarak başlatıyoruz
            self.collection.update_one(filter.clone(), initialize_items_if_null, None).await?;
        }
    
        // `items` alanına yeni itemi ekle
        let update = doc! {
            "$push": { "items": {
                "id": new_item.id.clone(), // UUID olarak `Binary` kullanıldı
                "item_name": new_item.item_name,
                "item_slug": new_item.item_slug,
                "reputation_points": new_item.reputation_points,
            }}
        };
    
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
    
        match self.collection.find_one_and_update(filter, update, options).await {
            Ok(updated_user) => Ok(updated_user), // Güncellenmiş kullanıcıyı döndür
            Err(e) => {
                eprintln!("Failed to update user: {:?}", e);
                Err(e) // Hata varsa döndür
            }
        }
    }
    
    pub async fn add_item_to_user_ton(
        &self,
        telegram_id: i64,
        new_item: Item,
        ton_cost: f64, // Ton miktarını temsil eden parametre
    ) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
    
        // Kullanıcıyı bul
        let user = self.collection.find_one(filter.clone(), None).await?;
    
        // Eğer kullanıcı yoksa, hatayı döndür
        if user.is_none() {
            return Ok(None);
        }
    
        // Eğer `items` alanı null ise, onu boş bir dizi olarak başlatıyoruz.
        if user.as_ref().unwrap().items.is_none() {
            let initialize_items_if_null = doc! {
                "$set": { "items": Bson::Array(vec![]) }
            };
    
            // Kullanıcıyı güncelleyip, `items` alanını boş dizi olarak başlatıyoruz
            self.collection.update_one(filter.clone(), initialize_items_if_null, None).await?;
        }
    
        // `items` alanına yeni itemi ekle ve ton_amount'u güncelle
        let update = doc! {
            "$push": { "items": {
                "id": new_item.id.clone(), // UUID olarak `Binary` kullanıldı
                "item_name": new_item.item_name,
                "item_slug": new_item.item_slug,
                "reputation_points": new_item.reputation_points,
            }},
            "$inc": { "ton_amount": -ton_cost } // Kullanıcının ton miktarını güncelle
        };
    
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
    
        match self.collection.find_one_and_update(filter, update, options).await {
            Ok(updated_user) => Ok(updated_user), // Güncellenmiş kullanıcıyı döndür
            Err(e) => {
                eprintln!("Failed to update user: {:?}", e);
                Err(e) // Hata varsa döndür
            }
        }
    }
    
    

    pub async fn remove_item_from_user(
        &self,
        telegram_id: i64,
        item_id: Binary, // UUID yerine Binary kullan
    ) -> Result<()> {
        let filter = doc! { "telegram_id": telegram_id };
        let update = doc! {
            "$pull": { "items": { "id": item_id } } // Binary formatında `id`
        };
    
        self.collection.update_one(filter, update, None).await.map(|_| ())
    }
    
    pub async fn update_user_references_and_friends(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "references": bson::to_bson(&user.references).unwrap_or(Bson::Null),
                "friends": bson::to_bson(&user.friends).unwrap_or(Bson::Null),
                "game_pass": user.game_pass, // game_pass güncellemesi
            }
        };
    
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();
    
        self.collection.find_one_and_update(filter, update, options).await.map(|_| ())
    }
    
    pub async fn update_user_ton_amount(&self, user: &User) -> Result<()> {
        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": { "ton_amount": user.ton_amount } // Artık `f64` değeri olarak ayarlanıyor
        };
    
        self.collection.update_one(filter, update, None).await.map(|_| ())
    }

    
    pub async fn apply_boost(
        &self,
        telegram_id: i64,
        requested_level: i32,
        currency_type: &str,
        amount: f64,
    ) -> Result<()> {
        let duration_days = match requested_level {
            1 => 1,
            2 => 3,
            3 => 10,
            _ => return Ok(()), // Geçersiz level girişi varsa işlem yapılmaz
        };

        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before Unix epoch")
            .as_secs() as i64;

        let new_boost = Boost {
            level: requested_level,
            start_time,
            duration_days,
        };

        if let Some(mut user) = self.find_user_by_telegram_id(telegram_id).await? {
            if let Some(existing_boost) = &user.boost {
                if (requested_level == 2 && existing_boost.level == 1)
                    || (requested_level == 3 && existing_boost.level <= 2)
                {
                    self.update_user_boost(&mut user, new_boost, currency_type, amount).await?;
                }
            } else {
                self.update_user_boost(&mut user, new_boost, currency_type, amount).await?;
            }
        }

        Ok(())
    }

    async fn update_user_boost(
        &self,
        user: &mut User,
        boost: Boost,
        currency_type: &str,
        amount: f64,
    ) -> Result<()> {
        user.boost = Some(boost);

        // `hp` veya `ton_amount` üzerinden güncelleme yap
        match currency_type {
            "ton" => {
                user.ton_amount = Some(user.ton_amount.unwrap_or(0.0) - amount);
            }
            "hp" => {
                user.hp = Some(user.hp.unwrap_or(0) - amount as i32);
            }
            _ => {}
        }

        let filter = doc! { "telegram_id": user.telegram_id };
        let update = doc! {
            "$set": {
                "boost": bson::to_bson(&user.boost).unwrap_or(Bson::Null),
                "ton_amount": user.ton_amount,
                "hp": user.hp,
            }
        };

        self.collection.update_one(filter, update, None).await.map(|_| ())
    }

    pub async fn update_user_profile(
        &self,
        telegram_id: i64,
        new_username: Option<String>,
        new_photo_url: Option<String>,
    ) -> Result<()> {
        let filter = doc! { "telegram_id": telegram_id };
        
        // Güncellenecek alanları oluşturuyoruz
        let mut update_fields = doc! {};
        if let Some(username) = new_username {
            update_fields.insert("username", username);
        }
        if let Some(photo_url) = new_photo_url {
            update_fields.insert("photo_url", photo_url);
        }
        
        // Eğer güncellenecek alan yoksa, hiçbir işlem yapılmaz
        if update_fields.is_empty() {
            return Ok(());
        }
    
        // Güncelleme komutunu oluşturuyoruz
        let update = doc! { "$set": update_fields };
    
        // Güncelleme işlemini yapıyoruz
        self.collection.update_one(filter, update, None).await.map(|_| ())
    }


}
