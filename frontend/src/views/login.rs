use dioxus::prelude::*;
use reqwest::Client;
use shared::{User, Role};
use serde::Deserialize;
use crate::{Route, config::API_BASE_URL};
use gloo_storage::{LocalStorage, Storage};

#[derive(Deserialize, Debug)]
struct LoginResponse {
    token: String,
    username: String,
    role: String,
}

#[component]
pub fn Login() -> Element {
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut status = use_signal(|| String::new());
    let mut err_email = use_signal(|| String::new());
    let mut err_password = use_signal(|| String::new());

    let nav = use_navigator();

    let handle_login = move |_| async move {
        // Reset
        err_email.set(String::new());
        err_password.set(String::new());
        status.set(String::new());

        let mail = email.read().trim().to_string();
        let pwd = password.read().clone();

        // Client-side validation
        let mut has_error = false;
        if mail.is_empty() {
            err_email.set("❌ Email requis".to_string());
            has_error = true;
        }
        if pwd.is_empty() {
            err_password.set("❌ Mot de passe requis".to_string());
            has_error = true;
        }
        if has_error {
            return;
        }

        status.set("⏳ Vérification...".to_string());

        let login_data = User {
            id: None,
            username: String::new(),
            email: mail,
            password: Some(pwd),
            created_at: 0,
            role: Role::User,
            banned: false,
            gender: String::new(),
            subscribed_channels: Vec::new(),
            avatar: None,
            is_premium: false,
            message_count: 0,
            days_active: 0,
            channels_joined: 0,
        };

        let client = Client::new();
        let response = client.post(format!("{}/api/login", API_BASE_URL))
            .json(&login_data)
            .send()
            .await;

        match response {
            Ok(res) => {
                match res.status().as_u16() {
                    200 => {
                        if let Ok(data) = res.json::<LoginResponse>().await {
                            let _ = LocalStorage::set("jwt", &data.token);
                            let _ = LocalStorage::set("username", &data.username);
                            let _ = LocalStorage::set("role", &data.role);
                            nav.push(Route::Chat {});
                        }
                    }
                    401 => {
                        status.set("❌ Email ou mot de passe incorrect".to_string());
                    }
                    403 => {
                        status.set("🚫 Compte banni".to_string());
                    }
                    429 => {
                        status.set("⏳ Trop de tentatives, réessayez dans 1 minute".to_string());
                    }
                    _ => {
                        let code = res.status().as_u16();
                        status.set(format!("❌ Erreur ({})", code));
                    }
                }
            },
            Err(_) => {
                status.set("⚠️ Serveur injoignable".to_string());
            }
        }
    };

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen bg-gray-900 text-white font-sans",
            h1 { class: "text-4xl font-bold mb-8 text-gray-200", "Connexion" }

            div { class: "bg-gray-800 p-8 rounded-lg shadow-2xl w-96 border border-gray-700",
                if !status.read().is_empty() {
                    div {
                        class: "mb-4 p-3 rounded font-bold text-center border",
                        class: if status.read().starts_with("✅") {
                            "border-green-600 bg-green-900 text-green-300"
                        } else if status.read().starts_with("⏳") {
                            "border-gray-600 bg-gray-700 text-gray-300"
                        } else if status.read().starts_with("⚠️") {
                            "border-yellow-600 bg-yellow-900 text-yellow-300"
                        } else {
                            "border-red-600 bg-red-900 text-red-300"
                        },
                        "{status}"
                    }
                }

                div { class: "mb-4",
                    label { class: "block mb-2 text-sm text-gray-400", "Email" }
                    input {
                        class: if !err_email.read().is_empty() {
                            "w-full p-3 rounded bg-gray-900 border border-red-500 focus:border-red-400 outline-none transition text-white"
                        } else {
                            "w-full p-3 rounded bg-gray-900 border border-gray-600 text-white outline-none focus:border-blue-500"
                        },
                        value: "{email}",
                        oninput: move |evt| email.set(evt.value())
                    }
                    if !err_email.read().is_empty() {
                        p { class: "text-red-400 text-xs mt-1", "{err_email}" }
                    }
                }

                div { class: "mb-6",
                    div { class: "flex items-center justify-between mb-2",
                        label { class: "text-sm text-gray-400", "Mot de passe" }
                        Link {
                            to: Route::ForgotPassword {},
                            class: "text-xs text-blue-400 hover:underline",
                            "Mot de passe oublié ?"
                        }
                    }
                    input {
                        class: if !err_password.read().is_empty() {
                            "w-full p-3 rounded bg-gray-900 border border-red-500 focus:border-red-400 outline-none transition text-white"
                        } else {
                            "w-full p-3 rounded bg-gray-900 border border-gray-600 text-white outline-none focus:border-blue-500"
                        },
                        r#type: "password",
                        value: "{password}",
                        oninput: move |evt| password.set(evt.value())
                    }
                    if !err_password.read().is_empty() {
                        p { class: "text-red-400 text-xs mt-1", "{err_password}" }
                    }
                }

                button {
                    class: "w-full bg-blue-700 hover:bg-blue-600 py-3 rounded font-bold transition transform active:scale-95 text-white",
                    onclick: handle_login,
                    "Se connecter"
                }

                div { class: "mt-6 text-center text-sm text-gray-500",
                    "Pas de compte ? "
                    Link { to: Route::Register {}, class: "text-blue-400 hover:underline", "S'inscrire" }
                }
            }
        }
    }
}
