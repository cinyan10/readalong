pub fn list_book_highlights(connection: &Connection, book_id: i64) -> Result<Vec<ReaderHighlight>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            book_id,
            chapter_index,
            block_index,
            start_token_index,
            end_token_index,
            start_offset,
            end_offset,
            text,
            created_at,
            updated_at
        FROM reader_highlights
        WHERE book_id = ?
        ORDER BY chapter_index, block_index, start_token_index, start_offset, id
        "#,
    )?;
    let highlights = statement
        .query_map(params![book_id], highlight_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into);
    highlights
}

pub fn toggle_highlight(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    block_index: i64,
    start_token_index: usize,
    end_token_index: usize,
    start_offset: usize,
    end_offset: usize,
    text: &str,
) -> Result<Option<ReaderHighlight>> {
    if end_token_index < start_token_index {
        return Err(anyhow!("Invalid highlight range."));
    }
    let selected_text = text.trim();
    if selected_text.is_empty() {
        return Err(anyhow!("Select text to highlight."));
    }

    let existing_id = connection
        .query_row(
            r#"
            SELECT id
            FROM reader_highlights
            WHERE book_id = ?
              AND chapter_index = ?
              AND block_index = ?
              AND start_token_index = ?
              AND end_token_index = ?
              AND start_offset = ?
              AND end_offset = ?
            "#,
            params![
                book_id,
                chapter_index,
                block_index,
                start_token_index as i64,
                end_token_index as i64,
                start_offset as i64,
                end_offset as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    if let Some(id) = existing_id {
        connection.execute("DELETE FROM reader_highlights WHERE id = ?", params![id])?;
        return Ok(None);
    }

    let timestamp = now_iso();
    connection.execute(
        r#"
        INSERT INTO reader_highlights (
            book_id,
            chapter_index,
            block_index,
            start_token_index,
            end_token_index,
            start_offset,
            end_offset,
            text,
            created_at,
            updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            book_id,
            chapter_index,
            block_index,
            start_token_index as i64,
            end_token_index as i64,
            start_offset as i64,
            end_offset as i64,
            selected_text,
            timestamp,
            timestamp,
        ],
    )?;
    let id = connection.last_insert_rowid();
    highlight_by_id(connection, id)
}

fn highlight_by_id(connection: &Connection, id: i64) -> Result<Option<ReaderHighlight>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                book_id,
                chapter_index,
                block_index,
                start_token_index,
                end_token_index,
                start_offset,
                end_offset,
                text,
                created_at,
                updated_at
            FROM reader_highlights
            WHERE id = ?
            "#,
            params![id],
            highlight_from_row,
        )
        .optional()
        .map_err(Into::into)
}

fn highlight_from_row(row: &Row<'_>) -> rusqlite::Result<ReaderHighlight> {
    Ok(ReaderHighlight {
        id: row.get(0)?,
        book_id: row.get(1)?,
        chapter_index: row.get(2)?,
        block_index: row.get(3)?,
        start_token_index: row.get::<_, i64>(4)? as usize,
        end_token_index: row.get::<_, i64>(5)? as usize,
        start_offset: row.get::<_, i64>(6)? as usize,
        end_offset: row.get::<_, i64>(7)? as usize,
        text: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}
