#[macro_use]
extern crate rocket;

mod models;      // Modellerinizi içe aktarın
mod repository;  // User repository'nizi içe aktarın
mod jwt;         // JWT işlemleriniz
mod services;
use mongodb::bson::{Binary, Bson};
use rocket::{http::Status, serde::{json::Json, Deserialize, Serialize}, State};
use services::redis_service::setup_redis; 
use rocket_db_pools::mongodb::Client;
use repository::{market_repository::MarketRepository, salon_repository::SalonRepository, table_repository::TableRepository, user_repository::UserRepository};
use models::{market::Market, salon::Salon, table::Table, user::{Item, ReferenceLevel, References, User}};
use rocket::{get, post, options, catch, catchers, routes};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};
use uuid::Uuid;
use crate::models::table::Player;


// CORS fairing tanımı
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PUT, DELETE, OPTIONS",
        ));
        response.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));
    }
}

// CORS OPTIONS route'u
#[options("/<path..>")]
fn all_options(path: std::path::PathBuf) -> Status {
    Status::Ok
}

// API yanıt şeması
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub message: String,
    pub result: Option<T>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TelegramId {
    pub telegram_id: i64,
}

// Tüm kullanıcıları alma
#[get("/users")]
async fn get_all_users(user_repo: &rocket::State<UserRepository>) -> Json<ApiResponse<Vec<User>>> {
    match user_repo.get_all_users().await {
        Ok(users) if !users.is_empty() => Json(ApiResponse {
            message: "200: Success".to_string(),
            result: Some(users),
        }),
        Ok(_) => Json(ApiResponse {
            message: "204: No Content".to_string(),
            result: None,
        }),
        Err(_) => Json(ApiResponse {
            message: "500: Internal Server Error".to_string(),
            result: None,
        }),
    }
}

// Yeni kullanıcı oluşturma
#[post("/users", format = "json", data = "<new_user>")]
async fn create_user(
    user_repo: &rocket::State<UserRepository>,
    new_user: Json<User>,
) -> (Status, Json<ApiResponse<(User, String)>>) {
    if new_user.telegram_id <= 0 {
        return (Status::BadRequest, Json(ApiResponse {
            message: "400: Bad Request - telegram_id is required".to_string(),
            result: None,
        }));
    }

    // Aynı telegram_id ile kullanıcı var mı kontrol et
    match user_repo.find_user_by_telegram_id(new_user.telegram_id).await {
        Ok(Some(existing_user)) => {
            // Kullanıcı zaten varsa, onu geri döndür ve yeni kayıt yapma
            let token = jwt::jwt_helper::create_token(existing_user.telegram_id)
                .unwrap_or_else(|_| "Error creating token".to_string());
                
            (Status::Ok, Json(ApiResponse {
                message: "200: User already exists".to_string(),
                result: Some((existing_user, token)),
            }))
        },
        Ok(None) => {
            // Kullanıcı yoksa, yeni kullanıcı oluştur
            match user_repo.create_user(new_user.into_inner()).await {
                Ok(Some(created_user)) => {
                    let token = jwt::jwt_helper::create_token(created_user.telegram_id)
                        .unwrap_or_else(|_| "Error creating token".to_string());
                    
                    (Status::Created, Json(ApiResponse {
                        message: "201: Created".to_string(),
                        result: Some((created_user, token)),
                    }))
                },
                Ok(None) => {
                    let error_message = "500: Internal Server Error - User was not created".to_string();
                    eprintln!("{}", error_message);
                    (Status::InternalServerError, Json(ApiResponse {
                        message: error_message,
                        result: None,
                    }))
                },
                Err(e) => {
                    eprintln!("Error creating user: {:?}", e);
                    let error_message = format!("500: Internal Server Error - {:?}", e);
                    (Status::InternalServerError, Json(ApiResponse {
                        message: error_message,
                        result: None,
                    }))
                }
            }
        },
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            let error_message = format!("500: Internal Server Error - {:?}", e);
            (Status::InternalServerError, Json(ApiResponse {
                message: error_message,
                result: None,
            }))
        }
    }
}

// Telegram ID'ye göre kullanıcıyı alma
#[get("/users/<telegram_id>")]
async fn get_user_by_telegram_id(
    user_repo: &rocket::State<UserRepository>,
    telegram_id: i64,
) -> (Status, Json<ApiResponse<User>>) {
    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(user)) => {
            (Status::Ok, Json(ApiResponse {
                message: "200: Success".to_string(),
                result: Some(user),
            }))
        },
        Ok(None) => {
            (Status::NotFound, Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }))
        },
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            (Status::InternalServerError, Json(ApiResponse {
                message: format!("500: Internal Server Error - {:?}", e),
                result: None,
            }))
        }
    }
}

