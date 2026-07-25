pub fn save_progress(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    block_index: i64,
    scroll_ratio: f64,
    audio_time_seconds: Option<f64>,
    audio_duration_seconds: Option<f64>,
    last_playing_block_index: Option<i64>,
    last_playing_token_index: Option<i64>,
    progress_percent: f64,
) -> Result<()> {
    let timestamp = now_iso();
    let progress = progress_percent.clamp(0.0, 100.0);
    let scroll = scroll_ratio.clamp(0.0, 1.0);
    let duration = audio_duration_seconds.filter(|value| value.is_finite() && *value > 0.0);
    let audio_time = audio_time_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| duration.map_or(value, |duration| value.min(duration)));
    let normalized = normalize_reading_progress(
        ReadingProgress {
            last_read_at: Some(timestamp.clone()),
            last_chapter_index: chapter_index,
            last_part_index: part_index,
            last_block_index: block_index,
            last_scroll_ratio: scroll,
            last_audio_time_seconds: audio_time,
            last_audio_duration_seconds: duration,
            last_playing_block_index,
            last_playing_token_index,
            progress_percent: progress,
        },
        &chapter_summaries(connection, book_id)?,
    );
    let progress_percent =
        progress_percent_for_block(connection, book_id, normalized.last_block_index)?
            .unwrap_or(normalized.progress_percent);
    connection.execute(
        r#"
        INSERT INTO reading_progress (
            book_id,
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
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(book_id) DO UPDATE SET
            last_read_at = excluded.last_read_at,
            last_chapter_index = excluded.last_chapter_index,
            last_part_index = excluded.last_part_index,
            last_block_index = excluded.last_block_index,
            last_scroll_ratio = excluded.last_scroll_ratio,
            last_audio_time_seconds = CASE
                WHEN excluded.last_audio_time_seconds IS NULL
                 AND excluded.last_chapter_index = reading_progress.last_chapter_index
                 AND excluded.last_part_index = reading_progress.last_part_index
                THEN reading_progress.last_audio_time_seconds
                ELSE excluded.last_audio_time_seconds
            END,
            last_audio_duration_seconds = CASE
                WHEN excluded.last_audio_duration_seconds IS NULL
                 AND excluded.last_chapter_index = reading_progress.last_chapter_index
                 AND excluded.last_part_index = reading_progress.last_part_index
                THEN reading_progress.last_audio_duration_seconds
                ELSE excluded.last_audio_duration_seconds
            END,
            last_playing_block_index = CASE
                WHEN excluded.last_playing_block_index IS NULL
                 AND excluded.last_chapter_index = reading_progress.last_chapter_index
                 AND excluded.last_part_index = reading_progress.last_part_index
                THEN reading_progress.last_playing_block_index
                ELSE excluded.last_playing_block_index
            END,
            last_playing_token_index = CASE
                WHEN excluded.last_playing_token_index IS NULL
                 AND excluded.last_chapter_index = reading_progress.last_chapter_index
                 AND excluded.last_part_index = reading_progress.last_part_index
                THEN reading_progress.last_playing_token_index
                ELSE excluded.last_playing_token_index
            END,
            progress_percent = excluded.progress_percent
        "#,
        params![
            book_id,
            timestamp,
            normalized.last_chapter_index,
            normalized.last_part_index,
            normalized.last_block_index,
            normalized.last_scroll_ratio,
            normalized.last_audio_time_seconds,
            normalized.last_audio_duration_seconds,
            normalized.last_playing_block_index,
            normalized.last_playing_token_index,
            progress_percent
        ],
    )?;
    Ok(())
}

