use dictionary_data::*;

#[derive(Clone, Debug, PartialEq)]
pub struct WordEntry {
    pub id: &'static str,
    pub kanji: Vec<&'static str>,
    pub kana: Vec<&'static str>,
    pub english: Vec<&'static str>,
    pub pos: Vec<&'static str>,
    pub is_common: bool,
}

fn read_string(offset: u32) -> &'static str {
    let start = offset as usize;
    let mut end = start;
    while end < JMDICT_STRINGS.len() && JMDICT_STRINGS[end] != 0 {
        end += 1;
    }
    unsafe { std::str::from_utf8_unchecked(&JMDICT_STRINGS[start..end]) }
}

pub fn get_word_entry(index: usize) -> WordEntry {
    let offset = JMDICT_ENTRY_OFFSETS[index] as usize;
    let data = &JMDICT_ENTRIES[offset..];
    
    let id_idx = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let kanji_count = data[4] as usize;
    let kana_count = data[5] as usize;
    let english_count = data[6] as usize;
    let pos_count = data[7] as usize;
    let is_common = data[8] != 0;
    
    let mut pos = 9;
    let mut kanji = Vec::new();
    let mut kana = Vec::new();
    let mut english = Vec::new();
    let mut pos_vec = Vec::new();
    
    // Read kanji indices
    for _ in 0..kanji_count {
        let idx = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        kanji.push(read_string(JMDICT_STRING_OFFSETS[idx as usize]));
        pos += 4;
    }
    
    // Read kana indices
    for _ in 0..kana_count {
        let idx = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        kana.push(read_string(JMDICT_STRING_OFFSETS[idx as usize]));
        pos += 4;
    }
    
    // Read english indices
    for _ in 0..english_count {
        let idx = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        english.push(read_string(JMDICT_STRING_OFFSETS[idx as usize]));
        pos += 4;
    }
    
    // Read pos indices
    for _ in 0..pos_count {
        let idx = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos_vec.push(read_string(JMDICT_STRING_OFFSETS[idx as usize]));
        pos += 4;
    }
    
    let id_offset_base = KANJI_STRINGS_COUNT + KANA_STRINGS_COUNT + ENGLISH_STRINGS_COUNT + POS_STRINGS_COUNT;
    let id = read_string(JMDICT_STRING_OFFSETS[(id_offset_base + id_idx) as usize]);
    
    WordEntry { id, kanji, kana, english, pos: pos_vec, is_common }
}