#[post("/salons", format = "json", data = "<new_salon>")]
async fn add_salon(
    new_salon: Json<Salon>, 
    salon_repo: &State<SalonRepository>
) -> (Status, Json<ApiResponse<String>>) {
    match salon_repo.add_salon(new_salon.into_inner()).await {
        Ok(_) => (
            Status::Created, 
            Json(ApiResponse {
                message: "Salon successfully created".to_string(),
                result: None,
            })
        ),
        Err(_) => (
            Status::InternalServerError, 
            Json(ApiResponse {
                message: "Failed to create salon".to_string(),
                result: None,
            })
        ),
    }
}

#[post("/salons/<salon_id>/tables", format = "json", data = "<new_table>")]
async fn add_table(
    salon_id: i32, 
    new_table: Json<Table>, 
    table_repo: &State<TableRepository>
) -> (Status, Json<ApiResponse<String>>) {
    match table_repo.add_table_to_salon(salon_id, new_table.into_inner()).await {
        Ok(_) => (
            Status::Ok, 
            Json(ApiResponse {
                message: "Table successfully added".to_string(),
                result: None,
            })
        ),
        Err(_) => (
            Status::InternalServerError, 
            Json(ApiResponse {
                message: "Failed to add table".to_string(),
                result: None,
            })
        ),
    }
}
#[get("/salons")]
async fn get_all_salons(
    salon_repo: &rocket::State<SalonRepository>
) -> (Status, Json<ApiResponse<Vec<Salon>>>) {
    match salon_repo.get_all_salons().await {
        Ok(salons) if !salons.is_empty() => (
            Status::Ok,
            Json(ApiResponse {
                message: "200: Success".to_string(),
                result: Some(salons),
            })
        ),
        Ok(_) => (
            Status::NoContent,
            Json(ApiResponse {
                message: "204: No Content".to_string(),
                result: None,
            })
        ),
        Err(_) => (
            Status::InternalServerError,
            Json(ApiResponse {
                message: "500: Internal Server Error".to_string(),
                result: None,
            })
        ),
    }
}


#[post("/salons/<salon_id>/tables/<table_id>/join", format = "json", data = "<telegram_data>")]
async fn join_table(
    salon_id: i32,
    table_id: i32,
    telegram_data: Json<TelegramId>,
    salon_repo: &State<SalonRepository>
) -> (Status, Json<ApiResponse<String>>) {
    let telegram_id = telegram_data.telegram_id;

    // Kullanıcı zaten başka bir masada mı kontrol et
    if let Ok(Some(salon)) = salon_repo.find_salon_by_id(salon_id).await {
        for table in &salon.tables {
            if table.players.iter().any(|p| p.player_id == telegram_id) {
                return (Status::Conflict, Json(ApiResponse {
                    message: format!("409: Conflict - Player {} is already seated at a table.", telegram_id),
                    result: None,
                }));
            }
        }

        // Masaya oyuncu ekleme işlemi
        let mut updated_salon = salon;
        if let Some(table) = updated_salon.tables.iter_mut().find(|t| t.table_id == table_id) {
            table.players.push(Player {
                player_id: telegram_id,
                has_paid: false,
                dice_rolls: vec![],
                is_active: true,
            });

            salon_repo.update_salon(updated_salon).await.unwrap();

            return (Status::Ok, Json(ApiResponse {
                message: format!("Player {} successfully joined table {}", telegram_id, table_id),
                result: None,
            }));
        } else {
            return (Status::NotFound, Json(ApiResponse {
                message: format!("404: Not Found - Table {} not found in Salon {}", table_id, salon_id),
                result: None,
            }));
        }
    } else {
        return (Status::NotFound, Json(ApiResponse {
            message: format!("404: Not Found - Salon {} not found", salon_id),
            result: None,
        }));
    }
}

#[post("/salons/<salon_id>/tables/<table_id>/leave", format = "json", data = "<telegram_data>")]
async fn leave_table(
    salon_id: i32,
    table_id: i32,
    telegram_data: Json<TelegramId>,
    salon_repo: &State<SalonRepository>
) -> (Status, Json<ApiResponse<String>>) {
    let telegram_id = telegram_data.telegram_id;

    // Salonu bulma ve kontrol etme
    if let Ok(Some(salon)) = salon_repo.find_salon_by_id(salon_id).await {
        let mut updated_salon = salon;

        // Masayı bulma ve oyuncunun o masada olup olmadığını kontrol etme
        if let Some(table) = updated_salon.tables.iter_mut().find(|t| t.table_id == table_id) {
            if let Some(player_index) = table.players.iter().position(|p| p.player_id == telegram_id) {
                // Oyuncuyu masadan kaldır
                table.players.remove(player_index);

                // Salonu güncelle
                salon_repo.update_salon(updated_salon).await.unwrap();

                return (Status::Ok, Json(ApiResponse {
                    message: format!("Player {} successfully left table {}", telegram_id, table_id),
                    result: None,
                }));
            } else {
                return (Status::NotFound, Json(ApiResponse {
                    message: format!("404: Not Found - Player {} not found at table {}", telegram_id, table_id),
                    result: None,
                }));
            }
        } else {
            return (Status::NotFound, Json(ApiResponse {
                message: format!("404: Not Found - Table {} not found in Salon {}", table_id, salon_id),
                result: None,
            }));
        }
    } else {
        return (Status::NotFound, Json(ApiResponse {
            message: format!("404: Not Found - Salon {} not found", salon_id),
            result: None,
        }));
    }
}

