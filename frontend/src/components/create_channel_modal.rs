use dioxus::prelude::*;
use reqwest::Client;
use shared::Channel;
use crate::config::API_BASE_URL;
use crate::fetch_creds::WithCredentials; // ✅ [H-8]
#[allow(unused_imports)]
use serde_json;

#[component]
pub fn CreateChannelModal(
    mut show_create_modal: Signal<bool>,
    mut new_channel_name: Signal<String>,
    mut new_channel_desc: Signal<String>,
    mut new_channel_emoji: Signal<String>,
    mut new_channel_media: Signal<String>,
    mut new_channel_topic: Signal<String>,
    mut channels: Signal<Vec<Channel>>,
) -> Element {

    rsx! {
        if *show_create_modal.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                onclick: move |_| show_create_modal.set(false),

                div {
                    class: "bg-gray-800 p-6 rounded-xl shadow-2xl w-96 border border-gray-700",
                    onclick: move |e| e.stop_propagation(),

                    h2 { class: "text-2xl font-bold mb-4", "Créer un nouveau salon" }

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm", "Nom du salon" }
                        input {
                            class: "w-full p-2 bg-gray-700 border border-gray-600 rounded-lg text-white",
                            placeholder: "ex: Vidéos, Gaming...",
                            value: "{new_channel_name}",
                            oninput: move |evt| new_channel_name.set(evt.value())
                        }
                    }

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm", "Description" }
                        input {
                            class: "w-full p-2 bg-gray-700 border border-gray-600 rounded-lg text-white",
                            placeholder: "Description du salon",
                            value: "{new_channel_desc}",
                            oninput: move |evt| new_channel_desc.set(evt.value())
                        }
                    }

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm", "Icône du salon" }
                        div { class: "grid grid-cols-5 gap-2",
                            {
                                let emojis = ["💬", "🔥", "💀", "🎬", "🥊", "⚽", "💸", "🔞", "🎵", "🚨"];
                                let selected = new_channel_emoji.read().clone();
                                rsx! {
                                    for emoji in emojis {
                                        {
                                            let e = emoji.to_string();
                                            let is_selected = selected == e;
                                            rsx! {
                                                button {
                                                    key: "{e}",
                                                    class: if is_selected {
                                                        "p-2 text-2xl bg-blue-600 rounded-lg hover:bg-blue-500 transition"
                                                    } else {
                                                        "p-2 text-2xl bg-gray-700 rounded-lg hover:bg-gray-600 transition"
                                                    },
                                                    onclick: move |ev| {
                                                        ev.stop_propagation();
                                                        new_channel_emoji.set(e.clone());
                                                    },
                                                    "{emoji}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm", "Texte du débat (optionnel)" }
                        textarea {
                            class: "w-full p-2 bg-gray-700 border border-gray-600 rounded-lg text-white",
                            placeholder: "Résumé du débat ou sujet...",
                            rows: "2",
                            value: "{new_channel_topic}",
                            oninput: move |evt| new_channel_topic.set(evt.value())
                        }
                    }

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm", "Média (URL YouTube, Image ou Site Web)" }
                        input {
                            class: "w-full p-2 bg-gray-700 border border-gray-600 rounded-lg text-white",
                            placeholder: "https://...",
                            value: "{new_channel_media}",
                            oninput: move |evt| new_channel_media.set(evt.value())
                        }
                        p { class: "text-xs text-gray-400 mt-1", "💡 YouTube, image ou n'importe quel site web" }
                    }

                    div { class: "flex gap-4",
                        button {
                            class: "flex-1 bg-gray-600 hover:bg-gray-500 py-2 rounded-lg",
                            onclick: move |_| {
                                show_create_modal.set(false);
                                new_channel_name.set(String::new());
                                new_channel_desc.set(String::new());
                                new_channel_emoji.set("💬".to_string());
                                new_channel_media.set(String::new());
                                new_channel_topic.set(String::new());
                            },
                            "Annuler"
                        }

                        button {
                            class: "flex-1 bg-green-600 hover:bg-green-500 py-2 rounded-lg font-bold",
                            onclick: move |_| {
                                let name = new_channel_name.read().clone();
                                let desc = new_channel_desc.read().clone();
                                let emoji = new_channel_emoji.read().clone();
                                let media = new_channel_media.read().clone();
                                let topic = new_channel_topic.read().clone();

                                if !name.is_empty() {
                                    spawn(async move {
                                        let client = Client::new();
                                        let id = name.to_lowercase().replace(" ", "_");
                                        let new_channel = Channel {
                                            id: Some(id),
                                            name: name,
                                            description: desc,
                                            channel_type: "text".to_string(),
                                            is_general: false,
                                            is_locked: false,
                                            is_gold: false,
                                            icon: Some(emoji),
                                            topic_media: if media.is_empty() { None } else { Some(media) },
                                            topic_text: if topic.is_empty() { None } else { Some(topic) },
                                            ai_description: None,
                                            topic: None,
                                            style_color: None,
                                            deleted: false,
                                        };

                                        if let Ok(res) = client.post(format!("{}/api/channels", API_BASE_URL))
                                            .with_credentials() // ✅ [H-8]
                                            .json(&new_channel)
                                            .send()
                                            .await
                                        {
                                            if res.status().is_success() {
                                                if let Ok(res) = client.get(format!("{}/api/channels", API_BASE_URL))
                                                    .with_credentials()
                                                    .send().await {
                                                    if let Ok(list) = res.json::<Vec<Channel>>().await {
                                                        channels.set(list);
                                                    }
                                                }
                                            }
                                        }
                                    });

                                    show_create_modal.set(false);
                                    new_channel_name.set(String::new());
                                    new_channel_desc.set(String::new());
                                    new_channel_emoji.set("💬".to_string());
                                    new_channel_media.set(String::new());
                                    new_channel_topic.set(String::new());
                                }
                            },
                            "Créer"
                        }
                    }
                }
            }
        }
    }
}
