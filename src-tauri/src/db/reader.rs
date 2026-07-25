pub fn get_reader(connection: &Connection, book_id: i64) -> Result<Option<ReaderPayload>> {
    let book = connection
        .query_row(
            "SELECT id, title, author, cover_asset_path FROM books WHERE id = ?",
            params![book_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((id, title, author, cover_asset_path)) = book else {
        return Ok(None);
    };

    let chapters = chapter_summaries(connection, book_id)?;
    let total_progress_units = chapters
        .iter()
        .map(|chapter| chapter.progress_units)
        .sum::<i64>();
    let total_blocks: i64 = connection.query_row(
        "SELECT COUNT(*) FROM chapter_blocks WHERE book_id = ?",
        params![book_id],
        |row| row.get(0),
    )?;
    let progress = connection
        .query_row(
            r#"
            SELECT
                last_read_at,
                last_chapter_index,
                last_part_index,
                last_block_index,
                last_scroll_ratio,
                last_audio_time_seconds,
                last_audio_duration_seconds,
                last_playing_block_index,
                last_playing_token_index,
                progress_percent
            FROM reading_progress
            WHERE book_id = ?
            "#,
            params![book_id],
            |row| {
                Ok(ReadingProgress {
                    last_read_at: row.get(0)?,
                    last_chapter_index: row.get(1)?,
                    last_part_index: row.get(2)?,
                    last_block_index: row.get(3)?,
                    last_scroll_ratio: row.get(4)?,
                    last_audio_time_seconds: row.get(5)?,
                    last_audio_duration_seconds: row.get(6)?,
                    last_playing_block_index: row.get(7)?,
                    last_playing_token_index: row.get(8)?,
                    progress_percent: row.get(9)?,
                })
            },
        )
        .optional()?
        .unwrap_or(ReadingProgress {
            last_read_at: None,
            last_chapter_index: 0,
            last_part_index: 0,
            last_block_index: 0,
            last_scroll_ratio: 0.0,
            last_audio_time_seconds: None,
            last_audio_duration_seconds: None,
            last_playing_block_index: None,
            last_playing_token_index: None,
            progress_percent: 0.0,
        });
    let mut progress = normalize_reading_progress(progress, &chapters);
    if let Some(percent) =
        progress_percent_for_block(connection, book_id, progress.last_block_index)?
    {
        progress.progress_percent = percent;
    }
    let bookmark = connection
        .query_row(
            r#"
            SELECT
                created_at,
                updated_at,
                chapter_index,
                part_index,
                block_index,
                token_index,
                word,
                root_word,
                scroll_ratio,
                progress_percent
            FROM bookmarks
            WHERE book_id = ?
            "#,
            params![book_id],
            |row| {
                Ok(ReadingBookmark {
                    created_at: row.get(0)?,
                    updated_at: row.get(1)?,
                    chapter_index: row.get(2)?,
                    part_index: row.get(3)?,
                    block_index: row.get(4)?,
                    token_index: row.get(5)?,
                    word: row.get(6)?,
                    root_word: row.get(7)?,
                    scroll_ratio: row.get(8)?,
                    progress_percent: row.get(9)?,
                })
            },
        )
        .optional()?
        .map(|bookmark| normalize_bookmark(bookmark, &chapters));

    Ok(Some(ReaderPayload {
        id,
        title,
        author,
        cover_asset_path,
        chapters,
        progress,
        bookmark,
        total_blocks,
        total_progress_units,
    }))
}

pub fn get_chapter(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
) -> Result<Option<ChapterPayload>> {
    let raw_chapters = raw_chapter_summaries(connection, book_id)?;
    let chapters = build_reader_chapters(
        &raw_chapters,
        &block_markers(connection, book_id, &raw_chapters)?,
    )?;
    let Some(chapter) = chapters
        .into_iter()
        .find(|item| item.chapter_index == chapter_index)
    else {
        return Ok(None);
    };

    let book_asset_source = connection
        .query_row(
            "SELECT stored_path, content_hash FROM books WHERE id = ?",
            params![book_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let mut statement = connection.prepare(
        r#"
        SELECT block_index, kind, text, asset_path, alt
        FROM chapter_blocks
        WHERE book_id = ?
          AND block_index BETWEEN ? AND ?
          AND NOT (kind = 'image' AND (asset_path LIKE '%Art_orn%' OR lower(COALESCE(asset_path, '')) LIKE '%orn%'))
        ORDER BY block_index
        "#,
    )?;
    let rows = statement.query_map(
        params![book_id, chapter.start_block_index, chapter.end_block_index],
        |row| {
            Ok(ChapterBlock {
                block_index: row.get(0)?,
                kind: row.get(1)?,
                text: row.get(2)?,
                asset_path: row.get(3)?,
                alt: row.get(4)?,
                tokens: Vec::new(),
            })
        },
    )?;

    let mut blocks = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if let Some((stored_path, content_hash)) = book_asset_source {
        resolve_chapter_image_assets(
            connection,
            book_id,
            Path::new(&stored_path),
            &content_hash,
            &mut blocks,
        )?;
    }
    let frequencies = book_word_frequency_map(connection, book_id)?;
    let blocks = readable_chapter_blocks(&chapter.title, blocks)
        .into_iter()
        .map(|block| with_reader_tokens(block, &frequencies))
        .collect();

    Ok(Some(ChapterPayload {
        book_id,
        chapter_index,
        title: chapter.title,
        blocks,
    }))
}

pub fn reader_chapter_title(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
) -> Result<Option<String>> {
    let raw_chapters = raw_chapter_summaries(connection, book_id)?;
    let chapters = build_reader_chapters(
        &raw_chapters,
        &block_markers(connection, book_id, &raw_chapters)?,
    )?;
    Ok(chapters
        .into_iter()
        .find(|item| item.chapter_index == chapter_index)
        .map(|chapter| chapter.title))
}

pub fn search_book(
    connection: &Connection,
    book_id: i64,
    query: &str,
) -> Result<Vec<BookSearchResult>> {
    let Some(query) = normalized_search_query(query) else {
        return Ok(Vec::new());
    };
    let chapters = chapter_summaries(connection, book_id)?;
    if chapters.is_empty() {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        r#"
        SELECT block_index, text
        FROM chapter_blocks
        WHERE book_id = ?
          AND kind = 'paragraph'
          AND text != ''
        ORDER BY block_index
        "#,
    )?;
    let rows = statement.query_map(params![book_id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        let (block_index, text) = row?;
        let Some((match_start, match_end)) = case_insensitive_match_range(&text, &query) else {
            continue;
        };
        let Some(chapter) = chapters.iter().find(|chapter| {
            block_index >= chapter.start_block_index && block_index <= chapter.end_block_index
        }) else {
            continue;
        };
        let (snippet, snippet_match_start, snippet_match_end) =
            search_snippet(&text, match_start, match_end);
        results.push(BookSearchResult {
            book_id,
            chapter_index: chapter.chapter_index,
            chapter_title: chapter.title.clone(),
            block_index,
            snippet,
            match_start: snippet_match_start,
            match_end: snippet_match_end,
            match_count: count_case_insensitive_matches(&text, &query),
        });
        if results.len() >= 100 {
            break;
        }
    }

    Ok(results)
}
