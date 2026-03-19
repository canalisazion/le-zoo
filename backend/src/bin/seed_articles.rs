use dotenvy::dotenv;
use mongodb::{bson::doc, Client};
use serde::{Deserialize, Serialize};
use std::env;
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

fn slugify(title: &str) -> String {
    let mut s = title.to_lowercase();
    let replacements = [
        ("à","a"),("â","a"),("é","e"),("è","e"),("ê","e"),("ë","e"),
        ("î","i"),("ï","i"),("ô","o"),("ù","u"),("û","u"),("ü","u"),("ç","c"),("œ","oe"),
    ];
    for (from, to) in replacements { s = s.replace(from, to); }
    s = s.chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    while s.contains("--") { s = s.replace("--", "-"); }
    s.trim_matches('-').chars().take(80).collect()
}

const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

fn content3() -> String {
    format!("{}\n\n{}\n\n{}", LOREM, LOREM, LOREM)
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mongo_uri = env::var("DATABASE_URL").expect("DATABASE_URL manquante");
    let db_name = env::var("DATABASE_NAME").unwrap_or("chat_forum".to_string());
    let client = Client::with_uri_str(&mongo_uri).await.expect("Connexion MongoDB échouée");
    let db = client.database(&db_name);
    let col = db.collection::<Article>("articles");

    let count = col.count_documents(doc! {}, None).await.unwrap_or(0);
    if count > 0 {
        println!("Collection non vide ({} articles). Aucun seed effectué.", count);
        return;
    }

    let now = Utc::now().timestamp();
    let articles = vec![
        ("La grande enquête sur les cafétérias scolaires", "dossier",
         "Ce que mange votre enfant à la cantine est-il vraiment sain ? Notre enquête dans 50 établissements révèle des pratiques surprenantes."),
        ("Un ours élu maire d'une ville américaine", "insolite",
         "La ville de Talkeetna, en Alaska, vient de confirmer la victoire de Stubbs, un ours de 3 ans, lors des élections municipales."),
        ("L'affaire du fromage volant enfin résolue", "faits-divers",
         "Trois ans après la disparition mystérieuse de 200 kilos de comté dans un village bourguignon, la gendarmerie annonce avoir retrouvé le coupable."),
        ("Pourquoi les films de zombie reviennent en force", "culture",
         "Après des années d'absence des salles, le film de zombie signe un retour fracassant. Analyse d'un phénomène culturel qui dit beaucoup sur notre époque."),
        ("Apprendre Rust en 30 jours : le guide complet", "formation",
         "Rust est réputé difficile, mais avec la bonne méthode, n'importe quel développeur peut maîtriser ce langage en un mois. Voici notre programme."),
        ("Le dernier roman de Michel Bussi vaut-il le détour ?", "critique",
         "Avec 'Maman a tort', Michel Bussi confirme son statut de maître du thriller français. Un roman haletant, mais qui peine à surprendre les habitués du genre."),
        ("Lettre ouverte aux conducteurs du dimanche", "billet",
         "Chaque week-end, une armée de conducteurs s'empare des routes de campagne. Il est temps de leur dire ce que tout le monde pense tout bas."),
        ("Les animaux qui nous ressemblent trop", "insolite",
         "Des pieuvres qui jouent, des corbeaux qui se vengent et des éléphants qui pleurent leurs morts : la frontière entre eux et nous est plus mince qu'on ne le croit."),
    ];

    let mut inserted = 0u32;
    for (title, category, excerpt) in articles {
        let slug = slugify(title);
        let article = Article {
            slug,
            title: title.to_string(),
            content: content3(),
            excerpt: excerpt.to_string(),
            category: category.to_string(),
            author_name: "Rédaction Le Zoo".to_string(),
            cover_image: None,
            published: true,
            created_at: now - inserted as i64 * 3600,
            updated_at: now,
        };
        match col.insert_one(&article, None).await {
            Ok(_) => { println!("✓ {}", title); inserted += 1; }
            Err(e) => eprintln!("✗ {} : {:?}", title, e),
        }
    }

    println!("\n{} articles seedés avec succès.", inserted);
}
