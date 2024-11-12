use std::io::Cursor;
use serde_json::json;
use rocket::{fairing::{Fairing, Info, Kind}, Request, Data, Response};
use rocket::http::Status;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};

type HmacSha256 = Hmac<Sha256>;

pub struct TelegramAuthFairing {
    pub bot_token: String,
}

impl TelegramAuthFairing {
    pub fn new(bot_token: &str) -> Self {
        TelegramAuthFairing {
            bot_token: bot_token.to_string(),
        }
    }

    fn verify_telegram_signature(&self, init_data: &str, hash: &str) -> bool {
        let secret_key = Sha256::digest(self.bot_token.as_bytes());
        let mut hmac = HmacSha256::new_from_slice(&secret_key).expect("HMAC can take key of any size");
        hmac.update(init_data.as_bytes());

        hmac.verify_slice(hash.as_bytes()).is_ok()
    }
}

#[rocket::async_trait]
impl Fairing for TelegramAuthFairing {
    fn info(&self) -> Info {
        Info {
            name: "Telegram Authentication Fairing",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let init_data = request.headers().get_one("X-Init-Data").unwrap_or_default();
        let hash = request.headers().get_one("X-Hash").unwrap_or_default();

        if !self.verify_telegram_signature(init_data, hash) {
            // Unauthorized durumunu `Option<Status>` içinde cache'e ekliyoruz
            request.local_cache::<Option<Status>, _>(|| Some(Status::Unauthorized));
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        // Saklanan değerin `Some(Status::Unauthorized)` olup olmadığını kontrol ediyoruz
        if let Some(status) = request.local_cache::<Option<Status>, _>(|| None) {
            if *status == Status::Unauthorized {
                response.set_status(Status::Unauthorized);

                let error_message = json!({
                    "error": "Unauthorized - Invalid Telegram signature"
                }).to_string();

                response.set_sized_body(None, Cursor::new(error_message));
                response.set_header(rocket::http::ContentType::JSON);
            }
        }
    }
}
