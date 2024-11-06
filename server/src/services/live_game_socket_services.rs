use futures_util::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_async;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::Arc;
use tokio::sync::Mutex;
use mongodb::{Client, Collection};
use mongodb::bson::{doc, Document};
use uuid::Uuid;
use std::collections::HashMap;

const MAX_ROLLS: usize = 5;
const ROLL_TIMEOUT: u64 = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub player_id: i64,
    #[serde(rename = "dice_rolls")]
    pub rolls: Vec<i32>,
    pub is_active: bool,
    pub last_roll_time: Option<u64>, // Zar atma süresi için Unix timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveGame {
    pub game_id: String,
    pub players: Vec<Player>,
    pub state: GameState,
    pub salon_id: String,
    pub table_id: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GameState {
    Waiting,
    Ready,
    Started,
    Completed,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameResult {
    pub game_id: String,
    pub winner_id: i64,
    pub players: Vec<PlayerResult>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerResult {
    pub player_id: i64,
    pub rolls: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandMessage {
    pub action: String,
    pub player_id: Option<i64>,
    pub roll: Option<i32>,
    pub players: Option<Vec<Player>>,
    pub salon_id: Option<String>,
    pub table_id: Option<String>,
}

pub async fn run_live_game_websocket_server(mongo_client: &Client) {
    let addr = "127.0.0.1:9003";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    println!("Live Game WebSocket sunucusu {} adresinde çalışıyor", addr);

    let active_games = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("WebSocket bağlantısı sırasında hata oluştu: {:?}", e);
                continue;
            }
        };

        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        tokio::spawn(broadcast_game_state_loop(write.clone(), active_games.clone()));
        tokio::spawn({
            let mongo_client = mongo_client.clone();
            let active_games = active_games.clone();
            async move {
                let mut live_game: Option<LiveGame> = None;
                let mut player_id = None;

                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    println!("Alınan mesaj: {:?}", text);
                                    match serde_json::from_str::<CommandMessage>(&text) {
                                        Ok(command) => {
                                            match command.action.as_str() {
                                                "start_game" => {
                                                    if let (Some(players), Some(salon_id), Some(table_id)) = (command.players, command.salon_id.clone(), command.table_id.clone()) {
                                                        live_game = Some(start_game(players, salon_id.clone(), table_id.clone()).await);
                                                        active_games.lock().await.insert((salon_id, table_id), live_game.clone().unwrap());

                                                        let message = json!({"action": "game_started", "game_id": live_game.as_ref().unwrap().game_id});
                                                        let _ = write.lock().await.send(Message::Text(message.to_string())).await;
                                                    } else {
                                                        eprintln!("Oyuncu listesi ya da salon/masa ID'si eksik");
                                                    }
                                                }
                                                "roll_dice" => {
                                                    println!("roll_dice komutu alındı");
                                                    if let Some(game) = &mut live_game {
                                                        if let Some(pid) = command.player_id {
                                                            player_id = Some(pid);
                                                            if let Some(roll_value) = command.roll {
                                                                handle_dice_roll(
                                                                    pid,
                                                                    roll_value,
                                                                    game.salon_id.clone(),
                                                                    game.table_id.clone(),
                                                                    active_games.clone(),
                                                                    write.clone()
                                                                ).await;


                                                                let response_message = json!({"action": "roll_acknowledged", "player_id": pid, "roll": roll_value});
                                                                let _ = write.lock().await.send(Message::Text(response_message.to_string())).await;
                                                                if game.players.iter().all(|p| p.rolls.len() == MAX_ROLLS) {
                                                                    check_winner(game, &mongo_client, write.clone(), active_games.clone()).await;
                                                                }
                                                            } else {
                                                                eprintln!("Zar değeri eksik");
                                                            }
                                                        } else {
                                                            eprintln!("Player ID eksik");
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("JSON ayrıştırma hatası: {:?}", e);
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    if let Some(pid) = player_id {
                                        if let Some(game) = &mut live_game {
                                            handle_player_disconnect(pid, game).await;
                                        }
                                    }
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });
    }
}

async fn handle_player_disconnect(player_id: i64, live_game: &mut LiveGame) {
    if let Some(player) = live_game.players.iter_mut().find(|p| p.player_id == player_id) {
        while player.rolls.len() < 5 {
            player.rolls.push(0);
        }
        println!("Player {} bağlantısı koptu, kalan zar hakları sıfırlandı.", player_id);
    }
}

async fn start_game(players: Vec<Player>, salon_id: String, table_id: String) -> LiveGame {
    let game_id = Uuid::new_v4().to_string();

    LiveGame {
        game_id,
        players,
        state: GameState::Started,
        salon_id,
        table_id,
    }
}


async fn handle_dice_roll<S>(
    player_id: i64,
    roll_value: i32,
    salon_id: String,
    table_id: String,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
) where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug + Send + 'static,
{
    // Oyunları kilitleyerek erişiyoruz
    let mut active_games_guard = active_games.lock().await;

    if let Some(game) = active_games_guard.get_mut(&(salon_id.clone(), table_id.clone())) {
        if let Some(player) = game.players.iter_mut().find(|p| p.player_id == player_id) {
            if player.rolls.len() < MAX_ROLLS {
                // Zar atıldıktan sonra kaydediliyor
                player.rolls.push(roll_value);
                player.last_roll_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

                println!("Player {} zar attı: {}. Şu anki zar listesi: {:?}", player_id, roll_value, player.rolls);

                // Güncel durumu tüm oyunculara yayınlama

                broadcast_game_state(Arc::new(Mutex::new(game.clone())), write.clone()).await;

            } else {
                println!("Player {} için maksimum zar sayısına ulaşıldı.", player_id);
            }
        }
    }
}
async fn broadcast_game_state<S>(
    live_game: Arc<Mutex<LiveGame>>,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
) where
    S: futures_util::Sink<Message> + Unpin + Send + 'static,
{
    let game_guard = live_game.lock().await;

    // Tüm oyuncuların güncel durumunu clientlara gönder
    let players_state: Vec<_> = game_guard.players.iter().map(|p| {
        json!({
            "player_id": p.player_id,
            "rolls": p.rolls,
            "total_roll": p.rolls.iter().sum::<i32>()
        })
    }).collect();

    let game_id = game_guard.game_id.clone();

    let game_state_message = json!({
        "action": "roll_update",
        "game_id": game_id,
        "players": players_state
    });

    if let Err(e) = write.lock().await.send(Message::Text(game_state_message.to_string())).await {

    }

    println!("Güncel oyun durumu gönderildi: {:?}", game_state_message);
}



async fn broadcast_game_state_loop<S>(
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
) where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
{
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        let games_guard = active_games.lock().await;

        for game in games_guard.values() {
            let players_state: Vec<_> = game.players.iter().map(|p| {
                json!({
                    "player_id": p.player_id,
                    "rolls": p.rolls,
                    "total_roll": p.rolls.iter().sum::<i32>()
                })
            }).collect();

            let game_state_message = json!({
                "action": "roll_update",
                "game_id": game.game_id,
                "players": players_state
            });

            if let Err(e) = write.lock().await.send(Message::Text(game_state_message.to_string())).await {
                eprintln!("Durum güncellenirken hata oluştu: {:?}", e);
            }

            println!("Güncel oyun durumu gönderildi: {:?}", game_state_message);
        }
    }
}


async fn check_winner<S>(
    live_game: &mut LiveGame,
    mongo_client: &Client,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
) where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
{
    if live_game.state == GameState::Completed {
        println!("Bu oyun zaten tamamlanmış.");
        return;
    }

    live_game.state = GameState::Completed;

    let mut sorted_players: Vec<_> = live_game.players.iter().collect();
    sorted_players.sort_by_key(|p| -p.rolls.iter().sum::<i32>());

    for (index, player) in sorted_players.iter().enumerate() {
        match index {
            0 => update_reputation_points(mongo_client, player.player_id, 20).await,
            1 => update_reputation_points(mongo_client, player.player_id, 10).await,
            2 => update_reputation_points(mongo_client, player.player_id, 5).await,
            _ => update_reputation_points(mongo_client, player.player_id, 1).await,
        }
    }

    let winner_id = sorted_players[0].player_id;
    send_winner_notification(winner_id, &live_game.players, write.clone()).await;

    let (bet_amount, percentage) = match live_game.salon_id.as_str() {
        "1" => (20, 0.80),
        "2" => (60, 0.85),
        "3" => (100, 0.90),
        "4" => (200, 0.95),
        "5" => (600, 0.98),
        _ => (20, 0.80),
    };
    let amount_to_add = (bet_amount as f64 * percentage) as i32;
    update_winner_ton_amount(mongo_client, winner_id, amount_to_add).await;

    save_game_result_to_db(live_game, winner_id, mongo_client).await;

    let filter = doc! {
        "salon_id": live_game.salon_id.parse::<i32>().unwrap(),
        "tables.table_id": live_game.table_id.parse::<i32>().unwrap()
    };

    let update = doc! {
        "$set": {
            "tables.$.players": []
        }
    };

    let salon_collection: Collection<Document> = mongo_client.database("salons").collection("salons");
    if let Err(e) = salon_collection.update_one(filter, update, None).await {
        eprintln!("Salon oyuncularını temizlerken hata: {:?}", e);
    } else {
        println!("Salon ve masadaki oyuncular temizlendi.");
    }

    let key = (live_game.salon_id.clone(), live_game.table_id.clone());
    active_games.lock().await.remove(&key);
    println!("Oyun salon_id: {}, table_id: {} sonlandı ve aktif oyunlardan kaldırıldı.", live_game.salon_id, live_game.table_id);
}

// Kullanıcının tüm item'lerinin reputation_points değerini oranla artırır
async fn update_reputation_points(mongo_client: &Client, player_id: i64, increase_percentage: i32) {
    let collection: Collection<Document> = mongo_client.database("users").collection("users");
    let filter = doc! { "telegram_id": player_id };

    if let Some(mut user_doc) = collection.find_one(filter.clone(), None).await.unwrap() {
        if let Some(items) = user_doc.get_array_mut("items").ok() {
            for item in items.iter_mut() {
                if let Some(item_doc) = item.as_document_mut() {
                    if let Some(rep_points) = item_doc.get_i32("reputation_points").ok() {
                        let updated_points = rep_points + rep_points * increase_percentage / 100;
                        item_doc.insert("reputation_points", updated_points);
                    }
                }
            }

            if let Err(e) = collection.update_one(filter, doc! { "$set": { "items": items } }, None).await {
                eprintln!("Reputation points güncellenirken hata oluştu: {:?}", e);
            } else {
                println!("Player {} için reputation points başarıyla güncellendi.", player_id);
            }
        }
    }
}

async fn update_winner_ton_amount(mongo_client: &Client, winner_id: i64, amount_to_add: i32) {
    let collection: Collection<Document> = mongo_client.database("users").collection("users");

    let filter = doc! { "telegram_id": winner_id };
    let update = doc! { "$inc": { "ton_amount": amount_to_add } };

    match collection.update_one(filter, update, None).await {
        Ok(_) => println!("Player {}'in ton_amount değeri başarıyla güncellendi. Eklenen miktar: {}", winner_id, amount_to_add),
        Err(e) => eprintln!("ton_amount güncellenirken hata oluştu: {:?}", e),
    }
}

async fn send_winner_notification<S>(
    winner_id: i64,
    players: &[Player],
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>
) where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
{
    for player in players {
        let message = if player.player_id == winner_id {
            json!({
                "action": "winner_announced",
                "message": "Kazandınız!",
                "winner_id": winner_id
            })
        } else {
            json!({
                "action": "winner_announced",
                "message": format!("Kazanan: Player {}", winner_id),
                "winner_id": winner_id
            })
        };

        let mut write_guard = write.lock().await;
        if let Err(e) = write_guard.send(Message::Text(message.to_string())).await {
            eprintln!("Mesaj gönderiminde hata: {:?}", e);
        }
    }
}

async fn save_game_result_to_db(live_game: &LiveGame, winner_id: i64, mongo_client: &Client) {
    let collection: Collection<Document> = mongo_client.database("games").collection("game_results");

    let game_result = GameResult {
        game_id: live_game.game_id.clone(),
        winner_id,
        players: live_game.players.iter().map(|p| PlayerResult {
            player_id: p.player_id,
            rolls: p.rolls.clone(),
        }).collect(),
    };

    let result_doc = doc! {
        "game_id": game_result.game_id,
        "winner_id": game_result.winner_id,
        "players": game_result.players.iter().map(|p| {
            doc! {
                "player_id": p.player_id,
                "rolls": p.rolls.clone(),
            }
        }).collect::<Vec<Document>>(),
    };

    if let Err(e) = collection.insert_one(result_doc, None).await {
        eprintln!("Oyun sonucu MongoDB'ye kaydedilirken hata oluştu: {:?}", e);
    } else {
        println!("Oyun sonucu başarıyla MongoDB'ye kaydedildi.");
    }
}
