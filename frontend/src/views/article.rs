use dioxus::prelude::*;
use reqwest::Client;
use shared::{Article as ArticleData, ArticleSummary, ArticleCategory};
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

#[component]
pub fn Article(slug: String) -> Element {
    let mut article: Signal<Option<Option<ArticleData>>> = use_signal(|| None);
    let mut related: Signal<Vec<ArticleSummary>> = use_signal(Vec::new);

    let slug_fetch = slug.clone();
    use_resource(move || {
        let s = slug_fetch.clone();
        async move {
            let url = format!("{}/api/articles/{}", API_BASE_URL, s);
            match Client::new().get(&url).with_credentials().send().await {
                Ok(resp) if resp.status() == 404 => article.set(Some(None)),
                Ok(resp) => {
                    if let Ok(data) = resp.json::<ArticleData>().await {
                        article.set(Some(Some(data)));
                    }
                }
                Err(_) => article.set(Some(None)),
            }
        }
    });

    let art_read = article.read();

    match &*art_read {
        // Loading
        None => rsx! {
            div { style: "padding: 4rem; text-align: center; color: var(--text-muted);", "Chargement…" }
        },

        // 404
        Some(None) => rsx! {
            div { style: "padding: 4rem; text-align: center; color: var(--text-muted);",
                p { style: "font-size: 1.2rem; margin-bottom: 1rem;", "Article introuvable." }
                Link { to: Route::Home {}, style: "color: var(--accent); text-decoration: underline;", "← Retour à l'accueil" }
            }
        },

        // Article trouvé
        Some(Some(a)) => {
            let color       = badge_color(&a.category);
            let cat_label   = a.category.to_string();
            let date        = fmt_date(a.created_at);
            let author      = a.author_name.clone();
            let title       = a.title.clone();
            let content     = a.content.clone();
            let cover       = a.cover_image.clone();
            let cat_str     = cat_label.clone();
            let slug_rel    = slug.clone();

            // Lettrine : premier char
            let (first_char, rest_content) = content
                .char_indices()
                .next()
                .map(|(_, c)| {
                    let fc = c.to_string();
                    let rest = content[c.len_utf8()..].to_string();
                    (fc, rest)
                })
                .unwrap_or_default();

            // Fetch articles liés (lancé une fois l'article connu)
            let cat_for_related = cat_str.clone();
            use_resource(move || {
                let cat = cat_for_related.clone();
                let excl = slug_rel.clone();
                async move {
                    let url = format!("{}/api/articles?category={}&limit=4", API_BASE_URL, cat);
                    if let Ok(resp) = Client::new().get(&url).with_credentials().send().await {
                        if let Ok(data) = resp.json::<Vec<ArticleSummary>>().await {
                            let filtered: Vec<ArticleSummary> = data
                                .into_iter()
                                .filter(|r| r.slug != excl)
                                .take(3)
                                .collect();
                            related.set(filtered);
                        }
                    }
                }
            });

            let related_read = related.read();

            rsx! {
                div {
                    style: "background: var(--bg-magazine); min-height: 100vh; padding: 2rem 1rem;",

                    div {
                        style: "max-width: 1100px; margin: 0 auto; display: flex; gap: 2.5rem; align-items: flex-start;",

                        // ── Contenu principal ─────────────────────────────
                        article { style: "flex: 1; min-width: 0;",

                            // Méta
                            div { style: "margin-bottom: 0.8rem;",
                                span {
                                    style: "background: {color}; color: #fff; font-size: 0.7rem; font-weight: 700; letter-spacing: 0.1em; padding: 2px 8px; border-radius: 2px; text-transform: uppercase;",
                                    "{cat_label}"
                                }
                            }

                            h1 {
                                style: "font-family: 'Playfair Display', Georgia, serif; font-size: 2.4rem; font-weight: 800; line-height: 1.2; margin: 0 0 0.8rem; color: var(--text-main);",
                                "{title}"
                            }

                            p { style: "font-size: 0.85rem; color: var(--text-muted); margin-bottom: 1.5rem;",
                                "{author} · {date}"
                            }

                            // Cover image
                            if let Some(src) = cover {
                                img {
                                    src: "{src}",
                                    style: "width: 100%; height: auto; max-height: 400px; object-fit: cover; border-radius: 8px; margin-bottom: 2rem; display: block;",
                                }
                            }

                            // Corps de l'article
                            div { style: "font-size: 1.05rem; line-height: 1.8; color: var(--text-main);",
                                p { style: "margin-bottom: 1.4rem;",
                                    // Lettrine
                                    span {
                                        style: "float: left; font-size: 3.5em; line-height: 0.8; padding-right: 0.1em; font-family: 'Playfair Display', Georgia, serif; color: var(--accent, #c0392b); font-weight: 700;",
                                        "{first_char}"
                                    }
                                    "{rest_content}"
                                }
                            }

                            // Bouton forum
                            div { style: "margin-top: 2rem; padding-top: 1.5rem; border-top: 1px solid var(--border);",
                                Link {
                                    to: Route::Chat {},
                                    style: "display: inline-block; background: var(--accent, #c0392b); color: #fff; padding: 0.7rem 1.5rem; font-size: 1rem; font-weight: 600; border-radius: 4px; text-decoration: none;",
                                    "💬 Discuter dans le forum"
                                }
                            }
                        }

                        // ── Sidebar ───────────────────────────────────────
                        if !related_read.is_empty() {
                            aside {
                                style: "width: 240px; flex-shrink: 0;",
                                div {
                                    style: "background: var(--bg-card); border-radius: 4px; padding: 1rem 1.2rem; box-shadow: 0 1px 4px rgba(0,0,0,0.08);",
                                    h3 {
                                        style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1rem; font-weight: 700; margin: 0 0 0.8rem; border-bottom: 1px solid var(--border); padding-bottom: 0.4rem; color: var(--text-main);",
                                        "Articles liés"
                                    }
                                    ul { style: "list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.7rem;",
                                        for r in related_read.iter() {
                                            li {
                                                Link {
                                                    to: Route::Article { slug: r.slug.clone() },
                                                    style: "color: var(--text-main); text-decoration: none; font-size: 0.88rem; line-height: 1.4;",
                                                    "{r.title}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div {}
                        }
                    }
                }
            }
        }
    }
}
