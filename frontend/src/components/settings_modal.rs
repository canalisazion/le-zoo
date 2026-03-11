use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use crate::config::API_BASE_URL;

#[component]
pub fn SettingsModal(
    mut show_settings: Signal<bool>,
    mut show_cgu: Signal<bool>,
    mut show_confidentialite: Signal<bool>,
) -> Element {
    rsx! {
        if *show_settings.read() {
            // Backdrop semi-transparent — clic ferme
            div {
                class: "fixed inset-0 bg-black bg-opacity-40 z-50",
                onclick: move |_| show_settings.set(false),

                // Panneau latéral droit
                div {
                    class: "absolute inset-y-0 right-0 w-72 sm:w-80 border-l shadow-2xl flex flex-col",
                    style: "background:var(--bg-main);border-color:var(--border);",
                    onclick: move |e| e.stop_propagation(),

                    // Header
                    div { class: "flex items-center justify-between px-5 py-4 border-b", style: "border-color:var(--border);",
                        h2 { class: "font-bold text-base", style: "color:var(--text-main);", "Paramètres" }
                        button {
                            class: "p-1.5 hover:bg-gray-700 rounded-lg text-gray-400 hover:text-white transition",
                            onclick: move |_| show_settings.set(false),
                            "✕"
                        }
                    }

                    // Contenu scrollable
                    div { class: "flex-1 overflow-y-auto px-4 py-5 space-y-5",
                        div {
                            p { class: "text-[11px] uppercase font-semibold mb-2 px-1", style: "color:var(--text-muted);", "Sécurité" }
                            div { class: "space-y-1",
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg transition text-sm flex items-center gap-3",
                                    style: "color:var(--text-main);",
                                    span { "🔑" }
                                    span { "Changer mon mot de passe" }
                                }
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg hover:bg-red-900/30 transition text-sm text-red-400 flex items-center gap-3",
                                    onclick: move |_| {
                                        if gloo_dialogs::confirm("Supprimer définitivement votre compte ?") {
                                            spawn(async move {
                                                let client = Client::new();
                                                let token = LocalStorage::get::<String>("jwt").unwrap_or_default();
                                                if client.delete(format!("{}/api/users/delete", API_BASE_URL))
                                                    .header("Authorization", format!("Bearer {}", token))
                                                    .send().await.is_ok()
                                                {
                                                    LocalStorage::delete("jwt");
                                                    LocalStorage::delete("username");
                                                    LocalStorage::delete("role");
                                                    let _ = dioxus::document::eval("window.location.href = '/'");
                                                }
                                            });
                                        }
                                    },
                                    span { "🗑️" }
                                    span { "Supprimer mon compte" }
                                }
                            }
                        }

                        div { class: "border-t", style: "border-color:var(--border);" }

                        div {
                            p { class: "text-[11px] uppercase font-semibold mb-2 px-1", style: "color:var(--text-muted);", "Légal" }
                            div { class: "space-y-1",
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg transition text-sm flex items-center gap-3",
                                    style: "color:var(--text-main);",
                                    onclick: move |_| {
                                        show_settings.set(false);
                                        show_cgu.set(true);
                                    },
                                    span { "📜" }
                                    span { "Conditions d'utilisation" }
                                }
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg transition text-sm flex items-center gap-3",
                                    style: "color:var(--text-main);",
                                    onclick: move |_| {
                                        show_settings.set(false);
                                        show_confidentialite.set(true);
                                    },
                                    span { "🔒" }
                                    span { "Politique de confidentialité" }
                                }
                            }
                        }
                    }

                    // Pied — déconnexion
                    div { class: "px-4 py-4 border-t", style: "border-color:var(--border);",
                        button {
                            class: "w-full py-3 rounded-xl bg-blue-600 hover:bg-blue-500 transition font-semibold text-sm flex items-center justify-center gap-2",
                            onclick: move |_| {
                                LocalStorage::delete("jwt");
                                LocalStorage::delete("username");
                                LocalStorage::delete("role");
                                let _ = dioxus::document::eval("window.location.href = '/login'");
                            },
                            span { "🚪" }
                            span { "Se déconnecter" }
                        }
                    }
                }
            }
        }
    }
}
