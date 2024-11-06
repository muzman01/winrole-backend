use mongodb::{Client, bson::doc};
use mongodb::error::Result;
use crate::models::user::User;

#[derive(Clone)]  // Clone trait'ini ekledik
pub struct UserRepository {
    pub collection: mongodb::Collection<User>,
}

impl UserRepository {
    pub fn new(client: &Client) -> Self {
        let db = client.database("users");
        let collection = db.collection::<User>("users");
        UserRepository { collection }
    }

    pub async fn find_user_by_telegram_id(&self, telegram_id: i64) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
        
        match self.collection.find_one(filter, None).await {
            Ok(Some(user)) => {
                // Eğer kullanıcı bulunduysa logla
                eprintln!("Kullanıcı bulundu: {:?}", user);
                Ok(Some(user))
            },
            Ok(None) => {
                // Eğer kullanıcı bulunamadıysa logla
                eprintln!("Kullanıcı bulunamadı: Telegram ID = {}", telegram_id);
                Ok(None)
            },
            Err(err) => {
                // Veritabanı hatası varsa logla
                eprintln!("Kullanıcı bulunurken hata oluştu: {:?}", err);
                Err(err)
            }
        }
    }
    
    
    pub async fn update_click_score(&self, telegram_id: i64, click_power: i32) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };

        if let Some(mut user) = self.collection.find_one(filter.clone(), None).await? {
            user.click_score += click_power; // Tıklama puanını ekle
            self.collection.update_one(filter, doc! { "$set": { "click_score": user.click_score } }, None).await?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
}
