use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use serde::Serialize;
use crate::config::API_BASE_URL;
use crate::fetch_creds::WithCredentials; // ✅ [H-8]

#[derive(Serialize)]
struct ChangePasswordPayload {
    old_password: String,
    new_password: String,
}

#[component]
pub fn SettingsModal(
    mut show_settings: Signal<bool>,
    mut show_cgu: Signal<bool>,
    mut show_confidentialite: Signal<bool>,
) -> Element {
    let mut show_change_pwd = use_signal(|| false);
    let mut old_pwd = use_signal(|| String::new());
    let mut new_pwd = use_signal(|| String::new());
    let mut confirm_pwd = use_signal(|| String::new());
    let mut pwd_status = use_signal(|| String::new());
    let mut pwd_loading = use_signal(|| false);

    let mut show_delete_form = use_signal(|| false);
    let mut delete_pwd = use_signal(|| String::new());
    let mut delete_status = use_signal(|| String::new());
    let mut delete_loading = use_signal(|| false);

    let handle_change_password = move |_| async move {
        let old = old_pwd.read().clone();
        let new = new_pwd.read().clone();
        let confirm = confirm_pwd.read().clone();
        pwd_status.set(String::new());

        if old.is_empty() || new.is_empty() || confirm.is_empty() {
            pwd_status.set("❌ Tous les champs sont requis".to_string());
            return;
        }
        if new.len() < 8 {
            pwd_status.set("❌ Minimum 8 caractères".to_string());
            return;
        }
        if new != confirm {
            pwd_status.set("❌ Les mots de passe ne correspondent pas".to_string());
            return;
        }

        pwd_loading.set(true);
        // ✅ [H-8] cookie HttpOnly envoyé automatiquement
        let res = Client::new()
            .post(format!("{}/api/auth/change-password", API_BASE_URL))
            .with_credentials()
            .json(&ChangePasswordPayload { old_password: old, new_password: new })
            .send()
            .await;

        pwd_loading.set(false);

        match res {
            Ok(r) => match r.status().as_u16() {
                200 => {
                    pwd_status.set("✅ Mot de passe modifié avec succès".to_string());
                    old_pwd.set(String::new());
                    new_pwd.set(String::new());
                    confirm_pwd.set(String::new());
                }
                401 => {
                    pwd_status.set("❌ Ancien mot de passe incorrect".to_string());
                }
                400 => {
                    let body = r.text().await.unwrap_or_default();
                    pwd_status.set(format!("❌ {}", body));
                }
                _ => {
                    pwd_status.set(format!("❌ Erreur ({})", r.status().as_u16()));
                }
            },
            Err(_) => {
                pwd_status.set("⚠️ Serveur injoignable".to_string());
            }
        }
    };

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
                                // Bouton toggle
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg transition text-sm flex items-center gap-3",
                                    style: "color:var(--text-main);",
                                    onclick: move |_| {
                                        let current = *show_change_pwd.read();
                                        show_change_pwd.set(!current);
                                        pwd_status.set(String::new());
                                    },
                                    span { "🔑" }
                                    span { "Changer mon mot de passe" }
                                    span { class: "ml-auto text-gray-500 text-xs",
                                        if *show_change_pwd.read() { "▲" } else { "▼" }
                                    }
                                }

                                // Formulaire inline
                                if *show_change_pwd.read() {
                                    div { class: "mx-1 mb-2 p-3 rounded-lg border border-gray-600 bg-gray-900/50 space-y-3",

                                        if !pwd_status.read().is_empty() {
                                            div {
                                                class: "p-2 rounded text-xs font-semibold text-center border",
                                                class: if pwd_status.read().starts_with("✅") {
                                                    "border-green-700 bg-green-900/40 text-green-300"
                                                } else if pwd_status.read().starts_with("⚠️") {
                                                    "border-yellow-700 bg-yellow-900/40 text-yellow-300"
                                                } else {
                                                    "border-red-700 bg-red-900/40 text-red-300"
                                                },
                                                "{pwd_status}"
                                            }
                                        }

                                        div {
                                            label { class: "block text-xs text-gray-400 mb-1", "Mot de passe actuel" }
                                            input {
                                                class: "w-full px-3 py-2 text-sm rounded bg-gray-800 border border-gray-600 text-white outline-none focus:border-blue-500 transition",
                                                r#type: "password",
                                                placeholder: "••••••••",
                                                value: "{old_pwd}",
                                                disabled: *pwd_loading.read(),
                                                oninput: move |e| old_pwd.set(e.value()),
                                            }
                                        }
                                        div {
                                            label { class: "block text-xs text-gray-400 mb-1", "Nouveau mot de passe" }
                                            input {
                                                class: "w-full px-3 py-2 text-sm rounded bg-gray-800 border border-gray-600 text-white outline-none focus:border-blue-500 transition",
                                                r#type: "password",
                                                placeholder: "Minimum 8 caractères",
                                                value: "{new_pwd}",
                                                disabled: *pwd_loading.read(),
                                                oninput: move |e| new_pwd.set(e.value()),
                                            }
                                        }
                                        div {
                                            label { class: "block text-xs text-gray-400 mb-1", "Confirmer le nouveau" }
                                            input {
                                                class: {
                                                    let base = "w-full px-3 py-2 text-sm rounded bg-gray-800 border text-white outline-none transition";
                                                    let c = confirm_pwd.read().clone();
                                                    let n = new_pwd.read().clone();
                                                    if !c.is_empty() && c != n {
                                                        format!("{} border-red-500", base)
                                                    } else if !c.is_empty() && c == n {
                                                        format!("{} border-green-500", base)
                                                    } else {
                                                        format!("{} border-gray-600 focus:border-blue-500", base)
                                                    }
                                                },
                                                r#type: "password",
                                                placeholder: "Répétez le mot de passe",
                                                value: "{confirm_pwd}",
                                                disabled: *pwd_loading.read(),
                                                oninput: move |e| confirm_pwd.set(e.value()),
                                            }
                                            // Indicateur correspondance
                                            {
                                                let c = confirm_pwd.read().clone();
                                                let n = new_pwd.read().clone();
                                                if !c.is_empty() {
                                                    if c == n {
                                                        rsx! { p { class: "text-green-400 text-[10px] mt-1", "✓ Correspondent" } }
                                                    } else {
                                                        rsx! { p { class: "text-red-400 text-[10px] mt-1", "✗ Ne correspondent pas" } }
                                                    }
                                                } else {
                                                    rsx! {}
                                                }
                                            }
                                        }

                                        button {
                                            class: "w-full py-2 rounded text-sm font-semibold transition text-white",
                                            class: if *pwd_loading.read() {
                                                "bg-blue-800 cursor-not-allowed"
                                            } else {
                                                "bg-blue-700 hover:bg-blue-600 active:scale-95"
                                            },
                                            disabled: *pwd_loading.read(),
                                            onclick: handle_change_password,
                                            if *pwd_loading.read() { "Modification..." } else { "Modifier le mot de passe" }
                                        }
                                    }
                                }

                                // Bouton toggle suppression
                                button {
                                    class: "w-full text-left px-3 py-3 rounded-lg hover:bg-red-900/30 transition text-sm text-red-400 flex items-center gap-3",
                                    onclick: move |_| {
                                        let current = *show_delete_form.read();
                                        show_delete_form.set(!current);
                                        delete_status.set(String::new());
                                        delete_pwd.set(String::new());
                                    },
                                    span { "🗑️" }
                                    span { "Supprimer mon compte" }
                                    span { class: "ml-auto text-gray-500 text-xs",
                                        if *show_delete_form.read() { "▲" } else { "▼" }
                                    }
                                }

                                // Formulaire inline suppression
                                if *show_delete_form.read() {
                                    div { class: "mx-1 mb-2 p-3 rounded-lg border border-red-700 bg-red-900/20 space-y-3",

                                        if !delete_status.read().is_empty() {
                                            div {
                                                class: "p-2 rounded text-xs font-semibold text-center border",
                                                class: if delete_status.read().starts_with("⚠️") {
                                                    "border-yellow-700 bg-yellow-900/40 text-yellow-300"
                                                } else {
                                                    "border-red-700 bg-red-900/40 text-red-300"
                                                },
                                                "{delete_status}"
                                            }
                                        }

                                        p { class: "text-xs text-red-300",
                                            "Cette action est irréversible. Confirmez avec votre mot de passe."
                                        }

                                        div {
                                            label { class: "block text-xs text-gray-400 mb-1", "Mot de passe" }
                                            input {
                                                class: "w-full px-3 py-2 text-sm rounded bg-gray-800 border border-gray-600 text-white outline-none focus:border-red-500 transition",
                                                r#type: "password",
                                                placeholder: "••••••••",
                                                value: "{delete_pwd}",
                                                disabled: *delete_loading.read(),
                                                oninput: move |e| delete_pwd.set(e.value()),
                                            }
                                        }

                                        div { class: "flex gap-2",
                                            button {
                                                class: "flex-1 py-2 rounded text-sm font-semibold bg-gray-600 hover:bg-gray-500 transition text-white",
                                                onclick: move |_| {
                                                    show_delete_form.set(false);
                                                    delete_pwd.set(String::new());
                                                    delete_status.set(String::new());
                                                },
                                                "Annuler"
                                            }
                                            button {
                                                class: "flex-1 py-2 rounded text-sm font-semibold transition text-white",
                                                class: if *delete_loading.read() {
                                                    "bg-red-800 cursor-not-allowed"
                                                } else {
                                                    "bg-red-700 hover:bg-red-600 active:scale-95"
                                                },
                                                disabled: *delete_loading.read(),
                                                onclick: move |_| {
                                                    let pwd = delete_pwd.read().clone();
                                                    if pwd.is_empty() {
                                                        delete_status.set("❌ Mot de passe requis".to_string());
                                                        return;
                                                    }
                                                    delete_loading.set(true);
                                                    spawn(async move {
                                                        // ✅ [H-8] cookie HttpOnly — le backend efface le cookie dans sa réponse
                                                        let res = Client::new()
                                                            .delete(format!("{}/api/users/delete", API_BASE_URL))
                                                            .with_credentials()
                                                            .json(&serde_json::json!({ "password": pwd }))
                                                            .send()
                                                            .await;
                                                        delete_loading.set(false);
                                                        match res {
                                                            Ok(r) if r.status().as_u16() == 200 => {
                                                                LocalStorage::delete("username");
                                                                LocalStorage::delete("role");
                                                                let _ = dioxus::document::eval("window.location.href = '/'");
                                                            }
                                                            Ok(r) if r.status().as_u16() == 401 => {
                                                                delete_status.set("❌ Mot de passe incorrect".to_string());
                                                            }
                                                            _ => {
                                                                delete_status.set("⚠️ Erreur serveur, réessayez".to_string());
                                                            }
                                                        }
                                                    });
                                                },
                                                if *delete_loading.read() { "Suppression..." } else { "Confirmer la suppression" }
                                            }
                                        }
                                    }
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
                        // ✅ [H-8] Déconnexion : appel /api/logout pour effacer le cookie HttpOnly
                        button {
                            class: "w-full py-3 rounded-xl bg-blue-600 hover:bg-blue-500 transition font-semibold text-sm flex items-center justify-center gap-2",
                            onclick: move |_| {
                                LocalStorage::delete("username");
                                LocalStorage::delete("role");
                                spawn(async move {
                                    let _ = Client::new()
                                        .post(format!("{}/api/logout", API_BASE_URL))
                                        .with_credentials()
                                        .send().await;
                                    let _ = dioxus::document::eval("window.location.href = '/login'");
                                });
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
