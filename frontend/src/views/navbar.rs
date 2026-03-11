use dioxus::prelude::*;
use dioxus::document::eval;
use crate::Route;

#[component]
pub fn Navbar() -> Element {
    use_effect(move || {
        let mut e = eval(r#"
            if (!document.querySelector('[data-font="fredoka"]')) {
                const l = document.createElement('link');
                l.rel = 'stylesheet';
                l.href = 'https://fonts.googleapis.com/css2?family=Fredoka+One&display=swap';
                l.setAttribute('data-font', 'fredoka');
                document.head.appendChild(l);
            }
        "#);
        spawn(async move { let _ = e.recv::<serde_json::Value>().await; });
    });

    let mut is_dark = use_signal(|| true);
    use_effect(move || {
        let mut e = eval(r#"
            dioxus.send(document.documentElement.classList.contains('dark'));
        "#);
        spawn(async move {
            if let Ok(v) = e.recv::<serde_json::Value>().await {
                is_dark.set(v.as_bool().unwrap_or(true));
            }
        });
    });

    rsx! {
        div { class: "flex flex-col h-screen",
            nav { class: "border-b px-4 py-2 flex justify-between items-center flex-shrink-0",
                style: "background:var(--bg-main);border-color:var(--border);",

                Link {
                    to: Route::Chat {},
                    class: "flex items-center gap-2 hover:opacity-80 transition",
                    span {
                        class: "text-2xl font-bold text-amber-400",
                        style: "font-family: 'Fredoka One', cursive; letter-spacing: 0.5px;",
                        "Le Zoo"
                    }
                }

                div {
                    class: "cursor-pointer",
                    onclick: move |_| {
                        let nd = !*is_dark.read();
                        is_dark.set(nd);
                        let js = if nd {
                            r#"document.documentElement.classList.add('dark');localStorage.setItem('theme','dark');"#
                        } else {
                            r#"document.documentElement.classList.remove('dark');localStorage.setItem('theme','light');"#
                        };
                        let mut e = eval(js);
                        spawn(async move { let _ = e.recv::<serde_json::Value>().await; });
                    },
                    div {
                        class: if *is_dark.read() {
                            "w-11 h-6 rounded-full bg-orange-500 cursor-pointer flex items-center px-0.5 transition-colors duration-300"
                        } else {
                            "w-11 h-6 rounded-full bg-gray-200 cursor-pointer flex items-center px-0.5 transition-colors duration-300"
                        },
                        div {
                            class: "w-5 h-5 rounded-full bg-white shadow transition-transform duration-300",
                            style: if *is_dark.read() { "transform: translateX(20px);" } else { "transform: translateX(0px);" }
                        }
                    }
                }
            }

            div { class: "flex-1 overflow-hidden",
                Outlet::<Route> {}
            }
        }
    }
}
