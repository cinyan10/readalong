use std::collections::HashMap;

use crate::cefr;

const BUILTIN_ALIASES: &[(&str, &str)] = &[
    ("Hachiman", "Hah-chee-mahn"),
    ("Hikigaya", "Hee-kee-gah-yah"),
    ("Hiratsuka", "Hee-lah-tsoo-kah"),
    ("Komachi", "Koh-mah-chee"),
    ("Meguri", "Meh-goo-lee"),
    ("Sagami", "Sah-gah-mee"),
    ("Yuigahama", "Yoo-ee-gah-hah-mah"),
    ("Yukino", "Yoo-kee-noh"),
    ("Yukinoshita", "Yoo-kee-noh-shee-tah"),
];

#[derive(Debug, Default)]
pub struct JapanesePronunciation {
    aliases: HashMap<String, String>,
}

impl JapanesePronunciation {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn for_book_texts(texts: &[String]) -> Self {
        let mut aliases = BUILTIN_ALIASES
            .iter()
            .map(|(name, spoken)| (name.to_ascii_lowercase(), (*spoken).to_string()))
            .collect::<HashMap<_, _>>();
        let mut occurrences = HashMap::<String, usize>::new();

        for text in texts {
            for word in ascii_words(text) {
                if is_title_cased(word) {
                    *occurrences.entry(word.to_ascii_lowercase()).or_default() += 1;
                }
            }
        }

        for (name, count) in occurrences {
            if count >= 3
                && !aliases.contains_key(&name)
                && !cefr::has_vocabulary_entry(&name)
                && has_japanese_marker(&name)
            {
                if let Some(spoken) = phonetic_romaji(&name) {
                    aliases.insert(name, capitalize_first(&spoken));
                }
            }
        }

        Self { aliases }
    }

    pub fn replace_names(&self, text: &str) -> String {
        let mut output = String::with_capacity(text.len());
        let mut start = 0;
        let bytes = text.as_bytes();

        while start < bytes.len() {
            if !bytes[start].is_ascii_alphabetic() {
                let character = text[start..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 text has a character at a valid byte index");
                output.push(character);
                start += character.len_utf8();
                continue;
            }

            let end = bytes[start..]
                .iter()
                .position(|byte| !byte.is_ascii_alphabetic())
                .map(|offset| start + offset)
                .unwrap_or(bytes.len());
            let word = &text[start..end];
            if let Some(spoken) = self.aliases.get(&word.to_ascii_lowercase()) {
                output.push_str(&match_case(spoken, word));
            } else {
                output.push_str(word);
            }
            start = end;
        }

        output
    }
}

fn ascii_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
}

fn is_title_cased(word: &str) -> bool {
    let mut characters = word.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
        && characters.all(|character| character.is_ascii_lowercase())
}

fn has_japanese_marker(name: &str) -> bool {
    [
        "shi", "chi", "tsu", "fu", "ji", "ya", "yu", "yo", "za", "zu", "ry", "ky", "gy", "ny",
        "hy", "my", "by", "py",
    ]
    .iter()
    .any(|marker| name.contains(marker))
}

fn phonetic_romaji(name: &str) -> Option<String> {
    let mut syllables: Vec<String> = Vec::new();
    let mut index = 0;
    let name = name.as_bytes();

    while index < name.len() {
        if index + 1 < name.len()
            && name[index].is_ascii_alphabetic()
            && name[index].eq_ignore_ascii_case(&name[index + 1])
            && !matches!(
                name[index].to_ascii_lowercase(),
                b'a' | b'e' | b'i' | b'o' | b'u' | b'n'
            )
        {
            let consonant = name[index].to_ascii_lowercase() as char;
            let previous = syllables.last_mut()?;
            previous.push(consonant);
            index += 1;
            continue;
        }

        let remainder = std::str::from_utf8(&name[index..]).ok()?;
        let (source, spoken) = romanized_syllable(remainder)?;
        index += source.len();
        if source == "n" && !syllables.is_empty() {
            syllables.last_mut()?.push('n');
        } else if is_long_vowel(source)
            && syllables
                .last()
                .is_some_and(|last| has_vowel_sound(last, vowel_sound(source)))
        {
            let previous = syllables.last_mut()?;
            if !previous.ends_with('h') {
                previous.push('h');
            }
        } else {
            syllables.push(spoken.to_string());
        }
    }

    Some(syllables.join("-"))
}

