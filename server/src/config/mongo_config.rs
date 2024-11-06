use mongodb::{Client, options::ClientOptions};

pub async fn setup_mongo() -> Client {
    let mongo_uri = "mongodb://localhost:27017";
    let mut client_options = ClientOptions::parse(mongo_uri).await.unwrap();
    client_options.app_name = Some("websocket-app".to_string());
    Client::with_options(client_options).unwrap()
}
