use axum::{
    extract::{
        ws::{Message as WsMsg, WebSocket, WebSocketUpgrade},
        ConnectInfo, DefaultBodyLimit, Query, State, Path, Request,
    },
    http::{StatusCode, HeaderMap},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, delete},
    Json, Router,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use bcrypt::verify as bcrypt_verify;
use dotenvy::dotenv;
use mongodb::{bson::doc, Client, Database, IndexModel};
use shared::{User, Channel, Role, MemberInfo, WsClientMsg, WsServerMsg, ChannelWithStats, MessageStatus};
use std::env;
use std::net::SocketAddr;
use axum::http::header::{
    AUTHORIZATION, CONTENT_TYPE,
    STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
    CONTENT_SECURITY_POLICY, REFERRER_POLICY, SERVER,
};
use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::CorsLayer;
use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use tokio::sync::broadcast;
use std::sync::Arc;
use dashmap::DashMap;
use futures_util::{StreamExt, SinkExt};
use tracing::{info, error, warn};
use once_cell::sync::Lazy;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as EmailMessage, Tokio1Executor,
    transport::smtp::authentication::Credentials,
    message::header::ContentType,
};
use uuid::Uuid;

// On garde ces imports pour éviter les warnings si du vieux code traîne
#[allow(unused_imports)]
use std::collections::{HashMap, HashSet};

// --- Regexes compilées une seule fois au démarrage ---
static USERNAME_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^[a-zA-Z0-9_]{3,20}$").unwrap());
static EMAIL_RE: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap());

async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    h.insert(X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"));
    h.insert(X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"));
    h.insert(CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; connect-src 'self' wss://lezoo.fr ws://localhost:3000; \
             script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"
        ));
    h.insert(REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"));
    h.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    h.remove(SERVER);
    res
}

fn is_valid_channel_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

// --- État global ---

#[derive(Clone)]
struct AppState {
    db: Database,
    jwt_secret: String,
    superadmin_username: String,
    // DASHMAP : Performance maximale, zéro blocage
    channels_tx: Arc<DashMap<String, broadcast::Sender<String>>>,
    online_users: Arc<DashMap<String, ()>>,
    presence_tx: broadcast::Sender<String>,
    user_tx: Arc<DashMap<String, broadcast::Sender<String>>>,
    rate_limiter: Arc<DashMap<String, Vec<i64>>>,
    jwt_blacklist: Arc<DashMap<String, i64>>,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    username: String,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    role: String,
    #[serde(default)]
    gender: String,
    exp: usize,
    #[serde(default)]
    iat: usize,
}


#[derive(Deserialize)]
struct MessagesQuery {
    channel: String,
    before_id: Option<String>,
}

#[derive(Deserialize)]
struct DeleteAccountRequest {
    password: String,
}

#[derive(Deserialize)]
struct PromoteRequest {
    username: String,
    role: String,
}

#[derive(Deserialize)]
struct BanRequest {
    username: String,
}

#[derive(Deserialize)]
struct SubscribeRequest {
    channel_id: String,
    subscribe: bool,
}

#[derive(Deserialize)]
struct DirectMessagesQuery {
    with: String,
    before_id: Option<String>,
}

#[derive(Deserialize)]
struct UpdateAvatarRequest {
    avatar: String,
}

#[derive(Deserialize)]
struct ReactionRequest {
    message_id: String,
    #[allow(dead_code)]
    emoji: String,
}

#[derive(Deserialize)]
struct MarkReadRequest {
    message_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ResetToken {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<mongodb::bson::oid::ObjectId>,
    token: String,
    email: String,
    expires_at: i64,
    used: bool,
}

#[derive(Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    old_password: String,
    new_password: String,
}

// --- Helpers ---

fn decode_token(state: &AppState, headers: &HeaderMap) -> Result<Claims, StatusCode> {
    let auth_header = headers.get("Authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let token = auth_header.strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if state.jwt_blacklist.contains_key(token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| StatusCode::UNAUTHORIZED)
}

fn role_from_str(s: &str) -> Role {
    match s {
        "super_admin" => Role::SuperAdmin,
        "admin" => Role::Admin,
        _ => Role::User,
    }
}

fn calculate_badges(user: &shared::User) -> Vec<String> {
    let mut badges = Vec::new();
    if user.days_active >= 7 {
        badges.push("habitue".to_string());
    }
    if user.message_count >= 100 {
        badges.push("causeur".to_string());
    }
    if user.channels_joined >= 5 {
        badges.push("decouvreur".to_string());
    }
    badges
}

async fn check_rate_limit(state: &AppState, username: &str) -> bool {
    let now = chrono::Utc::now().timestamp();
    let mut is_valid = true;

    state.rate_limiter
        .entry(username.to_string())
        .and_modify(|times| {
            times.retain(|&t| now - t < 60);
            is_valid = times.len() < 10;
            if is_valid {
                times.push(now);
            }
        })
        .or_insert_with(|| {
            vec![now]
        });

    is_valid
}

fn check_ip_rate_limit(limiter: &DashMap<String, Vec<i64>>, ip: &str, max: usize, window_secs: i64) -> bool {
    let now = Utc::now().timestamp();
    let mut allowed = true;
    limiter
        .entry(ip.to_string())
        .and_modify(|times| {
            times.retain(|&t| now - t < window_secs);
            if times.len() >= max {
                allowed = false;
            } else {
                times.push(now);
            }
        })
        .or_insert_with(|| vec![now]);
    allowed
}

fn sanitize_message(content: &str) -> String {
    content
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .chars()
        .take(2000)
        .collect()
}

fn contains_suspicious_url(content: &str) -> bool {
    let suspicious = ["bit.ly", "tinyurl", ".exe", ".zip", "discord.gg/"];
    suspicious.iter().any(|s| content.to_lowercase().contains(s))
}

fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if r"\.+*?()|[]{}^$#".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

async fn send_email(to_email: &str, subject: &str, html_body: &str) -> bool {
    let smtp_host = match std::env::var("SMTP_HOST") {
        Ok(h) if !h.is_empty() => h,
        _ => {
            info!("[DEV] Email simulé à <{}> — Sujet: {}", to_email, subject);
            return true;
        }
    };
    let smtp_user = std::env::var("SMTP_USERNAME").unwrap_or_default();
    let smtp_pass = std::env::var("SMTP_PASSWORD").unwrap_or_default();
    let smtp_port: u16 = std::env::var("SMTP_PORT").ok()
        .and_then(|p| p.parse().ok()).unwrap_or(587);
    let from_addr = std::env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "noreply@lezoo.fr".to_string());

    let email = match EmailMessage::builder()
        .from(format!("Le Zoo <{}>", from_addr).parse().unwrap_or_else(|_| from_addr.parse().unwrap()))
        .to(match to_email.parse() {
            Ok(a) => a,
            Err(e) => { error!("Email adresse invalide {}: {}", to_email, e); return false; }
        })
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body.to_string())
    {
        Ok(m) => m,
        Err(e) => { error!("Erreur construction email: {}", e); return false; }
    };

    let creds = Credentials::new(smtp_user, smtp_pass);
    let transport = if smtp_port == 465 {
        match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host) {
            Ok(b) => b.port(smtp_port).credentials(creds).build(),
            Err(e) => { error!("SMTP relay error: {}", e); return false; }
        }
    } else {
        match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_host) {
            Ok(b) => b.port(smtp_port).credentials(creds).build(),
            Err(e) => { error!("SMTP starttls error: {}", e); return false; }
        }
    };

    match transport.send(email).await {
        Ok(_) => { info!("Email envoyé à {}", to_email); true }
        Err(e) => { error!("SMTP send error à {}: {}", to_email, e); false }
    }
}

