pub fn list_wordlist_entries(connection: &Connection) -> Result<Vec<WordlistEntry>> {
    list_wordlist_entries_for_book(connection, None)
}

pub fn list_book_wordlist_entries(
    connection: &Connection,
    book_id: i64,
) -> Result<Vec<WordlistEntry>> {
    list_wordlist_entries_for_book(connection, Some(book_id))
}

pub fn save_wordlist_entry(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    block_index: i64,
    token_index: usize,
    word: &str,
    root_word: &str,
    context: &str,
    cefr_level: &str,
) -> Result<WordlistEntry> {
    let book_title = connection
        .query_row(
            "SELECT title FROM books WHERE id = ?",
            params![book_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Book not found."))?;
    let block_text = connection
        .query_row(
            r#"
            SELECT text
            FROM chapter_blocks
            WHERE book_id = ?
              AND block_index = ?
              AND kind = 'paragraph'
            "#,
            params![book_id, block_index],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Word token not found."))?;
    let tokens = cefr::tokenize_text(&block_text);
    let token = tokens
        .get(token_index)
        .ok_or_else(|| anyhow!("Word token not found."))?;
    let root = (if !token.root_text.is_empty() {
        Some(token.root_text.clone())
    } else {
        normalized_word_root(root_word).or_else(|| normalized_word_root(word))
    })
    .ok_or_else(|| anyhow!("Select one English word."))?;
    let original_word = if !token.normalized_text.is_empty() {
        token.text.clone()
    } else {
        word.trim().to_string()
    };
    if original_word.trim().is_empty() {
        return Err(anyhow!("Select one English word."));
    }
    let stored_context = token_sentence_context(&tokens, token_index)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| clean_context(context));
    let stored_cefr = token
        .cefr_level
        .map(cefr_level_to_storage)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| cefr_level.trim().to_string());
    let timestamp = now_iso();
    connection.execute(
        r#"
        INSERT OR IGNORE INTO wordlist_entries (
            book_id, book_title, chapter_index, block_index, token_index,
            root_word, original_word, cefr_level, context, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            book_id,
            book_title,
            chapter_index,
            block_index,
            token_index as i64,
            root,
            original_word,
            stored_cefr,
            stored_context,
            timestamp,
            timestamp
        ],
    )?;
    wordlist_entry_by_root(connection, &root)?.ok_or_else(|| anyhow!("Word list entry not found."))
}

pub fn delete_wordlist_entry(connection: &Connection, root_word: &str) -> Result<bool> {
    let root = normalized_word_root(root_word).unwrap_or_else(|| root_word.trim().to_lowercase());
    if root.is_empty() {
        return Err(anyhow!("Select one English word."));
    }
    let removed = connection.execute(
        "DELETE FROM wordlist_entries WHERE root_word = ?",
        params![root],
    )?;
    Ok(removed > 0)
}

pub fn wordlist_entry_for_lookup(
    connection: &Connection,
    entry_id: i64,
) -> Result<Option<WordlistEntry>> {
    wordlist_entry_by_id(connection, entry_id)
}

pub fn update_wordlist_entry_lookup(
    connection: &Connection,
    entry_id: i64,
    lookup: Option<&crate::dictionary::DictionaryLookup>,
    lookup_error: &str,
) -> Result<Option<WordlistEntry>> {
    let timestamp = now_iso();
    if let Some(lookup) = lookup {
        let choice = &lookup.context_definition;
        connection.execute(
            r#"
            UPDATE wordlist_entries
            SET
                word_type = COALESCE(NULLIF(?, ''), word_type),
                cefr_level = COALESCE(NULLIF(?, ''), cefr_level),
                definition_number = ?,
                definition = ?,
                definition_examples = ?,
                definition_phonetics = ?,
                definition_audio_url = ?,
                definition_source_url = ?,
                definition_lookup_error = '',
                simple_meaning = ?,
                in_context_meaning = ?,
                original_meaning = ?,
                ai_explanation = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            params![
                &lookup.word_type,
                &lookup.cefr_level,
                choice.definition_number.map(|number| number as i64),
                &choice.definition,
                serde_json::to_string(&choice.examples)?,
                serde_json::to_string(&lookup.phonetics)?,
                &lookup.audio_url,
                &lookup.source_url,
                &lookup.simple_meaning,
                &lookup.in_context_meaning,
                &lookup.original_meaning,
                &choice.ai_explanation,
                timestamp,
                entry_id
            ],
        )?;
    } else {
        connection.execute(
            r#"
            UPDATE wordlist_entries
            SET definition_lookup_error = ?, updated_at = ?
            WHERE id = ?
            "#,
            params![lookup_error, timestamp, entry_id],
        )?;
    }
    wordlist_entry_by_id(connection, entry_id)
}

fn list_wordlist_entries_for_book(
    connection: &Connection,
    book_id: Option<i64>,
) -> Result<Vec<WordlistEntry>> {
    let sql = format!(
        r#"
        SELECT
            w.id,
            w.book_id,
            COALESCE(NULLIF(b.title, ''), w.book_title) AS book_title,
            w.chapter_index,
            w.block_index,
            w.token_index,
            w.root_word,
            w.original_word,
            w.word_type,
            w.cefr_level,
            w.definition_number,
            w.definition,
            w.definition_examples,
            w.definition_phonetics,
            w.definition_audio_url,
            w.definition_source_url,
            w.definition_lookup_error,
            w.simple_meaning,
            w.in_context_meaning,
            w.original_meaning,
            w.ai_explanation,
            w.context,
            w.created_at,
            w.updated_at
        FROM wordlist_entries w
        LEFT JOIN books b ON b.id = w.book_id
        {}
        ORDER BY w.created_at DESC, w.id DESC
        "#,
        if book_id.is_some() {
            "WHERE w.book_id = ?"
        } else {
            ""
        }
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = if let Some(book_id) = book_id {
        statement.query_map(params![book_id], wordlist_entry_from_row)?
    } else {
        statement.query_map([], wordlist_entry_from_row)?
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn wordlist_entry_by_root(
    connection: &Connection,
    root_word: &str,
) -> Result<Option<WordlistEntry>> {
    wordlist_entry_by_column(connection, "root_word", root_word)
}

fn wordlist_entry_by_id(connection: &Connection, entry_id: i64) -> Result<Option<WordlistEntry>> {
    connection
        .query_row(
            r#"
            SELECT
                w.id,
                w.book_id,
                COALESCE(NULLIF(b.title, ''), w.book_title) AS book_title,
                w.chapter_index,
                w.block_index,
                w.token_index,
                w.root_word,
                w.original_word,
                w.word_type,
                w.cefr_level,
                w.definition_number,
                w.definition,
                w.definition_examples,
                w.definition_phonetics,
                w.definition_audio_url,
                w.definition_source_url,
                w.definition_lookup_error,
                w.simple_meaning,
                w.in_context_meaning,
                w.original_meaning,
                w.ai_explanation,
                w.context,
                w.created_at,
                w.updated_at
            FROM wordlist_entries w
            LEFT JOIN books b ON b.id = w.book_id
            WHERE w.id = ?
            "#,
            params![entry_id],
            wordlist_entry_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn wordlist_entry_by_column(
    connection: &Connection,
    column: &str,
    value: &str,
) -> Result<Option<WordlistEntry>> {
    let sql = format!(
        r#"
        SELECT
            w.id,
            w.book_id,
            COALESCE(NULLIF(b.title, ''), w.book_title) AS book_title,
            w.chapter_index,
            w.block_index,
            w.token_index,
            w.root_word,
            w.original_word,
            w.word_type,
            w.cefr_level,
            w.definition_number,
            w.definition,
            w.definition_examples,
            w.definition_phonetics,
            w.definition_audio_url,
            w.definition_source_url,
            w.definition_lookup_error,
            w.simple_meaning,
            w.in_context_meaning,
            w.original_meaning,
            w.ai_explanation,
            w.context,
            w.created_at,
            w.updated_at
        FROM wordlist_entries w
        LEFT JOIN books b ON b.id = w.book_id
        WHERE w.{column} = ?
        "#
    );
    connection
        .query_row(&sql, params![value], wordlist_entry_from_row)
        .optional()
        .map_err(Into::into)
}

fn wordlist_entry_from_row(row: &Row<'_>) -> rusqlite::Result<WordlistEntry> {
    let definition_examples: String = row.get(12)?;
    let definition_phonetics: String = row.get(13)?;
    let definition_number = row
        .get::<_, Option<i64>>(10)?
        .map(|number| number.max(0) as usize);
    Ok(WordlistEntry {
        id: row.get(0)?,
        book_id: row.get(1)?,
        book_title: row.get(2)?,
        chapter_index: row.get(3)?,
        block_index: row.get(4)?,
        token_index: row.get::<_, i64>(5)?.max(0) as usize,
        root_word: row.get(6)?,
        original_word: row.get(7)?,
        word_type: row.get(8)?,
        cefr_level: row.get(9)?,
        definition_number,
        definition: row.get(11)?,
        definition_examples: json_string_vec(&definition_examples),
        definition_phonetics: json_string_vec(&definition_phonetics),
        definition_audio_url: row.get(14)?,
        definition_source_url: row.get(15)?,
        definition_lookup_error: row.get(16)?,
        simple_meaning: row.get(17)?,
        in_context_meaning: row.get(18)?,
        original_meaning: row.get(19)?,
        ai_explanation: row.get(20)?,
        context: row.get(21)?,
        created_at: row.get(22)?,
        updated_at: row.get(23)?,
    })
}

fn json_string_vec(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn normalized_word_root(value: &str) -> Option<String> {
    cefr::tokenize_text(value)
        .into_iter()
        .find_map(|token| {
            if !token.root_text.is_empty() {
                Some(token.root_text)
            } else if !token.normalized_text.is_empty() {
                Some(token.normalized_text)
            } else {
                None
            }
        })
}

fn token_sentence_context(tokens: &[ReaderToken], token_index: usize) -> Option<String> {
    if token_index >= tokens.len() || tokens[token_index].normalized_text.is_empty() {
        return None;
    }
    let mut start = 0;
    for index in (0..token_index).rev() {
        if matches!(tokens[index].text.as_str(), "." | "!" | "?") {
            start = index + 1;
            break;
        }
    }
    let mut end = tokens.len();
    for index in token_index..tokens.len() {
        if matches!(tokens[index].text.as_str(), "." | "!" | "?") {
            end = (index + 1).min(tokens.len());
            while end < tokens.len() && matches!(tokens[end].text.trim(), "\"" | "'" | ")" | "]")
            {
                end += 1;
            }
            break;
        }
    }
    Some(clean_context(
        &tokens[start..end]
            .iter()
            .map(|token| token.text.as_str())
            .collect::<String>(),
    ))
}

fn clean_context(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|character| matches!(character, '"' | '\'' | '“' | '”' | '‘' | '’'))
        .to_string()
}
