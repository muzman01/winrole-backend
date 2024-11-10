use futures_util::{StreamExt, SinkExt}; 
use serde_json::json;
use tokio::net::TcpListener;
use tokio::time::{self, Duration, Interval};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use mongodb::Client;
use serde::{Deserialize, Serialize};
use crate::repository::salon_repository::SalonRepository;
use std::sync::Arc;
use tokio::sync::Mutex;
use mongodb::bson::{doc, Bson, Document}; // BSON doc makrosunu içe aktarıyoruz

#[derive(Serialize, Deserialize, Debug)]
struct SalonMessage {
    action: String,
    telegram_id: Option<i64>, // Telegram ID ile bağlanacağız
}

pub async fn run_salon_websocket_server(mongo_client: Client) {
    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    let salon_repo = Arc::new(SalonRepository::new(&mongo_client));
    let active_connections = Arc::new(Mutex::new(std::collections::HashMap::new())); // Kullanıcıların bağlantılarını tutuyoruz.

    println!("WebSocket sunucusu {} adresinde çalışıyor", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let salon_repo = Arc::clone(&salon_repo);
        let active_connections = Arc::clone(&active_connections);

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket bağlantısı sırasında hata oluştu: {:?}", e);
                    return;
                }
            };

            let (write, mut read) = ws_stream.split();
            let write = Arc::new(Mutex::new(write));
            let mut ping_interval = time::interval(Duration::from_secs(10));
            let mut salon_data_interval: Option<Interval> = None;
            let mut telegram_id: Option<i64> = None;

            // İlk mesajı al - Telegram ID ile bağlantı kuruyoruz
            let salon_message: SalonMessage = match read.next().await {
                Some(Ok(msg)) => {
                    if let Ok(text_message) = msg.to_text() {

                        match serde_json::from_str::<SalonMessage>(text_message) {
                            Ok(parsed) => {
                                telegram_id = parsed.telegram_id;
                                parsed
                            },
                            Err(_) => {
                                eprintln!("Geçersiz mesaj formatı: {}", text_message);
                                return;
                            }
                        }
                    } else {
                        eprintln!("Mesaj tipi text değil.");
                        return;
                    }
                }
                Some(Err(e)) => {
                    eprintln!("Mesaj alınırken hata oluştu: {:?}", e);
                    return;
                }
                None => {
                    eprintln!("Bağlantı kapandı.");
                    return;
                }
            };

            // Kullanıcıyı takip et
            if let Some(id) = telegram_id {
                let mut connections = active_connections.lock().await;
                connections.insert(id, Arc::clone(&write)); 

            }

            if salon_message.action == "saloon" {

                salon_data_interval = Some(time::interval(Duration::from_secs(1))); // Her 1 saniyede bir veriyi alıyoruz.

                let write_clone = Arc::clone(&write);
                let salon_repo_clone = Arc::clone(&salon_repo);

                tokio::spawn(async move {
                    while let Some(ref mut interval) = salon_data_interval {
                        interval.tick().await;

                        // Veritabanından her döngüde en güncel veriyi çek
                        if let Err(e) = send_salon_data(write_clone.clone(), salon_repo_clone.clone()).await {

                        }
                    }
                });
            }

            // Mesajları dinle ve bağlantı durumu kontrol et
            loop {
                tokio::select! {
                    // Ping gönder
                    _ = ping_interval.tick() => {
                        if let Some(id) = telegram_id {
                            let write_guard = active_connections.lock().await.get(&id).cloned();
                            if let Some(write) = write_guard {
                                let mut write_lock = write.lock().await;
                                if write_lock.send(Message::Ping(vec![])).await.is_err() {
                                    eprintln!("Ping gönderimi başarısız oldu, bağlantıyı kapatıyoruz.");
                                    break;
                                }
                            }
                        }
                    }

                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {


                                // Bağlantı koptuğunda MongoDB'den kullanıcıyı masadan kaldır
                                if let Some(id) = telegram_id {
                                    remove_user_from_tables(salon_repo.clone(), id).await; // MongoDB'den kullanıcıyı kaldır
                                    let mut connections = active_connections.lock().await;
                                    connections.remove(&id); // Aktif bağlantılardan da kaldır
                                }
                                break; // Bağlantı kapatıldı
                            }
                            _ => {} // Diğer mesajlar işlenmez
                        }
                    }
                }

                // Bağlantı koparsa kullanıcıyı masadan kaldır
                if read.next().await.is_none() || matches!(read.next().await, Some(Err(_))) {
                    if let Some(id) = telegram_id {
                        remove_user_from_tables(salon_repo.clone(), id).await;
                        let mut connections = active_connections.lock().await;
                        connections.remove(&id);

                    }
                    break;
                }
            }
        });
    }
}

// Kullanıcıyı tüm masalardan kaldırma işlemi
async fn remove_user_from_tables(salon_repo: Arc<SalonRepository>, telegram_id: i64) {
    if let Err(e) = salon_repo.get_collection().update_many(
        doc!{}, // Tüm salonları hedef al
        doc!{
            "$pull": { "tables.$[].players": { "player_id": telegram_id } } // Oyuncuyu sil
        },
        None
    ).await {
        eprintln!("Kullanıcı {} MongoDB'den silinirken hata oluştu: {:?}", telegram_id, e);
    } else {
        println!("Kullanıcı {} MongoDB'den başarıyla kaldırıldı.", telegram_id);
    }
}

async fn send_salon_data<S>(
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>, 
    salon_repo: Arc<SalonRepository>,
) -> Result<(), Box<dyn std::error::Error>>  
where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug, 
    <S as futures_util::Sink<Message>>::Error: std::fmt::Debug + std::error::Error + 'static,
{
    // `salon_id` 0 olan salonları getir
    if let Ok(mut salons) = salon_repo.get_all_salons().await {
        // Sadece `salon_id` 0 olanı bul
        if let Some(salon) = salons.iter_mut().find(|s| s.salon_id == 0) {
            // `players` sayısı 4'ten az veya boş olan masaları temizle
            for table in &mut salon.tables {
                if table.players.len() < 4 {
                    table.players.clear();
                }
            }

            // MongoDB'de güncellenmiş `tables` verisini yolla
            let tables_bson: Vec<Document> = salon.tables.iter().map(|table| {
                let players_bson: Vec<Bson> = table.players.iter().map(|player| {
                    Bson::from(doc! {
                        "player_id": player.player_id,
                        "is_active": player.is_active,
                        "has_paid": player.has_paid,
                        "dice_rolls": player.dice_rolls.clone()
                    })
                }).collect();

                doc! {
                    "table_id": table.table_id,
                    "players": players_bson,
                    "bet_amount": table.bet_amount,
                    "game_state": table.game_state.to_string(),
                }
            }).collect();

            let filter = doc! { "salon_id": 0 };
            let update = doc! { "$set": { "tables": tables_bson } };

            // Güncelleme işlemi MongoDB'de yapılır
            if let Err(e) = salon_repo.get_collection().update_one(filter, update, None).await {
                eprintln!("Boş salonları temizlerken hata oluştu: {:?}", e);
            }
        }

        // Salon verilerini tekrar gönder
        let response = json!({ "salons": salons });
        let mut write_guard = write.lock().await;

        // WebSocket mesajı gönderiliyor
        if let Err(e) = write_guard.send(Message::text(response.to_string())).await {
            return Err(e.into());
        }
    }

    Ok(())
}
