
// models/salon.rs

use serde::{Deserialize, Serialize};
use crate::models::table::Table;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Salon {
    pub salon_id: i32,
    pub name: String,        // Örnek: "Bronz", "Altın"
    pub entry_fee: i32,      // Örnek: 1000 TL
    pub tables: Vec<Table>,  // Salon içindeki masalar
    pub created_at: i64,     // Salona giriş zamanı
}