fn hash_token(token: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

// --- Main ---

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv().ok();
    
    println!(">>> DÉMARRAGE DU BACKEND (VERSION CORRIGÉE) <<<");

    let mongo_uri = env::var("DATABASE_URL").unwrap_or_else(|_| {
        error!("DATABASE_URL manquante");
        std::process::exit(1)
    });
    let db_name = env::var("DATABASE_NAME").unwrap_or_else(|_| "chat_forum".to_string());
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET env var is required");
    let superadmin_username = env::var("SUPERADMIN_USERNAME").unwrap_or_default();

    let client = Client::with_uri_str(&mongo_uri).await.expect("Impossible de se connecter à MongoDB");

    // INITIALISATION DASHMAP
    let channels_tx = Arc::new(DashMap::new());
    let online_users = Arc::new(DashMap::new());
    let (presence_tx, _) = broadcast::channel(100);
    let user_tx = Arc::new(DashMap::new());
    let rate_limiter = Arc::new(DashMap::new());
    let jwt_blacklist = Arc::new(DashMap::new());

    let state = AppState {
        db: client.database(&db_name),
        jwt_secret,
        superadmin_username: superadmin_username.clone(),
        channels_tx,
        online_users,
        presence_tx,
        user_tx,
        rate_limiter,
        jwt_blacklist,
    };

    info!("Connecté à MongoDB : {}", db_name);

  
    {
        let messages_col = state.db.collection::<shared::Message>("messages");
        
        // Index pour les salons publics
        let index_channel = IndexModel::builder()
            .keys(doc! { "channel_id": 1, "created_at": -1 })
            .build();
        let _ = messages_col.create_index(index_channel, None).await;

        // Index pour les messages privés (CRUCIAL pour la vitesse)
        let index_dm = IndexModel::builder()
            .keys(doc! { "direct_to": 1, "author_name": 1, "created_at": -1 })
            .build();
        let _ = messages_col.create_index(index_dm, None).await;
    }

    {
        // Index TTL sur reset_tokens : expire automatiquement après expiration
        let reset_col = state.db.collection::<ResetToken>("reset_tokens");
        let idx_token = IndexModel::builder()
            .keys(doc! { "token": 1 })
            .build();
        let idx_email = IndexModel::builder()
            .keys(doc! { "email": 1 })
            .build();
        let _ = reset_col.create_index(idx_token, None).await;
        let _ = reset_col.create_index(idx_email, None).await;
    }

    // Initialisation des salons dorés
    let channels_collection = state.db.collection::<Channel>("channels");
    let gold_channels = vec![
        ("general", "Général", "💬", "Salon de discussion général"),
        ("cinema", "Cinéma", "🎬", "Discussions sur les films et séries"),
        ("mediatheque", "Médiathèque", "📚", "Livres, musique et culture"),
        ("cantine", "Cantine", "🍽️", "Recettes et gastronomie"),
        ("sport", "Sport", "⚽", "Résultats sportifs et discussions"),
        ("infos", "Infos", "📰", "Actualités traduites automatiquement"),
    ];

    for (id, name, icon, desc) in gold_channels {
        if channels_collection.find_one(doc! { "id": id }, None).await.unwrap_or(None).is_none() {
            let channel = Channel {
                id: Some(id.to_string()),
                name: name.to_string(),
                description: desc.to_string(),
                channel_type: "text".to_string(),
                is_general: id == "general",
                is_locked: true,
                is_gold: true,
                icon: Some(icon.to_string()),
                topic_media: None,
                topic_text: None,
                ai_description: None,
                topic: None,
                style_color: None,
                deleted: false,
            };
            let _ = channels_collection.insert_one(channel, None).await;
            info!("Salon créé : {}", name);
        }
    }

    // SuperAdmin via variable d'environnement
    if !superadmin_username.is_empty() {
        let users_collection = state.db.collection::<User>("users");
        if let Ok(Some(_)) = users_collection.find_one(doc! { "username": &superadmin_username }, None).await {
            let _ = users_collection.update_one(
                doc! { "username": &superadmin_username },
                doc! { "$set": { "role": "super_admin" } },
                None
            ).await;
            info!("SuperAdmin défini : {}", superadmin_username);
        }
    }

    // --- CORS prod-ready ---
    let cors = CorsLayer::new()
        .allow_origin(
            std::env::var("ALLOWED_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:8080".to_string())
                .split(',')
                .filter_map(|s| s.trim().parse::<HeaderValue>().ok())
                .collect::<Vec<_>>()
        )
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    // --- JWT BLACKLIST CLEANUP ---
    {
        let blacklist = state.jwt_blacklist.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
                let now = Utc::now().timestamp();
                blacklist.retain(|_, exp| *exp > now);
            }
        });
    }

    // --- VEILLE ACTU AUTOMATIQUE ---
    let news_state = state.clone();

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/register", post(register_handler))
        .route("/api/login", post(login_handler))
        .route("/api/auth/forgot-password", post(forgot_password_handler))
        .route("/api/auth/reset-password", post(reset_password_handler))
        .route("/api/auth/change-password", post(change_password_handler))
        .route("/api/channels", get(channels_handler).post(create_channel_handler))
        .route("/api/channels/discover", get(discover_channels_handler))
        .route("/api/channels/:id", delete(delete_channel_handler))
        .route("/api/channels/:id/summary", post(summarize_channel_handler))
        .route("/api/messages", get(messages_handler))
        .route("/api/users", get(users_handler))
        .route("/api/users/me", get(me_handler))
        .route("/api/users/promote", post(promote_handler))
        .route("/api/users/ban", post(ban_handler))
        .route("/api/users/channels/subscribe", post(subscribe_channel_handler))
        .route("/api/users/avatar", post(update_avatar_handler))
        .route("/api/users/delete", delete(delete_account_handler))
        .route("/api/messages/react", post(react_to_message_handler))
        .route("/api/messages/:id", delete(delete_message_handler))
        .route("/api/messages/:id/pin", post(pin_message_handler))
        .route("/api/channels/:id/archive", get(archive_channel_handler))
        .route("/api/direct_messages", get(direct_messages_handler))
        .route("/api/direct_messages/read", post(mark_direct_message_read_handler))
        .route("/ws", get(ws_handler))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(cors)
        .layer(DefaultBodyLimit::max(2_097_152))
        .with_state(state);

    // --- RSS NEWS BACKGROUND TASK ---
    {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            let colab_url = std::env::var("COLAB_AI_URL")
                .unwrap_or_else(|_| "http://localhost:5000/generate".to_string());

            // (feed_url, target_channel, source_name, is_english)
            let rss_feeds: Vec<(&str, &str, &str, bool)> = vec![
                ("https://www.lemonde.fr/rss/une.xml", "infos", "Le Monde", false),
                ("https://www.lefigaro.fr/rss/figaro_actualites.xml", "infos", "Le Figaro", false),
                ("https://feeds.bbci.co.uk/news/world/europe/rss.xml", "infos", "BBC News", true),
                ("https://feeds.bbci.co.uk/sport/rss.xml", "sport", "BBC Sport", true),
            ];

            loop {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(20))
                    .build()
                    .unwrap_or_default();

                for (feed_url, target_channel, source_name, _is_english) in &rss_feeds {
                    let xml = match client.get(*feed_url).send().await {
                        Ok(resp) => match resp.bytes().await {
                            Ok(b) => b,
                            Err(e) => { error!("RSS {}: erreur lecture réponse: {}", source_name, e); continue; },
                        },
                        Err(e) => { error!("RSS {}: échec requête HTTP: {}", source_name, e); continue; },
                    };

                    let feed = match feed_rs::parser::parse(&xml[..]) {
                        Ok(f) => f,
                        Err(e) => { error!("RSS {}: échec parsing XML: {}", source_name, e); continue; },
                    };

                    for entry in feed.entries.iter().take(3) {
                        let title = entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_default();
                        let link = entry.links.first().map(|l| l.href.clone()).unwrap_or_default();

                        if title.is_empty() || link.is_empty() { continue; }

                        // Doublon check — lien échappé pour éviter les faux négatifs regex
                        let link_escaped = regex_escape(&link);
                        let already_posted = news_state.db
                            .collection::<shared::Message>("messages")
                            .find_one(doc! {
                                "channel_id": *target_channel,
                                "content": { "$regex": &link_escaped, "$options": "" }
                            }, None)
                            .await
                            .unwrap_or(None)
                            .is_some();

                        if already_posted { continue; }

                        // Translate ALL titles to French via Qwen, no exceptions
                        let titre_fr = {
                            let raw_title = title.clone();
                            let prompt_text = format!(
                                "Traduis ce titre en français naturel et concis. \
                                 Réponds UNIQUEMENT avec le titre traduit, sans guillemets, \
                                 sans explication, sans ponctuation finale.\nTitre: {}",
                                raw_title
                            );
                            let ai_res = client.post(&colab_url)
                                .json(&serde_json::json!({
                                    "prompt": prompt_text,
                                    "history": [],
                                    "channel": *target_channel,
                                    "system": "Tu es un traducteur expert. Traduis toujours en français. Réponds uniquement avec le titre traduit, sans guillemets ni commentaire."
                                }))
                                .send()
                                .await;
                            if let Ok(r) = ai_res {
                                if let Ok(j) = r.json::<serde_json::Value>().await {
                                    j["response"].as_str().unwrap_or(&title).trim().to_string()
                                } else { title.clone() }
                            } else { title.clone() }
                        };

                        let content = format!("{}\x00{}", titre_fr, link);

                        let news_msg = shared::Message {
                            id: None,
                            channel_id: target_channel.to_string(),
                            author_name: "RustChat News".to_string(),
                            content: content.clone(),
                            created_at: chrono::Utc::now().timestamp(),
                            author_role: shared::Role::Ai,
                            author_gender: "bot".to_string(),
                            deleted: false,
                            direct_to: None,
                            reactions: Vec::new(),
                            status: shared::MessageStatus::Sent,
                            pinned: false,
                        };

                        let _ = news_state.db
                            .collection::<shared::Message>("messages")
                            .insert_one(news_msg, None).await;

                        if let Ok(j) = serde_json::to_string(&shared::WsServerMsg::Chat {
                            author: "RustChat News".to_string(),
                            content,
                            role: shared::Role::Ai,
                            gender: "bot".to_string(),
                        }) {
                            if let Some(tx) = news_state.channels_tx.get(*target_channel) {
                                let _ = tx.send(j);
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(1800)).await;
            }
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Backend lancé sur http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("Impossible de binder le port 3000");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("Crash du serveur");
}

async fn health_check() -> &'static str { "OK" }

async fn delete_message_handler(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    use mongodb::bson::oid::ObjectId;
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if !role_from_str(&claims.role).is_at_least_admin() {
        return (StatusCode::FORBIDDEN, "Interdit").into_response();
    }
    let oid = match ObjectId::parse_str(&id) {
        Ok(i) => i,
        Err(_) => return (StatusCode::BAD_REQUEST, "ID invalide").into_response(),
    };
    let _ = state.db.collection::<shared::Message>("messages").update_one(
        doc! { "_id": oid },
        doc! { "$set": { "deleted": true } },
        None
    ).await;
    info!("ADMIN {} a supprimé le message {}", claims.username, id);
    (StatusCode::OK, "Message supprimé").into_response()
}

async fn pin_message_handler(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    use mongodb::bson::oid::ObjectId;
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if !role_from_str(&claims.role).is_at_least_admin() {
        return (StatusCode::FORBIDDEN, "Interdit").into_response();
    }
    let oid = match ObjectId::parse_str(&id) {
        Ok(i) => i,
        Err(_) => return (StatusCode::BAD_REQUEST, "ID invalide").into_response(),
    };
    let msg = match state.db.collection::<shared::Message>("messages").find_one(doc! { "_id": oid }, None).await {
        Ok(Some(m)) => m,
        _ => return (StatusCode::NOT_FOUND, "Message introuvable").into_response(),
    };
    let new_pinned = !msg.pinned;
    let _ = state.db.collection::<shared::Message>("messages").update_one(
        doc! { "_id": oid },
        doc! { "$set": { "pinned": new_pinned } },
        None
    ).await;
    (StatusCode::OK, if new_pinned { "Épinglé" } else { "Désépinglé" }).into_response()
}

async fn archive_channel_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;

    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let is_super = role_from_str(&claims.role) == Role::SuperAdmin
        && (state.superadmin_username.is_empty() || claims.username == state.superadmin_username);
    if !is_super {
        return (StatusCode::FORBIDDEN, "SuperAdmin requis").into_response();
    }

    let seven_days_ago = Utc::now().timestamp() - 7 * 24 * 3600;
    let filter = doc! {
        "channel_id": &id,
        "created_at": { "$lt": seven_days_ago },
        "$or": [{ "direct_to": { "$exists": false } }, { "direct_to": null }],
        "deleted": { "$ne": true }
    };

    let count = state.db.collection::<shared::Message>("messages")
        .count_documents(filter.clone(), None).await.unwrap_or(0);

    if count < 20 {
        return (StatusCode::OK, Json(serde_json::json!({ "archive": [] }))).into_response();
    }

    let opts = FindOptions::builder()
        .sort(doc! { "created_at": 1 })
        .limit(50)
        .build();

    let messages = match state.db.collection::<shared::Message>("messages")
        .find(filter, opts).await
    {
        Ok(cursor) => cursor.try_collect::<Vec<shared::Message>>().await.unwrap_or_default(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur DB").into_response(),
    };

    let oldest = messages.first().map(|m| m.created_at).unwrap_or(0);
    let messages_text: String = messages.iter()
        .map(|m| format!("{}: {}", m.author_name, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let colab_url = std::env::var("COLAB_AI_URL")
        .unwrap_or_else(|_| "http://localhost:5000/generate".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let res = client.post(&colab_url)
        .json(&serde_json::json!({
            "prompt": format!("Fais une synthèse concise en français des discussions suivantes en 3-5 points clés. Sois bref et factuel.\nMessages:\n{}", messages_text),
            "history": [],
            "channel": id,
            "system": "Tu résumes des conversations de forum en français. Sois concis et factuel."
        }))
        .send()
        .await;

    match res {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => {
                let summary = json["response"].as_str()
                    .unwrap_or("Impossible de générer la synthèse.")
                    .to_string();
                (StatusCode::OK, Json(serde_json::json!({
                    "summary": summary,
                    "count": count,
                    "oldest": oldest
                }))).into_response()
            }
            Err(_) => (StatusCode::BAD_GATEWAY, "Réponse IA invalide").into_response(),
        },
        Err(_) => (StatusCode::BAD_GATEWAY, "IA injoignable").into_response(),
    }
}

// --- HANDLERS ---

async fn channels_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if decode_token(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, "Token invalide").into_response();
    }
    use futures_util::stream::TryStreamExt;
    let collection = state.db.collection::<Channel>("channels");
    let filter = doc! { "$or": [ { "deleted": { "$exists": false } }, { "deleted": false } ] };
    match collection.find(filter, None).await {
        Ok(cursor) => match cursor.try_collect::<Vec<Channel>>().await {
            Ok(channels) => (StatusCode::OK, Json(channels)).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur désérialisation").into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur récupération salons").into_response(),
    }
}

async fn messages_handler(State(state): State<AppState>, headers: HeaderMap, Query(params): Query<MessagesQuery>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if !is_valid_channel_id(&params.channel) {
        return (StatusCode::BAD_REQUEST, "ID de salon invalide").into_response();
    }
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;
    use mongodb::bson::oid::ObjectId;

    // FIX MOY-06 : vérifier l'accès aux salons verrouillés/dorés
    let channels_col = state.db.collection::<Channel>("channels");
    if let Ok(Some(channel_doc)) = channels_col.find_one(doc! { "id": &params.channel }, None).await {
        if channel_doc.is_locked || channel_doc.is_gold {
            let is_admin = role_from_str(&claims.role).is_at_least_admin();
            if !is_admin {
                // Vérifier si l'utilisateur est abonné au salon
                let users_col = state.db.collection::<User>("users");
                let subscribed = match users_col.find_one(doc! { "username": &claims.username }, None).await {
                    Ok(Some(user)) => user.subscribed_channels.contains(&params.channel),
                    _ => false,
                };
                if !subscribed {
                    return (StatusCode::FORBIDDEN, "Accès refusé").into_response();
                }
            }
        }
    }

    let collection = state.db.collection::<shared::Message>("messages");
    let mut filter = doc! {
        "channel_id": &params.channel,
        "$and": [
            { "$or": [{ "direct_to": { "$exists": false } }, { "direct_to": null }] },
            { "$or": [{ "deleted": { "$exists": false } }, { "deleted": false }] }
        ]
    };

    // Pagination cursor : messages avant l'ID fourni
    if let Some(ref bid) = params.before_id {
        if let Ok(oid) = ObjectId::parse_str(bid) {
            filter.insert("_id", doc! { "$lt": oid });
        } else {
            return (StatusCode::BAD_REQUEST, "before_id invalide").into_response();
        }
    }

    // Récupérer les 100 derniers (tri desc), puis inverser pour ordre chronologique
    let options = FindOptions::builder()
        .sort(doc! { "_id": -1 })
        .limit(100)
        .build();

    match collection.find(filter, options).await {
        Ok(cursor) => match cursor.try_collect::<Vec<shared::Message>>().await {
            Ok(mut messages) => {
                messages.reverse();
                (StatusCode::OK, Json(messages)).into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
    }
}

async fn direct_messages_handler(State(state): State<AppState>, headers: HeaderMap, Query(params): Query<DirectMessagesQuery>) -> impl IntoResponse {
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;
    use mongodb::bson::oid::ObjectId;
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let collection = state.db.collection::<shared::Message>("messages");
    let mut base_filter = doc! {
        "$and": [
            { "$or": [
                { "author_name": &claims.username, "direct_to": &params.with },
                { "author_name": &params.with, "direct_to": &claims.username }
            ]},
            { "$or": [ { "deleted": { "$exists": false } }, { "deleted": false } ] }
        ]
    };
    // FIX MOY-10 : pagination cursor — messages avant l'ID fourni
    if let Some(ref bid) = params.before_id {
        match ObjectId::parse_str(bid) {
            Ok(oid) => { base_filter.insert("_id", doc! { "$lt": oid }); }
            Err(_) => return (StatusCode::BAD_REQUEST, "before_id invalide").into_response(),
        }
    }
    let options = FindOptions::builder()
        .sort(doc! { "_id": -1 })
        .limit(50)
        .build();
    match collection.find(base_filter, options).await {
        Ok(cursor) => match cursor.try_collect::<Vec<shared::Message>>().await {
            Ok(mut messages) => {
                messages.reverse();
                let mut response = (StatusCode::OK, Json(messages)).into_response();
                response.headers_mut().insert(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-store"),
                );
                response
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
    }
}

async fn mark_direct_message_read_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<MarkReadRequest>) -> impl IntoResponse {
    use mongodb::bson::oid::ObjectId;
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let oid = match ObjectId::parse_str(&req.message_id) {
        Ok(i) => i,
        Err(_) => return (StatusCode::BAD_REQUEST, "ID invalide").into_response(),
    };

    let msg = match state.db.collection::<shared::Message>("messages").find_one(doc! { "_id": oid }, None).await {
        Ok(Some(m)) => m,
        _ => return (StatusCode::NOT_FOUND, "Message introuvable").into_response(),
    };

    if msg.direct_to.as_ref() != Some(&claims.username) {
        return (StatusCode::FORBIDDEN, "Pas le destinataire").into_response();
    }

    let _ = state.db.collection::<shared::Message>("messages").update_one(
        doc! { "_id": oid },
        doc! { "$set": { "status": "read" } },
        None
    ).await;

    if let Ok(j) = serde_json::to_string(&WsServerMsg::MessageStatusUpdated { message_id: req.message_id, status: MessageStatus::Read }) {
        if let Some(author_tx) = state.user_tx.get(&msg.author_name) {
            let _ = author_tx.send(j);
        }
    }

    (StatusCode::OK, "Lu").into_response()
}

async fn me_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let collection = state.db.collection::<User>("users");
    
    // --- FIX: Récupération des contacts DM ---
    let messages_col = state.db.collection::<shared::Message>("messages");
    // 1. Gens à qui j'ai écrit
    let mut contacts = std::collections::HashSet::new();
    if let Ok(sent) = messages_col.distinct("direct_to", doc! {"author_name": &claims.username, "direct_to": { "$exists": true }}, None).await {
        for c in sent { if let Some(s) = c.as_str() { contacts.insert(s.to_string()); } }
    }
    // 2. Gens qui m'ont écrit
    if let Ok(received) = messages_col.distinct("author_name", doc! {"direct_to": &claims.username}, None).await {
        for c in received { if let Some(s) = c.as_str() { contacts.insert(s.to_string()); } }
    }
    let contact_list: Vec<String> = contacts.into_iter().collect();
    // ----------------------------------------

    match collection.find_one(doc! { "username": &claims.username }, None).await {
        Ok(Some(user)) => {
            let info = serde_json::json!({
                "username": user.username,
                "email": user.email,
                "role": user.role.to_string(),
                "gender": user.gender,
                "subscribed_channels": user.subscribed_channels,
                "direct_contacts": contact_list,
                "is_premium": user.is_premium,
            });
            let mut response = (StatusCode::OK, Json(info)).into_response();
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"),
            );
            response
        }
        _ => (StatusCode::NOT_FOUND, "Introuvable").into_response(),
    }
}

async fn users_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if decode_token(&state, &headers).is_err() {
        return (StatusCode::UNAUTHORIZED, "Token invalide").into_response();
    }
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;
    let collection = state.db.collection::<User>("users");
    let opts = FindOptions::builder().limit(500).build();
    match collection.find(None, opts).await {
        Ok(cursor) => match cursor.try_collect::<Vec<User>>().await {
            Ok(users) => {
                let members: Vec<MemberInfo> = users.iter()
                    .filter(|u| !u.banned)
                    .map(|u| MemberInfo {
                        username: u.username.clone(),
                        role: u.role.clone(),
                        online: state.online_users.contains_key(&u.username),
                        gender: u.gender.clone(),
                        avatar: u.avatar.clone(),
                        badges: calculate_badges(u),
                    })
                    .collect();
                (StatusCode::OK, Json(members)).into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
        },
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
    }
}

async fn update_avatar_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<UpdateAvatarRequest>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if req.avatar.len() > 500000 { return (StatusCode::BAD_REQUEST, "Trop gros").into_response(); }
    let _ = state.db.collection::<User>("users").update_one(doc! { "username": &claims.username }, doc! { "$set": { "avatar": &req.avatar } }, None).await;
    (StatusCode::OK, "Avatar maj").into_response()
}

async fn delete_account_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<DeleteAccountRequest>,
) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let col = state.db.collection::<User>("users");
    let user = match col.find_one(doc! { "username": &claims.username }, None).await.unwrap_or(None) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Utilisateur introuvable").into_response(),
    };
    let pwd_db = match user.password.as_ref() {
        Some(p) => p,
        None => return (StatusCode::FORBIDDEN, "Erreur").into_response(),
    };
    let argon_ok = if let Ok(parsed) = PasswordHash::new(pwd_db) {
        Argon2::default().verify_password(req.password.as_bytes(), &parsed).is_ok()
    } else { false };
    let bcrypt_ok = if !argon_ok { bcrypt_verify(&req.password, pwd_db).unwrap_or(false) } else { false };
    if !argon_ok && !bcrypt_ok {
        return (StatusCode::FORBIDDEN, "Mot de passe incorrect").into_response();
    }
    let _ = col.delete_one(doc! { "username": &claims.username }, None).await;
    info!("Compte supprimé : {}", claims.username);
    // Blacklist the JWT so it can't be reused
    let token_str = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();
    if !token_str.is_empty() {
        state.jwt_blacklist.insert(token_str, claims.exp as i64);
    }
    (StatusCode::OK, "Compte supprimé").into_response()
}