#[post("/salons/<salon_id>/tables/<table_id>/ready", format = "json", data = "<telegram_data>")]
async fn ready_table(
    salon_id: i32,
    table_id: i32,
    telegram_data: Json<TelegramId>,
    salon_repo: &State<SalonRepository>,
    user_repo: &State<UserRepository> // Add UserRepository to access user's game_pass
) -> (Status, Json<ApiResponse<String>>) {
    let telegram_id = telegram_data.telegram_id;

    // Determine required game passes based on salon_id
    let required_game_passes = match salon_id {
        1 => 1,
        2 => 3,
        3 => 5,
        4 => 10,
        5 => 15,
        _ => 0, // Default value if salon_id does not match any case
    };

    // Fetch user data to check game_pass count
    if let Ok(Some(mut user)) = user_repo.find_user_by_telegram_id(telegram_id).await {
        // Handle Option<i32> for game_pass safely
        if let Some(game_pass) = user.game_pass {
            if game_pass < required_game_passes {
                return (Status::BadRequest, Json(ApiResponse {
                    message: format!("Insufficient game passes. You need {} game passes to join salon {}", required_game_passes, salon_id),
                    result: None,
                }));
            }

            // Deduct the required game passes
            user.game_pass = Some(game_pass - required_game_passes);
            user_repo.update_user_game_pass(&user).await.unwrap(); // Ensure `update_user_hp` is the correct method name
        } else {
            return (Status::BadRequest, Json(ApiResponse {
                message: "Game pass information is unavailable.".to_string(),
                result: None,
            }));
        }

        // Salon and table retrieval and update logic
        if let Ok(Some(salon)) = salon_repo.find_salon_by_id(salon_id).await {
            let mut updated_salon = salon;

            // Find the table and update player's has_paid status
            if let Some(table) = updated_salon.tables.iter_mut().find(|t| t.table_id == table_id) {
                if let Some(player) = table.players.iter_mut().find(|p| p.player_id == telegram_id) {
                    player.has_paid = true;

                    // Save the updated salon
                    salon_repo.update_salon(updated_salon).await.unwrap();

                    return (Status::Ok, Json(ApiResponse {
                        message: format!("Player {} is now ready at table {}", telegram_id, table_id),
                        result: None,
                    }));
                } else {
                    return (Status::NotFound, Json(ApiResponse {
                        message: format!("404: Not Found - Player {} not found at table {}", telegram_id, table_id),
                        result: None,
                    }));
                }
            } else {
                return (Status::NotFound, Json(ApiResponse {
                    message: format!("404: Not Found - Table {} not found in Salon {}", table_id, salon_id),
                    result: None,
                }));
            }
        } else {
            return (Status::NotFound, Json(ApiResponse {
                message: format!("404: Not Found - Salon {} not found", salon_id),
                result: None,
            }));
        }
    } else {
        return (Status::NotFound, Json(ApiResponse {
            message: format!("404: Not Found - User with telegram_id {} not found", telegram_id),
            result: None,
        }));
    }
}


#[derive(Deserialize, Serialize, Debug)]
pub struct ConverRequest {
    pub telegram_id: i64,
    pub click_score: i32,
}
#[post("/convert", format = "json", data = "<conver_data>")]
async fn convert(
    user_repo: &State<UserRepository>,
    conver_data: Json<ConverRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = conver_data.telegram_id;
    let click_score_to_reduce = conver_data.click_score;

    // Kullanıcıyı telegram_id ile bulma
    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            if let Some(current_click_score) = user.click_score {
                if current_click_score >= click_score_to_reduce {
                    // click_score'u azalt ve hp'yi arttır
                    let hp_increase = click_score_to_reduce / 1000;
                    user.hp = Some(user.hp.unwrap_or(0) + hp_increase);
                    user.click_score = Some(current_click_score - click_score_to_reduce);

                    // Kullanıcıyı güncelle
                    match user_repo.update_user_hp(&user).await {
                        Ok(_) => (
                            Status::Ok,
                            Json(ApiResponse {
                                message: "User updated successfully".to_string(),
                                result: Some(user), // Güncellenmiş kullanıcıyı döndürüyoruz
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error updating user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Unable to update user".to_string(),
                                    result: None,
                                }),
                            )
                        }
                    }
                } else {
                    (
                        Status::BadRequest,
                        Json(ApiResponse {
                            message: "400: Bad Request - Insufficient click_score".to_string(),
                            result: None,
                        }),
                    )
                }
            } else {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        message: "400: Bad Request - User has no click_score set".to_string(),
                        result: None,
                    }),
                )
            }
        }
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error".to_string(),
                    result: None,
                }),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GamePassRequest {
    pub telegram_id: i64,
    pub hp: i32,
}

