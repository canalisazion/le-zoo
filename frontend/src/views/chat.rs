use crate::components::channels_sidebar::ChannelsSidebar;
use crate::components::create_channel_modal::CreateChannelModal;
use crate::components::discover_modal::DiscoverModal;
use crate::components::legal_modals::{CGUModal, ConfidentialityModal};
use crate::components::members_sidebar::MembersSidebar;
use crate::components::message_input::MessageInput;
use crate::components::messages_view::MessagesView;
use crate::components::settings_modal::SettingsModal;
use crate::components::user_admin_modal::UserAdminModal;
use crate::config::{API_BASE_URL, WS_BASE_URL};
use dioxus::document::eval;
use dioxus::prelude::*;
use futures_util::{
    future::{select, Either},
    SinkExt, StreamExt,
};
use futures_channel::mpsc;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use shared::{Channel, ChannelWithStats, MemberInfo, Role, WsServerMsg, MessageStatus};
use js_sys;
use crate::fetch_creds::WithCredentials; // ✅ [H-8]
use crate::Route;

#[derive(Clone)]
pub struct ChatMessage {
    pub author: String,
    pub content: String,
    pub role: Role,
    pub gender: String,
    pub is_direct: bool,
    pub pending: bool,
    pub created_at: i64,
    pub status: shared::MessageStatus,
    pub id: Option<String>,
    pub pinned: bool,
}

