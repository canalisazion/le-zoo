use dioxus::prelude::*;
use reqwest::Client;
use serde::Serialize;
use crate::{Route, config::API_BASE_URL};

#[derive(Serialize)]
struct ResetPasswordPayload {
    token: String,
    new_password: String,
}

fn get_token_from_url() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let search = match window.location().search() {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let params = match web_sys::UrlSearchParams::new_with_str(&search) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };
    params.get("token").unwrap_or_default()
}

#[component]
pub fn ResetPassword() -> Element {
    let token = use_memo(|| get_token_from_url());
    let mut new_password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut status = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut success = use_signal(|| false);
    let nav = use_navigator();

    let handle_submit = move |_| async move {
        let pwd = new_password.read().clone();
        let confirm = confirm_password.read().clone();
        let tok = token();
        status.set(String::new());

        if tok.is_empty() {
            status.set("❌ Lien invalide — aucun token trouvé".to_string());
            return;
        }
        if pwd.len() < 8 {
            status.set("❌ Le mot de passe doit faire au moins 8 caractères".to_string());
            return;
        }
        if pwd != confirm {
            status.set("❌ Les mots de passe ne correspondent pas".to_string());
            return;
        }

        loading.set(true);
        status.set("⏳ Réinitialisation...".to_string());

        let res = Client::new()
            .post(format!("{}/api/auth/reset-password", API_BASE_URL))
            .json(&ResetPasswordPayload { token: tok, new_password: pwd })
            .send()
            .await;

        loading.set(false);

        match res {
            Ok(r) => match r.status().as_u16() {
                200 => {
                    success.set(true);
                    status.set(String::new());
                    // Redirection automatique après 3s
                    gloo_timers::callback::Timeout::new(3_000, move || {
                        nav.push(Route::Login {});
                    }).forget();
                }
                400 => {
                    let body = r.text().await.unwrap_or_default();
                    status.set(format!("❌ {}", body));
                }
                _ => {
                    status.set(format!("❌ Erreur ({})", r.status().as_u16()));
                }
            },
            Err(_) => {
                status.set("⚠️ Serveur injoignable, réessayez".to_string());
            }
        }
    };

    let token_missing = token().is_empty();

    rsx! {
        div { class: "flex flex-col items-center justify-center min-h-screen bg-gray-900 text-white font-sans px-4",

            div { class: "bg-gray-800 p-8 rounded-xl shadow-2xl w-full max-w-md border border-gray-700",

                div { class: "mb-6 text-center",
                    div { class: "text-4xl mb-3", "🔐" }
                    h1 { class: "text-2xl font-bold text-gray-100", "Nouveau mot de passe" }
                    if !token_missing {
                        p { class: "text-gray-400 text-sm mt-2",
                            "Choisissez un nouveau mot de passe sécurisé."
                        }
                    }
                }

                if token_missing {
                    div { class: "p-4 rounded-lg bg-red-900/50 border border-red-700 text-center",
                        p { class: "text-red-300 font-semibold", "❌ Lien invalide" }
                        p { class: "text-gray-400 text-sm mt-2",
                            "Ce lien de réinitialisation est invalide ou a expiré."
                        }
                        div { class: "mt-4",
                            Link {
                                to: Route::ForgotPassword {},
                                class: "text-blue-400 hover:underline text-sm",
                                "Faire une nouvelle demande"
                            }
                        }
                    }
                } else if *success.read() {
                    div { class: "p-4 rounded-lg bg-green-900/50 border border-green-700 text-center",
                        div { class: "text-3xl mb-2", "✅" }
                        p { class: "text-green-300 font-semibold", "Mot de passe réinitialisé !" }
                        p { class: "text-gray-400 text-sm mt-2",
                            "Vous allez être redirigé vers la connexion..."
                        }
                        div { class: "mt-4",
                            Link {
                                to: Route::Login {},
                                class: "text-blue-400 hover:underline text-sm",
                                "Se connecter maintenant"
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

                    div { class: "mb-4",
                        label { class: "block mb-2 text-sm text-gray-400", "Nouveau mot de passe" }
                        input {
                            class: "w-full p-3 rounded-lg bg-gray-900 border border-gray-600 text-white outline-none focus:border-blue-500 transition",
                            r#type: "password",
                            placeholder: "Minimum 8 caractères",
                            value: "{new_password}",
                            disabled: *loading.read(),
                            oninput: move |e| new_password.set(e.value()),
                        }
                    }

                    div { class: "mb-6",
                        label { class: "block mb-2 text-sm text-gray-400", "Confirmer le mot de passe" }
                        input {
                            class: {
                                let base = "w-full p-3 rounded-lg bg-gray-900 border text-white outline-none transition";
                                let confirm = confirm_password.read().clone();
                                let pwd = new_password.read().clone();
                                if !confirm.is_empty() && confirm != pwd {
                                    format!("{} border-red-500 focus:border-red-400", base)
                                } else if !confirm.is_empty() && confirm == pwd {
                                    format!("{} border-green-500 focus:border-green-400", base)
                                } else {
                                    format!("{} border-gray-600 focus:border-blue-500", base)
                                }
                            },
                            r#type: "password",
                            placeholder: "Répétez le mot de passe",
                            value: "{confirm_password}",
                            disabled: *loading.read(),
                            oninput: move |e| confirm_password.set(e.value()),
                        }
                        // Indicateur correspondance
                        {
                            let confirm = confirm_password.read().clone();
                            let pwd = new_password.read().clone();
                            if !confirm.is_empty() {
                                if confirm == pwd {
                                    rsx! { p { class: "text-green-400 text-xs mt-1", "✓ Les mots de passe correspondent" } }
                                } else {
                                    rsx! { p { class: "text-red-400 text-xs mt-1", "✗ Les mots de passe ne correspondent pas" } }
                                }
                            } else {
                                rsx! {}
                            }
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
                        if *loading.read() { "Réinitialisation..." } else { "Réinitialiser le mot de passe" }
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
