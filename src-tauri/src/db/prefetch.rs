#[derive(Clone, Debug)]
pub struct DefinitionPrefetchJob {
    pub book_id: i64,
    pub block_index: i64,
    pub token_index: usize,
    pub word: String,
    pub root_word: String,
    pub cefr_level: String,
    pub context: String,
    pub context_key: String,
}

pub fn dictionary_context_for_token(
    connection: &Connection,
    book_id: i64,
    block_index: i64,
    token_index: usize,
) -> Result<Option<DefinitionPrefetchJob>> {
    let block_text = connection
        .query_row(
            "SELECT text FROM chapter_blocks WHERE book_id = ? AND block_index = ? AND kind = 'paragraph'",
            params![book_id, block_index],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(block_text) = block_text else {
        return Ok(None);
    };
    let tokens = cefr::tokenize_text(&block_text);
    let Some(token) = tokens.get(token_index) else {
        return Ok(None);
    };
    let root_word = if token.root_text.is_empty() {
        token.normalized_text.clone()
    } else {
        token.root_text.clone()
    };
    if root_word.is_empty() {
        return Ok(None);
    }
    let Some(sentence) = token_sentence_context(&tokens, token_index) else {
        return Ok(None);
    };
    let context = format!("{}\n\n{}", token.text, sentence);
    let context_key = crate::dictionary::context_cache_key(&context);
    if context_key.is_empty() {
        return Ok(None);
    }
    Ok(Some(DefinitionPrefetchJob {
        book_id,
        block_index,
        token_index,
        word: token.text.clone(),
        root_word,
        cefr_level: token.cefr_level.map(cefr_level_to_storage).unwrap_or("").to_string(),
        context,
        context_key,
    }))
}

pub fn mark_read_block_and_enqueue_blue_prefetch(
    connection: &Connection,
    book_id: i64,
    block_index: i64,
) -> Result<bool> {
    let timestamp = now_iso();
    let local_date = Local::now().date_naive().to_string();
    connection.execute(
        "DELETE FROM daily_read_blocks WHERE local_date < ?",
        params![local_date],
    )?;
    let inserted = connection.execute(
        r#"
        INSERT OR IGNORE INTO daily_read_blocks (book_id, local_date, block_index, read_at)
        VALUES (?, ?, ?, ?)
        "#,
        params![book_id, local_date, block_index, timestamp],
    )?;
    if inserted == 0 {
        return Ok(false);
    }

    let block_text = connection
        .query_row(
            "SELECT text FROM chapter_blocks WHERE book_id = ? AND block_index = ? AND kind = 'paragraph'",
            params![book_id, block_index],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(block_text) = block_text else {
        return Ok(false);
    };

    let mut saved_words = connection.prepare(
        "SELECT root_word, book_id, block_index, token_index FROM wordlist_entries",
    )?;
    let saved_words = saved_words
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?.max(0) as usize,
                ),
            ))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    if saved_words.is_empty() {
        return Ok(false);
    }

    let tokens = cefr::tokenize_text(&block_text);
    let mut queued = false;
    for (token_index, token) in tokens.iter().enumerate() {
        let root_word = if token.root_text.is_empty() {
            &token.normalized_text
        } else {
            &token.root_text
        };
        if root_word.is_empty() {
            continue;
        }
        let Some((saved_book_id, saved_block_index, saved_token_index)) = saved_words.get(root_word) else {
            continue;
        };
        // The saved location is green and already owns its enrichment job. Every other match is blue.
        if *saved_book_id == book_id && *saved_block_index == block_index && *saved_token_index == token_index {
            continue;
        }
        let Some(sentence) = token_sentence_context(&tokens, token_index) else {
            continue;
        };
        let context = format!("{}\n\n{}", token.text, sentence);
        let context_key = crate::dictionary::context_cache_key(&context);
        if context_key.is_empty() {
            continue;
        }
        let changed = connection.execute(
            r#"
            INSERT OR IGNORE INTO definition_prefetch_jobs (
                book_id, block_index, token_index, word, root_word, cefr_level, context, context_key,
                status, last_error, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'pending', '', ?, ?)
            "#,
            params![
                book_id,
                block_index,
                token_index as i64,
                token.text,
                root_word,
                token.cefr_level.map(cefr_level_to_storage).unwrap_or(""),
                context,
                context_key,
                timestamp,
                timestamp,
            ],
        )?;
        queued |= changed > 0;
    }
    Ok(queued)
}

pub fn next_definition_prefetch_job(connection: &Connection) -> Result<Option<DefinitionPrefetchJob>> {
    connection
        .query_row(
            r#"
            SELECT book_id, block_index, token_index, word, root_word, cefr_level, context, context_key
            FROM definition_prefetch_jobs
            WHERE status = 'pending'
            ORDER BY created_at, book_id, block_index, token_index
            LIMIT 1
            "#,
            [],
            |row| {
                Ok(DefinitionPrefetchJob {
                    book_id: row.get(0)?,
                    block_index: row.get(1)?,
                    token_index: row.get::<_, i64>(2)?.max(0) as usize,
                    word: row.get(3)?,
                    root_word: row.get(4)?,
                    cefr_level: row.get(5)?,
                    context: row.get(6)?,
                    context_key: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub fn set_definition_prefetch_job_status(
    connection: &Connection,
    job: &DefinitionPrefetchJob,
    status: &str,
    last_error: &str,
) -> Result<()> {
    connection.execute(
        r#"
        UPDATE definition_prefetch_jobs
        SET status = ?, last_error = ?, updated_at = ?
        WHERE book_id = ? AND block_index = ? AND token_index = ?
        "#,
        params![
            status,
            last_error,
            now_iso(),
            job.book_id,
            job.block_index,
            job.token_index as i64,
        ],
    )?;
    Ok(())
}

pub fn reset_definition_prefetch_jobs(connection: &Connection) -> Result<bool> {
    let updated = connection.execute(
        r#"
        UPDATE definition_prefetch_jobs
        SET status = 'pending', last_error = '', updated_at = ?
        WHERE status IN ('running', 'failed')
        "#,
        params![now_iso()],
    )?;
    Ok(updated > 0 || has_pending_definition_prefetch_jobs(connection)?)
}

pub fn has_pending_definition_prefetch_jobs(connection: &Connection) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM definition_prefetch_jobs WHERE status = 'pending')",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}
