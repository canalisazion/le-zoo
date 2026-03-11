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

    // Supprime tous les users
    let users = db.collection::<mongodb::bson::Document>("users");
    let r = users.delete_many(doc! {}, None).await.unwrap();
    println!("Users supprimés : {}", r.deleted_count);

    // Supprime tous les messages
    let messages = db.collection::<mongodb::bson::Document>("messages");
    let r = messages.delete_many(doc! {}, None).await.unwrap();
    println!("Messages supprimés : {}", r.deleted_count);

    // Supprime les canaux NON gold
    let channels = db.collection::<mongodb::bson::Document>("channels");
    let r = channels.delete_many(doc! { "is_gold": { "$ne": true } }, None).await.unwrap();
    println!("Canaux non-gold supprimés : {}", r.deleted_count);

    println!("Purge terminée. Les canaux gold sont conservés.");
}
