use dotenvy::dotenv;
use mongodb::{bson::doc, Client};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};
use chrono::Utc;

#[derive(Serialize, Deserialize)]
struct Article {
    slug: String,
    title: String,
    content: String,
    excerpt: String,
    category: String,
    author_name: String,
    cover_image: Option<String>,
    published: bool,
    created_at: i64,
    updated_at: i64,
}

struct ParsedArticle {
    slug: String,
    title: String,
    category: String,
    excerpt: String,
    author: String,
    content: String,
}

fn parse_file(path: &Path) -> Option<ParsedArticle> {
    let raw = fs::read_to_string(path).ok()?;
    let (header, content) = raw.split_once("\n---\n")
        .or_else(|| raw.split_once("\r\n---\r\n"))?;

    let mut slug = String::new();
    let mut title = String::new();
    let mut category = String::new();
    let mut excerpt = String::new();
    let mut author = "Rédaction Le Zoo".to_string();

    for line in header.lines() {
        if let Some(v) = line.strip_prefix("SLUG:") { slug = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("TITLE:") { title = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("CATEGORY:") { category = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("EXCERPT:") { excerpt = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("AUTHOR:") { author = v.trim().to_string(); }
    }

    if slug.is_empty() || title.is_empty() { return None; }

    Some(ParsedArticle { slug, title, category, excerpt, author, content: content.trim().to_string() })
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mongo_uri = env::var("DATABASE_URL").expect("DATABASE_URL manquante");
    let db_name = env::var("DATABASE_NAME").unwrap_or("chat_forum".to_string());
    let client = Client::with_uri_str(&mongo_uri).await.expect("Connexion MongoDB échouée");
    let db = client.database(&db_name);
    let col = db.collection::<Article>("articles");

    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut entries: Vec<_> = fs::read_dir(&data_dir)
        .expect("Dossier data/ introuvable")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "txt").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let now = Utc::now().timestamp();
    let mut inserted = 0u32;
    let mut skipped = 0u32;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let Some(art) = parse_file(&path) else {
            eprintln!("✗ Parse échoué : {:?}", path.file_name().unwrap());
            continue;
        };

        // Skip si slug déjà présent
        match col.find_one(doc! { "slug": &art.slug }, None).await {
            Ok(Some(_)) => {
                println!("⏭ Déjà présent : {}", art.slug);
                skipped += 1;
                continue;
            }
            Err(e) => { eprintln!("✗ DB error : {:?}", e); continue; }
            Ok(None) => {}
        }

        let record = Article {
            slug: art.slug.clone(),
            title: art.title.clone(),
            content: art.content,
            excerpt: art.excerpt,
            category: art.category,
            author_name: art.author,
            cover_image: None,
            published: true,
            created_at: now - i as i64 * 3600,
            updated_at: now,
        };

        match col.insert_one(&record, None).await {
            Ok(_) => { println!("✓ {}", art.title); inserted += 1; }
            Err(e) => eprintln!("✗ {} : {:?}", art.slug, e),
        }
    }

    println!("\n{} insérés, {} ignorés (déjà présents).", inserted, skipped);
}
