use tokio::net::TcpListener;
use tokio::time::{self, Duration};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::repository::user_repository::UserRepository;

#[derive(Serialize, Deserialize)]
struct IncomingMessage {
    telegram_id: i64,
    click_power: Option<i32>,
}

pub async fn run_websocket_server(mongo_client: Client) {
    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    let user_repo = Arc::new(UserRepository::new(&mongo_client));

    println!("WebSocket sunucusu {} adresinde çalışmaya başladı", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let user_repo = Arc::clone(&user_repo);

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => return,
            };

            let (write, mut read) = ws_stream.split();
            let write = Arc::new(Mutex::new(write));

            // İlk mesajı alarak telegram_id'yi belirle
            let telegram_id = match read.next().await {
                Some(Ok(msg)) => {
                    if let Ok(text) = msg.into_text() {
                        if let Ok(parsed_msg) = serde_json::from_str::<IncomingMessage>(&text) {
                            parsed_msg.telegram_id
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            };

            // Her saniye belirli `telegram_id` kullanıcısının verisini çekip gönderme
            let user_repo_clone = Arc::clone(&user_repo);
            let write_clone = Arc::clone(&write);
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;

                    if let Ok(Some(user)) = user_repo_clone.find_user_by_telegram_id(telegram_id).await {
                        let response = json!({
                            "telegram_id": user.telegram_id,
                            "first_name": user.first_name,
                            "last_name": user.last_name,
                            "username": user.username,
                            "photo_url": user.photo_url,
                            "language_code": user.language_code,
                            "hp": user.hp,
                            "ton_amount": user.ton_amount,
                            "wallet_address": user.wallet_address,
                            "boost": user.boost,
                            "references": user.references,
                            "game_pass": user.game_pass,
                            "reputation_points": user.reputation_points,
                            "items": user.items,
                            "friends": user.friends
                        });
                        if write_clone.lock().await.send(Message::text(response.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
            });

            // Diğer gelen mesajları işleme
            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        if let Ok(text) = msg.into_text() {
                            let incoming: Result<IncomingMessage, _> = serde_json::from_str(&text);
                            
                            match incoming {
                                Ok(msg) => {
                                    let click_power = msg.click_power.unwrap_or(0);

                                    match user_repo.find_user_by_telegram_id(telegram_id).await {
                                        Ok(Some(user)) => {
                                            let response = json!({
                                                "click_score": user.click_score,
                                                "click_power": user.click_power,
                                            });
                                            if write.lock().await.send(response.to_string().into()).await.is_err() {
                                                break;
                                            }
                                        }
                                        _ => {
                                            let error_response = json!({
                                                "error": "Kullanıcı bulunamadı veya eksik veri"
                                            });
                                            if write.lock().await.send(error_response.to_string().into()).await.is_err() {
                                                break;
                                            }
                                        }
                                    }

                                    if let Ok(Some(updated_user)) = user_repo.update_click_score(telegram_id, click_power).await {
                                        let updated_response = json!({
                                            "click_score": updated_user.click_score,
                                            "click_power": updated_user.click_power,
                                        });
                                        if write.lock().await.send(updated_response.to_string().into()).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                Err(_) => {
                                    let error_message = json!({
                                        "error": "Geçersiz mesaj formatı"
                                    });
                                    if write.lock().await.send(error_message.to_string().into()).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
}
