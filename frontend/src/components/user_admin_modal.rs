use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use shared::{Role, MemberInfo};
use crate::config::API_BASE_URL;
use crate::fetch_creds::WithCredentials; // ✅ [H-8]

#[component]
pub fn UserAdminModal(
    mut popup_user: Signal<Option<String>>,
    members: Signal<Vec<MemberInfo>>,
    mut members_version: Signal<u32>,
    is_admin: bool,
    is_super: bool,
    mut my_direct_chats: Signal<Vec<String>>,
    mut direct_chat_with: Signal<Option<String>>,
    messages: Signal<Vec<crate::views::chat::ChatMessage>>,
    mut show_members_mobile: Signal<bool>,
) -> Element {
    rsx! {
        {
            let popup_target = popup_user.read().clone();
            if let Some(target) = popup_target {
                let target_display = target.clone();
                let target_role = members.read().iter()
                    .find(|m| m.username == target)
                    .map(|m| m.role.clone())
                    .unwrap_or(Role::User);
                let show_promote_admin = !matches!(target_role, Role::Admin);
                let show_promote_super = is_super && !matches!(target_role, Role::SuperAdmin);
                let show_demote = !matches!(target_role, Role::User);
                let t1 = target.clone();
                let t2 = target.clone();
                let t3 = target.clone();
                let t4 = target.clone();

                rsx! {
                    div {
                        class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                        onclick: move |_| popup_user.set(None),

                        div {
                            class: "bg-gray-800 p-6 rounded-xl shadow-2xl w-80 border border-gray-700",
                            onclick: move |e| e.stop_propagation(),

                            h3 { class: "text-lg font-bold mb-4 text-blue-400", "Actions : {target_display}" }

                            div { class: "space-y-2",
                                button {
                                    class: "w-full text-left px-4 py-2 rounded-lg hover:bg-gray-700 text-purple-400 transition",
                                    onclick: move |_| {
                                        let target = target_display.clone();
                                        my_direct_chats.with_mut(|chats| {
                                            if !chats.contains(&target) {
                                                chats.push(target.clone());
                                                let _ = LocalStorage::set("my_direct_chats", chats.clone());
                                            }
                                        });
                                        direct_chat_with.set(Some(target));
                                        messages.set(Vec::new());
                                        popup_user.set(None);
                                        show_members_mobile.set(false);
                                    },
                                    "💬 Message privé"
                                }

                                div { class: "border-t border-gray-700 my-1" }

                                if is_admin && show_promote_admin {
                                    button {
                                        class: "w-full text-left px-4 py-2 rounded-lg hover:bg-gray-700 text-yellow-400 transition",
                                        onclick: move |_| {
                                            let t = t1.clone();
                                            spawn(async move {
                                                let client = Client::new();
                                                let _ = client.post(format!("{}/api/users/promote", API_BASE_URL))
                                                    .with_credentials() // ✅ [H-8]
                                                    .json(&serde_json::json!({"username": t, "role": "admin"}))
                                                    .send().await;
                                                popup_user.set(None);
                                                members_version.set(members_version() + 1);
                                            });
                                        },
                                        "⭐ Mettre Admin"
                                    }
                                }

                                if show_promote_super {
                                    button {
                                        class: "w-full text-left px-4 py-2 rounded-lg hover:bg-gray-700 text-purple-400 transition",
                                        onclick: move |_| {
                                            let t = t2.clone();
                                            spawn(async move {
                                                let client = Client::new();
                                                let _ = client.post(format!("{}/api/users/promote", API_BASE_URL))
                                                    .with_credentials() // ✅ [H-8]
                                                    .json(&serde_json::json!({"username": t, "role": "super_admin"}))
                                                    .send().await;
                                                popup_user.set(None);
                                                members_version.set(members_version() + 1);
                                            });
                                        },
                                        "👑 Mettre SuperAdmin"
                                    }
                                }

                                if is_admin && show_demote {
                                    button {
                                        class: "w-full text-left px-4 py-2 rounded-lg hover:bg-gray-700 text-gray-400 transition",
                                        onclick: move |_| {
                                            let t = t3.clone();
                                            spawn(async move {
                                                let client = Client::new();
                                                let _ = client.post(format!("{}/api/users/promote", API_BASE_URL))
                                                    .with_credentials() // ✅ [H-8]
                                                    .json(&serde_json::json!({"username": t, "role": "user"}))
                                                    .send().await;
                                                popup_user.set(None);
                                                members_version.set(members_version() + 1);
                                            });
                                        },
                                        "Rétrograder User"
                                    }
                                }

                                if is_admin {
                                    div { class: "border-t border-gray-700 my-1" }

                                    button {
                                        class: "w-full text-left px-4 py-2 rounded-lg hover:bg-red-900 text-red-400 transition",
                                        onclick: move |_| {
                                            let t = t4.clone();
                                            spawn(async move {
                                                let client = Client::new();
                                                let _ = client.post(format!("{}/api/users/ban", API_BASE_URL))
                                                    .with_credentials() // ✅ [H-8]
                                                    .json(&serde_json::json!({"username": t}))
                                                    .send().await;
                                                popup_user.set(None);
                                                members_version.set(members_version() + 1);
                                            });
                                        },
                                        "🚫 Bannir"
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
    }
}
