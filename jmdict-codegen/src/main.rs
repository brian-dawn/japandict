use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use flate2::read::GzDecoder;
use tar::Archive;

#[derive(Debug, Deserialize)]
struct JMDict {
    #[serde(rename = "commonOnly")]
    #[allow(dead_code)]
    common_only: bool,
    #[serde(rename = "dictDate")]
    #[allow(dead_code)]
    dict_date: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    languages: Vec<String>,
    words: Vec<Word>,
}

#[derive(Debug, Deserialize)]
struct Word {
    id: String,
    kanji: Option<Vec<KanjiEntry>>,
    kana: Vec<KanaEntry>,
    sense: Vec<Sense>,
}

#[derive(Debug, Deserialize)]
struct KanjiEntry {
    text: String,
    common: Option<bool>,
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
    #[allow(dead_code)]
    priority: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct KanaEntry {
    text: String,
    common: Option<bool>,
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
    #[allow(dead_code)]
    priority: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Sense {
    gloss: Vec<Gloss>,
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<Vec<String>>,
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
    #[allow(dead_code)]
    misc: Option<Vec<String>>,
    #[allow(dead_code)]
    info: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Gloss {
    lang: String,
    text: String,
}

#[derive(Parser)]
#[command(name = "generate_dictionary")]
#[command(about = "Generate static dictionary data from JMDict")]
struct Args {
    #[arg(long, default_value = "0")]
    limit: usize,
}

fn main() {
    let args = Args::parse();
    
    // For web builds, limit to common words only to reduce WASM size
    let limit_for_web = std::env::var("CARGO_CFG_TARGET_ARCH")
        .map(|arch| arch == "wasm32")
        .unwrap_or(false);
    
    let effective_limit = if limit_for_web && args.limit == 0 {
        15000 // Only most common words for web to keep WASM size reasonable
    } else {
        args.limit
    };
    
    let tgz_data = include_bytes!("../assets/jmdict-eng-3.6.1+20250818123231.json.tgz");
    let decoder = GzDecoder::new(&tgz_data[..]);
    let mut archive = Archive::new(decoder);
    
    let mut json_content = String::new();
    let entries = archive.entries().expect("Failed to read tar entries");
    
    for entry_result in entries {
        let mut entry = entry_result.expect("Failed to read tar entry");
        let path = entry.header().path().expect("Failed to read entry path");
        
        if let Some(path_str) = path.to_str() {
            if path_str.ends_with(".json") {
                entry.read_to_string(&mut json_content).expect("Failed to read JSON content");
                break;
            }
        }
    }
    
    let jmdict: JMDict = serde_json::from_str(&json_content).expect("Failed to parse JSON");
    
    // String pools for deduplication
    let mut kanji_pool: HashMap<String, u32> = HashMap::new();
    let mut kana_pool: HashMap<String, u32> = HashMap::new(); 
    let mut english_pool: HashMap<String, u32> = HashMap::new();
    let mut pos_pool: HashMap<String, u32> = HashMap::new();
    let mut id_pool: HashMap<String, u32> = HashMap::new();
    
    let mut kanji_strings = Vec::new();
    let mut kana_strings = Vec::new();
    let mut english_strings = Vec::new(); 
    let mut pos_strings = Vec::new();
    let mut id_strings = Vec::new();
    
    fn get_or_insert(pool: &mut HashMap<String, u32>, strings: &mut Vec<String>, s: &str) -> u32 {
        if let Some(&idx) = pool.get(s) {
            idx
        } else {
            let idx = strings.len() as u32;
            pool.insert(s.to_string(), idx);
            strings.push(s.to_string());
            idx
        }
    }
    
    let mut word_entries = Vec::new();
    
    // Sort words by common status first (common words first), then take the limit
    let mut words_to_process: Vec<&Word> = jmdict.words.iter().collect();
    words_to_process.sort_by_key(|word| {
        // Check if word is common (any kanji or kana entry marked as common)
        let is_common = word.kanji.as_ref().map_or(false, |kanji_entries| {
            kanji_entries.iter().any(|k| k.common.unwrap_or(false))
        }) || word.kana.iter().any(|k| k.common.unwrap_or(false));
        
        // Sort common words first (false sorts before true, so negate)
        !is_common
    });
    
    let word_count = if effective_limit > 0 { effective_limit.min(words_to_process.len()) } else { words_to_process.len() };
    
    for (_i, word) in words_to_process.iter().take(word_count).enumerate() {
        let id_idx = get_or_insert(&mut id_pool, &mut id_strings, &word.id);
        
        let mut kanji_indices = Vec::new();
        let mut kana_indices = Vec::new();
        let mut english_indices = Vec::new();
        let mut pos_indices = Vec::new();
        
        // Check if word is common (any kanji or kana entry marked as common)
        let mut is_common = false;
        
        // Process kanji
        if let Some(kanji_entries) = &word.kanji {
            for kanji_entry in kanji_entries {
                kanji_indices.push(get_or_insert(&mut kanji_pool, &mut kanji_strings, &kanji_entry.text));
                if kanji_entry.common.unwrap_or(false) {
                    is_common = true;
                }
            }
        }
        
        // Process kana
        for kana_entry in &word.kana {
            kana_indices.push(get_or_insert(&mut kana_pool, &mut kana_strings, &kana_entry.text));
            if kana_entry.common.unwrap_or(false) {
                is_common = true;
            }
        }
        
        // Process senses
        for sense in &word.sense {
            // English glosses
            for gloss in &sense.gloss {
                if gloss.lang == "eng" {
                    english_indices.push(get_or_insert(&mut english_pool, &mut english_strings, &gloss.text));
                }
            }
            
            // POS (only from first sense)
            if pos_indices.is_empty() {
                if let Some(pos_array) = &sense.part_of_speech {
                    for pos_str in pos_array {
                        pos_indices.push(get_or_insert(&mut pos_pool, &mut pos_strings, pos_str));
                    }
                }
            }
        }
        
        word_entries.push((id_idx, kanji_indices, kana_indices, english_indices, pos_indices, is_common));
        
    }
    
    
    // Create packed binary format
    let mut strings_data = Vec::new();
    let mut string_offsets = Vec::new();
    
    // Pack all strings into one byte array
    for strings in [&kanji_strings, &kana_strings, &english_strings, &pos_strings, &id_strings] {
        for s in strings {
            string_offsets.push(strings_data.len() as u32);
            strings_data.extend(s.as_bytes());
            strings_data.push(0); // null terminator
        }
    }
    
    // Pack entries into binary format
    let mut entries_data = Vec::new();
    let mut entry_offsets = Vec::new();
    
    for (id_idx, kanji_indices, kana_indices, english_indices, pos_indices, is_common) in &word_entries {
        entry_offsets.push(entries_data.len() as u32);
        
        // Pack entry: id(4) + kanji_count(1) + kana_count(1) + english_count(1) + pos_count(1) + is_common(1) + indices...
        entries_data.extend(id_idx.to_le_bytes());
        entries_data.push(kanji_indices.len() as u8);
        entries_data.push(kana_indices.len() as u8);
        entries_data.push(english_indices.len() as u8);
        entries_data.push(pos_indices.len() as u8);
        entries_data.push(if *is_common { 1 } else { 0 });
        
        // Add string indices (adjusted for string pool sections)
        let kanji_base = 0u32;
        let kana_base = kanji_strings.len() as u32;
        let english_base = kana_base + kana_strings.len() as u32;
        let pos_base = english_base + english_strings.len() as u32;
        let _id_base = pos_base + pos_strings.len() as u32;
        
        for &idx in kanji_indices {
            entries_data.extend((kanji_base + idx).to_le_bytes());
        }
        for &idx in kana_indices {
            entries_data.extend((kana_base + idx).to_le_bytes());
        }
        for &idx in english_indices {
            entries_data.extend((english_base + idx).to_le_bytes());
        }
        for &idx in pos_indices {
            entries_data.extend((pos_base + idx).to_le_bytes());
        }
    }
    
    // No more pre-built indices - use runtime caching instead
    
    
    // Generate compact dictionary data without indices
    let mut rust_code = String::new();
    rust_code.push_str("// Auto-generated compact dictionary data\n");
    
    // Packed binary data
    rust_code.push_str("pub static JMDICT_STRINGS: &[u8] = &[\n");
    for chunk in strings_data.chunks(16) {
        rust_code.push_str("    ");
        for &b in chunk {
            rust_code.push_str(&format!("{}, ", b));
        }
        rust_code.push_str("\n");
    }
    rust_code.push_str("];\n\n");
    
    rust_code.push_str("pub static JMDICT_ENTRIES: &[u8] = &[\n");
    for chunk in entries_data.chunks(16) {
        rust_code.push_str("    ");
        for &b in chunk {
            rust_code.push_str(&format!("{}, ", b));
        }
        rust_code.push_str("\n");
    }
    rust_code.push_str("];\n\n");
    
    rust_code.push_str("pub static JMDICT_ENTRY_OFFSETS: &[u32] = &[\n");
    for chunk in entry_offsets.chunks(8) {
        rust_code.push_str("    ");
        for &offset in chunk {
            rust_code.push_str(&format!("{}, ", offset));
        }
        rust_code.push_str("\n");
    }
    rust_code.push_str("];\n\n");
    
    rust_code.push_str("pub static JMDICT_STRING_OFFSETS: &[u32] = &[\n");
    for chunk in string_offsets.chunks(8) {
        rust_code.push_str("    ");
        for &offset in chunk {
            rust_code.push_str(&format!("{}, ", offset));
        }
        rust_code.push_str("\n");
    }
    rust_code.push_str("];\n\n");
    
    // No more static indices - runtime caching is used instead
    
    // String pool metadata
    rust_code.push_str(&format!("pub const KANJI_STRINGS_COUNT: u32 = {};\n", kanji_strings.len()));
    rust_code.push_str(&format!("pub const KANA_STRINGS_COUNT: u32 = {};\n", kana_strings.len()));
    rust_code.push_str(&format!("pub const ENGLISH_STRINGS_COUNT: u32 = {};\n", english_strings.len()));
    rust_code.push_str(&format!("pub const POS_STRINGS_COUNT: u32 = {};\n", pos_strings.len()));
    rust_code.push_str(&format!("pub const ID_STRINGS_COUNT: u32 = {};\n", id_strings.len()));
    rust_code.push_str(&format!("pub const WORD_COUNT: usize = {};\n", word_entries.len()));

    fs::write("../dictionary-data/src/lib.rs", rust_code).expect("Failed to write generated code");
}