async fn react_to_message_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<ReactionRequest>) -> impl IntoResponse {
    use mongodb::bson::oid::ObjectId;
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let oid = match ObjectId::parse_str(&req.message_id) {
        Ok(i) => i,
        Err(_) => return (StatusCode::BAD_REQUEST, "ID invalide").into_response(),
    };
    let _ = state.db.collection::<shared::Message>("messages").update_one(
        doc! { "_id": oid },
        doc! { "$addToSet": { format!("reactions.$[elem].users"): &claims.username } },
        None
    ).await;
    (StatusCode::OK, "Réaction ajoutée").into_response()
}

async fn promote_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<PromoteRequest>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let caller_role = role_from_str(&claims.role);
    if !caller_role.is_at_least_admin() { return (StatusCode::FORBIDDEN, "Interdit").into_response(); }
    // Whitelist stricte : seuls "user" et "admin" sont des valeurs acceptables
    if req.role != "user" && req.role != "admin" {
        return (StatusCode::BAD_REQUEST, "Rôle invalide").into_response();
    }
    // Seul le SuperAdmin désigné (env var) peut accorder ou révoquer le rôle super_admin
    if req.role == "super_admin" || role_from_str(&req.role) == Role::SuperAdmin {
        if state.superadmin_username.is_empty() || claims.username != state.superadmin_username {
            return (StatusCode::FORBIDDEN, "Seul le SuperAdmin peut attribuer ce rôle").into_response();
        }
    }
    let _ = state.db.collection::<User>("users").update_one(doc! { "username": &req.username }, doc! { "$set": { "role": &req.role } }, None).await;
    info!("ADMIN {} a changé le rôle de {} → {}", claims.username, req.username, req.role);
    (StatusCode::OK, "Promu").into_response()
}

