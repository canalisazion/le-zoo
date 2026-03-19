use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::ArticleSummary;
use crate::config::API_BASE_URL;
use crate::Route;
use crate::fetch_creds::WithCredentials;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn badge_color(cat: &str) -> &'static str {
    match cat {
        "dossier"     => "#c0392b",
        "insolite"    => "#27ae60",
        "faits-divers"=> "#e67e22",
        "culture"     => "#8e44ad",
        "formation"   => "#2980b9",
        "critique"    => "#e91e8c",
        "billet"      => "#f39c12",
        _             => "#666",
    }
}

fn fmt_date(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%d/%m/%Y").to_string())
        .unwrap_or_default()
}

// ── Payloads ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ArticlePayload {
    title: String,
    content: String,
    excerpt: String,
    category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cover_image: Option<String>,
    published: bool,
}


// ── Sous-composant : ligne de tableau ────────────────────────────────────────

#[component]
fn ArticleRow(
    art: ArticleSummary,
    on_edit: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    let color = badge_color(&art.category.to_string());
    let cat   = art.category.to_string();
    let date  = fmt_date(art.created_at);
    let slug  = art.slug.clone();
    let title = art.title.clone();

    rsx! {
        tr { style: "border-bottom: 1px solid var(--border);",
            td { style: "padding: 0.6rem 0.8rem; font-size: 0.9rem; color: var(--text-main); max-width: 300px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                "{title}"
            }
            td { style: "padding: 0.6rem 0.5rem;",
                span { style: "background: {color}; color: #fff; font-size: 0.62rem; font-weight: 700; letter-spacing: 0.08em; padding: 2px 6px; border-radius: 2px; text-transform: uppercase;",
                    "{cat}"
                }
            }
            td { style: "padding: 0.6rem 0.5rem; font-size: 0.8rem; color: var(--text-muted);", "{date}" }
            td { style: "padding: 0.6rem 0.5rem; font-size: 0.78rem; color: var(--text-muted);",
                // published n'est pas dans ArticleSummary — on l'infère via la présence dans la liste
                "publié"
            }
            td { style: "padding: 0.6rem 0.5rem; display: flex; gap: 0.4rem;",
                button {
                    style: "background: #2980b9; color: #fff; border: none; padding: 3px 10px; border-radius: 3px; cursor: pointer; font-size: 0.78rem;",
                    onclick: move |_| on_edit.call(slug.clone()),
                    "✏️ Modifier"
                }
                button {
                    style: "background: #c0392b; color: #fff; border: none; padding: 3px 10px; border-radius: 3px; cursor: pointer; font-size: 0.78rem;",
                    onclick: move |_| on_delete.call(art.slug.clone()),
                    "🗑️"
                }
            }
        }
    }
}

// ── Composant principal ───────────────────────────────────────────────────────

