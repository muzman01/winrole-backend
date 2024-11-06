#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveGame {
    pub game_id: String,
    pub players: Vec<Player>, // Oyuncular
    pub rolls: Vec<Vec<i32>>, // Her oyuncunun zar atışları
    pub winner_id: Option<i64>, // Kazananın ID'si
    pub state: GameState, // Oyun durumu
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub player_id: i64,
    pub rolls: Vec<i32>, // Oyuncunun attığı zarlar
    pub is_active: bool, // Oyuncu bağlantısı aktif mi
}

// Oyun durumlarını temsil eden enum
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameState {
    Waiting,
    Ready,
    Started,
    Completed,
}
