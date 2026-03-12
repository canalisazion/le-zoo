use dioxus::prelude::*;
use shared::{Role, MessageStatus};
use crate::views::chat::ChatMessage;
use crate::config::API_BASE_URL;
use chrono::{DateTime, Utc, Datelike};
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;

#[component]
pub fn MessagesView(messages: Signal<Vec<ChatMessage>>, current_user: Signal<String>, is_admin: bool) -> Element {
    rsx! {
        div { class: "flex-1 p-6 pb-24 md:pb-6 overflow-y-auto scrollbar-custom", id: "messages-container",
            {
                let msgs = messages.read();
                let user = current_user.read().clone();
                let mut last_date: Option<(i32, u32, u32)> = None;

                rsx! {
                    for (i, msg) in msgs.iter().enumerate() {
                        {
                            let author = msg.author.clone();
                            let is_ai = msg.role == Role::Ai;

                            let badge = match &msg.role {
                                Role::SuperAdmin => "👑 ",
                                Role::Admin => "🛡️ ",
                                Role::User => "⚔️ ",
                                Role::Ai => "✨ ",
                            };

                            let name_color = if is_ai {
                                "text-sm font-semibold text-purple-300"
                            } else if msg.gender == "femme" {
                                "text-sm font-semibold text-rose-500"
                            } else {
                                "text-sm font-semibold text-blue-400"
                            };

                            let is_mine = msg.is_direct && msg.author == user;
                            let bg_color = if is_ai {
                                "bg-indigo-950 border border-yellow-500"
                            } else if msg.is_direct {
                                if is_mine {
                                    "bg-blue-600 border border-blue-500"
                                } else {
                                    "bg-slate-800 border border-slate-700"
                                }
                            } else {
                                ""
                            };
                            let bg_style = if is_ai || msg.is_direct { "" } else { "background:var(--bg-card);" };
                            let opacity = if msg.pending { "opacity-50" } else { "opacity-100" };
                            let parts: Vec<String> = msg.content.split_whitespace().map(|s| s.to_string()).collect();

                            let dt = DateTime::<Utc>::from_timestamp(msg.created_at, 0).unwrap_or_else(|| Utc::now());
                            let current_date = (dt.year(), dt.month(), dt.day());
                            let show_separator = if let Some(last) = last_date {
                                last != current_date
                            } else {
                                true
                            };
                            last_date = Some(current_date);
                            let date_str = dt.format("%d/%m/%Y").to_string();

                            // Detect RSS card (content contains \x00 separator)
                            let is_rss = msg.content.contains('\x00');

                            rsx! {
                                if show_separator {
                                    div { class: "text-center text-xs text-gray-500 my-4", "{date_str}" }
                                }
                                div {
                                    key: "{i}",
                                    class: "mb-4",

                                    // Ligne Auteur
                                    div { class: "flex items-center gap-1 mb-1 ml-1",
                                        if msg.pinned {
                                            span { class: "text-yellow-400 text-xs mr-1", "📌" }
                                        }
                                        if !badge.is_empty() {
                                            span { class: "text-sm", "{badge}" }
                                        }
                                        span { class: "{name_color}", "{author}" }
                                        if msg.is_direct {
                                            span { class: "text-xs text-purple-300 ml-2", "(Privé)" }
                                        }
                                        if msg.is_direct && !msg.pending && is_mine {
                                            {
                                                let (color, is_double) = match &msg.status {
                                                    MessageStatus::Sent => ("text-gray-400", false),
                                                    MessageStatus::Delivered => ("text-gray-400", true),
                                                    MessageStatus::Read => ("text-blue-400", true),
                                                };
                                                if is_double {
                                                    rsx! {
                                                        span { class: "{color} text-xs ml-1",
                                                            svg {
                                                                xmlns: "http://www.w3.org/2000/svg",
                                                                view_box: "0 0 16 15",
                                                                width: "16",
                                                                height: "15",
                                                                fill: "currentColor",
                                                                path { d: "M15.01 3.316l-.478-.372a.365.365 0 0 0-.51.063L8.666 9.879a.32.32 0 0 1-.484.033l-.358-.325a.319.319 0 0 0-.484.032l-.378.483a.418.418 0 0 0 .036.541l1.32 1.266c.143.14.361.125.484-.033l6.272-8.048a.366.366 0 0 0-.064-.512zm-4.1 0l-.478-.372a.365.365 0 0 0-.51.063L4.566 9.879a.32.32 0 0 1-.484.033L1.891 7.769a.366.366 0 0 0-.515.006l-.423.433a.364.364 0 0 0 .006.514l3.258 3.185c.143.14.361.125.484-.033l6.272-8.048a.365.365 0 0 0-.063-.51z" }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    rsx! {
                                                        span { class: "{color} text-xs ml-1",
                                                            svg {
                                                                xmlns: "http://www.w3.org/2000/svg",
                                                                view_box: "0 0 16 15",
                                                                width: "16",
                                                                height: "15",
                                                                fill: "currentColor",
                                                                path { d: "M10.91 3.316l-.478-.372a.365.365 0 0 0-.51.063L4.566 9.879a.32.32 0 0 1-.484.033L1.891 7.769a.366.366 0 0 0-.515.006l-.423.433a.364.364 0 0 0 .006.514l3.258 3.185c.143.14.361.125.484-.033l6.272-8.048a.365.365 0 0 0-.063-.51z" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Boutons admin
                                    if is_admin && msg.id.is_some() {
                                        {
                                            let id_del = msg.id.clone().unwrap_or_default();
                                            let id_pin = msg.id.clone().unwrap_or_default();
                                            rsx! {
                                                div { class: "flex gap-1 ml-2 mb-1",
                                                    button {
                                                        class: "text-xs text-red-400 hover:text-red-300 px-2 py-0.5 rounded hover:bg-red-900/20 transition",
                                                        title: "Supprimer",
                                                        onclick: move |_| {
                                                            let id = id_del.clone();
                                                            spawn(async move {
                                                                let client = Client::new();
                                                                let token = LocalStorage::get::<String>("jwt").unwrap_or_default();
                                                                let _ = client.delete(format!("{}/api/messages/{}", API_BASE_URL, id))
                                                                    .header("Authorization", format!("Bearer {}", token))
                                                                    .send().await;
                                                            });
                                                        },
                                                        "🗑"
                                                    }
                                                    button {
                                                        class: "text-xs text-yellow-400 hover:text-yellow-300 px-2 py-0.5 rounded hover:bg-yellow-900/20 transition",
                                                        title: "Épingler / Désépingler",
                                                        onclick: move |_| {
                                                            let id = id_pin.clone();
                                                            spawn(async move {
                                                                let client = Client::new();
                                                                let token = LocalStorage::get::<String>("jwt").unwrap_or_default();
                                                                let _ = client.post(format!("{}/api/messages/{}/pin", API_BASE_URL, id))
                                                                    .header("Authorization", format!("Bearer {}", token))
                                                                    .send().await;
                                                            });
                                                        },
                                                        "📌"
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Bulle de message — RSS card ou bulle normale
                                    if is_rss {
                                        {
                                            let content_str = msg.content.clone();
                                            let mut iter = content_str.splitn(2, '\x00');
                                            let titre_with_source = iter.next().unwrap_or("").to_string();
                                            let lien = iter.next().unwrap_or("").trim().to_string();
                                            let sep = " \u{2014} ";
                                            let (titre_display, source) = if let Some(pos) = titre_with_source.rfind(sep) {
                                                (titre_with_source[..pos].to_string(), titre_with_source[pos + sep.len()..].to_string())
                                            } else {
                                                (titre_with_source, "News".to_string())
                                            };
                                            rsx! {
                                                div {
                                                    class: "{opacity} border border-orange-500/30 rounded-xl p-4 cursor-pointer hover:border-orange-500/60 transition max-w-2xl ml-2",
                                                style: "background:var(--bg-card);",
                                                    onclick: move |_| {
                                                        let url = lien.clone();
                                                        if url.starts_with("https://") {
                                                            if let Some(window) = web_sys::window() {
                                                                let _ = window.open_with_url_and_target(&url, "_blank");
                                                            }
                                                        }
                                                    },
                                                    div { class: "flex items-center gap-2 mb-2",
                                                        span { class: "text-orange-400 text-xs font-bold uppercase", "{source}" }
                                                        span { class: "text-gray-500 text-xs", "• RustChat News" }
                                                    }
                                                    p { class: "font-medium leading-snug", style: "color:var(--text-main);", "{titre_display}" }
                                                    div { class: "mt-2 text-orange-400/60 text-xs", "Lire l'article →" }
                                                }
                                            }
                                        }
                                    } else {
                                        div {
                                            class: "{bg_color} {opacity} p-3 rounded-xl max-w-2xl shadow-md ml-2 break-words",
                                            style: "{bg_style}color:var(--text-main);",
                                            for (j, part) in parts.iter().enumerate() {
                                                {
                                                    if part.starts_with("http") {
                                                        if part.contains("youtube.com") || part.contains("youtu.be") {
                                                            if let Some(video_id) = extract_youtube_id(part) {
                                                                let embed = format!("https://www.youtube.com/embed/{}", video_id);
                                                                rsx! {
                                                                    div { key: "yt-{j}", class: "w-full mt-2 mb-2",
                                                                        iframe {
                                                                            src: "{embed}",
                                                                            class: "w-full max-w-lg aspect-video rounded-lg border border-gray-200 dark:border-gray-700",
                                                                            allow: "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture",
                                                                            allowfullscreen: true
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! { a { key: "lnk-{j}", href: "{part}", target: "_blank", rel: "noopener noreferrer", class: "text-blue-400 underline mr-1 break-all", "{part} " } }
                                                            }
                                                        } else if is_image_url(part) {
                                                            rsx! {
                                                                a { key: "img-{j}", href: "{part}", target: "_blank", rel: "noopener noreferrer", class: "block mt-1 mb-1",
                                                                    img {
                                                                        src: "{part}",
                                                                        class: "max-w-full md:max-w-sm rounded-lg hover:opacity-90 transition border border-gray-200 dark:border-gray-700",
                                                                        alt: "Image"
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            rsx! { a { key: "lnk-{j}", href: "{part}", target: "_blank", rel: "noopener noreferrer", class: "text-blue-400 underline mr-1 break-all", "{part} " } }
                                                        }
                                                    } else {
                                                        rsx! { span { key: "txt-{j}", class: "mr-1", "{part} " } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn is_image_url(url: &str) -> bool {
    let u = url.to_lowercase();
    u.ends_with(".jpg") || u.ends_with(".jpeg") || u.ends_with(".png") ||
    u.ends_with(".gif") || u.ends_with(".webp") || u.contains("pbs.twimg.com")
}

fn extract_youtube_id(url: &str) -> Option<String> {
    let clean_url = url.trim_end_matches(|c| c == ')' || c == ',' || c == '.' || c == ';');

    if clean_url.contains("youtu.be/") {
        clean_url.split("youtu.be/").nth(1).map(|s| s.split('?').next().unwrap_or("").to_string())
    } else if clean_url.contains("watch?v=") {
        clean_url.split("watch?v=").nth(1).map(|s| s.split('&').next().unwrap_or("").to_string())
    } else {
        None
    }
}
