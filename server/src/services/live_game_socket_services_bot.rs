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

const MAX_ROLLS: usize = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub player_id: String,
    pub rolls: Vec<i32>,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveGame {
    pub game_id: String,
    pub players: Vec<Player>,
    pub salon_id: String,
    pub table_id: String,
    pub unique_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandMessage {
    pub action: String,
    pub player_id: Option<String>,
    pub roll: Option<i32>,
    pub bot_rolls: Option<HashMap<String, i32>>,
    pub salon_id: Option<String>,
    pub table_id: Option<String>,
}

pub async fn run_live_game_websocket_server_bots(mongo_client: &Client) {
    let addr = "127.0.0.1:9004";
    let listener = TcpListener::bind(addr).await.expect("WebSocket sunucusu başlatılamadı!");

    println!("Live Game WebSocket sunucusu {} adresinde çalışıyor", addr);

    let active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>> = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.expect("WebSocket bağlantısı sırasında hata oluştu");
        let (write, mut read) = ws_stream.split();
        let write = Arc::new(Mutex::new(write));
        let active_games_clone = active_games.clone();
        let mongo_client_clone = mongo_client.clone();

        tokio::spawn(async move {
            let mut live_game: Option<LiveGame> = None;

            // İlk gelen mesajı bekleyerek 'start_game' kontrolü yapılır
            if let Some(Ok(Message::Text(text))) = read.next().await {
                println!("Gelen başlangıç mesajı: {}", text); 
                if let Ok(command) = serde_json::from_str::<CommandMessage>(&text) {
                    if command.action == "start_game" {
                        if let (Some(salon_id), Some(table_id)) = (command.salon_id.clone(), command.table_id.clone()) {
                            live_game = Some(initialize_game(&mongo_client_clone, &salon_id, &table_id).await);
                            active_games_clone.lock().await.insert((salon_id.clone(), table_id.clone()), live_game.clone().unwrap());
                            let message = json!({
                                "action": "game_started",
                                "game_id": live_game.as_ref().unwrap().game_id,
                                "salon_id": salon_id,
                                "table_id": table_id,
                                "unique_key": live_game.as_ref().unwrap().unique_key
                            });
                            let _ = write.lock().await.send(Message::Text(message.to_string())).await;
                        }
                    }
                }
            }

            // Zar atma işlemleri
            while let Some(Ok(Message::Text(text))) = read.next().await {
                if let Ok(command) = serde_json::from_str::<CommandMessage>(&text) {
                    if let Some(ref mut game) = live_game {
                        if command.action == "roll_dice" {
                            handle_dice_roll(&mongo_client_clone, command, game, write.clone()).await;

                            if game.players.iter().all(|p| p.rolls.len() == MAX_ROLLS) {
                                check_winner(&mongo_client_clone, game, write.clone(), active_games_clone.clone()).await;
                            }
                        }
                    }
                }
            }

            // Bağlantı kesildiğinde oyunu bitir
            if let Some(ref game) = live_game {
                println!("Bağlantı koptu, oyunu bitiriliyor: {}", game.game_id);
                finish_game_on_disconnect(&mongo_client_clone, game, active_games_clone.clone()).await;
            }
        });
    }
}


async fn finish_game_on_disconnect(
    mongo_client: &Client,
    game: &LiveGame,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
) {
    let key = (game.salon_id.clone(), game.table_id.clone());
    active_games.lock().await.remove(&key);
    
    remove_players_from_salon(mongo_client, &game.salon_id, &game.table_id).await;
    save_game_result_to_db(&game.game_id, "Disconnected", &game.players, mongo_client).await;
}

// Salondan oyuncuları silen fonksiyon
async fn remove_players_from_salon(mongo_client: &Client, salon_id: &str, table_id: &str) {
    let salons_collection: Collection<Document> = mongo_client.database("salons").collection("salons");
    let filter = doc! { "salon_id": salon_id.parse::<i32>().unwrap(), "tables.table_id": table_id.parse::<i32>().unwrap() };
    let update = doc! { "$set": { "tables.$.players": [] } };
    let _ = salons_collection.update_one(filter, update, None).await;
}
async fn initialize_game(mongo_client: &Client, salon_id: &str, table_id: &str) -> LiveGame {
    let game_id = Uuid::new_v4().to_string();
    let unique_key = Uuid::new_v4().to_string();

    let salons_collection: Collection<Document> = mongo_client.database("salons").collection("salons");
    let filter = doc! { "salon_id": salon_id.parse::<i32>().unwrap(), "tables.table_id": table_id.parse::<i32>().unwrap() };
    let salon_doc = salons_collection.find_one(filter, None).await.unwrap().expect("Salon bulunamadı");

    let tables = salon_doc.get_array("tables").expect("Tables bulunamadı");
    let table = tables.iter().find(|table| table.as_document().unwrap().get("table_id").unwrap().as_i32().unwrap() == table_id.parse::<i32>().unwrap())
        .expect("Table bulunamadı")
        .as_document().unwrap();

    let players: Vec<Player> = table.get_array("players").unwrap().iter().map(|player| {
        let player_doc = player.as_document().unwrap();
        Player {
            player_id: player_doc.get_i64("player_id").unwrap().to_string(),
            rolls: vec![],
            is_active: player_doc.get_bool("is_active").unwrap_or(true),
        }
    }).collect();

    let live_game = LiveGame {
        game_id: game_id.clone(),
        players: players.clone(),
        salon_id: salon_id.to_string(),
        table_id: table_id.to_string(),
        unique_key: unique_key.clone(),
    };

    let collection: Collection<Document> = mongo_client.database("games").collection("live_games_bots");
    let doc = doc! {
        "game_id": &game_id,
        "salon_id": salon_id,
        "table_id": table_id,
        "unique_key": unique_key,
        "players": players.iter().map(|p| {
            doc! {
                "player_id": &p.player_id,
                "rolls": &p.rolls,
                "is_active": p.is_active,
            }
        }).collect::<Vec<Document>>(),
    };
    let _ = collection.insert_one(doc, None).await;

    live_game
}

