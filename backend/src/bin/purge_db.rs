use dotenvy::dotenv;
use mongodb::{bson::doc, Client};
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mongo_uri = env::var("DATABASE_URL").expect("DATABASE_URL manquante");
    let db_name = env::var("DATABASE_NAME").unwrap_or("chat_forum".to_string());
    let client = Client::with_uri_str(&mongo_uri).await.expect("Connexion MongoDB échouée");
    let db = client.database(&db_name);

    // Supprime tous les users SAUF les Dictateurs
    let users = db.collection::<mongodb::bson::Document>("users");
    let r = users.delete_many(doc! { "role": { "$ne": "dictateur" } }, None).await.unwrap();
    println!("Users supprimés (non-dictateurs) : {}", r.deleted_count);

    // Vide les messages des canaux essentiels uniquement
    let essential = ["general", "cinema", "mediatheque", "cantine", "sport", "infos"];
    let messages = db.collection::<mongodb::bson::Document>("messages");
    let r = messages.delete_many(
        doc! { "channel_id": { "$in": essential.to_vec() } },
        None
    ).await.unwrap();
    println!("Messages des canaux essentiels supprimés : {}", r.deleted_count);

    // Les canaux et les articles ne sont PAS touchés
    println!("Purge terminée. Canaux et articles conservés.");
}
