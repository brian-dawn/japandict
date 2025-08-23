use dioxus::prelude::*;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Script { src: "https://cdn.tailwindcss.com" }
        Router::<Route> {}
    }
}

#[component]
pub fn DictionarySearch() -> Element {
    rsx! {
        div {
            class: "max-w-2xl mx-auto px-4 sm:px-6",
            div {
                class: "text-center mb-6 sm:mb-8",
                h1 { class: "text-2xl sm:text-4xl font-bold text-white mb-2", "JapanDict" }
                p { class: "text-gray-400 text-sm sm:text-base", "Japanese to English Dictionary" }
            }
            div {
                class: "mb-4 sm:mb-6",
                input {
                    r#type: "text",
                    placeholder: "Search for Japanese words...",
                    class: "w-full px-3 py-2 sm:px-4 sm:py-3 text-base sm:text-lg bg-gray-800 text-white border border-gray-600 rounded-lg focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
                }
            }
            div {
                class: "bg-gray-800 rounded-lg p-3 sm:p-4 min-h-32",
                p { class: "text-gray-400 text-center text-sm sm:text-base", "Search results will appear here..." }
            }
        }
    }
}

/// Home page
#[component]
fn Home() -> Element {
    rsx! {
        DictionarySearch {}
    }
}

/// About page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div {
            class: "max-w-2xl mx-auto px-4 sm:px-6",
            div {
                class: "text-center mb-6",
                h1 { class: "text-2xl sm:text-3xl font-bold text-white mb-4", "About JapanDict" }
                p { class: "text-gray-300 text-sm sm:text-base leading-relaxed", "A simple Japanese to English dictionary app built with Dioxus and Rust." }
            }
        }
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div {
            class: "bg-gray-900 text-white min-h-screen",
            nav {
                class: "flex gap-4 p-4 sm:gap-6 sm:p-6 bg-gray-800",
                Link {
                    to: Route::Home {},
                    class: "text-white hover:text-blue-400 transition-colors duration-200 text-sm sm:text-base",
                    "Dictionary"
                }
                Link {
                    to: Route::Blog { id: 1 },
                    class: "text-white hover:text-blue-400 transition-colors duration-200 text-sm sm:text-base",
                    "About"
                }
            }
            div {
                class: "p-4 sm:p-6",
                Outlet::<Route> {}
            }
        }
    }
}
