use dioxus::prelude::*;
use reqwest::Client;
use shared::{ArticleSummary, ArticleCategory};
use gloo_storage::{LocalStorage, Storage};
use crate::config::API_BASE_URL;
use crate::Route;
use crate::fetch_creds::WithCredentials;

fn badge_color(cat: &ArticleCategory) -> &'static str {
    match cat {
        ArticleCategory::Dossier     => "#c0392b",
        ArticleCategory::Insolite    => "#27ae60",
        ArticleCategory::FaitsDivers => "#e67e22",
        ArticleCategory::Culture     => "#8e44ad",
        ArticleCategory::Formation   => "#2980b9",
        ArticleCategory::Critique    => "#e91e8c",
        ArticleCategory::Billet      => "#f39c12",
    }
}

fn fmt_date(ts: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%d/%m/%Y").to_string())
        .unwrap_or_default()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[component]
pub fn Rubrique(category: String) -> Element {
    let mut articles = use_signal(Vec::<ArticleSummary>::new);

    let cat_fetch = category.clone();
    use_resource(move || {
        let cat = cat_fetch.clone();
        async move {
            let url = format!("{}/api/articles?category={}&limit=20", API_BASE_URL, cat);
            if let Ok(resp) = Client::new().get(&url).with_credentials().send().await {
                if let Ok(data) = resp.json::<Vec<ArticleSummary>>().await {
                    articles.set(data);
                }
            }
        }
    });

    let display = capitalize(&category);
    let arts = articles.read();

    // Protection formation : vérifier connexion
    let is_formation = category == "formation";
    let username: String = LocalStorage::get("username").unwrap_or_default();
    let is_logged_in = !username.is_empty();

    if is_formation && !is_logged_in {
        return rsx! {
            div {
                style: "background: var(--bg-magazine); min-height: 100vh; padding: 2rem 1rem;",
                div { style: "max-width: 1100px; margin: 0 auto; padding: 4rem 1rem; text-align: center;",
                    h1 {
                        style: "font-family: 'Playfair Display', Georgia, serif; font-size: 2.2rem; font-weight: 800; margin-bottom: 1.5rem; color: var(--text-main);",
                        "Formations"
                    }
                    p {
                        style: "font-size: 1.1rem; color: var(--text-main); margin-bottom: 1.5rem;",
                        "Connectez-vous pour accéder aux formations gratuites"
                    }
                    Link {
                        to: Route::Login {},
                        style: "color: var(--accent); text-decoration: underline; font-weight: 600; font-size: 1rem;",
                        "Se connecter"
                    }
                }
            }
        };
    }

    rsx! {
        div {
            style: "background: var(--bg-magazine); min-height: 100vh; padding: 2rem 1rem;",

            div { style: "max-width: 1100px; margin: 0 auto;",

                h1 {
                    style: "font-family: 'Playfair Display', Georgia, serif; font-size: 2.2rem; font-weight: 800; border-bottom: 3px solid var(--border); padding-bottom: 0.5rem; margin-bottom: 2rem; color: var(--text-main);",
                    "{display}"
                }

                if arts.is_empty() {
                    p { style: "color: var(--text-muted); padding: 2rem 0;", "Aucun article dans cette rubrique." }
                } else {
                    div {
                        style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 1.5rem;",
                        for a in arts.iter() {
                            {
                                let color     = badge_color(&a.category);
                                let cat_label = a.category.to_string();
                                let date      = fmt_date(a.created_at);
                                let slug      = a.slug.clone();
                                let title     = a.title.clone();
                                let excerpt   = a.excerpt.clone();
                                let author    = a.author_name.clone();
                                let img       = a.cover_image.clone();

                                rsx! {
                                    div {
                                        style: "background: var(--bg-card); border-radius: 4px; overflow: hidden; box-shadow: 0 1px 4px rgba(0,0,0,0.08);",

                                        if let Some(src) = img {
                                            img {
                                                src: "{src}",
                                                style: "width: 100%; height: 150px; object-fit: cover; background: #c9c9c9;",
                                            }
                                        } else {
                                            div { style: "width: 100%; height: 150px; background: #c9c9c9;" }
                                        }

                                        div { style: "padding: 0.8rem 1rem 1rem;",
                                            span {
                                                style: "background: {color}; color: #fff; font-size: 0.65rem; font-weight: 700; letter-spacing: 0.1em; padding: 2px 7px; border-radius: 2px; text-transform: uppercase;",
                                                "{cat_label}"
                                            }
                                            h3 {
                                                style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1rem; font-weight: 700; margin: 0.4rem 0 0.4rem; line-height: 1.3; color: var(--text-main);",
                                                Link {
                                                    to: Route::Article { slug },
                                                    style: "color: inherit; text-decoration: none;",
                                                    "{title}"
                                                }
                                            }
                                            p { style: "font-size: 0.82rem; color: var(--text-muted); line-height: 1.5; margin-bottom: 0.3rem;", "{excerpt}" }
                                            p { style: "font-size: 0.78rem; color: var(--text-muted);", "{author} · {date}" }
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