fn romanized_syllable(text: &str) -> Option<(&'static str, &'static str)> {
    const SYLLABLES: &[(&str, &str)] = &[
        ("kya", "kyah"),
        ("kyu", "kyoo"),
        ("kyo", "kyoh"),
        ("gya", "gyah"),
        ("gyu", "gyoo"),
        ("gyo", "gyoh"),
        ("sha", "shah"),
        ("shu", "shoo"),
        ("sho", "shoh"),
        ("cha", "chah"),
        ("chu", "choo"),
        ("cho", "choh"),
        ("nya", "nyah"),
        ("nyu", "nyoo"),
        ("nyo", "nyoh"),
        ("hya", "hyah"),
        ("hyu", "hyoo"),
        ("hyo", "hyoh"),
        ("mya", "myah"),
        ("myu", "myoo"),
        ("myo", "myoh"),
        ("rya", "lyah"),
        ("ryu", "lyoo"),
        ("ryo", "lyoh"),
        ("bya", "byah"),
        ("byu", "byoo"),
        ("byo", "byoh"),
        ("pya", "pyah"),
        ("pyu", "pyoo"),
        ("pyo", "pyoh"),
        ("shi", "shee"),
        ("chi", "chee"),
        ("tsu", "tsoo"),
        ("fu", "foo"),
        ("ja", "jah"),
        ("ju", "joo"),
        ("jo", "joh"),
        ("ka", "kah"),
        ("ki", "kee"),
        ("ku", "koo"),
        ("ke", "keh"),
        ("ko", "koh"),
        ("ga", "gah"),
        ("gi", "gee"),
        ("gu", "goo"),
        ("ge", "geh"),
        ("go", "goh"),
        ("sa", "sah"),
        ("si", "see"),
        ("su", "soo"),
        ("se", "seh"),
        ("so", "soh"),
        ("za", "zah"),
        ("zi", "zee"),
        ("zu", "zoo"),
        ("ze", "zeh"),
        ("zo", "zoh"),
        ("ta", "tah"),
        ("ti", "tee"),
        ("tu", "too"),
        ("te", "teh"),
        ("to", "toh"),
        ("da", "dah"),
        ("di", "dee"),
        ("du", "doo"),
        ("de", "deh"),
        ("do", "doh"),
        ("na", "nah"),
        ("ni", "nee"),
        ("nu", "noo"),
        ("ne", "neh"),
        ("no", "noh"),
        ("ha", "hah"),
        ("hi", "hee"),
        ("hu", "hoo"),
        ("he", "heh"),
        ("ho", "hoh"),
        ("ba", "bah"),
        ("bi", "bee"),
        ("bu", "boo"),
        ("be", "beh"),
        ("bo", "boh"),
        ("pa", "pah"),
        ("pi", "pee"),
        ("pu", "poo"),
        ("pe", "peh"),
        ("po", "poh"),
        ("ma", "mah"),
        ("mi", "mee"),
        ("mu", "moo"),
        ("me", "meh"),
        ("mo", "moh"),
        ("ya", "yah"),
        ("yu", "yoo"),
        ("yo", "yoh"),
        ("ra", "lah"),
        ("ri", "lee"),
        ("ru", "loo"),
        ("re", "leh"),
        ("ro", "loh"),
        ("wa", "wah"),
        ("wo", "woh"),
        ("a", "ah"),
        ("i", "ee"),
        ("u", "oo"),
        ("e", "eh"),
        ("o", "oh"),
        ("n", "n"),
    ];
    SYLLABLES
        .iter()
        .find(|(source, _)| text.starts_with(source))
        .copied()
}

fn is_long_vowel(source: &str) -> bool {
    matches!(source, "a" | "i" | "u" | "e" | "o")
}

fn vowel_sound(source: &str) -> char {
    match source {
        "a" => 'a',
        "i" => 'e',
        "u" => 'o',
        "e" => 'e',
        "o" => 'o',
        _ => unreachable!(),
    }
}

fn has_vowel_sound(spoken: &str, vowel: char) -> bool {
    spoken.ends_with(vowel) || spoken.ends_with(&format!("{vowel}h"))
}

fn match_case(spoken: &str, source: &str) -> String {
    if source
        .chars()
        .all(|character| character.is_ascii_uppercase())
    {
        spoken.to_ascii_uppercase()
    } else if source
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
    {
        spoken.to_ascii_lowercase()
    } else {
        spoken.to_string()
    }
}

fn capitalize_first(value: &str) -> String {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    format!("{}{}", first.to_ascii_uppercase(), characters.as_str())
}

#[cfg(test)]
mod tests {
    use super::{phonetic_romaji, JapanesePronunciation};

    #[test]
    fn phoneticizes_japanese_r_row() {
        assert_eq!(
            phonetic_romaji("rarirurero"),
            Some("lah-lee-loo-leh-loh".to_string())
        );
    }

    #[test]
    fn preserves_long_vowels_without_turning_them_into_two_syllables() {
        assert_eq!(phonetic_romaji("touma"), Some("toh-mah".to_string()));
    }

    #[test]
    fn converts_seeded_names_without_changing_display_text() {
        let pronunciation = JapanesePronunciation::for_book_texts(&[]);
        assert_eq!(
            pronunciation.replace_names("Hachiman Hikigaya's answer."),
            "Hah-chee-mahn Hee-kee-gah-yah's answer."
        );
    }

    #[test]
    fn learns_repeated_romaji_but_not_english_words() {
        let texts = vec![
            "Zaimokuza arrived. London waited.".to_string(),
            "Zaimokuza spoke. London listened.".to_string(),
            "Zaimokuza left. London remained.".to_string(),
        ];
        let pronunciation = JapanesePronunciation::for_book_texts(&texts);
        assert_eq!(
            pronunciation.replace_names("Zaimokuza met London."),
            "Zah-ee-moh-koo-zah met London."
        );
    }
}
