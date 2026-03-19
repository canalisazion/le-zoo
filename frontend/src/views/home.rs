use dioxus::prelude::*;
use reqwest::Client;
use shared::{ArticleSummary, ArticleCategory};
use crate::config::API_BASE_URL;
use crate::Route;
use crate::fetch_creds::WithCredentials;

fn badge_color(cat: &ArticleCategory) -> &'static str {
    match cat {
        ArticleCategory::Dossier    => "#c0392b",
        ArticleCategory::Insolite   => "#27ae60",
        ArticleCategory::FaitsDivers => "#e67e22",
        ArticleCategory::Culture    => "#8e44ad",
        ArticleCategory::Formation  => "#2980b9",
        ArticleCategory::Critique   => "#e91e8c",
        ArticleCategory::Billet     => "#f39c12",
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
pub fn Home() -> Element {
    let mut articles = use_signal(|| Vec::<ArticleSummary>::new());

    use_resource(move || async move {
        let url = format!("{}/api/articles?limit=7", API_BASE_URL);
        if let Ok(resp) = Client::new().get(&url).with_credentials().send().await {
            if let Ok(data) = resp.json::<Vec<ArticleSummary>>().await {
                articles.set(data);
            }
        }
    });

    let arts = articles.read();

    rsx! {
        div {
            style: "background: var(--bg-magazine); min-height: 100vh; padding: 2rem 1rem;",

            // Hero
            div {
                style: "max-width: 1100px; margin: 0 auto 2.5rem auto; text-align: center; padding: 3rem 1rem 2rem;",
                h1 {
                    style: "font-family: 'Playfair Display', Georgia, serif; font-size: 3.5rem; font-weight: 800; margin: 0 0 0.5rem; letter-spacing: -1px; color: var(--accent);",
                    "Le Zoo"
                }
                p {
                    style: "font-size: 1rem; color: var(--text-muted); letter-spacing: 0.15em; text-transform: uppercase;",
                    "Dossiers · Insolite · Faits Divers · Culture"
                }
            }

            div { style: "max-width: 1100px; margin: 0 auto;",

                if arts.is_empty() {
                    p { style: "text-align: center; color: var(--text-muted); padding: 3rem 0;", "Aucun article pour le moment." }
                } else {
                    // À la une
                    h2 {
                        style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.4rem; font-weight: 700; border-bottom: 2px solid var(--border); padding-bottom: 0.4rem; margin-bottom: 1.5rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-main);",
                        "À la une"
                    }

                    {
                        let a = &arts[0];
                        let color = badge_color(&a.category);
                        let cat_label = a.category.to_string();
                        let date = fmt_date(a.created_at);
                        let slug = a.slug.clone();
                        let title = a.title.clone();
                        let excerpt = a.excerpt.clone();
                        let author = a.author_name.clone();
                        let img = a.cover_image.clone();

                        rsx! {
                            div {
                                style: "display: flex; gap: 1.5rem; margin-bottom: 2.5rem; flex-wrap: wrap;",

                                // Cover
                                if let Some(src) = img {
                                    img {
                                        src: "{src}",
                                        style: "width: 400px; height: 250px; object-fit: cover; flex-shrink: 0; border-radius: 4px; background: #c9c9c9;",
                                    }
                                } else {
                                    div { style: "width: 400px; height: 250px; background: #c9c9c9; flex-shrink: 0; border-radius: 4px;" }
                                }

                                div { style: "flex: 1; min-width: 200px;",
                                    span {
                                        style: "background: {color}; color: #fff; font-size: 0.7rem; font-weight: 700; letter-spacing: 0.1em; padding: 2px 8px; border-radius: 2px; text-transform: uppercase;",
                                        "{cat_label}"
                                    }
                                    h2 { style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.8rem; font-weight: 700; margin: 0.6rem 0 0.5rem; line-height: 1.2; color: var(--text-main);",
                                        Link {
                                            to: Route::Article { slug },
                                            style: "color: inherit; text-decoration: none;",
                                            "{title}"
                                        }
                                    }
                                    p { style: "color: var(--text-main); font-size: 0.95rem; line-height: 1.6; margin: 0 0 1rem;", "{excerpt}" }
                                    p { style: "font-size: 0.8rem; color: var(--text-muted);", "{author} · {date}" }
                                }
                            }
                        }
                    }

                    // Grille des articles suivants
                    if arts.len() > 1 {
                        h2 {
                            style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.4rem; font-weight: 700; border-bottom: 2px solid var(--border); padding-bottom: 0.4rem; margin-bottom: 1.5rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-main);",
                            "En ce moment"
                        }
                        div {
                            style: "display: grid; grid-template-columns: repeat(2, 1fr); gap: 1.5rem;",
                            for a in arts.iter().skip(1) {
                                {
                                    let color = badge_color(&a.category);
                                    let cat_label = a.category.to_string();
                                    let date = fmt_date(a.created_at);
                                    let slug = a.slug.clone();
                                    let title = a.title.clone();
                                    let excerpt = a.excerpt.clone();
                                    let author = a.author_name.clone();
                                    let img = a.cover_image.clone();

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
                                                h3 { style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1rem; font-weight: 700; margin: 0.4rem 0 0.4rem; line-height: 1.3; color: var(--text-main);",
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
}
