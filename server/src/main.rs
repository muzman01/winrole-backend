mod services;
mod repository;
mod models;
mod config;

use config::mongo_config::setup_mongo;
use services::websocket_service::run_websocket_server;
use services::salon_websocket_service::run_salon_websocket_server; // Salon için WebSocket fonksiyonu
use services::redis_service::setup_redis;
use crate::services::live_game_socket_services::run_live_game_websocket_server;


#[tokio::main]
async fn main() {
    let mongo_client = setup_mongo().await;
    let _redis_conn = setup_redis().await.unwrap(); // Redis bağlantısını kur

    let mongo_client_clone = mongo_client.clone(); // Clone yapıyoruz
    let mongo_client_clone_clone = mongo_client.clone(); // Clone yapıyoruz
    let mongo_client_clone_clone_clone = mongo_client.clone(); // Clone yapıyoruz

    let user_socket = tokio::spawn(async move {
        run_websocket_server(mongo_client_clone).await;
    });

    let salon_socket = tokio::spawn(async move {
        run_salon_websocket_server(mongo_client_clone_clone).await;
    });
    
    let live_game_socket = tokio::spawn(async move {
        run_live_game_websocket_server(&mongo_client_clone_clone_clone).await;

    });
    let _ = tokio::join!(user_socket, salon_socket,live_game_socket);
}