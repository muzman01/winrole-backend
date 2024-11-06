use serde::{Deserialize, Serialize};
use mongodb::bson::{doc, spec::BinarySubtype, Binary, Bson, Document};
use uuid::Uuid;
use std::convert::From;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Boost {
    pub level: i32,
    pub start_time: i64, // Unix timestamp, başlangıç zamanı
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

impl Item {
    pub fn new(item_name: String, item_slug: String, reputation_points: i32) -> Self {
        Item {
            id: Binary {
                subtype: BinarySubtype::Uuid,
                bytes: Uuid::new_v4().as_bytes().to_vec(),
            },
            item_name,
            item_slug,
            reputation_points,
        }
    }
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

// Document dönüşümü
impl From<User> for Document {
    fn from(user: User) -> Self {
        let mut doc = doc! {
            "telegram_id": user.telegram_id,
            "first_name": user.first_name,
            "last_name": user.last_name,
            "username": user.username,
            "photo_url": user.photo_url,
            "language_code": user.language_code,
            "hp": user.hp,
            "ton_amount": user.ton_amount,
            "wallet_address": user.wallet_address,
            "click_score": user.click_score.unwrap_or(0),
            "click_power": user.click_power.unwrap_or(1),
            "boost": user.boost.map(|b| {
                doc! {
                    "level": b.level,
                    "start_time": b.start_time,
                    "duration_days": b.duration_days,
                }
            }),
            "references": user.references.map(|r| {
                doc! {
                    "level1": {
                        "total_reference_required": r.level1.total_reference_required,
                        "is_started": r.level1.is_started,
                        "is_finished": r.level1.is_finished,
                        "current_reference": r.level1.current_reference,
                    },
                    "level2": {
                        "total_reference_required": r.level2.total_reference_required,
                        "is_started": r.level2.is_started,
                        "is_finished": r.level2.is_finished,
                        "current_reference": r.level2.current_reference,
                    },
                    "level3": {
                        "total_reference_required": r.level3.total_reference_required,
                        "is_started": r.level3.is_started,
                        "is_finished": r.level3.is_finished,
                        "current_reference": r.level3.current_reference,
                    },
                    "level4": {
                        "total_reference_required": r.level4.total_reference_required,
                        "is_started": r.level4.is_started,
                        "is_finished": r.level4.is_finished,
                        "current_reference": r.level4.current_reference,
                    },
                }
            }),
            "game_pass": user.game_pass,
            "reputation_points": user.reputation_points,
        };

        if let Some(items) = user.items {
            doc.insert("items", items.into_iter().map(|item| {
                doc! {
                    "id": item.id.clone(),
                    "item_name": item.item_name,
                    "item_slug": item.item_slug,
                    "reputation_points": item.reputation_points,
                }
            }).collect::<Vec<_>>());
        }

        if let Some(friends) = user.friends {
            doc.insert("friends", friends);
        }

        doc
    }
}