#[post("/buy_gamepass", format = "json", data = "<gamepass_data>")]
async fn buy_gamepass(
    user_repo: &rocket::State<UserRepository>,
    gamepass_data: Json<GamePassRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = gamepass_data.telegram_id;
    let hp_to_use = gamepass_data.hp;

    // Kullanıcıyı telegram_id ile bul
    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            // Kullanıcının yeterli hp'si var mı kontrol et
            if let Some(current_hp) = user.hp {
                if current_hp >= hp_to_use {
                    // HP'yi düşür ve game_pass ekle
                    let gamepass_to_add = hp_to_use / 100; // 100 HP = 1 game_pass
                    user.hp = Some(current_hp - hp_to_use);
                    user.game_pass = Some(user.game_pass.unwrap_or(0) + gamepass_to_add);

                    // Kullanıcıyı güncelle
                    match user_repo.update_user_hp_and_gamepass(&user).await {
                        Ok(_) => (
                            Status::Ok,
                            Json(ApiResponse {
                                message: "Gamepass purchased successfully".to_string(),
                                result: Some(user), // Güncellenmiş kullanıcı verisi
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error updating user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Unable to update user".to_string(),
                                    result: None,
                                }),
                            )
                        }
                    }
                } else {
                    (
                        Status::BadRequest,
                        Json(ApiResponse {
                            message: "400: Bad Request - Insufficient HP".to_string(),
                            result: None,
                        }),
                    )
                }
            } else {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        message: "400: Bad Request - User has no HP set".to_string(),
                        result: None,
                    }),
                )
            }
        },
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error".to_string(),
                    result: None,
                }),
            )
        }
    }
}


#[post("/buy_gamepass_with_ton", format = "json", data = "<gamepass_data>")]
async fn buy_gamepass_with_ton(
    user_repo: &rocket::State<UserRepository>,
    gamepass_data: Json<GamePassRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = gamepass_data.telegram_id;
    let price = gamepass_data.hp as f64; // Gönderilen fiyatı `f64` tipine dönüştür

    // Kullanıcıyı telegram_id ile bul
    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            // Kullanıcının yeterli `ton_amount` değeri var mı kontrol et
            if let Some(current_ton) = user.ton_amount {
                if current_ton >= price {
                    // Kaç `game_pass` alabileceğini hesapla
                    let gamepass_to_add = (price / 5.0).floor() as i32; // `i32` olarak `game_pass` sayısını belirle
                    let ton_to_deduct = gamepass_to_add as f64 * 5.0; // Toplam düşülecek TON miktarını `f64` olarak hesapla

                    // `ton_amount` miktarını güncelle ve `game_pass` ekle
                    user.ton_amount = Some(current_ton - ton_to_deduct);
                    user.game_pass = Some(user.game_pass.unwrap_or(0) + gamepass_to_add);

                    // Kullanıcıyı güncelle
                    match user_repo.update_user_hp_and_gamepasston(&user).await {
                        Ok(_) => (
                            Status::Ok,
                            Json(ApiResponse {
                                message: "Gamepasses purchased successfully".to_string(),
                                result: Some(user), // Güncellenmiş kullanıcı verisi
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error updating user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Unable to update user".to_string(),
                                    result: None,
                                }),
                            )
                        }
                    }
                } else {
                    (
                        Status::BadRequest,
                        Json(ApiResponse {
                            message: "400: Bad Request - Insufficient TON amount".to_string(),
                            result: None,
                        }),
                    )
                }
            } else {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        message: "400: Bad Request - User has no TON amount set".to_string(),
                        result: None,
                    }),
                )
            }
        },
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error".to_string(),
                    result: None,
                }),
            )
        }
    }
}



