//! Dictionary search using feature-based scoring
//! 
//! 1. Detect query type (kanji/kana/english)
//! 2. Generate candidates with exact/prefix/fuzzy matching  
//! 3. Score with weighted features prioritizing common words
//! 4. Tie-break consistently

use crate::dictionary::*;
use dictionary_data::WORD_COUNT;
use std::collections::HashMap;
use std::sync::OnceLock;

// Global search indices - built once on startup
static ENGLISH_INDEX: OnceLock<HashMap<String, Vec<usize>>> = OnceLock::new();
static KANJI_INDEX: OnceLock<HashMap<String, Vec<usize>>> = OnceLock::new();
static KANA_INDEX: OnceLock<HashMap<String, Vec<usize>>> = OnceLock::new();

/// Build complete search indices on startup - much faster than on-demand caching
pub fn build_search_indices() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Build indices in parallel for better performance on native platforms
        std::thread::scope(|s| {
            let english_handle = s.spawn(|| build_english_index());
            let kanji_handle = s.spawn(|| build_kanji_index()); 
            let kana_handle = s.spawn(|| build_kana_index());
            
            ENGLISH_INDEX.set(english_handle.join().unwrap()).unwrap();
            KANJI_INDEX.set(kanji_handle.join().unwrap()).unwrap();
            KANA_INDEX.set(kana_handle.join().unwrap()).unwrap();
        });
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Build indices sequentially on WASM since threading is not supported
        ENGLISH_INDEX.set(build_english_index()).unwrap();
        KANJI_INDEX.set(build_kanji_index()).unwrap();
        KANA_INDEX.set(build_kana_index()).unwrap();
    }
}

