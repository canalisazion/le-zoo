use dioxus::prelude::*;
use crate::views::{Register, Login, Chat, Navbar};

mod components;
mod views;
mod config;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    Register {},
    
    #[route("/login")]
    Login {},

    #[layout(Navbar)]
        #[route("/chat")]
        Chat {},
}

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
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