#[get("/market")]
async fn get_all_market(item_repo: &State<MarketRepository>) -> Json<ApiResponse<Vec<Market>>> {
    match item_repo.get_all_market().await {
        Ok(markets) => Json(ApiResponse {
            message: "Market items retrieved successfully".to_string(),
            result: Some(markets),
        }),
        Err(_) => Json(ApiResponse {
            message: "Failed to retrieve market items".to_string(),
            result: None,
        }),
    }
}
#[post("/market", format = "json", data = "<new_item>")]
async fn add_market_item(
    market_repo: &State<MarketRepository>,
    user_repo: &State<UserRepository>,
    new_item: Json<Market>,
) -> Json<ApiResponse<User>> {
    let item = new_item.into_inner();
    let telegram_id = item.seller;
    let item_id = item.id.clone();

    // Attempt to add the item to the market
    match market_repo.add_item(item).await {
        Ok(_) => {
            // If the item was successfully added to the market, attempt to remove it from the user's inventory
            match user_repo.remove_item_from_user(telegram_id, item_id.clone()).await {
                Ok(_) => {
                    // Fetch the updated user to return in the response
                    match user_repo.find_user_by_telegram_id(telegram_id).await {
                        Ok(Some(updated_user)) => Json(ApiResponse {
                            message: "Item added to market and removed from user inventory successfully".to_string(),
                            result: Some(updated_user),
                        }),
                        Ok(None) => Json(ApiResponse {
                            message: "User not found after item removal".to_string(),
                            result: None,
                        }),
                        Err(e) => {
                            eprintln!("Error fetching updated user: {:?}", e);
                            Json(ApiResponse {
                                message: "Failed to fetch updated user after item removal".to_string(),
                                result: None,
                            })
                        }
                    }
                }
                Err(_) => Json(ApiResponse {
                    message: "Item added to market, but failed to remove from user inventory".to_string(),
                    result: None,
                }),
            }
        }
        Err(_) => Json(ApiResponse {
            message: "Failed to add item to market".to_string(),
            result: None,
        }),
    }
}


#[derive(Deserialize)]
pub struct AddItemRequest {
    pub item_name: String,
    pub item_slug: String,
    pub reputation_points: i32,
    pub telegram_id: i64,
    pub hp: i32,
}
#[derive(Deserialize)]
pub struct AddItemRequestTon {
    pub item_name: String,
    pub item_slug: String,
    pub reputation_points: i32,
    pub telegram_id: i64,
    pub ton_amount: f64, // TON miktarı
}


#[post("/buy_item_sistem", format = "json", data = "<item_data>")]
async fn buy_item_sistem(
    user_repo: &State<UserRepository>,
    item_data: Json<AddItemRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = item_data.telegram_id;
    let hp_cost = item_data.hp;

    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            if let Some(current_hp) = user.hp {
                if current_hp >= hp_cost {
                    // Kullanıcıya eklenmeye çalışılan item bilgisi
                    let new_item = Item::new(
                        item_data.item_name.clone(),
                        item_data.item_slug.clone(),
                        item_data.reputation_points,
                    );

                    // Kullanıcı envanterinde aynı item var mı diye kontrol et
                    if user.items.as_ref().unwrap_or(&vec![]).iter().any(|item| item.id == new_item.id) {
                        return (
                            Status::Conflict,
                            Json(ApiResponse {
                                message: "409: Conflict - Item already exists in inventory".to_string(),
                                result: None,
                            }),
                        );
                    }

                    // Kullanıcıya item ekleme ve hp güncelleme
                    match user_repo.add_item_to_user(telegram_id, new_item, hp_cost).await {
                        Ok(Some(updated_user)) => (
                            Status::Ok,
                            Json(ApiResponse {
                                message: "Item added successfully".to_string(),
                                result: Some(updated_user),
                            }),
                        ),
                        Ok(None) => (
                            Status::NotFound,
                            Json(ApiResponse {
                                message: "404: Not Found - User not found after update".to_string(),
                                result: None,
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error while adding item to user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Failed to add item".to_string(),
                                    result: None,
                                }),
                            )
                        },
                    }
                } else {
                    (
                        Status::BadRequest,
                        Json(ApiResponse {
                            message: "400: Bad Request - Insufficient HP".to_string(),
                            result: None,
                        }),
                    )
                }
            } else {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        message: "400: Bad Request - User has no HP set".to_string(),
                        result: None,
                    }),
                )
            }
        }
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error while finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error - Failed to find user".to_string(),
                    result: None,
                }),
            )
        },
    }
}

