use dioxus::prelude::*;

#[component]
pub fn CGUModal(mut show_cgu: Signal<bool>) -> Element {
    rsx! {
        if *show_cgu.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4",
                onclick: move |_| show_cgu.set(false),
                div {
                    class: "bg-gray-800 p-6 rounded-xl shadow-2xl w-full max-w-2xl max-h-[80vh] overflow-y-auto",
                    onclick: move |e| e.stop_propagation(),
                    h2 { class: "text-2xl font-bold mb-4", "📜 Conditions Générales d'Utilisation" }
                    div { class: "space-y-3 text-sm text-gray-300",
                        p { "En utilisant ce service, vous acceptez les conditions suivantes :" }
                        p { "1. Vous devez avoir au moins 13 ans pour utiliser ce service." }
                        p { "2. Vous êtes responsable du contenu que vous publiez." }
                        p { "3. Tout contenu illégal, offensant ou nuisible sera supprimé et peut entraîner un bannissement." }
                        p { "4. Nous nous réservons le droit de supprimer tout contenu sans préavis." }
                        p { "5. Votre compte peut être supprimé à tout moment." }
                        p { "6. Ce service est fourni \"tel quel\" sans garantie." }
                    }
                    button {
                        class: "mt-4 w-full bg-blue-600 hover:bg-blue-500 py-2 rounded-lg",
                        onclick: move |_| show_cgu.set(false),
                        "Fermer"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ConfidentialityModal(mut show_confidentialite: Signal<bool>) -> Element {
    rsx! {
        if *show_confidentialite.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4",
                onclick: move |_| show_confidentialite.set(false),
                div {
                    class: "bg-gray-800 p-6 rounded-xl shadow-2xl w-full max-w-2xl max-h-[80vh] overflow-y-auto",
                    onclick: move |e| e.stop_propagation(),
                    h2 { class: "text-2xl font-bold mb-4", "🔒 Politique de Confidentialité (RGPD)" }
                    div { class: "space-y-3 text-sm text-gray-300",
                        p { "Vos données personnelles sont traitées conformément au RGPD :" }
                        p { "1. **Données collectées** : email, username, messages, avatar." }
                        p { "2. **Utilisation** : Fonctionnement du service et modération." }
                        p { "3. **Conservation** : Tant que votre compte est actif." }
                        p { "4. **Vos droits** : Accès, rectification, suppression de vos données." }
                        p { "5. **Suppression** : Vous pouvez supprimer votre compte dans Paramètres." }
                        p { "6. **Partage** : Nous ne partageons pas vos données avec des tiers." }
                        p { "7. **Cookies** : Nous utilisons localStorage pour maintenir votre session." }
                    }
                    button {
                        class: "mt-4 w-full bg-blue-600 hover:bg-blue-500 py-2 rounded-lg",
                        onclick: move |_| show_confidentialite.set(false),
                        "Fermer"
                    }
                }
            }
        }
    }
}
