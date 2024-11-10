use core::fmt;

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
    pub player_id: i64,
    pub is_active: bool,
    pub has_paid: bool,        // Ödeme yaptı mı?
    pub dice_rolls: Vec<i32>,  // Oyuncunun attığı zarlar
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameState {
    Waiting,
    Ready,    // Tüm oyuncular oturdu ve ödeme yaptı
    Started,  // Oyun başladı
    Completed // Oyun tamamlandı
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = match *self {
            GameState::Waiting => "Waiting",
            GameState::Ready => "Ready",
            GameState::Started => "Started",
            GameState::Completed => "Completed",
        };
        write!(f, "{}", state)
    }
}