async fn handle_dice_roll<S>(
    mongo_client: &Client,
    command: CommandMessage,
    live_game: &mut LiveGame,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
) where
    S: futures_util::Sink<Message> + Unpin + Send + 'static,
{
    // Gerçek kullanıcı zarını işle
    if let Some(player_id) = command.player_id.clone() {
        if let Some(roll_value) = command.roll {
            if let Some(player) = live_game.players.iter_mut().find(|p| p.player_id == player_id) {
                player.rolls.push(roll_value);
            }
        }
    }

    // Bot zarlarını işle
    if let Some(bot_rolls) = command.bot_rolls {
        for (bot_id, roll) in bot_rolls {
            if let Some(bot) = live_game.players.iter_mut().find(|p| p.player_id == bot_id) {
                bot.rolls.push(roll);
            }
        }
    }

    // Veritabanını güncelle
    update_game_in_db(mongo_client, live_game).await;

    // Güncel zar sonuçlarını tüm clientlara gönder
    let players_state: Vec<_> = live_game.players.iter().map(|p| {
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

    let _ = write.lock().await.send(Message::Text(message.to_string())).await;
}


async fn update_game_in_db(mongo_client: &Client, live_game: &LiveGame) {
    let collection: Collection<Document> = mongo_client.database("games").collection("live_games_bots");
    let filter = doc! { "game_id": &live_game.game_id };
    let update = doc! { "$set": {
        "players": live_game.players.iter().map(|p| {
            doc! {
                "player_id": &p.player_id,
                "rolls": &p.rolls,
                "is_active": p.is_active,
            }
        }).collect::<Vec<Document>>()
    }};
    let _ = collection.update_one(filter, update, None).await;
}

async fn check_winner<S>(
    mongo_client: &Client,
    live_game: &LiveGame,
    write: Arc<Mutex<futures_util::stream::SplitSink<S, Message>>>,
    active_games: Arc<Mutex<HashMap<(String, String), LiveGame>>>,
) where
    S: futures_util::Sink<Message> + Unpin + std::fmt::Debug,
    <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
{
    if live_game.players.iter().all(|p| p.rolls.len() == MAX_ROLLS) {
        let winner = live_game.players.iter().max_by_key(|p| p.rolls.iter().sum::<i32>()).unwrap();
        let winner_id = winner.player_id.clone();

        let message = json!({
            "action": "winner_announced",
            "winner_id": winner_id
        });

        let _ = write.lock().await.send(Message::Text(message.to_string())).await;

        save_game_result_to_db(&live_game.game_id, &winner_id, &live_game.players, mongo_client).await;

        // Salondan oyuncuları temizleyin
        remove_players_from_salon(mongo_client, &live_game.salon_id, &live_game.table_id).await;

        // LiveGame'yi veritabanından silin
        let collection: Collection<Document> = mongo_client.database("games").collection("live_games_bots");
        let _ = collection.delete_one(doc! { "game_id": &live_game.game_id }, None).await;

        let key = (live_game.salon_id.clone(), live_game.table_id.clone());
        active_games.lock().await.remove(&key);
    }
}

async fn save_game_result_to_db(game_id: &str, winner_id: &str, players: &[Player], mongo_client: &Client) {
    let collection: Collection<Document> = mongo_client.database("games").collection("game_results_bots");

    let result_doc = doc! {
        "game_id": game_id.to_string(),
        "winner_id": winner_id,
        "players": players.iter().map(|p| {
            doc! {
                "player_id": &p.player_id,
                "rolls": p.rolls.clone(),
            }
        }).collect::<Vec<Document>>(),
    };

    let _ = collection.insert_one(result_doc, None).await;
}
