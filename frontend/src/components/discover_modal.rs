use dioxus::prelude::*;
use reqwest::Client;
use shared::ChannelWithStats;
use crate::config::API_BASE_URL;
use crate::fetch_creds::WithCredentials; // ✅ [H-8]

#[component]
pub fn DiscoverModal(
    mut show_discover_modal: Signal<bool>,
    discover_channels: Signal<Vec<ChannelWithStats>>,
    mut subscribed_channels: Signal<Vec<String>>,
) -> Element {
    let mut selected = use_signal(|| Option::<ChannelWithStats>::None);

    rsx! {
        if *show_discover_modal.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-60 flex items-end sm:items-center justify-center z-50",
                onclick: move |_| { show_discover_modal.set(false); selected.set(None); },

                div {
                    class: "w-full sm:max-w-lg rounded-t-2xl sm:rounded-2xl shadow-2xl border max-h-[90vh] flex flex-col",
                    style: "background:var(--bg-main);border-color:var(--border);",
                    onclick: move |e| e.stop_propagation(),

                    // Header
                    div { class: "flex items-center justify-between px-4 py-3 border-b flex-shrink-0",
                        style: "border-color:var(--border);",
                        {
                            let is_detail = selected.read().is_some();
                            if is_detail {
                                rsx! {
                                    button {
                                        class: "flex items-center gap-1 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition text-sm font-semibold py-1",
                                        onclick: move |_| selected.set(None),
                                        "← Retour"
                                    }
                                    button {
                                        class: "p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg text-gray-500 dark:text-gray-400",
                                        onclick: move |_| { show_discover_modal.set(false); selected.set(None); },
                                        "✕"
                                    }
                                }
                            } else {
                                rsx! {
                                    h2 { class: "text-lg font-bold text-gray-900 dark:text-gray-100", "🔍 Découvrir" }
                                    button {
                                        class: "p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg text-gray-500 dark:text-gray-400",
                                        onclick: move |_| show_discover_modal.set(false),
                                        "✕"
                                    }
                                }
                            }
                        }
                    }

                    // Body
                    div { class: "flex-1 overflow-y-auto scrollbar-custom",
                        {
                            let sel = selected.read().clone();
                            let subs = subscribed_channels.read().clone();

                            if let Some(ch_stat) = sel {
                                let ch = ch_stat.channel.clone();
                                let channel_id = ch.id.clone().unwrap_or_default();
                                let icon = ch.icon.clone().unwrap_or_else(|| "💬".to_string());
                                let is_subscribed = subs.contains(&channel_id);
                                let media_tag = ch_stat.media_tag.clone();
                                let ch_id_detail = channel_id.clone();

                                rsx! {
                                    div { class: "p-6 flex flex-col items-center",
                                        div { class: "text-6xl mb-4", "{icon}" }
                                        h3 { class: "text-xl font-bold mb-2 text-center text-gray-900 dark:text-gray-100", "{ch.name}" }
                                        p { class: "text-sm text-center mb-5 max-w-xs text-gray-500 dark:text-gray-400", "{ch.description}" }
                                        if let Some(tag) = media_tag {
                                            span { class: "text-xs bg-indigo-900 text-indigo-200 px-3 py-1 rounded-full mb-6", "{tag}" }
                                        } else {
                                            span {}
                                        }
                                        button {
                                            class: if is_subscribed {
                                                "w-full max-w-xs py-3 rounded-xl bg-green-600 hover:bg-green-500 font-bold text-white transition"
                                            } else {
                                                "w-full max-w-xs py-3 rounded-xl bg-blue-600 hover:bg-blue-500 font-bold text-white transition"
                                            },
                                            onclick: move |_| {
                                                let cid = ch_id_detail.clone();
                                                let subscribe = !is_subscribed;
                                                spawn(async move {
                                                    let client = Client::new();
                                                    let _ = client.post(format!("{}/api/users/channels/subscribe", API_BASE_URL))
                                                        .with_credentials() // ✅ [H-8]
                                                        .json(&serde_json::json!({ "channel_id": cid, "subscribe": subscribe }))
                                                        .send().await;
                                                    subscribed_channels.with_mut(|s| {
                                                        if subscribe {
                                                            if !s.contains(&cid) { s.push(cid.clone()); }
                                                        } else {
                                                            s.retain(|id| id != &cid);
                                                        }
                                                    });
                                                });
                                            },
                                            if is_subscribed { "✓ Abonné" } else { "Rejoindre ce canal" }
                                        }
                                    }
                                }
                            } else {
                                let all_channels = discover_channels.read().clone();
                                let visible: Vec<ChannelWithStats> = all_channels
                                    .into_iter()
                                    .filter(|cs| !cs.channel.is_gold)
                                    .collect();

                                rsx! {
                                    div { class: "p-3 space-y-2",
                                        if visible.is_empty() {
                                            div { class: "text-center text-gray-500 py-12", "Chargement..." }
                                        } else {
                                            div {}
                                        }
                                        for cs in visible.into_iter() {
                                            {
                                                let ch = cs.channel.clone();
                                                let channel_id = ch.id.clone().unwrap_or_default();
                                                let icon = ch.icon.clone().unwrap_or_else(|| "💬".to_string());
                                                let is_subscribed = subs.contains(&channel_id);
                                                let media_tag = cs.media_tag.clone();
                                                let cs_click = cs.clone();
                                                let ch_id_btn = channel_id.clone();

                                                rsx! {
                                                    div {
                                                        key: "discover-{channel_id}",
                                                        class: "border border-gray-200 dark:border-gray-800 bg-white dark:bg-[#0a0a0a] rounded-xl p-4 flex items-center gap-3 cursor-pointer hover:border-orange-500/40 transition",
                                                        onclick: move |_| selected.set(Some(cs_click.clone())),

                                                        span { class: "text-3xl flex-shrink-0", "{icon}" }
                                                        div { class: "flex-1 min-w-0",
                                                            p { class: "font-semibold truncate text-gray-900 dark:text-gray-100", "{ch.name}" }
                                                            p { class: "text-sm truncate text-gray-500 dark:text-gray-400", "{ch.description}" }
                                                            if let Some(tag) = media_tag {
                                                                span { class: "text-[10px] bg-indigo-900 text-indigo-200 px-2 py-0.5 rounded-full mt-1 inline-block", "{tag}" }
                                                            } else {
                                                                span {}
                                                            }
                                                        }
                                                        button {
                                                            class: if is_subscribed {
                                                                "flex-shrink-0 w-10 h-10 rounded-xl bg-green-700 hover:bg-green-600 text-white font-bold text-lg transition flex items-center justify-center"
                                                            } else {
                                                                "flex-shrink-0 w-10 h-10 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-bold text-lg transition flex items-center justify-center"
                                                            },
                                                            onclick: move |e| {
                                                                e.stop_propagation();
                                                                let cid = ch_id_btn.clone();
                                                                let subscribe = !is_subscribed;
                                                                spawn(async move {
                                                                    let client = Client::new();
                                                                    let _ = client.post(format!("{}/api/users/channels/subscribe", API_BASE_URL))
                                                                        .with_credentials() // ✅ [H-8]
                                                                        .json(&serde_json::json!({ "channel_id": cid, "subscribe": subscribe }))
                                                                        .send().await;
                                                                    subscribed_channels.with_mut(|s| {
                                                                        if subscribe {
                                                                            if !s.contains(&cid) { s.push(cid.clone()); }
                                                                        } else {
                                                                            s.retain(|id| id != &cid);
                                                                        }
                                                                    });
                                                                });
                                                            },
                                                            if is_subscribed { "✓" } else { "+" }
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
}