#[post("/buy_item_system_ton", format = "json", data = "<item_data>")]
async fn buy_item_system_ton(
    user_repo: &State<UserRepository>,
    item_data: Json<AddItemRequestTon>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = item_data.telegram_id;
    let ton_cost = item_data.ton_amount;

    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            if let Some(current_ton) = user.ton_amount {
                // f64 ile karşılaştırma
                if current_ton >= ton_cost {
                    // Kullanıcıya eklenmeye çalışılan item bilgisi
                    let new_item = Item::new(
                        item_data.item_name.clone(),
                        item_data.item_slug.clone(),
                        item_data.reputation_points,
                    );

                    // Kullanıcı envanterinde aynı item var mı diye kontrol et
                    if user.items.as_ref().unwrap_or(&vec![]).iter().any(|item| item.id == new_item.id) {
                        return (
                            Status::Conflict,
                            Json(ApiResponse {
                                message: "409: Conflict - Item already exists in inventory".to_string(),
                                result: None,
                            }),
                        );
                    }

                    // Kullanıcıya item ekleme ve ton_amount'u güncelleme
                    match user_repo.add_item_to_user_ton(telegram_id, new_item, ton_cost).await {
                        Ok(Some(updated_user)) => {
                            // TON miktarını güncelle
                            user.ton_amount = Some(current_ton - ton_cost);
                            // Kullanıcı bilgilerini güncelle
                            user_repo.update_user_ton_amount(&user).await.unwrap();

                            (
                                Status::Ok,
                                Json(ApiResponse {
                                    message: "Item added successfully".to_string(),
                                    result: Some(updated_user),
                                }),
                            )
                        },
                        Ok(None) => (
                            Status::NotFound,
                            Json(ApiResponse {
                                message: "404: Not Found - User not found after update".to_string(),
                                result: None,
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error while adding item to user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Failed to add item".to_string(),
                                    result: None,
                                }),
                            )
                        },
                    }
                } else {
                    (
                        Status::BadRequest,
                        Json(ApiResponse {
                            message: "400: Bad Request - Insufficient TON amount".to_string(),
                            result: None,
                        }),
                    )
                }
            } else {
                (
                    Status::BadRequest,
                    Json(ApiResponse {
                        message: "400: Bad Request - User has no TON amount set".to_string(),
                        result: None,
                    }),
                )
            }
        }
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error while finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error - Failed to find user".to_string(),
                    result: None,
                }),
            )
        },
    }
}

#[derive(Deserialize)]
pub struct PurchaseRequest {
    pub buyer_telegram_id: i64, // Ürünü satın alan kişinin telegram_id'si
    pub item_id: Binary, // Satın alınacak ürünün id'si
}

#[post("/purchase_item", format = "json", data = "<purchase_data>")]
async fn purchase_item(
    user_repo: &State<UserRepository>,
    market_repo: &State<MarketRepository>,
    purchase_data: Json<PurchaseRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let buyer_telegram_id = purchase_data.buyer_telegram_id;
    let item_id = purchase_data.item_id.clone();

    // 1. Market veritabanında ürünü bul
    match market_repo.find_item_by_id(item_id.clone()).await {
        Ok(Some(item)) => {
            let item_price = item.price;
            let seller_telegram_id = item.seller;

            // 2. Alıcının `ton_amount`'unu kontrol et ve düşür
            match user_repo.find_user_by_telegram_id(buyer_telegram_id).await {
                Ok(Some(mut buyer)) => {
                    if let Some(current_ton) = buyer.ton_amount {
                        if current_ton >= item_price as f64 {
                            // 3. Alıcının envanterine item'ı ekle
                            let new_item = Item::new(
                                item.item_name.clone(),
                                item.item_slug.clone(),
                                item.reputation_points,
                            );
                            buyer.ton_amount = Some(current_ton - item_price as f64);

                            // 4. Satıcının `ton_amount`'unu güncelle
                            match user_repo.find_user_by_telegram_id(seller_telegram_id).await {
                                Ok(Some(mut seller)) => {
                                    let seller_current_ton = seller.ton_amount.unwrap_or(0.0);
                                    seller.ton_amount = Some(seller_current_ton + item_price as f64);

                                    // 5. Satıcı ve alıcı güncellemelerini kaydet
                                    user_repo.update_user_ton_amount(&buyer).await.unwrap();
                                    user_repo.update_user_ton_amount(&seller).await.unwrap();

                                    // 6. Market veritabanından ürünü kaldır
                                    market_repo.delete_item_from_market(item_id).await.unwrap();

                                    // 7. Alıcı envanterine item ekleme
                                    match user_repo.add_item_to_user_market(buyer_telegram_id, new_item).await {
                                        Ok(Some(updated_buyer)) => (
                                            Status::Ok,
                                            Json(ApiResponse {
                                                message: "Item successfully purchased.".to_string(),
                                                result: Some(updated_buyer),
                                            }),
                                        ),
                                        _ => (
                                            Status::InternalServerError,
                                            Json(ApiResponse {
                                                message: "Failed to add item to buyer inventory.".to_string(),
                                                result: None,
                                            }),
                                        ),
                                    }
                                },
                                _ => (
                                    Status::NotFound,
                                    Json(ApiResponse {
                                        message: "Seller not found.".to_string(),
                                        result: None,
                                    }),
                                ),
                            }
                        } else {
                            // Yeterli TON miktarı yoksa hata döndür
                            (
                                Status::BadRequest,
                                Json(ApiResponse {
                                    message: "Insufficient TON amount.".to_string(),
                                    result: None,
                                }),
                            )
                        }
                    } else {
                        // Alıcıda ton_amount ayarlanmamışsa hata döndür
                        (
                            Status::BadRequest,
                            Json(ApiResponse {
                                message: "Buyer TON amount not set.".to_string(),
                                result: None,
                            }),
                        )
                    }
                },
                _ => (
                    Status::NotFound,
                    Json(ApiResponse {
                        message: "Buyer not found.".to_string(),
                        result: None,
                    }),
                ),
            }
        },
        _ => (
            Status::NotFound,
            Json(ApiResponse {
                message: "Item not found in market.".to_string(),
                result: None,
            }),
        ),
    }
}


