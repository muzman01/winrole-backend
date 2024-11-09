use futures_util::{StreamExt, SinkExt};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_async;
use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use mongodb::{Client, Collection};
use mongodb::bson::{doc, Document};
use uuid::Uuid;
use std::collections::HashMap;

const MAX_ROLLS: usize = 5;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub player_id: i64,
    #[serde(rename = "dice_rolls")]
    pub rolls: Vec<i32>,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveGame {
    pub game_id: String,
    pub players: Vec<Player>,
    pub salon_id: String,
    pub table_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandMessage {
    pub action: String,
    pub player_id: Option<i64>,
    pub roll: Option<i32>,
    pub bot_rolls: Option<HashMap<i64, i32>>,  // Bot zarları
    pub salon_id: Option<String>,
    pub table_id: Option<String>,
}

pub async fn run_live_game_websocket_server_bots(mongo_client: &Client) {
    let addr = "127.0.0.1:9004";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    println!("Live Game WebSocket sunucusu {} adresinde çalışıyor", addr);

    let active_games = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.expect("WebSocket bağlantısı sırasında hata oluştu");
        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));

        tokio::spawn({
            let mongo_client = mongo_client.clone();
            let active_games = active_games.clone();
            async move {
                let mut live_game: Option<LiveGame> = None;

                // Bağlantı açılır açılmaz oyunu başlat
                let game_id = Uuid::new_v4().to_string();
                let player_id = Uuid::new_v4().as_u128() as i64;
                let salon_id = "default_salon".to_string();
                let table_id = "default_table".to_string();

                let players = vec![
                    Player { player_id, rolls: vec![], is_active: true },
                    Player { player_id: Uuid::new_v4().as_u128() as i64, rolls: vec![], is_active: true },
                    Player { player_id: Uuid::new_v4().as_u128() as i64, rolls: vec![], is_active: true },
                    Player { player_id: Uuid::new_v4().as_u128() as i64, rolls: vec![], is_active: true }
                ];

                live_game = Some(LiveGame {
                    game_id: game_id.clone(),
                    players,
                    salon_id: salon_id.clone(),
                    table_id: table_id.clone(),
                });

                active_games.lock().await.insert((salon_id.clone(), table_id.clone()), live_game.clone().unwrap());

                let message = json!({
                    "action": "game_started",
                    "game_id": game_id,
                    "salon_id": salon_id,
                    "table_id": table_id,
                });
                let _ = write.lock().await.send(Message::Text(message.to_string())).await;

                // Gelen zar atma komutlarını işle
                loop {
                    if let Some(Ok(Message::Text(text))) = read.next().await {
                        match serde_json::from_str::<CommandMessage>(&text) {
                            Ok(command) => {
                                if command.action == "roll_dice" {
                                    if let Some(game) = &mut live_game {
                                        if let Some(player_id) = command.player_id {
                                            if let Some(roll_value) = command.roll {
                                                let bot_rolls = command.bot_rolls.unwrap_or_default();
                                                handle_dice_roll(player_id, roll_value, bot_rolls, game, active_games.clone(), write.clone()).await;

                                                if game.players.iter().all(|p| p.rolls.len() == MAX_ROLLS) {
                                                    check_winner(game, &mongo_client, write.clone(), active_games.clone()).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("JSON ayrıştırma hatası: {:?}", e);
                            }
                        }
                    }
                }
            }
        });
    }
}

async fn handle_dice_roll<S>(
    player_id: i64,
    roll_value: i32,
    bot_rolls: HashMap<i64, i32>, // Bot zarları
    live_game: &mut LiveGame,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
) where
    S: futures_util::Sink<Message> + Unpin + Send + 'static,
{
    let mut active_games_guard = active_games.lock().await;

    if let Some(game) = active_games_guard.get_mut(&(live_game.salon_id.clone(), live_game.table_id.clone())) {
        // Gerçek kullanıcı zarını ekle
        if let Some(player) = game.players.iter_mut().find(|p| p.player_id == player_id) {
            player.rolls.push(roll_value);
        }

        // Bot zarlarını ekle (istemciden gelen bot_rolls kullanılıyor)
        for (bot_id, bot_roll) in bot_rolls.iter() {
            if let Some(bot) = game.players.iter_mut().find(|p| p.player_id == *bot_id) {
                bot.rolls.push(*bot_roll);
            }
        }

        // Tüm oyuncuların zar durumu güncellendikten sonra tüm oyunculara durumu gönder
        let players_state: Vec<_> = game.players.iter().map(|p| {
            json!({
                "player_id": p.player_id,
                "rolls": p.rolls,
                "total_roll": p.rolls.iter().sum::<i32>()
            })
        }).collect();

        let message = json!({
            "action": "roll_result",
            "players": players_state
        });

        if let Err(e) = write.lock().await.send(Message::Text(message.to_string())).await {

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
    if live_game.players.iter().all(|p| p.rolls.len() == MAX_ROLLS) {
        let winner = live_game.players.iter().max_by_key(|p| p.rolls.iter().sum::<i32>()).unwrap();
        let winner_id = winner.player_id;

        let message = json!({
            "action": "winner_announced",
            "winner_id": winner_id
        });

        let _ = write.lock().await.send(Message::Text(message.to_string())).await;

        save_game_result_to_db(&live_game.game_id, winner_id, &live_game.players, mongo_client).await;

        let key = (live_game.salon_id.clone(), live_game.table_id.clone());
        active_games.lock().await.remove(&key);
    }
}

async fn save_game_result_to_db(game_id: &str, winner_id: i64, players: &[Player], mongo_client: &Client) {
    let collection: Collection<Document> = mongo_client.database("games").collection("game_results");

    let result_doc = doc! {
        "game_id": game_id.to_string(),
        "winner_id": winner_id,
        "players": players.iter().map(|p| {
            doc! {
                "player_id": p.player_id,
                "rolls": p.rolls.clone(),
            }
        }).collect::<Vec<Document>>(),
    };

    let _ = collection.insert_one(result_doc, None).await;
}
