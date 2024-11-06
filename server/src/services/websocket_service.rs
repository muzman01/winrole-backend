use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};  // futures_util'i ekledik
use serde_json::json;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use crate::repository::user_repository::UserRepository;

#[derive(Serialize, Deserialize)]
struct IncomingMessage {
    telegram_id: i64,
    click_power: Option<i32>, // tıklama gücü isteğe bağlı olabilir
}

pub async fn run_websocket_server(mongo_client: Client) {
    let addr = "127.0.0.1:9001";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    let user_repo = UserRepository::new(&mongo_client);

    println!("WebSocket sunucusu {} adresinde çalışıyor", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let user_repo = user_repo.clone();  // Artık Clone trait'i olduğu için çalışacak

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket bağlantısı sırasında hata oluştu: {:?}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();  // split metodu burada artık çalışacak

            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) => {
                        if let Ok(text) = msg.into_text() {
                            let incoming: Result<IncomingMessage, _> = serde_json::from_str(&text);
                            
                            match incoming {
                                Ok(msg) => {
                                    let telegram_id = msg.telegram_id;
                                    let click_power = msg.click_power.unwrap_or(0); // Gelen click_power yoksa varsayılan 0 olsun

                                    if let Some(user) = user_repo.find_user_by_telegram_id(telegram_id).await.unwrap_or(None) {
                                        // Eğer click_score ve click_power zaten i32 tipindeyse direkt kullan
                                        let click_score = user.click_score; // Doğrudan i32 tipi
                                        let click_power = user.click_power; // Doğrudan i32 tipi
                                    
                                        let response = json!({
                                            "click_score": click_score,
                                            "click_power": click_power,
                                        });
                                        write.send(response.to_string().into()).await.unwrap();
                                    } else {
                                        // Eğer kullanıcı bulunamazsa veya hata olursa mesaj döndür
                                        let error_response = json!({
                                            "error": "Kullanıcı bulunamadı veya eksik veri"
                                        });
                                        write.send(error_response.to_string().into()).await.unwrap();
                                    }
                                    

                                    // Kullanıcının click_score'unu güncelle
                                    if let Some(updated_user) = user_repo.update_click_score(telegram_id, click_power).await.unwrap() {
                                        let updated_response = json!({
                                            "click_score": updated_user.click_score,
                                            "click_power": updated_user.click_power,
                                        });
                                        write.send(updated_response.to_string().into()).await.unwrap();
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Mesaj parse edilemedi: {}", err);
                                    let error_message = json!({
                                        "error": "Geçersiz mesaj formatı"
                                    });
                                    write.send(error_message.to_string().into()).await.unwrap();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Mesaj okunurken hata oluştu: {:?}", e);
                        break; // Hata oluştuğunda döngüden çıkıp bağlantıyı kapat
                    }
                }
            }
        });
    }
}
