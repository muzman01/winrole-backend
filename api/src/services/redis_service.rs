use redis::{AsyncCommands, Client, aio::Connection};

pub struct RedisConnection {
    pub connection: Connection,
}

pub async fn setup_redis() -> Result<RedisConnection, redis::RedisError> {
    let client = Client::open("redis://127.0.0.1/")?;
    let connection = client.get_async_connection().await?;

    // Redis'e başarılı bir şekilde bağlandığında log yazdırıyoruz
    println!("Connected to Redis successfully!");

    Ok(RedisConnection { connection })
}