async fn ban_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<BanRequest>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let caller_role = role_from_str(&claims.role);
    if !caller_role.is_at_least_admin() { return (StatusCode::FORBIDDEN, "Interdit").into_response(); }
    // Fetch target user and check role hierarchy
    let user_col = state.db.collection::<User>("users");
    let target_user = match user_col.find_one(doc! { "username": &req.username }, None).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::NOT_FOUND, "Utilisateur introuvable").into_response(),
    };
    let target_role = role_from_str(&target_user.role.to_string());
    if !caller_role.can_manage(&target_role) {
        return (StatusCode::FORBIDDEN, "Hiérarchie insuffisante").into_response();
    }
    let _ = user_col.update_one(doc! { "username": &req.username }, doc! { "$set": { "banned": true } }, None).await;
    state.online_users.remove(&req.username);
    info!("ADMIN {} a banni {}", claims.username, req.username);
    (StatusCode::OK, "Banni").into_response()
}

async fn create_channel_handler(State(state): State<AppState>, headers: HeaderMap, Json(new_channel): Json<Channel>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token manquant ou invalide").into_response(),
    };
    let caller_role = role_from_str(&claims.role);
    if !caller_role.is_at_least_admin() { return (StatusCode::FORBIDDEN, "Interdit").into_response(); }
    let _ = state.db.collection::<Channel>("channels").insert_one(new_channel, None).await;
    (StatusCode::CREATED, "Salon créé").into_response()
}

