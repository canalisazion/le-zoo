use dioxus::prelude::*;
use reqwest::Client;
use serde::Serialize;
use crate::{Route, config::API_BASE_URL};

#[derive(Serialize)]
struct ForgotPasswordPayload {
    email: String,
}

#[component]
pub fn ForgotPassword() -> Element {
    let mut email = use_signal(|| String::new());
    let mut status = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut sent = use_signal(|| false);

    let handle_submit = move |_| async move {
        let mail = email.read().trim().to_string();
        status.set(String::new());

        if mail.is_empty() {
            status.set("❌ Veuillez saisir votre email".to_string());
            return;
        }

        loading.set(true);
        status.set("⏳ Envoi en cours...".to_string());

        let res = Client::new()
            .post(format!("{}/api/auth/forgot-password", API_BASE_URL))
            .json(&ForgotPasswordPayload { email: mail })
            .send()
            .await;

        loading.set(false);

        match res {
            Ok(_) => {
                // Toujours montrer succès (anti-énumération)
                sent.set(true);
                status.set(String::new());
            }
            Err(_) => {
                status.set("⚠️ Serveur injoignable, réessayez".to_string());
            }
        }
    };

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen bg-gray-900 text-white font-sans px-4",

            div { class: "bg-gray-800 p-8 rounded-xl shadow-2xl w-full max-w-md border border-gray-700",

                div { class: "mb-6 text-center",
                    div { class: "text-4xl mb-3", "🔑" }
                    h1 { class: "text-2xl font-bold text-gray-100", "Mot de passe oublié" }
                    p { class: "text-gray-400 text-sm mt-2",
                        "Saisissez votre email pour recevoir un lien de réinitialisation."
                    }
                }

                if *sent.read() {
                    div { class: "p-4 rounded-lg bg-green-900/50 border border-green-700 text-center",
                        div { class: "text-3xl mb-2", "✉️" }
                        p { class: "text-green-300 font-semibold", "Email envoyé !" }
                        p { class: "text-gray-400 text-sm mt-2",
                            "Si cet email est enregistré, vous recevrez un lien dans quelques instants. "
                            "Vérifiez vos spams si besoin."
                        }
                        div { class: "mt-4",
                            Link {
                                to: Route::Login {},
                                class: "text-blue-400 hover:underline text-sm",
                                "← Retour à la connexion"
                            }
                        }
                    }
                } else {
                    if !status.read().is_empty() {
                        div {
                            class: "mb-4 p-3 rounded font-bold text-center border text-sm",
                            class: if status.read().starts_with("⏳") {
                                "border-gray-600 bg-gray-700 text-gray-300"
                            } else if status.read().starts_with("⚠️") {
                                "border-yellow-600 bg-yellow-900/50 text-yellow-300"
                            } else {
                                "border-red-600 bg-red-900/50 text-red-300"
                            },
                            "{status}"
                        }
                    }

                    div { class: "mb-5",
                        label { class: "block mb-2 text-sm text-gray-400", "Adresse email" }
                        input {
                            class: "w-full p-3 rounded-lg bg-gray-900 border border-gray-600 text-white outline-none focus:border-blue-500 transition",
                            r#type: "email",
                            placeholder: "votre@email.com",
                            value: "{email}",
                            disabled: *loading.read(),
                            oninput: move |e| email.set(e.value()),
                        }
                    }

                    button {
                        class: "w-full py-3 rounded-lg font-bold transition text-white",
                        class: if *loading.read() {
                            "bg-blue-800 cursor-not-allowed"
                        } else {
                            "bg-blue-700 hover:bg-blue-600 active:scale-95"
                        },
                        disabled: *loading.read(),
                        onclick: handle_submit,
                        if *loading.read() { "Envoi..." } else { "Envoyer le lien" }
                    }

                    div { class: "mt-5 text-center text-sm text-gray-500",
                        Link {
                            to: Route::Login {},
                            class: "text-blue-400 hover:underline",
                            "← Retour à la connexion"
                        }
                    }
                }
            }
        }
    }
}
