use dioxus::prelude::*;
use dioxus::events::KeyboardEvent;
use gloo_storage::{LocalStorage, Storage};
use shared::{WsClientMsg, Role, MessageStatus};
use js_sys;

#[component]
pub fn MessageInput(
    mut draft: Signal<String>,
    current_channel: Signal<String>,
    direct_chat_with: Signal<Option<String>>,
    username: Signal<String>,
    user_role: Signal<Role>,
    mut messages: Signal<Vec<crate::views::chat::ChatMessage>>,
    mut pending_send: Signal<Option<(String, String)>>,
) -> Element {
    rsx! {
        div {
            class: "flex-shrink-0 p-3 flex gap-2",
            style: "position:sticky;bottom:0;z-index:10;background:var(--bg-main);border-top:1px solid var(--border);",
            input {
                class: "flex-1 rounded-lg px-3 py-2 text-sm outline-none focus:border-orange-500 border",
                style: "background:var(--bg-card);color:var(--text-main);border-color:var(--border);",
                placeholder: "Écrivez votre message...",
                value: "{draft}",
                oninput: move |evt| draft.set(evt.value()),
                onkeydown: move |evt: KeyboardEvent| {
                    if evt.key() == Key::Enter && !draft.read().is_empty() {
                        let content = draft.read().clone();
                        let ch = current_channel.read().clone();
                        let direct_target = direct_chat_with.read().clone();

                        let json = if let Some(target) = direct_target {
                            serde_json::to_string(&WsClientMsg::DirectMessage { to: target, content: content.clone() }).ok()
                        } else {
                            serde_json::to_string(&WsClientMsg::Chat { content: content.clone() }).ok()
                        };

                        if let Some(json_str) = json {
                            pending_send.set(Some((ch, json_str)));

                            let my_username = username.read().clone();
                            let my_role = user_role.read().clone();
                            let my_gender = LocalStorage::get::<String>("gender").unwrap_or_default();
                            if direct_chat_with.read().is_some() {
                                messages.with_mut(|v| v.push(crate::views::chat::ChatMessage {
                                    author: my_username,
                                    content,
                                    role: my_role,
                                    gender: my_gender,
                                    is_direct: true,
                                    pending: true,
                                    created_at: js_sys::Date::now() as i64 / 1000,
                                    status: MessageStatus::Sent,
                                    id: None,
                                    pinned: false,
                                }));
                            } else {
                                messages.with_mut(|v| v.push(crate::views::chat::ChatMessage {
                                    author: my_username,
                                    content,
                                    role: my_role,
                                    gender: my_gender,
                                    is_direct: false,
                                    pending: false,
                                    created_at: js_sys::Date::now() as i64 / 1000,
                                    status: MessageStatus::Sent,
                                    id: None,
                                    pinned: false,
                                }));
                            }
                        }
                        draft.set(String::new());
                    }
                }
            }
            button {
                class: "bg-blue-600 hover:bg-blue-500 px-4 py-2 rounded-lg font-bold transition text-lg",
                onclick: move |_| {
                    if !draft.read().is_empty() {
                        let content = draft.read().clone();
                        let ch = current_channel.read().clone();
                        let direct_target = direct_chat_with.read().clone();

                        let json = if let Some(target) = direct_target {
                            serde_json::to_string(&WsClientMsg::DirectMessage { to: target, content: content.clone() }).ok()
                        } else {
                            serde_json::to_string(&WsClientMsg::Chat { content: content.clone() }).ok()
                        };

                        if let Some(json_str) = json {
                            pending_send.set(Some((ch, json_str)));

                            let my_username = username.read().clone();
                            let my_role = user_role.read().clone();
                            let my_gender = LocalStorage::get::<String>("gender").unwrap_or_default();
                            if direct_chat_with.read().is_some() {
                                messages.with_mut(|v| v.push(crate::views::chat::ChatMessage {
                                    author: my_username,
                                    content,
                                    role: my_role,
                                    gender: my_gender,
                                    is_direct: true,
                                    pending: true,
                                    created_at: js_sys::Date::now() as i64 / 1000,
                                    status: MessageStatus::Sent,
                                    id: None,
                                    pinned: false,
                                }));
                            } else {
                                messages.with_mut(|v| v.push(crate::views::chat::ChatMessage {
                                    author: my_username,
                                    content,
                                    role: my_role,
                                    gender: my_gender,
                                    is_direct: false,
                                    pending: false,
                                    created_at: js_sys::Date::now() as i64 / 1000,
                                    status: MessageStatus::Sent,
                                    id: None,
                                    pinned: false,
                                }));
                            }
                        }
                        draft.set(String::new());
                    }
                },
                "➤"
            }
        }
    }
}
