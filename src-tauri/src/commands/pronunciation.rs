use std::collections::HashMap;

use crate::cefr;

const BUILTIN_NAMES: &[&str] = &[
    "Hachiman",
    "Hikigaya",
    "Hiratsuka",
    "Komachi",
    "Meguri",
    "Sagami",
    "Yuigahama",
    "Yukino",
    "Yukinoshita",
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
        let mut aliases = BUILTIN_NAMES
            .iter()
            .filter_map(|name| r_to_l_alias(name))
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
                if let Some((name, spoken)) = r_to_l_alias(&name) {
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

fn r_to_l_alias(name: &str) -> Option<(String, String)> {
    let spoken = name
        .chars()
        .map(|character| match character {
            'r' => 'l',
            'R' => 'L',
            _ => character,
        })
        .collect::<String>();
    (spoken != name).then(|| (name.to_ascii_lowercase(), spoken))
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
    use super::{r_to_l_alias, JapanesePronunciation};

    #[test]
    fn changes_only_r_to_l() {
        assert_eq!(
            r_to_l_alias("Riruka"),
            Some(("riruka".to_string(), "Liluka".to_string()))
        );
        assert_eq!(r_to_l_alias("Hachiman"), None);
    }

    #[test]
    fn converts_seeded_names_without_phonetic_spelling() {
        let pronunciation = JapanesePronunciation::for_book_texts(&[]);
        assert_eq!(
            pronunciation.replace_names("Hachiman met Hiratsuka and Meguri."),
            "Hachiman met Hilatsuka and Meguli."
        );
    }

    #[test]
    fn learns_repeated_romaji_but_not_english_words() {
        let texts = vec![
            "Kuriyama arrived. Reader waited.".to_string(),
            "Kuriyama spoke. Reader listened.".to_string(),
            "Kuriyama left. Reader remained.".to_string(),
        ];
        let pronunciation = JapanesePronunciation::for_book_texts(&texts);
        assert_eq!(
            pronunciation.replace_names("Kuriyama met Reader."),
            "Kuliyama met Reader."
        );
    }
}
