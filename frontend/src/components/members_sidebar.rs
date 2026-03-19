use dioxus::prelude::*;
use shared::{Role, MemberInfo};
use gloo_storage::{LocalStorage, Storage};

#[component]
pub fn MembersSidebar(
    mut show_members_mobile: Signal<bool>,
    members: Signal<Vec<MemberInfo>>,
    username: Signal<String>,
    mut popup_user: Signal<Option<String>>,
) -> Element {
    let direct_contacts = use_signal(|| {
        let u = LocalStorage::get::<String>("username").unwrap_or_default();
        LocalStorage::get::<Vec<String>>(&format!("dms_{}", u)).unwrap_or_default()
    });
    rsx! {
        div {
            class: if *show_members_mobile.read() {
                "fixed inset-y-0 right-0 w-64 border-l flex flex-col flex-shrink-0 z-50 md:relative md:flex"
            } else {
                "hidden md:flex w-64 border-l flex-col flex-shrink-0"
            },
            style: "background:var(--bg-main);border-color:var(--border);",
            div { class: "p-4 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between",
                h2 { class: "text-xl font-bold text-gray-900 dark:text-gray-100", "Membres" }
                button {
                    class: "md:hidden p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded",
                    onclick: move |_| show_members_mobile.set(false),
                    "✕"
                }
            }

            div { class: "flex-1 overflow-y-auto scrollbar-custom p-2",
                {
                    let contacts = direct_contacts.read().clone();
                    if !contacts.is_empty() {
                        rsx! {
                            p { class: "text-xs text-purple-400 uppercase mb-2 px-2 font-semibold",
                                "💬 Vos conversations récentes"
                            }
                            for contact in contacts.iter() {
                                {
                                    let name = contact.clone();
                                    let name_click = contact.clone();
                                    let member_info = members.read().iter().find(|m| &m.username == contact).cloned();
                                    if let Some(m) = member_info {
                                        let avatar_opt = m.avatar.clone();
                                        let is_online = m.online;
                                        let name_class = if m.gender == "femme" {
                                            "text-rose-500 text-sm truncate"
                                        } else {
                                            "text-blue-300 text-sm truncate"
                                        };
                                        rsx! {
                                            button {
                                                key: "recent-{name}",
                                                class: "w-full text-left px-3 py-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 flex items-center gap-2 transition cursor-pointer mb-1",
                                                onclick: move |_| {
                                                    popup_user.set(Some(name_click.clone()));
                                                },
                                                div { class: "relative",
                                                    if let Some(avatar) = avatar_opt {
                                                        img {
                                                            src: "{avatar}",
                                                            class: "w-8 h-8 rounded-full object-cover"
                                                        }
                                                    } else {
                                                        div {
                                                            class: "w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-xs text-gray-700 dark:text-gray-300",
                                                            "{name.chars().next().unwrap_or('?').to_uppercase()}"
                                                        }
                                                    }
                                                }
                                                span {
                                                    class: if is_online { "w-2 h-2 rounded-full bg-green-400 flex-shrink-0" } else { "w-2 h-2 rounded-full bg-gray-400 dark:bg-gray-600 flex-shrink-0" },
                                                    style: if is_online { "box-shadow: 0 0 6px #4ade80;" } else { "" }
                                                }
                                                span { class: "{name_class}", "{name}" }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                            div { class: "border-t border-gray-200 dark:border-gray-800 my-3" }
                        }
                    } else {
                        rsx! {}
                    }
                }
                {
                    let current_user = username.read().clone();
                    let online_count = members.read().iter().filter(|m| m.online && m.username != current_user).count();
                    rsx! {
                        p { class: "text-xs text-gray-500 dark:text-gray-400 uppercase mb-2 px-2 font-semibold",
                            "En ligne — {online_count}"
                        }
                    }
                }
                for member in members.read().iter().filter(|m| m.online && m.username != *username.read()) {
                    {
                        let name_display = member.username.clone();
                        let name_click = member.username.clone();
                        let avatar_opt = member.avatar.clone();
                        let user_badges = member.badges.clone();
                        let badge = match member.role {
                            Role::Dictateur  => "⚜️",
                            Role::SuperAdmin => "👑",
                            Role::Admin      => "🛡️",
                            Role::User | Role::Ai => "",
                        };
                        let name_class = if member.gender == "femme" {
                            "text-pink-400 text-sm truncate"
                        } else {
                            "text-blue-300 text-sm truncate"
                        };

                        rsx! {
                            button {
                                key: "on-{name_display}",
                                class: "w-full text-left px-3 py-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 flex items-center gap-2 transition cursor-pointer",
                                onclick: move |_| {
                                    popup_user.set(Some(name_click.clone()));
                                },
                                div { class: "relative",
                                    if let Some(avatar) = avatar_opt {
                                        img {
                                            src: "{avatar}",
                                            class: "w-8 h-8 rounded-full object-cover"
                                        }
                                    } else {
                                        div {
                                            class: "w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center text-xs text-gray-700 dark:text-gray-300",
                                            "{name_display.chars().next().unwrap_or('?').to_uppercase()}"
                                        }
                                    }
                                    if !user_badges.is_empty() {
                                        div {
                                            class: "absolute -bottom-1 -right-1 flex gap-0.5",
                                            for badge_name in user_badges.iter() {
                                                {
                                                    let badge_emoji = match badge_name.as_str() {
                                                        "habitue" => "⭐",
                                                        "causeur" => "💬",
                                                        "decouvreur" => "🔍",
                                                        _ => "",
                                                    };
                                                    rsx! {
                                                        span {
                                                            class: "w-3 h-3 rounded-full bg-yellow-400 flex items-center justify-center text-[8px]",
                                                            "{badge_emoji}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                span {
                                    class: "w-2 h-2 rounded-full bg-green-400 flex-shrink-0",
                                    style: "box-shadow: 0 0 6px #4ade80;"
                                }
                                if !badge.is_empty() {
                                    span { class: "text-xs", "{badge}" }
                                }
                                span { class: "{name_class}", "{name_display}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
