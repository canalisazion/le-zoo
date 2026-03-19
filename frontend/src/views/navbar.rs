use dioxus::prelude::*;
use dioxus::document::eval;
use gloo_storage::{LocalStorage, Storage};
use crate::Route;
use crate::components::chat_bubble::ChatBubble;

#[component]
pub fn Navbar() -> Element {
    // Load Google Fonts
    use_effect(move || {
        let mut e = eval(r#"
            if (!document.querySelector('[data-font="playfair"]')) {
                const l = document.createElement('link');
                l.rel = 'stylesheet';
                l.href = 'https://fonts.googleapis.com/css2?family=Playfair+Display:wght@700;800&family=Inter:wght@400;600&display=swap';
                l.setAttribute('data-font', 'playfair');
                document.head.appendChild(l);
            }
        "#);
        spawn(async move { let _ = e.recv::<serde_json::Value>().await; });
    });

    let mut burger_open = use_signal(|| false);
    let role: String = LocalStorage::get("role").unwrap_or_default();
    let is_admin = role == "admin" || role == "super_admin";

    let nav_links = [
        ("Dossiers",    "/rubrique/dossier"),
        ("Insolite",    "/rubrique/insolite"),
        ("Faits Divers","/rubrique/faits-divers"),
        ("Culture",     "/rubrique/culture"),
        ("Formations",  "/rubrique/formation"),
    ];

    rsx! {
        div { style: "display: flex; flex-direction: column; min-height: 100vh;",

            // ── Top navbar ──────────────────────────────────────────────
            nav {
                style: "background: var(--bg-main); border-bottom: 1px solid var(--border); padding: 0 1.5rem; display: flex; align-items: center; justify-content: space-between; height: 56px; flex-shrink: 0; position: sticky; top: 0; z-index: 40;",

                // Logo
                Link {
                    to: Route::Home {},
                    style: "font-family: 'Playfair Display', Georgia, serif; font-size: 1.6rem; font-weight: 800; color: var(--accent, #c0392b); text-decoration: none; letter-spacing: -0.5px;",
                    "Le Zoo"
                }

                // Desktop links
                div {
                    class: "nav-links-desktop",
                    style: "font-family: Inter, sans-serif; font-size: 0.85rem; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase;",

                    for (label, href) in nav_links.iter() {
                        if *label == "Formations" {
                            a {
                                href: *href,
                                style: "color: var(--text-main, #222); text-decoration: none; opacity: 0.8; transition: opacity 0.15s;",
                                "{label}"
                                span {
                                    style: "font-size: 0.55rem; background: #27ae60; color: white; padding: 1px 4px; border-radius: 2px; vertical-align: super; margin-left: 3px; font-weight: 700; letter-spacing: 0.05em;",
                                    "GRATUIT"
                                }
                            }
                        } else {
                            a {
                                href: *href,
                                style: "color: var(--text-main, #222); text-decoration: none; opacity: 0.8; transition: opacity 0.15s;",
                                "{label}"
                            }
                        }
                    }

                    // Lien éditeur (admin uniquement)
                    if is_admin {
                        Link {
                            to: Route::ArticleEditor {},
                            style: "color: var(--text-main, #222); text-decoration: none; opacity: 0.8; font-size: 0.85rem; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase;",
                            "✏️ Écrire"
                        }
                    }

                    // Forum button
                    Link {
                        to: Route::Chat {},
                        style: "position: relative; background: #111; color: var(--accent); border: 1.5px solid var(--accent); border-radius: 6px; padding: 0.25rem 0.9rem; text-decoration: none; text-align: center; line-height: 1.2; display: inline-block;",
                        span { style: "font-size: 0.6rem; display: block; letter-spacing: 0.1em;", "Chat" }
                        span { style: "font-size: 0.75rem; display: block; font-weight: 800;", "Forum" }
                        div {
                            style: "position: absolute; top: -4px; right: -4px; width: 18px; height: 18px; border-radius: 50%; background: #333; border: 1.5px solid #555; display: flex; align-items: center; justify-content: center; animation: globe-spin 3s linear infinite;",
                            span { style: "color: #fff; font-size: 8px; line-height: 1;", "●" }
                        }
                    }

                    // Burger (mobile)
                    button {
                        class: "nav-burger",
                        style: "background: none; border: none; font-size: 1.4rem; cursor: pointer; color: var(--text-main, #222);",
                        onclick: move |_| { let v = *burger_open.read(); burger_open.set(!v); },
                        "☰"
                    }
                }
            }

            // ── Mobile burger menu ──────────────────────────────────────
            if *burger_open.read() {
                div {
                    style: "background: var(--bg-main); border-bottom: 1px solid var(--border); display: flex; flex-direction: column; gap: 0; font-family: Inter, sans-serif;",
                    for (label, href) in nav_links.iter() {
                        if *label == "Formations" {
                            a {
                                href: *href,
                                style: "padding: 0.75rem 1.5rem; color: var(--text-main, #222); text-decoration: none; font-size: 0.9rem; font-weight: 600; border-bottom: 1px solid var(--border); opacity: 0.85;",
                                "{label}"
                                span {
                                    style: "font-size: 0.55rem; background: #27ae60; color: white; padding: 1px 4px; border-radius: 2px; vertical-align: super; margin-left: 3px; font-weight: 700;",
                                    "GRATUIT"
                                }
                            }
                        } else {
                            a {
                                href: *href,
                                style: "padding: 0.75rem 1.5rem; color: var(--text-main, #222); text-decoration: none; font-size: 0.9rem; font-weight: 600; border-bottom: 1px solid var(--border); opacity: 0.85;",
                                "{label}"
                            }
                        }
                    }
                    if is_admin {
                        Link {
                            to: Route::ArticleEditor {},
                            style: "padding: 0.75rem 1.5rem; color: var(--text-main, #222); text-decoration: none; font-size: 0.9rem; font-weight: 600; border-bottom: 1px solid var(--border);",
                            "✏️ Écrire"
                        }
                    }
                    Link {
                        to: Route::Chat {},
                        style: "padding: 0.75rem 1.5rem; color: var(--accent); text-decoration: none; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 0.5rem;",
                        div { style: "background: #111; border: 1.5px solid var(--accent); border-radius: 6px; padding: 0.2rem 0.8rem; text-align: center; line-height: 1.2;",
                            span { style: "font-size: 0.6rem; display: block; letter-spacing: 0.1em; color: var(--accent);", "Chat" }
                            span { style: "font-size: 0.75rem; display: block; font-weight: 800; color: var(--accent);", "Forum" }
                        }
                    }
                }
            } else {
                div {}
            }

            // ── Page content ────────────────────────────────────────────
            div { style: "flex: 1;",
                Outlet::<Route> {}
            }

            ChatBubble {}
        }
    }
}