#[component]
pub fn Chat() -> Element {
    let nav = use_navigator();

    // Auth guard — page teaser si non connecté
    let username_check: String = LocalStorage::get("username").unwrap_or_default();
    if username_check.is_empty() || username_check == "Anonyme" {
        return rsx! {
            div {
                style: "background: var(--bg-magazine); min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 2rem 1rem;",
                div {
                    style: "max-width: 560px; width: 100%; text-align: center;",

                    // Icône
                    div { style: "font-size: 3.5rem; margin-bottom: 1rem; line-height: 1;", "💬" }

                    // Titre
                    h1 {
                        style: "font-family: 'Playfair Display', Georgia, serif; font-size: 2rem; font-weight: 800; color: var(--accent); margin: 0 0 0.75rem;",
                        "Le Forum du Zoo"
                    }

                    // Accroche
                    p {
                        style: "font-size: 1.05rem; color: var(--text-main); line-height: 1.7; margin-bottom: 0.6rem;",
                        "Des dizaines de salons actifs, des débats qui piquent, des révélations qui choquent."
                    }
                    p {
                        style: "font-size: 1rem; color: var(--text-muted); line-height: 1.7; margin-bottom: 1.8rem;",
                        "Rejoins la communauté, poste en temps réel, réagis à chaud — et fais partie des initiés qui voient tout avant les autres."
                    }

                    // Séparateur features
                    div {
                        style: "display: flex; justify-content: center; gap: 1.5rem; flex-wrap: wrap; margin-bottom: 2rem;",
                        span { style: "font-size: 0.82rem; color: var(--text-muted); background: var(--bg-card); border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.9rem;", "🔥 Salons thématiques" }
                        span { style: "font-size: 0.82rem; color: var(--text-muted); background: var(--bg-card); border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.9rem;", "⚡ Temps réel" }
                        span { style: "font-size: 0.82rem; color: var(--text-muted); background: var(--bg-card); border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.9rem;", "🎭 Messages privés" }
                        span { style: "font-size: 0.82rem; color: var(--text-muted); background: var(--bg-card); border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.9rem;", "🆓 100 % gratuit" }
                    }

                    // CTA principal
                    Link {
                        to: Route::Register {},
                        style: "display: inline-block; background: var(--accent); color: #fff; font-family: Inter, sans-serif; font-size: 1rem; font-weight: 700; padding: 0.85rem 2.2rem; border-radius: 6px; text-decoration: none; letter-spacing: 0.03em; margin-bottom: 1rem;",
                        "Créer mon compte — c'est gratuit"
                    }

                    // Lien login
                    p { style: "font-size: 0.85rem; color: var(--text-muted);",
                        "Déjà membre ? "
                        Link {
                            to: Route::Login {},
                            style: "color: var(--accent); text-decoration: underline; font-weight: 600;",
                            "Se connecter"
                        }
                    }
                }
            }
        };
    }

    // --- STATE ---
    let mut messages = use_signal(|| Vec::<ChatMessage>::new());
    let draft = use_signal(|| String::new());
    let mut channels = use_signal(|| Vec::<Channel>::new());
    let mut current_channel = use_signal(|| {
        LocalStorage::get::<String>("current_channel").unwrap_or_else(|_| "general".to_string())
    });
    let username = use_signal(|| {
        LocalStorage::get::<String>("username").unwrap_or_else(|_| "Anonyme".to_string())
    });
    let user_role = use_signal(|| {
        let role_str = LocalStorage::get::<String>("role").unwrap_or_else(|_| "user".to_string());
        match role_str.as_str() {
            "super_admin" => Role::SuperAdmin,
            "admin" => Role::Admin,
            _ => Role::User,
        }
    });
    let mut members = use_signal(|| Vec::<MemberInfo>::new());
    let members_version = use_signal(|| 0u32);
    let is_admin = user_role.read().is_at_least_admin();
    let is_super = matches!(*user_role.read(), Role::SuperAdmin);

    let mut show_create_modal = use_signal(|| false);
    let new_channel_name = use_signal(|| String::new());
    let new_channel_desc = use_signal(|| String::new());
    let new_channel_emoji = use_signal(|| "💬".to_string());
    let new_channel_media = use_signal(|| String::new());
    let new_channel_topic = use_signal(|| String::new());
    let popup_user = use_signal(|| Option::<String>::None);
    let mut confirm_delete_channel = use_signal(|| Option::<String>::None);
    let mut ws_outgoing = use_signal(|| Option::<mpsc::UnboundedSender<String>>::None);
    let mut show_channels_mobile = use_signal(|| false);
    let mut show_members_mobile = use_signal(|| false);
    let mut show_discover_modal = use_signal(|| false);
    let mut discover_channels = use_signal(|| Vec::<ChannelWithStats>::new());
    let mut subscribed_channels = use_signal(|| Vec::<String>::new());
    let mut direct_chat_with = use_signal(|| Option::<String>::None);
    // FIX SÉCURITÉ : Clé dynamique basée sur le pseudo pour éviter de voir les amis des autres
    let mut my_direct_chats = use_signal(|| {
        let u = LocalStorage::get::<String>("username").unwrap_or_else(|_| "Anonyme".to_string());
        LocalStorage::get::<Vec<String>>(&format!("dms_{}", u)).unwrap_or_default()
    });
    let mut show_direct_chats = use_signal(|| false);
    let mut show_my_channels = use_signal(|| true);
    let mut show_gold_channels = use_signal(|| true);
    let mut show_settings = use_signal(|| false);
    let show_cgu = use_signal(|| false);
    let show_confidentialite = use_signal(|| false);
    let _avatar_data = use_signal(|| Option::<String>::None);
    let unread_count = use_signal(|| 0u32);
    let mut summary_text = use_signal(|| String::new());
    let _summary_loading = use_signal(|| false);
    let mut user_is_premium = use_signal(|| false);
    let mut show_archive_modal = use_signal(|| false);
    let mut archive_text = use_signal(|| String::new());
    let mut oldest_msg_id = use_signal(|| Option::<String>::None);
    let mut has_more_msgs = use_signal(|| false);
    let mut oldest_dm_id = use_signal(|| Option::<String>::None);
    let mut has_more_dms = use_signal(|| false);

    // --- RESOURCES ---

    // Charger la liste des salons
    let _channels_loader = use_resource(move || async move {
        let client = Client::new();
        // ✅ [H-8] cookie HttpOnly envoyé automatiquement
        if let Ok(res) = client
            .get(format!("{}/api/channels", API_BASE_URL))
            .with_credentials()
            .send()
            .await
        {
            if let Ok(list) = res.json::<Vec<Channel>>().await {
                channels.set(list);
            }
        }
    });

    // Charger la liste des membres (re-fetch quand members_version change)
    let _members_loader = use_resource(move || {
        let _v = *members_version.read();
        async move {
            let client = Client::new();
            if let Ok(res) = client
                .get(format!("{}/api/users", API_BASE_URL))
                .with_credentials()
                .send()
                .await
            {
                if let Ok(list) = res.json::<Vec<MemberInfo>>().await {
                    members.set(list);
                }
            }
        }
    });

    // Charger les souscriptions de l'utilisateur
    let _subscriptions_loader = use_resource(move || async move {
        let client = Client::new();
        if let Ok(res) = client
            .get(format!("{}/api/users/me", API_BASE_URL))
            .with_credentials()
            .send()
            .await
        {
            if let Ok(user_info) = res.json::<serde_json::Value>().await {
                if let Some(subs) = user_info
                    .get("subscribed_channels")
                    .and_then(|v| v.as_array())
                {
                    let subs_vec: Vec<String> = subs
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    subscribed_channels.set(subs_vec);
                }

                if let Some(premium) = user_info.get("is_premium").and_then(|v| v.as_bool()) {
                    user_is_premium.set(premium);
                }

                if let Some(dms) = user_info.get("direct_contacts").and_then(|v| v.as_array()) {
                    let dms_vec: Vec<String> = dms
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    // On fusionne avec le cache local pour ne rien perdre
                    let mut current = my_direct_chats.read().clone();
                    for dm in dms_vec {
                        if !current.contains(&dm) {
                            current.push(dm);
                        }
                    }
                    my_direct_chats.set(current);
                }
            }
        }
    });

    // Charger l'historique du salon actuel OU conversation privée
    let _history_loader = use_resource(move || {
        let channel = current_channel.read().clone();
        let direct_with = direct_chat_with.read().clone();
        async move {
            let client = Client::new();
            // ✅ [H-8] cookie HttpOnly envoyé automatiquement via credentials:include

            // Reset pagination state on channel/DM switch
            oldest_msg_id.set(None);
            has_more_msgs.set(false);
            oldest_dm_id.set(None);
            has_more_dms.set(false);

            if let Some(target) = direct_with {
                // Charger messages privés
                let url = format!("{}/api/direct_messages?with={}", API_BASE_URL, target);
                if let Ok(res) = client
                    .get(url)
                    .with_credentials()
                    .send()
                    .await
                {
                    if let Ok(msgs) = res.json::<Vec<shared::Message>>().await {
                        let formatted: Vec<ChatMessage> = msgs
                            .iter()
                            .map(|m| ChatMessage {
                                author: m.author_name.clone(),
                                content: m.content.clone(),
                                role: m.author_role.clone(),
                                gender: m.author_gender.clone(),
                                is_direct: true,
                                pending: false,
                                created_at: m.created_at,
                                status: m.status.clone(),
                                id: m.id.clone(),
                                pinned: m.pinned,
                            })
                            .collect();
                        has_more_dms.set(msgs.len() == 50);
                        oldest_dm_id.set(formatted.first().and_then(|m| m.id.clone()));
                        messages.set(formatted.clone());

                        let current_user = LocalStorage::get::<String>("username").unwrap_or_default();
                        for msg in formatted.iter() {
                            if msg.author != current_user && msg.id.is_some() {
                                let msg_id = msg.id.clone().unwrap();
                                spawn(async move {
                                    let client = Client::new();
                                    let _ = client.post(format!("{}/api/direct_messages/read", API_BASE_URL))
                                        .with_credentials()
                                        .json(&serde_json::json!({ "message_id": msg_id }))
                                        .send().await;
                                });
                            }
                        }
                    }
                }
            } else {
                // Charger messages du salon
                let url = format!("{}/api/messages?channel={}", API_BASE_URL, channel);
                if let Ok(res) = client.get(url).with_credentials().send().await {
                    if let Ok(msgs) = res.json::<Vec<shared::Message>>().await {
                        let formatted: Vec<ChatMessage> = msgs
                            .iter()
                            .map(|m| ChatMessage {
                                author: m.author_name.clone(),
                                content: m.content.clone(),
                                role: m.author_role.clone(),
                                gender: m.author_gender.clone(),
                                is_direct: false,
                                pending: false,
                                created_at: m.created_at,
                                status: MessageStatus::Sent,
                                id: m.id.clone(),
                                pinned: m.pinned,
                            })
                            .collect();
                        has_more_msgs.set(msgs.len() == 200); // ✅ [M-4]
                        oldest_msg_id.set(formatted.first().and_then(|m| m.id.clone()));
                        messages.set(formatted);
                    }
                }
            }
        }
    });

    // ✅ [H-1][H-8] WebSocket : JWT dans cookie HttpOnly, reconnexion auto avec backoff exponentiel
    let _ws_future = use_resource(move || {
        let channel = current_channel.read().clone();
        let ws_url = format!("{}/ws", WS_BASE_URL);
        let _ = direct_chat_with.read(); // lu pour la réactivité

        async move {
            // Canal mpsc pour les messages sortants (remplace le polling 5ms)
            let (tx, mut rx) = mpsc::unbounded::<String>();
            ws_outgoing.set(Some(tx));

            let mut delay_ms = 2000u64;
            loop {
                let ws = match WebSocket::open(&ws_url) {
                    Ok(ws) => ws,
                    Err(_) => {
                        gloo_timers::future::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(30_000);
                        continue;
                    }
                };
                delay_ms = 2000;
                let (mut write, mut read) = ws.split();

                let auth_msg = serde_json::json!({ "type": "auth", "channel": channel }).to_string();
                if write.send(Message::Text(auth_msg)).await.is_err() {
                    gloo_timers::future::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms * 2).min(30_000);
                    continue;
                }

                loop {
                    match select(read.next(), rx.next()).await {
                        Either::Left((Some(Ok(Message::Text(msg))), _)) => {
                            if let Ok(server_msg) = serde_json::from_str::<WsServerMsg>(&msg) {
                                match server_msg {
                                    WsServerMsg::Chat { author, content, role, gender } => {
                                        if direct_chat_with.read().is_none() {
                                            messages.with_mut(|v| {
                                                v.push(ChatMessage {
                                                    author,
                                                    content,
                                                    role,
                                                    gender,
                                                    is_direct: false,
                                                    pending: false,
                                                    created_at: js_sys::Date::now() as i64 / 1000,
                                                    status: MessageStatus::Sent,
                                                    id: None,
                                                    pinned: false,
                                                })
                                            });
                                        }
                                    }
                                    WsServerMsg::DirectMessage { from, content, role, gender, id } => {
                                        if let Some(target) = direct_chat_with.read().clone() {
                                            if from == target {
                                                messages.with_mut(|v| {
                                                    v.push(ChatMessage {
                                                        author: from,
                                                        content,
                                                        role,
                                                        gender,
                                                        is_direct: true,
                                                        pending: false,
                                                        created_at: js_sys::Date::now() as i64 / 1000,
                                                        status: MessageStatus::Delivered,
                                                        id: Some(id),
                                                        pinned: false,
                                                    })
                                                });
                                            }
                                        }
                                    }
                                    WsServerMsg::Presence { online } => {
                                        members.with_mut(|list| {
                                            for m in list.iter_mut() {
                                                m.online = online.contains(&m.username);
                                            }
                                            for name in &online {
                                                if !list.iter().any(|m| &m.username == name) {
                                                    list.push(MemberInfo {
                                                        username: name.clone(),
                                                        role: Role::User,
                                                        online: true,
                                                        gender: String::new(),
                                                        avatar: None,
                                                        badges: Vec::new(),
                                                    });
                                                }
                                            }
                                        });
                                    }
                                    WsServerMsg::Ack { ok, message_id } => {
                                        if ok {
                                            messages.with_mut(|v| {
                                                if let Some(last) = v.last_mut() {
                                                    if last.pending {
                                                        last.pending = false;
                                                        last.status = MessageStatus::Sent;
                                                        last.id = message_id;
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    WsServerMsg::MessageStatusUpdated { message_id, status } => {
                                        messages.with_mut(|v| {
                                            for msg in v.iter_mut() {
                                                if msg.id.as_ref() == Some(&message_id) {
                                                    msg.status = status.clone();
                                                    break;
                                                }
                                            }
                                        });
                                    }
                                    WsServerMsg::MessageDeleted { id } => {
                                        messages.with_mut(|v| v.retain(|m| m.id.as_deref() != Some(&id)));
                                    }
                                }
                            }
                        }
                        Either::Left((_, _)) => break, // connexion fermée
                        Either::Right((Some(text), _)) => {
                            if write.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Either::Right((None, _)) => break, // canal mpsc fermé
                    }
                }

                gloo_timers::future::sleep(std::time::Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(30_000);
            }
        }
    });

    // Auto-scroll vers le bas quand les messages changent
    use_effect(move || {
        let _ = messages.read().len();
        let mut eval = eval(
            r#"
            const container = document.getElementById('messages-container');
            if (container) {
                container.scrollTop = container.scrollHeight;
            }
            "#,
        );
        spawn(async move {
            let _ = eval.recv::<serde_json::Value>().await;
        });
    });

    // --- RENDER ---
    rsx! {
            div { class: "flex h-screen w-full overflow-hidden bg-white dark:bg-[#0a0a0a] text-gray-900 dark:text-gray-100 font-sans relative",

                // ========== BACKDROP MOBILE ==========
                if *show_channels_mobile.read() || *show_members_mobile.read() {
                    div {
                        class: "fixed inset-0 bg-black bg-opacity-50 z-40 md:hidden",
                        onclick: move |_| {
                            show_channels_mobile.set(false);
                            show_members_mobile.set(false);
                        }
                    }
                }

                // ========== SIDEBAR GAUCHE : Salons ==========
                ChannelsSidebar {
                    channels,
                    current_channel,
                    messages,
                    show_channels_mobile,
                    subscribed_channels,
                    show_gold_channels,
                    show_my_channels,
                    show_direct_chats,
                    my_direct_chats,
                    direct_chat_with,
                    unread_count,
                    show_settings,
                    show_discover_modal,
                    discover_channels,
                    show_create_modal,
                    confirm_delete_channel,
                    is_admin,
                    username,
                    user_role,
                }

                                // ========== ZONE DE CHAT PRINCIPALE ==========
                div { class: "flex-1 flex flex-col min-w-0",
                    // Header du salon
                    div { class: "h-14 flex-shrink-0 flex items-center px-4 justify-between border-b",
                        style: "background:var(--bg-main);border-color:var(--border);",
                        // Bouton menu canaux (mobile)
                        button {
                            class: "md:hidden p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded",
                            onclick: move |_| show_channels_mobile.set(true),
                            "☰"
                        }
                        {
                            let direct_with = direct_chat_with.read().clone();
                            if let Some(target) = direct_with {
                                rsx! {
                                    div { class: "flex items-center gap-2",
                                        h1 { class: "text-xl md:text-2xl font-bold text-purple-400", "💬 Message privé avec {target}" }
                                        button {
                                            class: "text-sm px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-white",
                                            onclick: move |_| {
                                                direct_chat_with.set(None);
                                                messages.set(Vec::new());
                                            },
                                            "✕ Fermer"
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    span {
                                        style: "color:var(--text-muted);font-size:0.75rem;font-style:italic;letter-spacing:0.05em;",
                                        "ESPACE PARTENAIRES"
                                    }
                                }
                            }
                        }
                        // Bouton membres (mobile)
                        button {
                            class: "md:hidden p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded",
                            onclick: move |_| show_members_mobile.set(true),
                            "👥"
                        }
                    }

                    // Overlay résumé IA
                    if !summary_text.read().is_empty() {
                        div { class: "absolute top-14 left-4 right-4 z-50 bg-white dark:bg-[#0a0a0a] border border-purple-600 rounded-lg p-4 shadow-2xl max-h-64 overflow-y-auto",
                            div { class: "flex justify-between items-start mb-2",
                                p { class: "text-sm font-semibold text-purple-400", "📋 Résumé IA" }
                                button {
                                    class: "text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white text-lg",
                                    onclick: move |_| summary_text.set(String::new()),
                                    "✕"
                                }
                            }
                            p { class: "text-sm whitespace-pre-wrap text-gray-900 dark:text-gray-100", "{summary_text}" }
                        }
                    }

                    // Zone Média/Débat (si topic_media ou topic_text présent)
                    {
                        if direct_chat_with.read().is_none() {
                            let current_ch = current_channel.read().clone();
                            let ch_data = channels.read().iter().find(|c| c.id.as_ref().map(|id| id == &current_ch).unwrap_or(false)).cloned();
                            
                            if let Some(channel) = ch_data {
                                let media_opt = channel.topic_media.clone();
                                let topic_opt = channel.topic_text.clone();
                                
                                if media_opt.is_some() || topic_opt.is_some() {
                                    rsx! {
                                        div { class: "border-b p-4 flex-shrink-0",
                                            style: "background:var(--bg-main);border-color:var(--border);",
                                            if let Some(topic) = topic_opt {
                                                div { class: "text-sm mb-3 italic text-gray-500 dark:text-gray-400", "📌 {topic}" }
                                            }
                                            if let Some(media_url) = media_opt {
                                                {
                                                    let is_youtube = media_url.contains("youtube.com") || media_url.contains("youtu.be");
                                                    let is_image = media_url.ends_with(".jpg") || media_url.ends_with(".jpeg") ||
                                                                  media_url.ends_with(".png") || media_url.ends_with(".gif") ||
                                                                  media_url.ends_with(".webp") || media_url.starts_with("data:image");

                                                    if is_youtube {
                                                        let video_id = if media_url.contains("youtu.be/") {
                                                            media_url.split("youtu.be/").nth(1).unwrap_or("").split('?').next().unwrap_or("")
                                                        } else if media_url.contains("watch?v=") {
                                                            media_url.split("watch?v=").nth(1).unwrap_or("").split('&').next().unwrap_or("")
                                                        } else {
                                                            ""
                                                        };
                                                        let embed_url = format!("https://www.youtube.com/embed/{}", video_id);
                                                        rsx! {
                                                            iframe {
                                                                src: "{embed_url}",
                                                                class: "w-full rounded-lg border border-gray-600",
                                                                style: "height: 280px;",
                                                                allowfullscreen: true
                                                            }
                                                        }
                                                    } else if is_image {
                                                        rsx! {
                                                            img {
                                                                src: "{media_url}",
                                                                class: "w-full rounded-lg max-h-60 object-cover border border-gray-600"
                                                            }
                                                        }
                                                    } else {
                                                        rsx! {
                                                            div {
                                                                iframe {
                                                                    src: "{media_url}",
                                                                    class: "w-full rounded-lg border border-gray-600",
                                                                    style: "height: 400px;",
                                                                    title: "Aperçu du site"
                                                                }
                                                                p { class: "text-xs text-gray-500 dark:text-gray-400 mt-2",
                                                                    "🔗 ",
                                                                    a { href: "{media_url}", target: "_blank", rel: "noopener noreferrer", class: "underline hover:text-blue-400",
                                                                        "Ouvrir dans un nouvel onglet"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            } else {
                                rsx! {}
                            }
                        } else {
                            rsx! {}
                        }
                    }

                    // Bouton "Charger plus" pour les messages privés (DM)
                    if *has_more_dms.read() && direct_chat_with.read().is_some() {
                        div { class: "flex justify-center py-2 flex-shrink-0",
                            button {
                                class: "px-4 py-1.5 rounded-full text-xs font-semibold bg-gray-700 hover:bg-gray-600 text-gray-300 transition",
                                onclick: move |_| {
                                    let target = direct_chat_with.read().clone();
                                    let before = oldest_dm_id.read().clone();
                                    if let (Some(dm_target), Some(before_id)) = (target, before) {
                                        spawn(async move {
                                            let client = Client::new();
                                            let url = format!("{}/api/direct_messages?with={}&before_id={}", API_BASE_URL, dm_target, before_id);
                                            if let Ok(res) = client
                                                .get(url)
                                                .with_credentials()
                                                .send()
                                                .await
                                            {
                                                if let Ok(msgs) = res.json::<Vec<shared::Message>>().await {
                                                    let older: Vec<ChatMessage> = msgs
                                                        .iter()
                                                        .map(|m| ChatMessage {
                                                            author: m.author_name.clone(),
                                                            content: m.content.clone(),
                                                            role: m.author_role.clone(),
                                                            gender: m.author_gender.clone(),
                                                            is_direct: true,
                                                            pending: false,
                                                            created_at: m.created_at,
                                                            status: m.status.clone(),
                                                            id: m.id.clone(),
                                                            pinned: m.pinned,
                                                        })
                                                        .collect();
                                                    has_more_dms.set(msgs.len() == 50);
                                                    oldest_dm_id.set(older.first().and_then(|m| m.id.clone()));
                                                    messages.with_mut(|v| {
                                                        let mut combined = older;
                                                        combined.append(v);
                                                        *v = combined;
                                                    });
                                                }
                                            }
                                        });
                                    }
                                },
                                "Charger plus"
                            }
                        }
                    }

                    // Bouton "Charger les messages précédents"
                    if *has_more_msgs.read() && direct_chat_with.read().is_none() {
                        div { class: "flex justify-center py-2 flex-shrink-0",
                            button {
                                class: "px-4 py-1.5 rounded-full text-xs font-semibold bg-gray-700 hover:bg-gray-600 text-gray-300 transition",
                                onclick: move |_| {
                                    let channel = current_channel.read().clone();
                                    let before = oldest_msg_id.read().clone();
                                    if let Some(before_id) = before {
                                        spawn(async move {
                                            let client = Client::new();
                                            let url = format!("{}/api/messages?channel={}&before_id={}", API_BASE_URL, channel, before_id);
                                            if let Ok(res) = client
                                                .get(url)
                                                .with_credentials()
                                                .send()
                                                .await
                                            {
                                                if let Ok(msgs) = res.json::<Vec<shared::Message>>().await {
                                                    let older: Vec<ChatMessage> = msgs
                                                        .iter()
                                                        .map(|m| ChatMessage {
                                                            author: m.author_name.clone(),
                                                            content: m.content.clone(),
                                                            role: m.author_role.clone(),
                                                            gender: m.author_gender.clone(),
                                                            is_direct: false,
                                                            pending: false,
                                                            created_at: m.created_at,
                                                            status: MessageStatus::Sent,
                                                            id: m.id.clone(),
                                                            pinned: m.pinned,
                                                        })
                                                        .collect();
                                                    has_more_msgs.set(msgs.len() == 200); // ✅ [M-4]
                                                    oldest_msg_id.set(older.first().and_then(|m| m.id.clone()));
                                                    messages.with_mut(|v| {
                                                        let mut combined = older;
                                                        combined.append(v);
                                                        *v = combined;
                                                    });
                                                }
                                            }
                                        });
                                    }
                                },
                                "⬆ Charger les messages précédents"
                            }
                        }
                    }

                    MessagesView { messages, current_user: username, is_admin }

                    MessageInput { draft, current_channel, direct_chat_with, username, user_role, messages, ws_outgoing }
                }

                // ========== PANNEAU DROIT DESKTOP ==========
                div {
                    class: "hidden lg:flex w-72 flex-col flex-shrink-0 border-l",
                    style: "border-color:var(--border);background:var(--bg-sidebar);",

                    div {
                        style: "padding:1rem;border-bottom:1px solid var(--border);",
                        span {
                            style: "color:var(--text-main);font-weight:700;font-size:1rem;",
                            "📋 Infos du canal"
                        }
                    }

                    {
                        let current_ch = current_channel.read().clone();
                        let msg_count = messages.read().len();
                        rsx! {
                            div {
                                style: "padding:1rem;border-bottom:1px solid var(--border);",
                                p { style: "color:var(--accent);font-weight:600;font-size:1.1rem;", "# {current_ch}" }
                                p { style: "color:var(--text-muted);font-size:0.8rem;margin-top:0.25rem;", "{msg_count} messages" }
                            }
                        }
                    }

                    div {
                        style: "padding:1rem;border-bottom:1px solid var(--border);flex:1;overflow-y:auto;",
                        p { style: "color:var(--text-muted);font-size:0.75rem;font-weight:600;text-transform:uppercase;letter-spacing:0.05em;margin-bottom:0.5rem;", "EN LIGNE" }
                        {
                            let members_list = members.read();
                            let online: Vec<_> = members_list.iter().filter(|m| m.online).cloned().collect();
                            rsx! {
                                for member in online {
                                    div {
                                        key: "{member.username}",
                                        class: "flex items-center gap-2 py-1",
                                        div { style: "width:8px;height:8px;border-radius:50%;background:#22c55e;flex-shrink:0;" }
                                        span { style: "color:var(--text-main);font-size:0.875rem;", "{member.username}" }
                                    }
                                }
                            }
                        }
                    }

                    if is_super {
                        div {
                            style: "padding:1rem;display:flex;flex-direction:column;gap:0.5rem;",
                            button {
                                style: "width:100%;padding:0.5rem;background:var(--accent);color:white;border-radius:0.5rem;font-size:0.875rem;font-weight:600;cursor:pointer;",
                                onclick: move |_| {
                                    let ch = current_channel.read().clone();
                                    spawn(async move {
                                        let client = Client::new();
                                        if let Ok(res) = client.post(format!("{}/api/channels/{}/summary", API_BASE_URL, ch))
                                            .with_credentials()
                                            .send().await
                                        {
                                            if let Ok(json) = res.json::<serde_json::Value>().await {
                                                if let Some(s) = json["summary"].as_str() {
                                                    summary_text.set(s.to_string());
                                                }
                                            }
                                        }
                                    });
                                },
                                "📋 Résumé IA"
                            }
                            button {
                                style: "width:100%;padding:0.5rem;background:#7c3aed;color:white;border-radius:0.5rem;font-size:0.875rem;font-weight:600;cursor:pointer;",
                                onclick: move |_| {
                                    let ch = current_channel.read().clone();
                                    spawn(async move {
                                        let client = Client::new();
                                        if let Ok(res) = client.get(format!("{}/api/channels/{}/archive", API_BASE_URL, ch))
                                            .with_credentials()
                                            .send().await
                                        {
                                            if let Ok(json) = res.json::<serde_json::Value>().await {
                                                if let Some(arr) = json["archive"].as_array() {
                                                    if arr.is_empty() {
                                                        archive_text.set("Pas d'anciens messages à synthétiser.".to_string());
                                                    }
                                                } else if let Some(s) = json["summary"].as_str() {
                                                    archive_text.set(s.to_string());
                                                }
                                                show_archive_modal.set(true);
                                            }
                                        }
                                    });
                                },
                                "📚 Anciennes discussions"
                            }
                        }
                    }
                }

                MembersSidebar { show_members_mobile, members, username, popup_user }

                UserAdminModal {
                    popup_user,
                    members,
                    members_version,
                    is_admin,
                    is_super,
                    my_direct_chats,
                    direct_chat_with,
                    messages,
                    show_members_mobile
                }

                // ========== MODAL CONFIRMATION SUPPRESSION ==========
                {
                    let del_ch = confirm_delete_channel.read().clone();
                    if let Some(ch_id) = del_ch {
                        let ch_name = channels.read().iter().find(|c| c.id.as_ref() == Some(&ch_id)).map(|c| c.name.clone()).unwrap_or_default();
                        let del_id = ch_id.clone();
                        rsx! {
                            div {
                                class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                                onclick: move |_| confirm_delete_channel.set(None),
                                div {
                                    class: "p-6 rounded-xl shadow-2xl w-96 border border-red-600",
                                    style: "background:var(--bg-main);",
                                    onclick: move |e| e.stop_propagation(),
                                    h3 { class: "text-xl font-bold mb-4 text-red-400", "⚠️ Confirmer la suppression" }
                                    p { class: "mb-6 text-gray-500 dark:text-gray-400", "Voulez-vous vraiment supprimer le salon \"{ch_name}\" ?" }
                                    div { class: "flex gap-3",
                                        button {
                                            class: "flex-1 bg-gray-600 hover:bg-gray-500 py-2 rounded-lg transition",
                                            onclick: move |_| confirm_delete_channel.set(None),
                                            "Annuler"
                                        }
                                        button {
                                            class: "flex-1 bg-red-600 hover:bg-red-500 py-2 rounded-lg font-bold transition",
                                            onclick: move |_| {
                                                let ch = del_id.clone();
                                                spawn(async move {
                                                    let client = Client::new();
                                                    let _ = client.delete(format!("{}/api/channels/{}", API_BASE_URL, ch))
                                                        .with_credentials()
                                                        .send().await;
                                                    if let Ok(res) = client.get(format!("{}/api/channels", API_BASE_URL))
                                                        .with_credentials()
                                                        .send().await {
                                                        if let Ok(list) = res.json::<Vec<Channel>>().await {
                                                            channels.set(list);
                                                        }
                                                    }
                                                });
                                                confirm_delete_channel.set(None);
                                            },
                                            "🗑 Supprimer"
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }

                // ========== MODAL DÉCOUVERTE DES CANAUX ==========
                DiscoverModal { show_discover_modal, discover_channels, subscribed_channels }

                // ========== MODAL CRÉATION DE SALON ==========
                CreateChannelModal {
                    show_create_modal,
                    new_channel_name,
                    new_channel_desc,
                    new_channel_emoji,
                    new_channel_media,
                    new_channel_topic,
                    channels,
                }

                SettingsModal { show_settings, show_cgu, show_confidentialite }

                CGUModal { show_cgu }
                ConfidentialityModal { show_confidentialite }

                // ========== MODAL ARCHIVE ==========
                if *show_archive_modal.read() {
                    div {
                        class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                        onclick: move |_| show_archive_modal.set(false),
                        div {
                            class: "p-6 rounded-xl shadow-2xl w-96 max-h-96 overflow-y-auto border",
                            style: "background:var(--bg-main);border-color:var(--border);",
                            onclick: move |e| e.stop_propagation(),
                            div { class: "flex justify-between items-start mb-4",
                                h3 { class: "text-lg font-bold", style: "color:var(--text-main);", "📚 Synthèse des anciennes discussions" }
                                button {
                                    class: "text-gray-500 hover:text-white text-xl",
                                    onclick: move |_| show_archive_modal.set(false),
                                    "✕"
                                }
                            }
                            p { class: "text-sm whitespace-pre-wrap", style: "color:var(--text-main);", "{archive_text}" }
                        }
                    }
                }
            }
        }
}