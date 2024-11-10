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

                Ok(Some(user))
            },
            Ok(None) => {
                // Eğer kullanıcı bulunamadıysa logla

                Ok(None)
            },
            Err(err) => {
                // Veritabanı hatası varsa logla

                Err(err)
            }
        }
    }
    
    
    pub async fn update_click_score(&self, telegram_id: i64, click_power: i32) -> Result<Option<User>> {
        let filter = doc! { "telegram_id": telegram_id };
    
        if let Some(mut user) = self.collection.find_one(filter.clone(), None).await? {
            // Eğer click_score Some ise değeri güncelle, değilse click_power ile başlat
            user.click_score = Some(user.click_score.unwrap_or(0) + click_power);
    
            // Veritabanında click_score'u güncelle
            self.collection.update_one(filter, doc! { "$set": { "click_score": user.click_score } }, None).await?;
            
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
    
}
