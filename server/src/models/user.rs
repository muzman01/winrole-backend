use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, spec::BinarySubtype, Binary, Bson, Document};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Boost {
    pub level: i32,
    pub start_time: i64,    // Unix timestamp, başlangıç zamanı
    pub duration_days: i32, // Boost kaç gün sürecek
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReferenceLevel {
    pub total_reference_required: i32, // Level tamamlamak için gereken toplam referans
    pub is_started: bool, // Level başladı mı?
    pub is_finished: bool, // Level bitti mi?
    pub current_reference: i32, // Mevcut referans sayısı
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct References {
    pub level1: ReferenceLevel,
    pub level2: ReferenceLevel,
    pub level3: ReferenceLevel,
    pub level4: ReferenceLevel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item {
    pub id: Binary, // Binary türünde UUID
    pub item_name: String,
    pub item_slug: String,
    pub reputation_points: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub telegram_id: i64,      // Zorunlu alan
    pub first_name: Option<String>, // İsteğe bağlı
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub photo_url: Option<String>,
    pub language_code: Option<String>,
    pub hp: Option<i32>, // Tab Tab puanlarından kazanılan HP
    pub ton_amount: Option<f64>, // Kullanıcının TON miktarı
    pub wallet_address: Option<String>, // Kullanıcının Ton Wallet adresi

    pub click_score: Option<i32>, // Toplam tıklama puanı
    pub click_power: Option<i32>, // Her tıklamada kazanacağı puan

    pub boost: Option<Boost>, // Kullanıcının Boost bilgisi
    pub references: Option<References>, // Referans sistemi

    pub game_pass: Option<i32>, // Varsayılan olarak sıfır
    pub reputation_points: Option<i32>, // Varsayılan olarak sıfır
    pub items: Option<Vec<Item>>, // İtemler listesi
    pub friends: Option<Vec<i64>>, // Kullanıcının referans olduğu Telegram ID'leri
}

impl User {
    pub fn update_click_power(&mut self) {
        if let Some(boost) = &self.boost {
            // Boost süresi içinde mi kontrol edelim
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64;

            let boost_end_time = boost.start_time + (boost.duration_days as i64 * 86400);
            if current_time <= boost_end_time {
                // Boost seviyesiyle click_power güncelleme
                let boost_increment = match boost.level {
                    1 => 2,
                    2 => 10,
                    3 => 20,
                    _ => 0,
                };

                // click_power değerini güncelleme
                if let Some(ref mut power) = self.click_power {
                    *power += boost_increment;
                } else {
                    // Eğer click_power None ise, boost_increment ile başlat
                    self.click_power = Some(boost_increment);
                }
            } else {
                // Boost süresi dolmuşsa boost'u sıfırla
                self.boost = None;
            }
        }
    }
}