async fn discover_channels_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;
    let _ = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let cursor = match state.db.collection::<Channel>("channels").find(doc! { "$or": [ { "deleted": false }, { "deleted": { "$exists": false } } ] }, None).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur DB").into_response(),
    };
    let channels = cursor.try_collect::<Vec<Channel>>().await.unwrap_or_default();
    let mut stats = Vec::new();
    for c in channels {
        let cid = c.id.clone().unwrap_or_default();
        let count = state.db.collection::<shared::Message>("messages").count_documents(doc! { "channel_id": &cid }, None).await.unwrap_or(0);
        let snippet = {
            let opts = FindOptions::builder().sort(doc! { "created_at": -1 }).limit(1).build();
            if let Ok(mut cursor) = state.db.collection::<shared::Message>("messages").find(doc! { "channel_id": &cid }, opts).await {
                if let Ok(Some(msg)) = cursor.try_next().await {
                    let preview = if msg.content.len() > 60 {
                        format!("{}...", &msg.content[..60])
                    } else {
                        msg.content.clone()
                    };
                    Some(preview)
                } else {
                    None
                }
            } else {
                None
            }
        };
        let media_tag = if let Some(ref media) = c.topic_media {
            let m_lower = media.to_lowercase();
            if m_lower.contains("youtube.com") || m_lower.contains("youtu.be") {
                Some("🎬 Vidéo épinglée".to_string())
            } else if m_lower.contains(".jpg") || m_lower.contains(".png") || m_lower.contains(".jpeg") || m_lower.contains(".gif") || m_lower.contains(".webp") {
                Some("🖼️ Photo épinglée".to_string())
            } else {
                Some("💬 Libre sujet".to_string())
            }
        } else {
            Some("💬 Libre sujet".to_string())
        };
        stats.push(ChannelWithStats { channel: c, message_count: count as i64, last_message_snippet: snippet, media_tag });
    }
    stats.sort_by(|a, b| b.message_count.cmp(&a.message_count));
    (StatusCode::OK, Json(stats)).into_response()
}

async fn subscribe_channel_handler(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<SubscribeRequest>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if !is_valid_channel_id(&req.channel_id) {
        return (StatusCode::BAD_REQUEST, "ID de salon invalide").into_response();
    }
    let update = if req.subscribe { doc! { "$addToSet": { "subscribed_channels": &req.channel_id } } } else { doc! { "$pull": { "subscribed_channels": &req.channel_id } } };
    let _ = state.db.collection::<User>("users").update_one(doc! { "username": &claims.username }, update, None).await;
    (StatusCode::OK, "Succès").into_response()
}

async fn delete_channel_handler(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    if !role_from_str(&claims.role).is_at_least_admin() { return (StatusCode::FORBIDDEN, "Interdit").into_response(); }
    let _ = state.db.collection::<Channel>("channels").update_one(doc! { "id": id }, doc! { "$set": { "deleted": true } }, None).await;
    (StatusCode::OK, "Supprimé").into_response()
}

