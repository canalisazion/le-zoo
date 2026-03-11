use dioxus::prelude::*;
use reqwest::Client;
use shared::{User, Role};
use crate::{Route, config::API_BASE_URL};

#[component]
pub fn Register() -> Element {
    let mut username = use_signal(|| String::new());
    let mut email = use_signal(|| String::new());
    let mut password = use_signal(|| String::new());
    let mut gender = use_signal(|| "homme".to_string());
    let mut message_status = use_signal(|| String::new());
    let mut err_username = use_signal(|| String::new());
    let mut err_email = use_signal(|| String::new());
    let mut err_password = use_signal(|| String::new());

    let handle_register = move |_| async move {
        // Reset errors
        err_username.set(String::new());
        err_email.set(String::new());
        err_password.set(String::new());
        message_status.set(String::new());

        // Client-side validation
        let uname = username.read().clone();
        let mail = email.read().clone();
        let pwd = password.read().clone();

        let mut has_error = false;
        if uname.trim().len() < 3 {
            err_username.set("❌ Pseudo requis (min 3 caractères)".to_string());
            has_error = true;
        }
        if !mail.contains('@') || !mail.contains('.') {
            err_email.set("❌ Email invalide".to_string());
            has_error = true;
        }
        if pwd.len() < 8 {
            err_password.set("❌ Minimum 8 caractères".to_string());
            has_error = true;
        }
        if has_error {
            return;
        }

        let new_user = User {
            id: None,
            username: uname,
            email: mail,
            password: Some(pwd),
            created_at: 0,
            role: Role::User,
            banned: false,
            gender: gender.read().clone(),
            subscribed_channels: Vec::new(),
            avatar: None,
            is_premium: false,
            message_count: 0,
            days_active: 0,
            channels_joined: 0,
        };

        let client = Client::new();
        match client.post(format!("{}/api/register", API_BASE_URL))
            .json(&new_user)
            .send()
            .await
        {
            Ok(res) => {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                match status.as_u16() {
                    200 | 201 => {
                        message_status.set("✅ Inscription réussie ! Connectez-vous.".to_string());
                    }
                    409 => {
                        if body.contains("nom d'utilisateur") || body.contains("utilisateur") {
                            err_username.set("❌ Ce pseudo est déjà pris".to_string());
                        } else if body.contains("email") || body.contains("Email") {
                            err_email.set("❌ Cet email est déjà utilisé".to_string());
                        } else {
                            message_status.set(format!("❌ {}", body));
                        }
                    }
                    400 => {
                        if body.contains("Username") || body.contains("utilisateur") {
                            err_username.set("❌ 3-20 caractères, lettres/chiffres/underscore uniquement".to_string());
                        } else if body.contains("Mot de passe") || body.contains("passe") {
                            err_password.set("❌ Minimum 8 caractères".to_string());
                        } else if body.contains("Email") || body.contains("email") {
                            err_email.set("❌ Email invalide".to_string());
                        } else {
                            message_status.set(format!("❌ {}", body));
                        }
                    }
                    _ => {
                        message_status.set(format!("❌ Erreur ({})", status.as_u16()));
                    }
                }
            },
            Err(_) => message_status.set("⚠️ Serveur injoignable".to_string()),
        }
    };

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen bg-gray-900 text-white font-sans",
            h1 { class: "text-4xl font-bold mb-8 text-gray-200", "Créer un compte" }

            div { class: "bg-gray-800 p-8 rounded-lg shadow-2xl w-96 border border-gray-700",
                if !message_status.read().is_empty() {
                    div {
                        class: "mb-4 p-3 rounded font-bold text-center border",
                        class: if message_status.read().starts_with("✅") {
                            "border-green-600 bg-green-900 text-green-300"
                        } else {
                            "border-red-600 bg-red-900 text-red-300"
                        },
                        "{message_status}"
                    }
                }

                div { class: "mb-4",
                    label { class: "block mb-2 text-sm text-gray-400", "Pseudo" }
                    input {
                        class: if !err_username.read().is_empty() {
                            "w-full p-3 rounded bg-gray-900 border border-red-500 focus:border-red-400 outline-none transition text-white"
                        } else {
                            "w-full p-3 rounded bg-gray-900 border border-gray-600 focus:border-gray-400 outline-none transition text-white"
                        },
                        placeholder: "Votre pseudo (3-20 caractères)",
                        value: "{username}",
                        oninput: move |evt| username.set(evt.value())
                    }
                    if !err_username.read().is_empty() {
                        p { class: "text-red-400 text-xs mt-1", "{err_username}" }
                    }
                }

                div { class: "mb-4",
                    label { class: "block mb-2 text-sm text-gray-400", "Email" }
                    input {
                        class: if !err_email.read().is_empty() {
                            "w-full p-3 rounded bg-gray-900 border border-red-500 focus:border-red-400 outline-none transition text-white"
                        } else {
                            "w-full p-3 rounded bg-gray-900 border border-gray-600 focus:border-gray-400 outline-none transition text-white"
                        },
                        r#type: "email",
                        placeholder: "nom@exemple.com",
                        value: "{email}",
                        oninput: move |evt| email.set(evt.value())
                    }
                    if !err_email.read().is_empty() {
                        p { class: "text-red-400 text-xs mt-1", "{err_email}" }
                    }
                }

                div { class: "mb-4",
                    label { class: "block mb-2 text-sm text-gray-400", "Genre" }
                    select {
                        class: "w-full p-3 rounded bg-gray-900 border border-gray-600 focus:border-gray-400 outline-none transition text-white",
                        value: "{gender}",
                        onchange: move |evt| gender.set(evt.value()),
                        option { value: "homme", "Homme" }
                        option { value: "femme", "Femme" }
                    }
                }

                div { class: "mb-6",
                    label { class: "block mb-2 text-sm text-gray-400", "Mot de passe" }
                    input {
                        class: if !err_password.read().is_empty() {
                            "w-full p-3 rounded bg-gray-900 border border-red-500 focus:border-red-400 outline-none transition text-white"
                        } else {
                            "w-full p-3 rounded bg-gray-900 border border-gray-600 focus:border-gray-400 outline-none transition text-white"
                        },
                        r#type: "password",
                        placeholder: "•••••••• (min 8 caractères)",
                        value: "{password}",
                        oninput: move |evt| password.set(evt.value())
                    }
                    if !err_password.read().is_empty() {
                        p { class: "text-red-400 text-xs mt-1", "{err_password}" }
                    }
                }

                button {
                    class: "w-full bg-gradient-to-r from-gray-700 to-gray-900 hover:from-gray-600 hover:to-gray-800 text-white font-bold py-3 px-4 rounded transition shadow-lg transform active:scale-95",
                    onclick: handle_register,
                    "S'inscrire"
                }

                div { class: "mt-6 text-center text-sm text-gray-500",
                    "Déjà inscrit ? "
                    Link { to: Route::Login {}, class: "text-gray-300 hover:text-white hover:underline", "Se connecter" }
                }
            }
        }
    }
}