#[component]
pub fn ArticleEditor() -> Element {
    let nav = use_navigator();

    let username: String = LocalStorage::get("username").unwrap_or_default();
    if username.is_empty() || username == "Anonyme" {
        nav.push(Route::Login {});
        return rsx! { div {} };
    }

    let role: String = LocalStorage::get("role").unwrap_or_default();
    if role != "admin" && role != "super_admin" {
        nav.push(Route::Home {});
        return rsx! { div {} };
    }

    // None = liste, Some("") = nouveau, Some(slug) = édition
    let mut editing_slug: Signal<Option<String>> = use_signal(|| None);

    // ── État formulaire ──
    let mut f_title     = use_signal(String::new);
    let mut f_category  = use_signal(|| "dossier".to_string());
    let mut f_excerpt   = use_signal(String::new);
    let mut f_content   = use_signal(String::new);
    let mut f_cover     = use_signal(String::new);
    let mut f_published = use_signal(|| true);
    let mut form_error  = use_signal(String::new);
    let mut saving      = use_signal(|| false);

    // ── Liste articles ──
    let mut articles    = use_signal(Vec::<ArticleSummary>::new);
    let mut list_ver    = use_signal(|| 0u32);

    use_resource(move || async move {
        let _ = list_ver();
        let url = format!("{}/api/articles?limit=100&all=true", API_BASE_URL);
        if let Ok(resp) = Client::new().get(&url).with_credentials().send().await {
            if let Ok(data) = resp.json::<Vec<ArticleSummary>>().await {
                articles.set(data);
            }
        }
    });

    // ── Pré-remplissage en mode édition ──
    use_resource(move || async move {
        if let Some(slug) = editing_slug() {
            if slug.is_empty() {
                // Nouveau article : reset le formulaire
                f_title.set(String::new());
                f_category.set("dossier".to_string());
                f_excerpt.set(String::new());
                f_content.set(String::new());
                f_cover.set(String::new());
                f_published.set(true);
                form_error.set(String::new());
                return;
            }
            // Édition : fetch l'article
            let url = format!("{}/api/articles/{}", API_BASE_URL, slug);
            if let Ok(resp) = Client::new().get(&url).with_credentials().send().await {
                if let Ok(art) = resp.json::<shared::Article>().await {
                    f_title.set(art.title);
                    f_category.set(art.category.to_string());
                    f_excerpt.set(art.excerpt);
                    f_content.set(art.content);
                    f_cover.set(art.cover_image.unwrap_or_default());
                    f_published.set(art.published);
                    form_error.set(String::new());
                }
            }
        }
    });

    // ── Helper submit ──
    let do_save = move |_| {
        let title   = f_title.read().trim().to_string();
        let excerpt = f_excerpt.read().trim().to_string();
        let content = f_content.read().trim().to_string();
        let cat     = f_category.read().clone();
        let cv      = f_cover.read().trim().to_string();
        let pub_    = *f_published.read();
        let slug    = editing_slug().unwrap_or_default();

        if title.is_empty() || excerpt.is_empty() || content.is_empty() {
            form_error.set("Titre, extrait et contenu sont requis.".to_string());
            return;
        }
        form_error.set(String::new());
        saving.set(true);

        let payload = ArticlePayload {
            title,
            content,
            excerpt,
            category: cat,
            cover_image: if cv.is_empty() { None } else { Some(cv) },
            published: pub_,
        };

        spawn(async move {
            let url = if slug.is_empty() {
                format!("{}/api/articles", API_BASE_URL)
            } else {
                format!("{}/api/articles/{}/edit", API_BASE_URL, slug)
            };
            let req = if slug.is_empty() {
                Client::new().post(&url)
            } else {
                Client::new().post(&url)
            };
            match req.with_credentials().json(&payload).send().await {
                Ok(resp) if resp.status().is_success() => {
                    saving.set(false);
                    editing_slug.set(None);
                    list_ver += 1;
                }
                Ok(resp) => {
                    let msg = resp.text().await.unwrap_or_else(|_| "Erreur".to_string());
                    form_error.set(format!("Erreur : {}", msg));
                    saving.set(false);
                }
                Err(e) => {
                    form_error.set(format!("Réseau : {}", e));
                    saving.set(false);
                }
            }
        });
    };

    let do_delete = move |slug: String| {
        if !gloo_dialogs::confirm(&format!("Supprimer « {} » ?", slug)) {
            return;
        }
        spawn(async move {
            let url = format!("{}/api/articles/{}/delete", API_BASE_URL, slug);
            let _ = Client::new().delete(&url).with_credentials().send().await;
            list_ver += 1;
        });
    };

    let input_style = "width: 100%; padding: 0.55rem 0.75rem; border: 1px solid var(--border); border-radius: 4px; font-size: 0.95rem; font-family: Inter, sans-serif; box-sizing: border-box; background: var(--bg-card); color: var(--text-main);";
    let label_style = "display: block; font-size: 0.82rem; font-weight: 600; margin-bottom: 0.3rem; color: var(--text-muted);";

    let mode = editing_slug.read().clone();

    rsx! {
        div { style: "background: var(--bg-magazine); min-height: 100vh; padding: 2rem 1rem;",
            div { style: "max-width: 1000px; margin: 0 auto;",

                match mode {
                    // ── MODE LISTE ───────────────────────────────────────
                    None => rsx! {
                        div { style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 1.5rem;",
                            h1 { style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.8rem; font-weight: 800;",
                                "📰 Dashboard éditorial"
                            }
                            button {
                                style: "background: var(--accent, #c0392b); color: #fff; border: none; padding: 0.55rem 1.2rem; border-radius: 4px; font-size: 0.9rem; font-weight: 700; cursor: pointer;",
                                onclick: move |_| editing_slug.set(Some(String::new())),
                                "➕ Nouvel article"
                            }
                        }

                        div { style: "background: var(--bg-card); border-radius: 6px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,0.08);",
                            table { style: "width: 100%; border-collapse: collapse;",
                                thead {
                                    tr { style: "background: var(--bg-sidebar, #f1f5f9); font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-muted);",
                                        th { style: "padding: 0.6rem 0.8rem; text-align: left;", "Titre" }
                                        th { style: "padding: 0.6rem 0.5rem; text-align: left;", "Catégorie" }
                                        th { style: "padding: 0.6rem 0.5rem; text-align: left;", "Date" }
                                        th { style: "padding: 0.6rem 0.5rem; text-align: left;", "Statut" }
                                        th { style: "padding: 0.6rem 0.5rem;", "Actions" }
                                    }
                                }
                                tbody {
                                    for art in articles.read().iter() {
                                        ArticleRow {
                                            art: art.clone(),
                                            on_edit: move |slug| editing_slug.set(Some(slug)),
                                            on_delete: do_delete,
                                        }
                                    }
                                }
                            }
                            if articles.read().is_empty() {
                                p { style: "padding: 2rem; text-align: center; color: var(--text-muted);", "Aucun article." }
                            }
                        }
                    },

                    // ── MODE ÉDITION ─────────────────────────────────────
                    Some(slug) => rsx! {
                        div { style: "display: flex; align-items: center; gap: 1rem; margin-bottom: 1.5rem;",
                            button {
                                style: "background: none; border: 1px solid var(--border); padding: 0.4rem 0.9rem; border-radius: 4px; cursor: pointer; color: var(--text-main); font-size: 0.85rem;",
                                onclick: move |_| editing_slug.set(None),
                                "← Liste"
                            }
                            h1 { style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.6rem; font-weight: 800;",
                                if slug.is_empty() { "Nouvel article" } else { "Modifier l'article" }
                            }
                        }

                        div { style: "background: var(--bg-card); border-radius: 6px; padding: 1.5rem 2rem; box-shadow: 0 1px 4px rgba(0,0,0,0.08); display: flex; flex-direction: column; gap: 1.1rem;",

                            // Titre
                            div {
                                label { style: "{label_style}", "Titre" }
                                input {
                                    r#type: "text",
                                    style: "{input_style}",
                                    placeholder: "Titre de l'article",
                                    value: "{f_title}",
                                    oninput: move |e| f_title.set(e.value()),
                                }
                            }

                            // Catégorie
                            div {
                                label { style: "{label_style}", "Catégorie" }
                                select {
                                    style: "{input_style}",
                                    value: "{f_category}",
                                    onchange: move |e| f_category.set(e.value()),
                                    option { value: "dossier",      "Dossier" }
                                    option { value: "insolite",     "Insolite" }
                                    option { value: "faits-divers", "Faits Divers" }
                                    option { value: "culture",      "Culture" }
                                    option { value: "formation",    "Formation" }
                                    option { value: "critique",     "Critique" }
                                    option { value: "billet",       "Billet" }
                                }
                            }

                            // Excerpt
                            div {
                                label { style: "{label_style}", "Extrait (max 300 caractères)" }
                                textarea {
                                    style: "{input_style} resize: vertical; min-height: 70px;",
                                    maxlength: "300",
                                    placeholder: "Résumé court…",
                                    value: "{f_excerpt}",
                                    oninput: move |e| f_excerpt.set(e.value()),
                                }
                            }

                            // Contenu
                            div {
                                label { style: "{label_style}", "Contenu" }
                                textarea {
                                    style: "{input_style} resize: vertical; min-height: 500px; font-family: 'Courier New', monospace; font-size: 0.9rem; line-height: 1.6;",
                                    placeholder: "Corps de l'article…",
                                    value: "{f_content}",
                                    oninput: move |e| f_content.set(e.value()),
                                }
                            }

                            // Cover image
                            div {
                                label { style: "{label_style}", "Image de couverture (URL, optionnel)" }
                                input {
                                    r#type: "text",
                                    style: "{input_style}",
                                    placeholder: "https://…",
                                    value: "{f_cover}",
                                    oninput: move |e| f_cover.set(e.value()),
                                }
                            }

                            // Publié
                            div { style: "display: flex; align-items: center; gap: 0.5rem;",
                                input {
                                    r#type: "checkbox",
                                    id: "published",
                                    checked: *f_published.read(),
                                    onchange: move |e| f_published.set(e.checked()),
                                }
                                label {
                                    r#for: "published",
                                    style: "font-size: 0.9rem; color: var(--text-main); cursor: pointer;",
                                    "Publié (décocher pour brouillon)"
                                }
                            }

                            // Erreur + bouton
                            if !form_error.read().is_empty() {
                                p { style: "color: #c0392b; font-size: 0.88rem;", "{form_error}" }
                            }

                            button {
                                style: "background: var(--accent, #c0392b); color: #fff; border: none; padding: 0.7rem 2rem; font-size: 1rem; font-weight: 700; border-radius: 4px; cursor: pointer; align-self: flex-start;",
                                disabled: *saving.read(),
                                onclick: do_save,
                                if *saving.read() { "Enregistrement…" } else { "💾 Enregistrer" }
                            }
                        }
                    }
                }
            }
        }
    }
}
