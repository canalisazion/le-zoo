use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use crate::Route;

#[component]
pub fn ChatBubble() -> Element {
    let nav = use_navigator();
    let current_route = use_route::<Route>();

    // Don't render on /chat
    if current_route == (Route::Chat {}) {
        return rsx! {};
    }

    rsx! {
        div {
            style: "position: fixed; bottom: 1.5rem; right: 1.5rem; z-index: 50;",

            // Pulse ring
            div {
                style: "position: absolute; inset: 0; border-radius: 50%; background: #333; opacity: 0.35; animation: bubble-pulse 2s ease-in-out infinite;",
            }

            // Main circle button
            button {
                style: "position: relative; width: 56px; height: 56px; border-radius: 50%; background: #333; border: 1.5px solid #555; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 1.5rem; box-shadow: 0 4px 14px rgba(0,0,0,0.45); animation: bubble-pulse-shadow 2s ease-in-out infinite;",
                onclick: move |_| {
                    let username: String = LocalStorage::get("username").unwrap_or_default();
                    if username.is_empty() || username == "Anonyme" {
                        nav.push(Route::Login {});
                    } else {
                        nav.push(Route::Chat {});
                    }
                },
                "🤖"
            }
        }
    }
}
