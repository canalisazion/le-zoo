use dioxus::prelude::*;
use shared::{Channel, ChannelWithStats, Role};
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use crate::config::API_BASE_URL;
use crate::views::chat::ChatMessage;

#[component]
pub fn ChannelsSidebar(
    channels: Signal<Vec<Channel>>,
    mut current_channel: Signal<String>,
    mut messages: Signal<Vec<ChatMessage>>,
    mut show_channels_mobile: Signal<bool>,
    subscribed_channels: Signal<Vec<String>>,
    mut show_gold_channels: Signal<bool>,
    mut show_my_channels: Signal<bool>,
    mut show_direct_chats: Signal<bool>,
    my_direct_chats: Signal<Vec<String>>,
    mut direct_chat_with: Signal<Option<String>>,
    unread_count: Signal<u32>,
    mut show_settings: Signal<bool>,
    mut show_discover_modal: Signal<bool>,
    mut discover_channels: Signal<Vec<ChannelWithStats>>,
    mut show_create_modal: Signal<bool>,
    mut confirm_delete_channel: Signal<Option<String>>,
    is_admin: bool,
    username: Signal<String>,
    user_role: Signal<Role>,
) -> Element {
    let username_val = username.read().clone();
    let username_initial = username_val
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    let role_label = match *user_role.read() {
        Role::SuperAdmin => "Super Admin",
        Role::Admin => "Modérateur",
        _ => "Membre",
    };

    rsx! {
        div {
            class: if *show_channels_mobile.read() {
                "fixed inset-y-0 left-0 w-56 border-r flex flex-col flex-shrink-0 z-50 md:relative md:flex"
            } else {
                "hidden md:flex w-56 border-r flex-col flex-shrink-0"
            },
            style: "background:var(--bg-main);border-color:var(--border);",

            // ── Header utilisateur ──────────────────────────────────────
            div { class: "px-4 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center gap-3",
                div {
                    class: "w-9 h-9 rounded-full bg-orange-500 flex items-center justify-center text-white font-bold text-sm flex-shrink-0",
                    "{username_initial}"
                }
                div { class: "flex flex-col min-w-0",
                    span { class: "text-gray-900 dark:text-gray-100 text-sm font-semibold truncate", "{username_val}" }
                    span { class: "text-orange-400 text-xs", "{role_label}" }
                }
                button {
                    class: "ml-auto text-gray-500 dark:text-gray-400 hover:text-orange-400 transition",
                    onclick: move |_| show_settings.set(true),
                    "☰"
                }
                button {
                    class: "md:hidden text-gray-500 dark:text-gray-400 hover:text-orange-400 transition",
                    onclick: move |_| show_channels_mobile.set(false),
                    "✕"
                }
            }

            // ── Corps scrollable ────────────────────────────────────────
            div { class: "flex-1 overflow-y-auto scrollbar-custom",

                // Canaux essentiels (gold)
                {
                    let gold_channels: Vec<Channel> = channels.read().iter()
                        .filter(|c| c.is_gold)
                        .cloned()
                        .collect();

                    if !gold_channels.is_empty() {
                        rsx! {
                            div { class: "px-3 pt-3",
                                button {
                                    class: "w-full flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-orange-500/10 transition",
                                    onclick: move |_| { let v = *show_gold_channels.read(); show_gold_channels.set(!v); },
                                    span { class: "text-orange-400 text-xs font-bold uppercase tracking-wider", "⭐ Essentiels" }
                                    span { class: "text-orange-400 text-xs",
                                        if *show_gold_channels.read() { "▾" } else { "▸" }
                                    }
                                }
                                if *show_gold_channels.read() {
                                    div { class: "mt-1 space-y-0.5",
                                        for channel in gold_channels.iter() {
                                            {
                                                let channel_id = channel.id.clone().unwrap_or_else(|| "unknown".to_string());
                                                let channel_name = channel.name.clone();
                                                let is_active = current_channel.read().as_str() == channel_id;
                                                let icon = channel.icon.clone().unwrap_or_else(|| "💬".to_string());
                                                rsx! {
                                                    button {
                                                        key: "gold-{channel_id}",
                                                        class: if is_active {
                                                            "w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-orange-500/15 border-l-2 border-orange-400 text-gray-900 dark:text-gray-100 text-sm font-medium"
                                                        } else {
                                                            "w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400 text-sm transition"
                                                        },
                                                        onclick: move |_| {
                                                            let ch = channel_id.clone();
                                                            let _ = LocalStorage::set("current_channel", &ch);
                                                            current_channel.set(ch);
                                                            messages.set(Vec::new());
                                                            show_channels_mobile.set(false);
                                                            direct_chat_with.set(None);
                                                        },
                                                        span { class: "flex-shrink-0", "{icon}" }
                                                        span { class: "truncate flex-1", "{channel_name}" }
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
                }

                // Mes canaux (subscribed)
                {
                    let subs = subscribed_channels.read().clone();
                    let subscribed: Vec<Channel> = channels.read().iter()
                        .filter(|c| !c.is_gold && subs.contains(&c.id.clone().unwrap_or_default()))
                        .cloned()
                        .collect();

                    if !subscribed.is_empty() {
                        rsx! {
                            div { class: "px-3 pt-3",
                                button {
                                    class: "w-full flex items-center justify-between px-2 py-1.5 rounded-lg hover:bg-orange-500/10 transition",
                                    onclick: move |_| { let v = *show_my_channels.read(); show_my_channels.set(!v); },
                                    span { class: "text-gray-500 dark:text-gray-400 text-xs font-bold uppercase tracking-wider", "Mes canaux" }
                                    span { class: "text-gray-500 dark:text-gray-400 text-xs",
                                        if *show_my_channels.read() { "▾" } else { "▸" }
                                    }
                                }
                                if *show_my_channels.read() {
                                    div { class: "mt-1 space-y-0.5",
                                        for channel in subscribed.iter() {
                                            {
                                                let channel_id = channel.id.clone().unwrap_or_else(|| "unknown".to_string());
                                                let channel_name = channel.name.clone();
                                                let is_active = current_channel.read().as_str() == channel_id;
                                                let icon = channel.icon.clone().unwrap_or_else(|| "#".to_string());
                                                let is_locked = channel.is_locked;
                                                rsx! {
                                                    div { class: "relative group",
                                                        button {
                                                            key: "sub-{channel_id}",
                                                            class: if is_active {
                                                                "w-full flex items-center gap-2 px-3 py-2 rounded-lg bg-orange-500/15 border-l-2 border-orange-400 text-gray-900 dark:text-gray-100 text-sm font-medium"
                                                            } else {
                                                                "w-full flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400 text-sm transition"
                                                            },
                                                            onclick: move |_| {
                                                                let ch = channel_id.clone();
                                                                let _ = LocalStorage::set("current_channel", &ch);
                                                                current_channel.set(ch);
                                                                messages.set(Vec::new());
                                                                show_channels_mobile.set(false);
                                                                direct_chat_with.set(None);
                                                            },
                                                            span { class: "flex-shrink-0", "{icon}" }
                                                            span { class: "truncate flex-1", "{channel_name}" }
                                                        }
                                                        if is_admin && !is_locked {
                                                            {
                                                                let del_id = channel_id.clone();
                                                                rsx! {
                                                                    button {
                                                                        class: "absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition bg-red-600 hover:bg-red-500 text-white text-[10px] w-5 h-5 flex items-center justify-center rounded",
                                                                        onclick: move |e| {
                                                                            e.stop_propagation();
                                                                            confirm_delete_channel.set(Some(del_id.clone()));
                                                                        },
                                                                        "✕"
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            span {}
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
                }

                // ── Boutons Mes discussions + Découvrir ─────────────────
                div { class: "px-3 mt-3 space-y-2",

                    // Mes discussions
                    button {
                        class: "w-full flex items-center gap-2 px-3 py-2.5 rounded-xl bg-orange-500 hover:bg-orange-600 text-white text-sm font-bold transition",
                        onclick: move |_| {
                            let current = *show_direct_chats.read();
                            show_direct_chats.set(!current);
                        },
                        span { "💬" }
                        {
                            let showing = *show_direct_chats.read();
                            let unread = *unread_count.read();
                            rsx! {
                                span { class: "flex-1 text-left",
                                    if showing { "Masquer discussions" } else { "Mes discussions" }
                                }
                                if unread > 0 {
                                    span {
                                        class: "bg-red-500 text-white text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center flex-shrink-0",
                                        "{unread}"
                                    }
                                }
                            }
                        }
                    }

                    // Liste DM
                    if *show_direct_chats.read() {
                        div { class: "space-y-1",
                            {
                                let chats = my_direct_chats.read().clone();
                                if chats.is_empty() {
                                    rsx! {
                                        p { class: "text-xs text-gray-500 dark:text-gray-400 px-2 italic", "Aucune conversation" }
                                    }
                                } else {
                                    rsx! {
                                        for chat_user in chats.iter() {
                                            {
                                                let user = chat_user.clone();
                                                let is_active = direct_chat_with.read().as_ref() == Some(&user);
                                                rsx! {
                                                    button {
                                                        key: "dm-{user}",
                                                        class: if is_active {
                                                            "w-full text-left px-3 py-2 rounded-lg bg-purple-700 hover:bg-purple-600 transition text-sm text-white"
                                                        } else {
                                                            "w-full text-left px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition text-sm text-gray-500 dark:text-gray-400"
                                                        },
                                                        onclick: move |_| {
                                                            direct_chat_with.set(Some(user.clone()));
                                                            messages.set(Vec::new());
                                                            show_channels_mobile.set(false);
                                                        },
                                                        "💬 {user}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Découvrir
                    button {
                        class: "w-full flex items-center gap-2 px-3 py-2.5 rounded-xl bg-orange-500 hover:bg-orange-600 text-white text-sm font-bold transition",
                        onclick: move |_| {
                            show_discover_modal.set(true);
                            direct_chat_with.set(None);
                            spawn(async move {
                                let client = Client::new();
                                let token = LocalStorage::get::<String>("jwt").unwrap_or_default();
                                if let Ok(res) = client
                                    .get(format!("{}/api/channels/discover", API_BASE_URL))
                                    .header("Authorization", format!("Bearer {}", token))
                                    .send().await
                                {
                                    if let Ok(list) = res.json::<Vec<ChannelWithStats>>().await {
                                        discover_channels.set(list);
                                    }
                                }
                            });
                        },
                        span { "🔍" }
                        span { "Découvrir" }
                    }

                    // Nouveau Salon (admin only)
                    if is_admin {
                        button {
                            class: "w-full flex items-center gap-2 px-3 py-2.5 rounded-xl bg-green-600 hover:bg-green-500 text-white text-sm font-bold transition",
                            onclick: move |_| show_create_modal.set(true),
                            span { "+" }
                            span { "Nouveau Salon" }
                        }
                    }
                }
            }
        }
    }
}
