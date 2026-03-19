use dioxus::prelude::*;
use crate::views::{Register, Login, Chat, Navbar, ForgotPassword, ResetPassword, Home, Article, Rubrique, ArticleEditor};

mod components;
mod views;
mod config;
pub mod fetch_creds; // ✅ [H-8]

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
        #[route("/")]
        Home {},

        #[route("/article/:slug")]
        Article { slug: String },

        #[route("/rubrique/:category")]
        Rubrique { category: String },

        #[route("/chat")]
        Chat {},

        #[route("/login")]
        Login {},

        #[route("/register")]
        Register {},

        #[route("/forgot-password")]
        ForgotPassword {},

        #[route("/reset-password")]
        ResetPassword {},

        #[route("/editor")]
        ArticleEditor {},
}

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const CUSTOM_CSS: Asset = asset!("/assets/custom.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: CUSTOM_CSS }
        document::Meta { name: "description", content: "Le Zoo — Forum de discussion en temps réel. Rejoignez la communauté francophone pour parler cinéma, sport, actu et plus encore." }
        document::Meta { name: "keywords", content: "forum, discussion, chat, temps réel, francophone, cinéma, sport, actualités" }
        document::Meta { property: "og:title", content: "Le Zoo — Forum en temps réel" }
        document::Meta { property: "og:description", content: "Rejoignez Le Zoo, le forum de discussion francophone en temps réel." }
        document::Meta { property: "og:type", content: "website" }
        document::Meta { name: "robots", content: "index, follow" }
        document::Link { rel: "canonical", href: "https://lezoo.fr" }

        Router::<Route> {}
    }
}