fn build_english_index() -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    
    for idx in 0..WORD_COUNT {
        let entry = get_word_entry(idx);
        
        for english in &entry.english {
            let normalized = normalize_query(english);
            
            // Index the full meaning
            index.entry(normalized.clone()).or_default().push(idx);
            
            // Index individual words within the meaning
            let words: Vec<&str> = normalized.split_whitespace().collect();
            for word in words {
                if word.len() > 1 { // Skip single letters
                    index.entry(word.to_string()).or_default().push(idx);
                }
            }
            
            // Index first word of each semicolon-separated meaning
            let meanings: Vec<&str> = normalized.split(';').collect();
            for meaning in meanings {
                let meaning = meaning.trim();
                if !meaning.is_empty() {
                    index.entry(meaning.to_string()).or_default().push(idx);
                    
                    // Also index first word of the meaning
                    if let Some(first_word) = meaning.split_whitespace().next() {
                        if first_word.len() > 1 {
                            index.entry(first_word.to_string()).or_default().push(idx);
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate all vectors
    for vec in index.values_mut() {
        vec.sort_unstable();
        vec.dedup();
    }
    
    index
}

fn build_kanji_index() -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    
    for idx in 0..WORD_COUNT {
        let entry = get_word_entry(idx);
        
        for kanji in &entry.kanji {
            index.entry(kanji.to_string()).or_default().push(idx);
        }
    }
    
    index
}

fn build_kana_index() -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    
    for idx in 0..WORD_COUNT {
        let entry = get_word_entry(idx);
        
        for kana in &entry.kana {
            index.entry(kana.to_string()).or_default().push(idx);
        }
    }
    
    index
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: WordEntry,
    pub score: f32,
    pub features: Features,
}

#[derive(Debug, Clone, Default)]
pub struct Features {
    pub exact_form: bool,        // exact kanji match
    pub exact_reading: bool,     // exact kana match  
    pub prefix: bool,            // starts with query
    pub edit_distance: u8,       // edit distance on readings
    pub has_common: bool,        // JMdict common word
    pub shorter_lemma: bool,     // prefer shorter forms
    pub gloss_hit: bool,         // english definition match
    pub first_gloss: bool,       // first word in definition
    pub exact_english: bool,     // query matches exact English word (not compound)
    pub learner_friendly: bool,  // basic form for learners  
    pub simple_form: bool,       // simple basic form vs compound
}

fn score_features(features: &Features) -> f32 {
    let mut score = 0.0;
    
    // Exact matches get highest priority
    if features.exact_form     { score += 100.0; }
    if features.exact_reading  { score += 95.0; }
    
    // Exact English word match gets highest priority
    if features.exact_english  { score += 250.0; }
    
    // First meaning exact match for English queries - much higher priority!
    if features.first_gloss    { score += 200.0; }
    
    // Common words bonus (but not overwhelming)  
    if features.has_common     { score += 50.0; }
    
    // Prefix matches
    if features.prefix         { score += 30.0; }
    
    // General English matches  
    if features.gloss_hit && !features.first_gloss { score += 10.0; }
    
    // Simple basic forms preferred for learners
    if features.simple_form { score += 25.0; }
    
    // Quality signals  
    if features.shorter_lemma  { score += 5.0; }
    
    // Edit distance penalty
    score -= 2.0 * (features.edit_distance as f32);
    
    score
}

#[derive(Debug)]
enum QueryType {
    Kanji,      // contains kanji characters
    Kana,       // all hiragana/katakana
    English,    // latin letters
}

fn detect_query_type(query: &str) -> QueryType {
    let has_kanji = query.chars().any(|c| {
        // Basic kanji range (there are more, but this covers most)
        '\u{4E00}' <= c && c <= '\u{9FAF}'
    });
    
    let has_kana = query.chars().any(|c| {
        // Hiragana and katakana ranges
        ('\u{3040}' <= c && c <= '\u{309F}') || ('\u{30A0}' <= c && c <= '\u{30FF}')
    });
    
    if has_kanji || has_kana {
        if has_kanji {
            QueryType::Kanji
        } else {
            QueryType::Kana
        }
    } else {
        QueryType::English
    }
}

fn normalize_query(query: &str) -> String {
    // Basic normalization: lowercase, trim
    query.trim().to_lowercase()
}

fn detect_simple_form(entry: &WordEntry, _query: &str) -> bool {
    // For adjectives, prefer i-adjective forms over nouns
    if entry.pos.iter().any(|p| p.contains("adj-i")) && entry.kanji.iter().any(|k| k.ends_with("い")) {
        return true;
    }
    
    // Prefer simple forms (single kanji or short kana) over compounds
    let has_simple_kanji = entry.kanji.iter().any(|k| k.chars().count() <= 2);
    let has_simple_kana = entry.kana.iter().any(|k| k.chars().count() <= 4);
    
    has_simple_kanji || has_simple_kana
}

fn simple_edit_distance(a: &str, b: &str) -> u8 {
    // Simple implementation - for production consider using strsim crate
    if a == b { return 0; }
    if (a.len() as i32 - b.len() as i32).abs() > 2 { return 3; } // early exit
    
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    
    if a_chars.len() < b_chars.len() {
        return simple_edit_distance(b, a);
    }
    
    // Very basic distance - count differing positions
    let mut diff = 0;
    for i in 0..a_chars.len().min(b_chars.len()) {
        if a_chars[i] != b_chars[i] {
            diff += 1;
        }
    }
    diff += (a_chars.len() - b_chars.len()) as u8;
    diff.min(3) // cap at 3
}

fn evaluate_entry(entry: &WordEntry, query: &str, query_type: &QueryType) -> Option<SearchResult> {
    let normalized_query = normalize_query(query);
    let mut features = Features::default();
    
    
    // Check for exact matches first
    match query_type {
        QueryType::Kanji => {
            // Check kanji forms
            for kanji in &entry.kanji {
                if kanji.to_lowercase() == normalized_query {
                    features.exact_form = true;
                    break;
                }
                if kanji.to_lowercase().starts_with(&normalized_query) {
                    features.prefix = true;
                }
            }
            
            // Also check kana readings for mixed queries
            for kana in &entry.kana {
                if kana.to_lowercase() == normalized_query {
                    features.exact_reading = true;
                    break;
                }
            }
        }
        
        QueryType::Kana => {
            // Check kana readings
            for kana in &entry.kana {
                let kana_lower = kana.to_lowercase();
                if kana_lower == normalized_query {
                    features.exact_reading = true;
                    break;
                }
                if kana_lower.starts_with(&normalized_query) {
                    features.prefix = true;
                }
                
                // Compute edit distance for fuzzy matching
                let dist = simple_edit_distance(&kana_lower, &normalized_query);
                if dist <= 2 && features.edit_distance == 0 {
                    features.edit_distance = dist;
                }
            }
        }
        
        QueryType::English => {
            // Check English glosses - be more precise about word boundaries
            let mut is_very_first = true;
            for english in &entry.english {
                let english_lower = english.to_lowercase();
                
                // Split by semicolon for separate meanings
                let meanings: Vec<&str> = english_lower.split(';').collect();
                
                for (i, meaning) in meanings.iter().enumerate() {
                    let clean_meaning = meaning.trim();
                    let is_first_meaning = is_very_first && i == 0;
                    
                    
                    // Exact meaning match (full definition)
                    if clean_meaning == normalized_query {
                        features.gloss_hit = true;
                        features.exact_english = true;
                        if is_first_meaning {
                            features.first_gloss = true;
                        }
                        break;
                    }
                    
                    // Exact "to [verb]" meaning match
                    if clean_meaning == format!("to {}", normalized_query) {
                        features.gloss_hit = true;
                        features.exact_english = true;
                        if is_first_meaning {
                            features.first_gloss = true;
                        }
                        break;
                    }
                    
                    // Check first word of meaning - handle "to verb" case for Japanese verbs
                    let words: Vec<&str> = clean_meaning.split_whitespace().collect();
                    if let Some(first_word) = words.first() {
                        let clean_first = first_word.trim_matches(|c: char| !c.is_alphabetic());
                        if clean_first == normalized_query {
                            features.gloss_hit = true;
                            // If it's a single word meaning, it's also exact
                            if words.len() == 1 {
                                features.exact_english = true;
                            }
                            if is_first_meaning {
                                features.first_gloss = true;
                            }
                            break;
                        }
                    }
                    
                    // Handle "to [verb]" case - check second word if first is "to"
                    if words.len() >= 2 && words[0] == "to" {
                        let second_word = words[1].trim_matches(|c: char| !c.is_alphabetic());
                        if second_word == normalized_query {
                            features.gloss_hit = true;
                            // If it's just "to [verb]", it's exact
                            if words.len() == 2 {
                                features.exact_english = true;
                            }
                            if is_first_meaning {
                                features.first_gloss = true;
                            }
                            break;
                        }
                    }
                    
                    // Check if query appears as a complete word (not substring)
                    if clean_meaning.split_whitespace().any(|word| {
                        let clean_word = word.trim_matches(|c: char| !c.is_alphabetic());
                        clean_word == normalized_query
                    }) {
                        features.gloss_hit = true;
                        // Don't set first_gloss here since it's not necessarily first word
                    }
                }
                
                is_very_first = false;
                if features.first_gloss { break; }
            }
        }
    }
    
    
    // If no matches found, skip this entry
    if !features.exact_form && !features.exact_reading && !features.prefix && 
       !features.gloss_hit && features.edit_distance == 0 {
        return None;
    }
    
    // Set quality features
    features.has_common = entry.is_common;
    
    // Shorter lemma bonus - prefer simpler forms
    features.shorter_lemma = entry.kanji.iter().any(|k| k.chars().count() <= 2) ||
                            entry.kana.iter().any(|k| k.chars().count() <= 3);
    
    // Simple form: prefer basic single-concept words
    features.simple_form = detect_simple_form(entry, query);
    
    let score = score_features(&features);
    
    
    Some(SearchResult {
        entry: entry.clone(),
        score,
        features,
    })
}

fn find_indexed_entries(query: &str, query_type: &QueryType) -> Vec<usize> {
    let normalized_query = normalize_query(query);
    let mut candidates = Vec::new();
    
    match query_type {
        QueryType::English => {
            if let Some(english_index) = ENGLISH_INDEX.get() {
                // Exact match first
                if let Some(indices) = english_index.get(&normalized_query) {
                    candidates.extend_from_slice(indices);
                }
                
                // Prefix matches if no exact match
                if candidates.is_empty() {
                    for (word, indices) in english_index {
                        if word.starts_with(&normalized_query) && word != &normalized_query {
                            candidates.extend_from_slice(indices);
                        }
                    }
                }
            }
        },
        QueryType::Kanji => {
            if let Some(kanji_index) = KANJI_INDEX.get() {
                // Exact and prefix matches
                for (kanji, indices) in kanji_index {
                    if kanji == &normalized_query || kanji.starts_with(&normalized_query) {
                        candidates.extend_from_slice(indices);
                    }
                }
            }
        },
        QueryType::Kana => {
            if let Some(kana_index) = KANA_INDEX.get() {
                // Exact and prefix matches
                for (kana, indices) in kana_index {
                    if kana == &normalized_query || kana.starts_with(&normalized_query) {
                        candidates.extend_from_slice(indices);
                    }
                }
            }
        }
    }
    
    // Remove duplicates and limit
    candidates.sort_unstable();
    candidates.dedup();
    candidates.truncate(1000);
    candidates
}

pub fn search_dictionary(query: &str) -> Vec<WordEntry> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    
    let query_type = detect_query_type(query);
    
    // Try index-based search first for exact/prefix matches
    let indexed_candidates = find_indexed_entries(query, &query_type);
    
    let mut results = Vec::with_capacity(200);
    
    if !indexed_candidates.is_empty() {
        // Process indexed candidates first
        for &idx in &indexed_candidates {
            let entry = crate::dictionary::get_word_entry(idx);
            if let Some(search_result) = evaluate_entry(&entry, query, &query_type) {
                results.push(search_result);
            }
        }
    } else {
        // Fallback to full scan for fuzzy matches
        for i in 0..WORD_COUNT.min(5000) { // Limit scan for performance
            let entry = crate::dictionary::get_word_entry(i);
            if let Some(search_result) = evaluate_entry(&entry, query, &query_type) {
                results.push(search_result);
                if results.len() >= 200 {
                    break;
                }
            }
        }
    }
    
    // Sort by score (highest first), then by consistent tie-breakers
    results.sort_by(|a, b| {
        // Primary: score (higher is better)
        let score_cmp = b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }
        
        // Tie-breaker 1: Common words first
        let common_cmp = b.entry.is_common.cmp(&a.entry.is_common);
        if common_cmp != std::cmp::Ordering::Equal {
            return common_cmp;
        }
        
        // Tie-breaker 2: Shorter kanji/kana forms first (simpler)
        let a_len = a.entry.kanji.iter().chain(&a.entry.kana).map(|s| s.len()).min().unwrap_or(100);
        let b_len = b.entry.kanji.iter().chain(&b.entry.kana).map(|s| s.len()).min().unwrap_or(100);
        let len_cmp = a_len.cmp(&b_len);
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }
        
        // Tie-breaker 3: Lexicographic order for consistency
        let a_key = a.entry.kanji.first().or(a.entry.kana.first()).unwrap_or(&"");
        let b_key = b.entry.kanji.first().or(b.entry.kana.first()).unwrap_or(&"");
        a_key.cmp(b_key)
    });
    
    results.into_iter()
        .take(50)
        .map(|result| result.entry)
        .collect()
}