pub fn save_bookmark(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    block_index: i64,
    token_index: i64,
    word: &str,
    root_word: &str,
    scroll_ratio: f64,
    progress_percent: f64,
) -> Result<ReadingBookmark> {
    let timestamp = now_iso();
    let scroll = scroll_ratio.clamp(0.0, 1.0);
    let progress = progress_percent.clamp(0.0, 100.0);
    let bookmark = normalize_bookmark(
        ReadingBookmark {
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
            chapter_index,
            part_index,
            block_index,
            token_index,
            word: word.to_string(),
            root_word: root_word.to_string(),
            scroll_ratio: scroll,
            progress_percent: progress,
        },
        &chapter_summaries(connection, book_id)?,
    );
    connection.execute(
        r#"
        INSERT INTO bookmarks (
            book_id,
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
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(book_id) DO UPDATE SET
            created_at = bookmarks.created_at,
            updated_at = excluded.updated_at,
            chapter_index = excluded.chapter_index,
            part_index = excluded.part_index,
            block_index = excluded.block_index,
            token_index = excluded.token_index,
            word = excluded.word,
            root_word = excluded.root_word,
            scroll_ratio = excluded.scroll_ratio,
            progress_percent = excluded.progress_percent
        "#,
        params![
            book_id,
            &bookmark.created_at,
            &bookmark.updated_at,
            bookmark.chapter_index,
            bookmark.part_index,
            bookmark.block_index,
            bookmark.token_index,
            &bookmark.word,
            &bookmark.root_word,
            bookmark.scroll_ratio,
            bookmark.progress_percent
        ],
    )?;
    Ok(bookmark)
}

fn normalize_reading_progress(
    mut progress: ReadingProgress,
    chapters: &[ChapterSummary],
) -> ReadingProgress {
    if let Some(chapter) = chapters.iter().find(|chapter| {
        progress.last_block_index >= chapter.start_block_index
            && progress.last_block_index <= chapter.end_block_index
    }) {
        progress.last_chapter_index = chapter.chapter_index;
        progress.last_part_index = chapter
            .parts
            .iter()
            .find(|part| {
                progress.last_block_index >= part.start_block_index
                    && progress.last_block_index <= part.end_block_index
            })
            .map(|part| part.part_index)
            .unwrap_or(0);
        return progress;
    }

    if let Some(chapter) = chapters
        .iter()
        .find(|chapter| chapter.chapter_index == progress.last_chapter_index)
    {
        if !chapter
            .parts
            .iter()
            .any(|part| part.part_index == progress.last_part_index)
        {
            progress.last_part_index = 0;
        }
        return progress;
    }

    progress.last_chapter_index = 0;
    progress.last_part_index = 0;
    progress.last_block_index = 0;
    progress
}

fn normalize_bookmark(
    mut bookmark: ReadingBookmark,
    chapters: &[ChapterSummary],
) -> ReadingBookmark {
    if let Some(chapter) = chapters.iter().find(|chapter| {
        bookmark.block_index >= chapter.start_block_index
            && bookmark.block_index <= chapter.end_block_index
    }) {
        bookmark.chapter_index = chapter.chapter_index;
        bookmark.part_index = chapter
            .parts
            .iter()
            .find(|part| {
                bookmark.block_index >= part.start_block_index
                    && bookmark.block_index <= part.end_block_index
            })
            .map(|part| part.part_index)
            .unwrap_or(0);
        return bookmark;
    }

    if let Some(chapter) = chapters
        .iter()
        .find(|chapter| chapter.chapter_index == bookmark.chapter_index)
    {
        if !chapter
            .parts
            .iter()
            .any(|part| part.part_index == bookmark.part_index)
        {
            bookmark.part_index = 0;
        }
        bookmark.block_index = chapter.start_block_index;
        return bookmark;
    }

    bookmark.chapter_index = 0;
    bookmark.part_index = 0;
    bookmark.block_index = 0;
    bookmark.token_index = 0;
    bookmark
}

fn progress_percent_for_block(
    connection: &Connection,
    book_id: i64,
    block_index: i64,
) -> Result<Option<f64>> {
    let chapters = chapter_summaries(connection, book_id)?;
    let total_units = chapters
        .iter()
        .map(|chapter| chapter.progress_units)
        .sum::<i64>();
    if total_units <= 0 {
        return Ok(None);
    }

    let Some(chapter) = chapters.iter().find(|chapter| {
        block_index >= chapter.start_block_index && block_index <= chapter.end_block_index
    }) else {
        return Ok(None);
    };

    let mut units = chapter.progress_start_unit;
    if chapter.contributes_to_progress {
        let chapter_units: i64 = connection.query_row(
            r#"
            SELECT COALESCE(SUM(length(text)), 0)
            FROM chapter_blocks
            WHERE book_id = ?
              AND kind = 'paragraph'
              AND block_index >= ?
              AND block_index < ?
            "#,
            params![book_id, chapter.start_block_index, block_index],
            |row| row.get(0),
        )?;
        units += chapter_units.max(0);
    }

    let percent = (units as f64 / total_units as f64) * 100.0;
    Ok(Some((percent.clamp(0.0, 100.0) * 10.0).round() / 10.0))
}