async fn summarize_channel_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    use futures_util::stream::TryStreamExt;
    use mongodb::options::FindOptions;

    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };
    let is_super = role_from_str(&claims.role) == Role::SuperAdmin
        && (state.superadmin_username.is_empty() || claims.username == state.superadmin_username);
    if !is_super {
        return (StatusCode::FORBIDDEN, "SuperAdmin requis").into_response();
    }

    let opts = FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .limit(50)
        .build();
    let filter = doc! {
        "channel_id": &id,
        "$or": [{ "direct_to": { "$exists": false } }, { "direct_to": null }],
        "deleted": { "$ne": true }
    };

    let messages = match state.db.collection::<shared::Message>("messages")
        .find(filter, opts).await
    {
        Ok(cursor) => cursor.try_collect::<Vec<shared::Message>>().await.unwrap_or_default(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur DB").into_response(),
    };

    if messages.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({ "summary": "Aucun message dans ce canal." }))).into_response();
    }

    let mut msgs_ordered = messages;
    msgs_ordered.reverse();
    let context: String = msgs_ordered.iter()
        .map(|m| format!("{}: {}", m.author_name, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let colab_url = std::env::var("COLAB_AI_URL")
        .unwrap_or_else(|_| "http://localhost:5000/generate".to_string());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let res = client.post(&colab_url)
        .json(&serde_json::json!({
            "prompt": format!("Voici les derniers messages du canal #{} :\n\n{}\n\nFais un résumé structuré du débat en 5 points maximum.", id, context),
            "history": [],
            "channel": id,
            "system": "Tu résumes des conversations de forum en français. Sois structuré, concis. Utilise des bullet points. Maximum 5 points."
        }))
        .send()
        .await;

    match res {
        Ok(response) => match response.json::<serde_json::Value>().await {
            Ok(json) => {
                let summary = json["response"].as_str()
                    .unwrap_or("Impossible de générer le résumé.")
                    .to_string();
                (StatusCode::OK, Json(serde_json::json!({ "summary": summary }))).into_response()
            }
            Err(_) => (StatusCode::BAD_GATEWAY, "Réponse IA invalide").into_response(),
        },
        Err(_) => (StatusCode::BAD_GATEWAY, "IA injoignable").into_response(),
    }
}

// --- WEBSOCKET HANDLERS ---

/// Le token JWT ne passe plus par l'URL — il arrive dans le premier message WS.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn broadcast_presence(state: &AppState) {
    let online: Vec<String> = state.online_users.iter().map(|r| r.key().clone()).collect();
    if let Ok(json) = serde_json::to_string(&WsServerMsg::Presence { online }) {
        let _ = state.presence_tx.send(json);
    }
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();

    // === PHASE D'AUTHENTIFICATION (délai max 5 secondes) ===
    // Le client doit envoyer {"type":"auth","token":"<JWT>","channel":"<id>"} en premier.
    let auth_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            loop {
                match stream.next().await {
                    Some(Ok(WsMsg::Text(text))) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if val.get("type").and_then(|t| t.as_str()) == Some("auth") {
                                let token = val["token"].as_str().unwrap_or("").to_string();
                                let channel = val["channel"].as_str().unwrap_or("general").to_string();
                                return Some((token, channel));
                            }
                        }
                        // Message non-auth avant authentification → ignoré
                    }
                    Some(Ok(_)) => {} // Ping/Pong/Binary → ignoré
                    Some(Err(_)) | None => return None,
                }
            }
        },
    )
    .await;

    let (token_str, channel_id) = match auth_result {
        Ok(Some(pair)) => pair,
        Ok(None) | Err(_) => {
            // Timeout ou fermeture sans auth valide
            let _ = sink.send(WsMsg::Close(None)).await;
            return;
        }
    };

    if !is_valid_channel_id(&channel_id) {
        let _ = sink.send(WsMsg::Close(None)).await;
        return;
    }

    let claims = match decode::<Claims>(
        &token_str,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(d) => d.claims,
        Err(_) => {
            let _ = sink.send(WsMsg::Text(r#"{"error":"Token invalide"}"#.to_string())).await;
            let _ = sink.send(WsMsg::Close(None)).await;
            return;
        }
    };

    let username = claims.username;
    let role_str = claims.role;
    let gender = claims.gender;

    // === CONNEXION ÉTABLIE ===
    state.online_users.insert(username.clone(), ());
    broadcast_presence(&state).await;

    let tx = state.channels_tx.entry(channel_id.clone()).or_insert_with(|| broadcast::channel(100).0).clone();
    let u_tx = state.user_tx.entry(username.clone()).or_insert_with(|| broadcast::channel(100).0).clone();

    let mut c_rx = tx.subscribe();
    let mut u_rx = u_tx.subscribe();
    let mut p_rx = state.presence_tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(m) = c_rx.recv() => { if sink.send(WsMsg::Text(m)).await.is_err() { break; } }
                Ok(m) = u_rx.recv() => { if sink.send(WsMsg::Text(m)).await.is_err() { break; } }
                Ok(m) = p_rx.recv() => { if sink.send(WsMsg::Text(m)).await.is_err() { break; } }
            }
        }
    });

    let state_c = state.clone();
    let username_c = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(WsMsg::Text(text))) = stream.next().await {
            if text.len() > 16_384 {
                warn!("Message WS surdimensionné ({} octets) de {}, ignoré", text.len(), username_c);
                continue;
            }
            let role_enum = role_from_str(&role_str);
            if let Ok(client_msg) = serde_json::from_str::<WsClientMsg>(&text) {
                match client_msg {
                    WsClientMsg::Chat { content } => {
                        let content = sanitize_message(&content);
                        if !check_rate_limit(&state_c, &username_c).await || contains_suspicious_url(&content) { continue; }

                        let new_msg = shared::Message {
                            id: None,
                            channel_id: channel_id.clone(),
                            author_name: username_c.clone(),
                            content: content.clone(),
                            created_at: Utc::now().timestamp(),
                            author_role: role_enum.clone(),
                            author_gender: gender.clone(),
                            deleted: false,
                            direct_to: None,
                            reactions: Vec::new(),
                            status: MessageStatus::Sent,
                            pinned: false,
                        };
                        let insert_result = state_c.db.collection::<shared::Message>("messages").insert_one(new_msg, None).await;
                        let inserted_oid = insert_result.ok().and_then(|r| r.inserted_id.as_object_id());
                        let _ = state_c.db.collection::<User>("users").update_one(
                            doc! { "username": &username_c },
                            doc! { "$inc": { "message_count": 1 } },
                            None
                        ).await;

                        if let Ok(j) = serde_json::to_string(&WsServerMsg::Chat {
                            author: username_c.clone(), content: content.clone(),
                            role: role_enum, gender: gender.clone(),
                        }) {
                            let _ = tx.send(j);
                        }

                        // Modération automatique (uniquement si mots suspects)
                        {
                            let bad_words = ["connard", "pute", "merde", "fdp", "ntm", "nique"];
                            let lower = content.to_lowercase();
                            let flagged = content.len() > 50 && bad_words.iter().any(|w| lower.contains(w));
                            if flagged {
                                let mod_db = state_c.db.clone();
                                let mod_channel = channel_id.clone();
                                let mod_author = username_c.clone();
                                let mod_colab = std::env::var("COLAB_AI_URL")
                                    .unwrap_or_else(|_| "http://localhost:5000/generate".to_string());
                                let mod_presence = state_c.presence_tx.clone();
                                let mod_content = content.clone();
                                tokio::spawn(async move {
                                    let client = reqwest::Client::builder()
                                        .timeout(std::time::Duration::from_secs(15))
                                        .build()
                                        .unwrap_or_default();
                                    let res = client.post(&mod_colab)
                                        .json(&serde_json::json!({
                                            "prompt": format!("Ce message de forum est-il toxique ou inapproprié ? Réponds UNIQUEMENT par 'OUI' ou 'NON'. Message: '{}'", mod_content),
                                            "history": [],
                                            "system": "Tu es un modérateur. Réponds uniquement OUI ou NON."
                                        }))
                                        .send().await;
                                    if let Ok(response) = res {
                                        if let Ok(json) = response.json::<serde_json::Value>().await {
                                            let ai_text = json["response"].as_str().unwrap_or("").trim().to_uppercase();
                                            if ai_text.contains("OUI") {
                                                if let Some(oid) = inserted_oid {
                                                    let _ = mod_db.collection::<shared::Message>("messages").update_one(
                                                        doc! { "_id": oid },
                                                        doc! { "$set": { "deleted": true } },
                                                        None
                                                    ).await;
                                                }
                                                info!("MODÉRATION IA: message supprimé dans #{} (auteur: {})", mod_channel, mod_author);
                                                let _ = mod_presence.send(format!(
                                                    "{{\"type\":\"moderation\",\"content\":\"Message supprimé automatiquement dans #{}\"}}",
                                                    mod_channel
                                                ));
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    WsClientMsg::DirectMessage { to, content } => {
                        let content = sanitize_message(&content);
                        if !check_rate_limit(&state_c, &username_c).await { continue; }
                        let new_msg = shared::Message {
                            id: None,
                            channel_id: "direct".to_string(),
                            author_name: username_c.clone(),
                            content: content.clone(),
                            created_at: Utc::now().timestamp(),
                            author_role: role_enum.clone(),
                            author_gender: gender.clone(),
                            deleted: false,
                            direct_to: Some(to.clone()),
                            reactions: Vec::new(),
                            status: MessageStatus::Sent,
                            pinned: false,
                        };
                        let insert_result = state_c.db.collection::<shared::Message>("messages").insert_one(new_msg, None).await;
                        let inserted_id = insert_result.as_ref().ok()
                            .and_then(|r| r.inserted_id.as_object_id())
                            .map(|id| id.to_hex());
                        let insert_ok = insert_result.is_ok();

                        if let Ok(ack) = serde_json::to_string(&WsServerMsg::Ack { ok: insert_ok, message_id: inserted_id.clone() }) {
                            let _ = u_tx.send(ack);
                        }
                        if insert_ok {
                            if let Ok(j) = serde_json::to_string(&WsServerMsg::DirectMessage {
                                from: username_c.clone(), content,
                                role: role_enum, gender: gender.clone(),
                                id: inserted_id.unwrap_or_default(),
                            }) {
                                if let Some(t_tx) = state_c.user_tx.get(&to) {
                                    let _ = t_tx.send(j);
                                }
                            }
                        }
                    }
                    WsClientMsg::MarkAsRead { message_id } => {
                        use mongodb::bson::oid::ObjectId;
                        if let Ok(oid) = ObjectId::parse_str(&message_id) {
                            let msg_col = state_c.db.collection::<shared::Message>("messages");
                            if let Ok(Some(msg)) = msg_col.find_one(doc! { "_id": &oid }, None).await {
                                if msg.direct_to.as_deref() == Some(username_c.as_str()) {
                                    let _ = msg_col.update_one(
                                        doc! { "_id": oid },
                                        doc! { "$set": { "status": "read" } },
                                        None
                                    ).await;
                                    if let Ok(j) = serde_json::to_string(&WsServerMsg::MessageStatusUpdated {
                                        message_id, status: MessageStatus::Read,
                                    }) {
                                        let _ = u_tx.send(j);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
    state.online_users.remove(&username);
    broadcast_presence(&state).await;
}

async fn register_handler(State(state): State<AppState>, headers: HeaderMap, Json(mut user): Json<User>) -> impl IntoResponse {
    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',').next().unwrap_or("unknown")
        .trim()
        .to_string();
    let ip_key = format!("register_{}", ip);
    if !check_ip_rate_limit(&state.rate_limiter, &ip_key, 5, 3600) {
        return (StatusCode::TOO_MANY_REQUESTS, "Trop de tentatives").into_response();
    }
    // Validation username: 3-20 chars, alphanumeric + underscore
    if !USERNAME_RE.is_match(&user.username) {
        return (StatusCode::BAD_REQUEST, "Username invalide : 3-20 caractères, lettres/chiffres/underscore uniquement").into_response();
    }
    // Validation email
    if !EMAIL_RE.is_match(&user.email) {
        return (StatusCode::BAD_REQUEST, "Email invalide").into_response();
    }
    // Validation password: min 8 chars
    let pwd_raw = match &user.password {
        Some(p) if p.len() >= 8 => p.clone(),
        Some(_) => return (StatusCode::BAD_REQUEST, "Mot de passe trop court (minimum 8 caractères)").into_response(),
        None => return (StatusCode::BAD_REQUEST, "Mot de passe manquant").into_response(),
    };
    let col = state.db.collection::<User>("users");
    // Unicité username
    if col.find_one(doc! { "username": &user.username }, None).await.unwrap_or(None).is_some() {
        return (StatusCode::CONFLICT, "Ce nom d'utilisateur est déjà pris").into_response();
    }
    // Unicité email
    if col.find_one(doc! { "email": &user.email }, None).await.unwrap_or(None).is_some() {
        return (StatusCode::CONFLICT, "Cet email est déjà utilisé").into_response();
    }
    let salt = SaltString::generate(&mut OsRng);
    let pwd = match Argon2::default().hash_password(pwd_raw.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur hash").into_response(),
    };
    user.password = Some(pwd);
    user.role = Role::User;
    user.created_at = Utc::now().timestamp();
    let _ = col.insert_one(user, None).await;
    (StatusCode::CREATED, "OK").into_response()
}

async fn login_handler(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(login_data): Json<User>,
) -> impl IntoResponse {
    let col = state.db.collection::<User>("users");
    let pwd_input = match login_data.password {
        Some(p) => p,
        None => return (StatusCode::UNAUTHORIZED, "Invalide").into_response(),
    };
    let now = chrono::Utc::now().timestamp();
    // Rate limit par IP (20 tentatives / minute)
    let ip_key = format!("login_ip_{}", addr.ip());
    {
        let mut blocked = false;
        state.rate_limiter
            .entry(ip_key.clone())
            .and_modify(|times| {
                times.retain(|&t| now - t < 60);
                if times.len() >= 20 {
                    blocked = true;
                } else {
                    times.push(now);
                }
            })
            .or_insert_with(|| vec![now]);
        if blocked {
            warn!("Rate limit IP login dépassé: {}", addr.ip());
            return (StatusCode::TOO_MANY_REQUESTS, "Trop de tentatives, réessayez dans 1 minute").into_response();
        }
    }
    // Rate limit par email (5 tentatives / minute)
    let rate_key = format!("login_{}", login_data.email);
    {
        let mut blocked = false;
        state.rate_limiter
            .entry(rate_key.clone())
            .and_modify(|times| {
                times.retain(|&t| now - t < 60);
                if times.len() >= 5 {
                    blocked = true;
                } else {
                    times.push(now);
                }
            })
            .or_insert_with(|| vec![now]);
        if blocked {
            return (StatusCode::TOO_MANY_REQUESTS, "Trop de tentatives, réessayez dans 1 minute").into_response();
        }
    }

    let user = match col.find_one(doc! { "email": &login_data.email }, None).await.unwrap_or(None) {
        Some(u) if u.banned => {
            return (StatusCode::FORBIDDEN, "Compte banni").into_response();
        }
        Some(u) => {
            let pwd_db = match u.password.as_ref() {
                Some(p) => p,
                None => return (StatusCode::UNAUTHORIZED, "Invalide").into_response(),
            };
            let argon_ok = if let Ok(parsed) = PasswordHash::new(pwd_db) {
                Argon2::default().verify_password(pwd_input.as_bytes(), &parsed).is_ok()
            } else {
                false
            };
            let bcrypt_ok = if !argon_ok {
                bcrypt_verify(&pwd_input, pwd_db).unwrap_or(false)
            } else {
                false
            };
            if argon_ok || bcrypt_ok {
                u
            } else {
                return (StatusCode::UNAUTHORIZED, "Invalide").into_response();
            }
        }
        None => return (StatusCode::UNAUTHORIZED, "Invalide").into_response(),
    };

    // Reset login rate limits on success
    state.rate_limiter.remove(&rate_key);
    state.rate_limiter.remove(&ip_key);
    let exp = match Utc::now().checked_add_signed(Duration::hours(24)) {
        Some(t) => t.timestamp() as usize,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
    };
    let iat = Utc::now().timestamp() as usize;
    let token = match encode(&Header::default(), &Claims { sub: user.email.clone(), username: user.username.clone(), role: user.role.to_string(), gender: user.gender.clone(), exp, iat }, &EncodingKey::from_secret(state.jwt_secret.as_bytes())) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur").into_response(),
    };
    let response = LoginResponse { token, username: user.username, role: user.role.to_string() };
    info!("Connexion: {}", response.username);
    (StatusCode::OK, Json(response)).into_response()
}

// --- MOT DE PASSE ---

async fn forgot_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ForgotPasswordRequest>,
) -> impl IntoResponse {
    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',').next().unwrap_or("unknown")
        .trim()
        .to_string();
    let ip_key = format!("forgot_{}", ip);
    if !check_ip_rate_limit(&state.rate_limiter, &ip_key, 3, 3600) {
        return (StatusCode::TOO_MANY_REQUESTS, "Trop de tentatives").into_response();
    }

    let email = req.email.trim().to_lowercase();
    // Réponse identique qu'il existe ou non (anti-énumération)
    let ok = (StatusCode::OK, "Si cet email est enregistré, un lien vous a été envoyé");

    if !EMAIL_RE.is_match(&email) {
        return ok.into_response();
    }

    let col = state.db.collection::<User>("users");
    let user = match col.find_one(doc! { "email": &email }, None).await.unwrap_or(None) {
        Some(u) => u,
        None => return ok.into_response(),
    };

    // Invalider les anciens tokens non utilisés pour cet email
    let reset_col = state.db.collection::<ResetToken>("reset_tokens");
    let _ = reset_col.update_many(
        doc! { "email": &email, "used": false },
        doc! { "$set": { "used": true } },
        None,
    ).await;

    let token = Uuid::new_v4().to_string();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now().timestamp() + 15 * 60;

    if let Err(e) = reset_col.insert_one(ResetToken {
        id: None, token: token_hash, email: email.clone(), expires_at, used: false,
    }, None).await {
        error!("Erreur insertion reset_token: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur serveur").into_response();
    }

    let base = std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let link = format!("{}/reset-password?token={}", base, token);

    let html = format!(r#"<!DOCTYPE html>
<html lang="fr"><body style="margin:0;background:#111827;font-family:Arial,sans-serif;">
<div style="max-width:520px;margin:40px auto;background:#1f2937;border-radius:12px;padding:36px;border:1px solid #374151;">
  <h2 style="color:#60a5fa;margin-top:0;">🔑 Réinitialisation de mot de passe</h2>
  <p style="color:#d1d5db;">Bonjour <strong style="color:#f9fafb;">{}</strong>,</p>
  <p style="color:#d1d5db;">Vous avez demandé à réinitialiser votre mot de passe sur <strong>Le Zoo</strong>. Cliquez sur le bouton ci-dessous :</p>
  <div style="text-align:center;margin:32px 0;">
    <a href="{}" style="background:#2563eb;color:#fff;padding:14px 28px;border-radius:8px;text-decoration:none;font-weight:bold;font-size:15px;">Réinitialiser mon mot de passe</a>
  </div>
  <p style="color:#9ca3af;font-size:13px;">Ce lien est valable <strong style="color:#f9fafb;">15 minutes</strong>. Si vous n'avez pas fait cette demande, ignorez cet email.</p>
  <p style="color:#6b7280;font-size:12px;margin-bottom:0;">— L'équipe Le Zoo</p>
</div>
</body></html>"#, user.username, link);

    send_email(&email, "Réinitialisation de votre mot de passe — Le Zoo", &html).await;
    let mut response = ok.into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

async fn reset_password_handler(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    if req.new_password.len() < 8 {
        return (StatusCode::BAD_REQUEST, "Mot de passe trop court (minimum 8 caractères)").into_response();
    }

    let reset_col = state.db.collection::<ResetToken>("reset_tokens");
    let token_hash = hash_token(&req.token);
    let token_doc = match reset_col.find_one(doc! { "token": &token_hash, "used": false }, None).await {
        Ok(Some(t)) => t,
        _ => return (StatusCode::BAD_REQUEST, "Token invalide ou déjà utilisé").into_response(),
    };

    if Utc::now().timestamp() > token_doc.expires_at {
        return (StatusCode::BAD_REQUEST, "Ce lien a expiré (15 minutes). Refaites une demande.").into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed = match Argon2::default().hash_password(req.new_password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur hachage").into_response(),
    };

    let users_col = state.db.collection::<User>("users");
    if let Err(e) = users_col.update_one(
        doc! { "email": &token_doc.email },
        doc! { "$set": { "password": &hashed } },
        None,
    ).await {
        error!("Erreur maj mot de passe: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur serveur").into_response();
    }

    // Invalider le token
    let _ = reset_col.update_one(
        doc! { "token": &token_hash },
        doc! { "$set": { "used": true } },
        None,
    ).await;

    info!("Mot de passe réinitialisé pour {}", token_doc.email);

    let html = r#"<!DOCTYPE html>
<html lang="fr"><body style="margin:0;background:#111827;font-family:Arial,sans-serif;">
<div style="max-width:520px;margin:40px auto;background:#1f2937;border-radius:12px;padding:36px;border:1px solid #374151;">
  <h2 style="color:#34d399;margin-top:0;">✅ Mot de passe modifié</h2>
  <p style="color:#d1d5db;">Votre mot de passe sur <strong>Le Zoo</strong> a été réinitialisé avec succès.</p>
  <p style="color:#d1d5db;">Si vous n'êtes pas à l'origine de cette action, contactez-nous immédiatement.</p>
  <p style="color:#6b7280;font-size:12px;margin-bottom:0;">— L'équipe Le Zoo</p>
</div>
</body></html>"#;

    send_email(&token_doc.email, "Votre mot de passe a été modifié — Le Zoo", html).await;
    let mut response = (StatusCode::OK, "Mot de passe réinitialisé avec succès").into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

async fn change_password_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let claims = match decode_token(&state, &headers) {
        Ok(c) => c,
        Err(s) => return (s, "Token invalide").into_response(),
    };

    if req.new_password.len() < 8 {
        return (StatusCode::BAD_REQUEST, "Nouveau mot de passe trop court (minimum 8 caractères)").into_response();
    }

    let col = state.db.collection::<User>("users");
    let user = match col.find_one(doc! { "username": &claims.username }, None).await.unwrap_or(None) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Utilisateur introuvable").into_response(),
    };

    let pwd_db = match user.password.as_ref() {
        Some(p) => p,
        None => return (StatusCode::UNAUTHORIZED, "Erreur").into_response(),
    };

    let argon_ok = if let Ok(parsed) = PasswordHash::new(pwd_db) {
        Argon2::default().verify_password(req.old_password.as_bytes(), &parsed).is_ok()
    } else { false };
    let bcrypt_ok = if !argon_ok { bcrypt_verify(&req.old_password, pwd_db).unwrap_or(false) } else { false };

    if !argon_ok && !bcrypt_ok {
        return (StatusCode::UNAUTHORIZED, "Ancien mot de passe incorrect").into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed = match Argon2::default().hash_password(req.new_password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erreur hachage").into_response(),
    };

    let _ = col.update_one(
        doc! { "username": &claims.username },
        doc! { "$set": { "password": &hashed } },
        None,
    ).await;

    info!("Mot de passe changé pour {}", claims.username);

    let html = format!(r#"<!DOCTYPE html>
<html lang="fr"><body style="margin:0;background:#111827;font-family:Arial,sans-serif;">
<div style="max-width:520px;margin:40px auto;background:#1f2937;border-radius:12px;padding:36px;border:1px solid #374151;">
  <h2 style="color:#34d399;margin-top:0;">🔐 Mot de passe modifié</h2>
  <p style="color:#d1d5db;">Bonjour <strong style="color:#f9fafb;">{}</strong>,</p>
  <p style="color:#d1d5db;">Votre mot de passe sur <strong>Le Zoo</strong> vient d'être modifié depuis votre espace paramètres.</p>
  <p style="color:#fca5a5;">Si vous n'êtes pas à l'origine de cette action, <strong>changez immédiatement votre mot de passe</strong> ou contactez-nous.</p>
  <p style="color:#6b7280;font-size:12px;margin-bottom:0;">— L'équipe Le Zoo</p>
</div>
</body></html>"#, claims.username);

    send_email(&user.email, "Votre mot de passe a été modifié — Le Zoo", &html).await;
    let mut response = (StatusCode::OK, "Mot de passe modifié avec succès").into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}