#[derive(Deserialize, Serialize, Debug)]
pub struct TrackUserRequest {
    pub telegram_id: i64, // Yeni kullanıcı
    pub referrer_id: i64, // Referans eden kullanıcının ID'si
}

#[post("/trackUser", format = "json", data = "<track_user_data>")]
async fn track_user(
    user_repo: &rocket::State<UserRepository>,
    track_user_data: Json<TrackUserRequest>,
) -> (Status, Json<ApiResponse<String>>) {
    let new_user_id = track_user_data.telegram_id;
    let referrer_id = track_user_data.referrer_id;

    // Referans eden kullanıcıyı `referrer_id` ile bul
    match user_repo.find_user_by_telegram_id(referrer_id).await {
        Ok(Some(mut referrer)) => {
            // Referans seviyesini güncelle
            let references = referrer.references.get_or_insert_with(|| References {
                level1: ReferenceLevel {
                    total_reference_required: 5,
                    is_started: true,
                    is_finished: false,
                    current_reference: 0,
                },
                level2: ReferenceLevel {
                    total_reference_required: 100,
                    is_started: false,
                    is_finished: false,
                    current_reference: 0,
                },
                level3: ReferenceLevel {
                    total_reference_required: 500,
                    is_started: false,
                    is_finished: false,
                    current_reference: 0,
                },
                level4: ReferenceLevel {
                    total_reference_required: 0, // Gerekli değil
                    is_started: false,
                    is_finished: false,
                    current_reference: 0,
                },
            });

            // Referans seviyesini güncelleme ve ödül kontrolü
            if references.level1.is_started && !references.level1.is_finished {
                references.level1.current_reference += 1;
                if references.level1.current_reference >= references.level1.total_reference_required {
                    references.level1.is_finished = true;
                    references.level2.is_started = true;
                    referrer.game_pass = Some(referrer.game_pass.unwrap_or(0) + 1);
                }
            } else if references.level2.is_started && !references.level2.is_finished {
                references.level2.current_reference += 1;
                if references.level2.current_reference >= references.level2.total_reference_required {
                    references.level2.is_finished = true;
                    references.level3.is_started = true;
                    referrer.ton_amount = Some(referrer.ton_amount.unwrap_or(0.0) + 5.0); // f64 güncellemesi
                }
            } else if references.level3.is_started && !references.level3.is_finished {
                references.level3.current_reference += 1;
                if references.level3.current_reference >= references.level3.total_reference_required {
                    references.level3.is_finished = true;
                    referrer.ton_amount = Some(referrer.ton_amount.unwrap_or(0.0) + 15.0); // f64 güncellemesi
                }
            }

            // Yeni kullanıcıyı friends listesine ekle
            let friends = referrer.friends.get_or_insert(vec![]);
            friends.push(new_user_id);

            // Güncellenmiş referans veren kullanıcıyı kaydet
            match user_repo.update_user_references_and_friends(&referrer).await {
                Ok(_) => (
                    Status::Ok,
                    Json(ApiResponse {
                        message: "User tracked and referrer updated successfully".to_string(),
                        result: None,
                    }),
                ),
                Err(e) => {
                    eprintln!("Error updating referrer: {:?}", e);
                    (
                        Status::InternalServerError,
                        Json(ApiResponse {
                            message: "500: Internal Server Error - Unable to update referrer".to_string(),
                            result: None,
                        }),
                    )
                }
            }
        },
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: format!("404: Not Found - Referrer user not found for id {}", referrer_id),
                result: None,
            }),
        ),
        Err(e) => {
            eprintln!("Error finding referrer user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error - Unable to find referrer".to_string(),
                    result: None,
                }),
            )
        }
    }
}



#[derive(Deserialize, Serialize, Debug)]
pub struct DepositTonRequest {
    pub telegram_id: i64,
    pub ton_amount: f64, // Artık f64 olarak tamamen işlem yapıyoruz
}

