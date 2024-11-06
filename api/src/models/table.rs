// models/table.rs

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Table {
    pub table_id: i32,
    pub players: Vec<Player>, // Masadaki oyuncular
    pub bet_amount: i32,      // Bahis miktarı
    pub game_state: GameState, // Oyunun durumu
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub player_id: i64,        // Oyuncunun ID'si (telegram_id)
    pub has_paid: bool,        // Ödeme yaptı mı?
    pub dice_rolls: Vec<i32>,  // Oyuncunun attığı zarlar
    pub is_active: bool,       // Oyuncu aktif mi?
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameState {
    Waiting,
    Ready,    // Tüm oyuncular oturdu ve ödeme yaptı
    Started,  // Oyun başladı
    Completed // Oyun tamamlandı
}
