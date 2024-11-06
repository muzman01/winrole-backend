use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Boost {
    pub level: i32,
    pub start_time: i64,    // Unix timestamp, başlangıç zamanı
    pub duration_days: i32, // Boost kaç gün sürecek
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub telegram_id: i64,
    pub click_score: i32,       // Toplam tıklama puanı
    pub click_power: i32,       // Her tıklamada kazanacağı puan
    pub boost: Option<Boost>,   // Kullanıcının Boost bilgisi
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
                self.click_power += match boost.level {
                    1 => 2,
                    2 => 10,
                    3 => 20,
                    _ => 0,
                };
            } else {
                // Boost süresi dolmuşsa boost'u sıfırla
                self.boost = None;
            }
        }
    }
}