// Bu fonksiyon f64 olarak çalışır
#[post("/deposit_ton", format = "json", data = "<deposit_data>")]
async fn deposit_ton(
    user_repo: &State<UserRepository>,
    deposit_data: Json<DepositTonRequest>,
) -> (Status, Json<ApiResponse<User>>) {
    let telegram_id = deposit_data.telegram_id;
    let additional_ton = deposit_data.ton_amount; // Artık doğrudan f64

    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(mut user)) => {
            user.ton_amount = Some(user.ton_amount.unwrap_or(0.0) + additional_ton);

            match user_repo.update_user_ton_amount(&user).await {
                Ok(_) => {
                    match user_repo.find_user_by_telegram_id(telegram_id).await {
                        Ok(Some(updated_user)) => (
                            Status::Ok,
                            Json(ApiResponse {
                                message: "Deposit successful".to_string(),
                                result: Some(updated_user),
                            }),
                        ),
                        Ok(None) => (
                            Status::NotFound,
                            Json(ApiResponse {
                                message: "User not found after update".to_string(),
                                result: None,
                            }),
                        ),
                        Err(e) => {
                            eprintln!("Error fetching updated user: {:?}", e);
                            (
                                Status::InternalServerError,
                                Json(ApiResponse {
                                    message: "500: Internal Server Error - Failed to fetch updated user".to_string(),
                                    result: None,
                                }),
                            )
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error updating user: {:?}", e);
                    (
                        Status::InternalServerError,
                        Json(ApiResponse {
                            message: "500: Internal Server Error - Unable to update user".to_string(),
                            result: None,
                        }),
                    )
                }
            }
        }
        Ok(None) => (
            Status::NotFound,
            Json(ApiResponse {
                message: "404: Not Found - User not found".to_string(),
                result: None,
            })),
        Err(e) => {
            eprintln!("Error finding user: {:?}", e);
            (
                Status::InternalServerError,
                Json(ApiResponse {
                    message: "500: Internal Server Error".to_string(),
                    result: None,
                }),
            )
        }
    }
}

#[derive(Deserialize)]
pub struct BoostRequest {
    telegram_id: i64,
    requested_level: i32,
    currency_type: String, // "hp" veya "ton" olmalı
    amount: f64,           // HP için tam sayı olarak değerlendirilir, TON için float
}

#[post("/apply_boost", format = "json", data = "<boost_data>")]
async fn apply_boost(
    user_repo: &State<UserRepository>,
    boost_data: Json<BoostRequest>,
) -> (Status, String) {
    // İstek verilerini alıyoruz
    let telegram_id = boost_data.telegram_id;
    let requested_level = boost_data.requested_level;
    let currency_type = &boost_data.currency_type;
    let amount = boost_data.amount;

    // Kullanıcı verilerini al
    match user_repo.find_user_by_telegram_id(telegram_id).await {
        Ok(Some(user)) => {
            // HP veya TON miktarını kontrol et
            match currency_type.as_str() {
                "hp" => {
                    if let Some(current_hp) = user.hp {
                        if current_hp < amount as i32 {
                            return (Status::BadRequest, "Not enough HP amount.".to_string());
                        }
                    } else {
                        return (Status::BadRequest, "HP information not found.".to_string());
                    }
                }
                "ton" => {
                    if let Some(current_ton) = user.ton_amount {
                        if current_ton < amount {
                            return (Status::BadRequest, "Not enough TON amount.".to_string());
                        }
                    } else {
                        return (Status::BadRequest, "TON information not found.".to_string());
                    }
                }
                _ => return (Status::BadRequest, "Geçersiz para birimi.".to_string()),
            }

            // Boost uygulama işlemini başlat
            match user_repo.apply_boost(telegram_id, requested_level, currency_type, amount).await {
                Ok(_) => (Status::Ok, "Boost implemented successfully.".to_string()),
                Err(_) => (Status::InternalServerError, "Boost application failed.".to_string()),
            }
        }
        Ok(None) => (Status::NotFound, "Kullanıcı bulunamadı.".to_string()),
        Err(_) => (Status::InternalServerError, "Kullanıcı verisi alınamadı.".to_string()),
    }
}


// 404 Yakalama
#[catch(404)]
fn not_found(req: &Request) -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        message: format!("404: '{}' route not found", req.uri()),
        result: None,
    })
}

#[launch]
async fn rocket() -> _ {
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let user_repo = UserRepository::new(&client);

    // Salon ve Table repository'lerini burada oluşturup yönetin
    let salon_repo = SalonRepository::new(&client);
    let table_repo = TableRepository::new(&client);
    let market_repo = MarketRepository::new(&client);

    let redis_conn = setup_redis().await.unwrap(); // Redis bağlantısını kur
    rocket::build()
        .manage(user_repo)
        .manage(salon_repo)  // SalonRepository'yi yönetin
        .manage(table_repo)  // TableRepository'yi yönetin
        .manage(market_repo)  // TableRepository'yi yönetin
        .attach(CORS) // CORS fairing ekleniyor
        .mount("/", routes![
            get_all_users,
            create_user,
            get_user_by_telegram_id,
            all_options, // CORS için OPTIONS route
            add_salon,
            add_table,  // Salon ve table ekleme fonksiyonları
            get_all_salons,  // Tüm salonları getirin fonksiyonu
            join_table, // Eklenen join_table route'u
            leave_table,
            ready_table,
            convert,
            buy_gamepass,
            get_all_market,
            add_market_item,
            buy_item_sistem,
            buy_gamepass_with_ton,
            track_user,
            deposit_ton,
            apply_boost,
            buy_item_system_ton,
            purchase_item
        ])
        .register("/", catchers![not_found]) // 404 yakalayıcı
}
