use dioxus::prelude::*;
use japandict_core::{search_dictionary, WordEntry};

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();
    
    // Build search indices on startup for fast searches
    japandict_core::search::build_search_indices();
    
    launch(App);
}

#[component]
fn App() -> Element {
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<WordEntry>::new);
    let mut perform_search = move |q: String| {
        let search_results = search_dictionary(&q);
        results.set(search_results);
    };

    rsx! {
        div {
            class: "min-h-screen bg-gray-50",
            
            header {
                class: "bg-white shadow-sm border-b",
                div {
                    class: "max-w-4xl mx-auto px-4 py-6",
                    h1 {
                        class: "text-3xl font-bold text-gray-900 flex items-center gap-2",
                        "🗾 Japanese Dictionary"
                    }
                    p {
                        class: "text-gray-600 mt-2",
                        "Search Japanese words and phrases with English translations"
                    }
                }
            }
            
            main {
                class: "max-w-4xl mx-auto px-4 py-8",
                
                SearchBox {
                    query: query.read().clone(),
                    on_search: move |q: String| {
                        query.set(q.clone());
                        if !q.trim().is_empty() {
                            perform_search(q);
                        } else {
                            results.set(Vec::new());
                        }
                    }
                }
                
                if !results.read().is_empty() {
                    ResultsSection {
                        results: results.read().clone(),
                        query: query.read().clone()
                    }
                }
            }
        }
    }
}

#[component]
fn SearchBox(query: String, on_search: EventHandler<String>) -> Element {
    rsx! {
        div {
            class: "mb-8",
            
            div {
                class: "relative",
                input {
                    r#type: "text",
                    value: "{query}",
                    placeholder: "Search for Japanese words or English translations...",
                    class: "w-full px-4 py-3 pl-12 text-lg border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none transition-all",
                    oninput: move |e| on_search.call(e.value().clone()),
                }
                
                div {
                    class: "absolute left-4 top-1/2 transform -translate-y-1/2 text-gray-400",
                    "🔍"
                }
            }
            
            div {
                class: "mt-2 text-sm text-gray-500",
                "Try searching for: \"dog\", \"water\", \"good\", \"犬\", \"水\", or \"良い\""
            }
        }
    }
}

#[component]
fn ResultsSection(
    results: Vec<WordEntry>, 
    query: String
) -> Element {
    rsx! {
        div {
            class: "space-y-6",
            
            div {
                class: "flex items-center justify-between",
                h2 {
                    class: "text-xl font-semibold text-gray-900",
                    "Search Results for \"{query}\""
                }
                div {
                    class: "text-sm text-gray-500",
                    "{results.len()} results found"
                }
            }
            
            div {
                class: "grid gap-4",
{results.iter().take(20).enumerate().map(|(i, entry)| {
                    rsx! {
                        ResultCard {
                            key: "{i}",
                            entry: entry.clone(),
                            rank: i + 1
                        }
                    }
                })}
            }
            
            if results.len() > 20 {
                div {
                    class: "text-center py-4 text-gray-500",
                    "... and {results.len() - 20} more results"
                }
            }
        }
    }
}

#[component]
fn ResultCard(entry: WordEntry, rank: usize) -> Element {
    rsx! {
        div {
            class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow",
            
            div {
                class: "flex items-start gap-4",
                
                div {
                    class: "flex-shrink-0 w-8 h-8 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center text-sm font-medium",
                    "{rank}"
                }
                
                div {
                    class: "flex-1 min-w-0",
                    
                    div {
                        class: "flex flex-wrap items-center gap-2 mb-2",
                        
                        // Kanji
                        if !entry.kanji.is_empty() {
                            div {
                                class: "flex flex-wrap gap-1",
{entry.kanji.iter().map(|kanji| rsx! {
                                    span {
                                        class: "text-2xl font-bold text-purple-600",
                                        "{kanji}"
                                    }
                                })}
                            }
                        }
                        
                        // Kana
                        if !entry.kana.is_empty() {
                            div {
                                class: "flex flex-wrap gap-1",
                                "("
{entry.kana.iter().enumerate().map(|(i, kana)| rsx! {
                                    span {
                                        class: "text-lg text-blue-600",
                                        "{kana}"
                                        if i < entry.kana.len() - 1 { ", " }
                                    }
                                })}
                                ")"
                            }
                        }
                        
                        // Common word indicator
                        if entry.is_common {
                            span {
                                class: "inline-flex items-center px-2 py-1 text-xs font-medium bg-yellow-100 text-yellow-800 rounded-full",
                                "⭐ Common"
                            }
                        }
                    }
                    
                    // English definitions
                    if !entry.english.is_empty() {
                        div {
                            class: "text-gray-700 mb-2",
{entry.english.iter().take(3).enumerate().map(|(i, eng)| rsx! {
                                span {
                                    "{eng}"
                                    if i < entry.english.len().min(3) - 1 { "; " }
                                }
                            })}
                        }
                    }
                    
                    // Part of speech
                    if !entry.pos.is_empty() {
                        div {
                            class: "flex flex-wrap gap-1",
{entry.pos.iter().map(|pos| rsx! {
                                span {
                                    class: "inline-flex items-center px-2 py-1 text-xs font-medium bg-gray-100 text-gray-600 rounded",
                                    "{pos}"
                                }
                            })}
                        }
                    }
                }
            }
        }
    }
}