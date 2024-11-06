// src/jwt/jwt_helper.rs
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey, errors::Result as JwtResult};
use crate::jwt::claims::Claims;
use std::time::{SystemTime, UNIX_EPOCH};

const SECRET: &[u8] = b"your_secret_key"; // Gizli anahtarınızı buraya koyun

pub fn create_token(telegram_id: i64) -> JwtResult<String> {
    let my_claims = Claims {
        sub: telegram_id.to_string(),
        exp: (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 60 * 60) as usize, // 1 saat geçerlilik süresi
    };

    encode(&Header::default(), &my_claims, &EncodingKey::from_secret(SECRET))
}

pub fn verify_token(token: &str) -> Result<Claims, String> {
    decode::<Claims>(token, &DecodingKey::from_secret(SECRET), &Validation::default())
        .map(|data| data.claims)
        .map_err(|_| "Invalid token".to_string())
}