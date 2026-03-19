use dotenvy::dotenv;
use mongodb::{bson::doc, Client};
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let username = env::args().nth(1).expect("Usage: promote_user <username>");
    let mongo_uri = env::var("DATABASE_URL").expect("DATABASE_URL manquante");
    let db_name = env::var("DATABASE_NAME").unwrap_or("chat_forum".to_string());
    let client = Client::with_uri_str(&mongo_uri).await.expect("Connexion MongoDB échouée");
    let db = client.database(&db_name);
    let col = db.collection::<mongodb::bson::Document>("users");

    let result = col.update_one(
        doc! { "username": &username },
        doc! { "$set": { "role": "super_admin" } },
        None,
    ).await.expect("Update échoué");

    if result.matched_count == 0 {
        eprintln!("✗ Utilisateur '{}' introuvable.", username);
    } else {
        println!("✓ '{}' promu super_admin.", username);
    }
}
