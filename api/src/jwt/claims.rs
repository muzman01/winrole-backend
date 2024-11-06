// src/jwt/claims.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,            // Kullanıcı ID'si
    pub exp: usize,            // Son kullanma zamanı
}
