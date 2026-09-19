pub fn get_part_audio(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
) -> Result<Option<PartAudioPayload>> {
    connection
        .query_row(
            r#"
            SELECT
                p.book_id,
                p.chapter_index,
                p.part_index,
                p.voice,
                p.audio_path,
                p.paragraph_count,
                p.duration_seconds,
                p.updated_at,
                COALESCE(a.alignment_path, ''),
                COALESCE(a.last_error, '')
            FROM audio_parts p
            LEFT JOIN audio_alignments a
              ON a.book_id = p.book_id
             AND a.chapter_index = p.chapter_index
             AND a.part_index = p.part_index
             AND a.voice = p.voice
            WHERE p.book_id = ?
              AND p.chapter_index = ?
              AND p.part_index = ?
              AND p.voice = ?
            "#,
            params![book_id, chapter_index, part_index, voice],
            |row| {
                let alignment_path: String = row.get(8)?;
                let alignment_error: String = row.get(9)?;
                Ok(PartAudioPayload {
                    book_id: row.get(0)?,
                    chapter_index: row.get(1)?,
                    part_index: row.get(2)?,
                    voice: row.get(3)?,
                    audio_path: row.get(4)?,
                    paragraph_count: row.get(5)?,
                    duration_seconds: row.get(6)?,
                    generated_at: row.get(7)?,
                    alignment_available: !alignment_path.is_empty()
                        && Path::new(&alignment_path).exists(),
                    alignment_error: if alignment_error.is_empty() {
                        None
                    } else {
                        Some(alignment_error)
                    },
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub fn part_audio_paragraphs(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
) -> Result<Vec<AudioParagraphSource>> {
    let raw_chapters = raw_chapter_summaries(connection, book_id)?;
    let chapters = build_reader_chapters(
        &raw_chapters,
        &block_markers(connection, book_id, &raw_chapters)?,
    )?;
    let Some(chapter) = chapters
        .into_iter()
        .find(|item| item.chapter_index == chapter_index)
    else {
        return Err(anyhow!("Chapter not found."));
    };
    let Some(part) = chapter
        .parts
        .iter()
        .find(|item| item.part_index == part_index)
    else {
        return Err(anyhow!("Part not found."));
    };

    let mut statement = connection.prepare(
        r#"
        SELECT block_index, kind, text, asset_path, alt
        FROM chapter_blocks
        WHERE book_id = ?
          AND block_index BETWEEN ? AND ?
          AND kind = 'paragraph'
        ORDER BY block_index
        "#,
    )?;
    let rows = statement.query_map(
        params![book_id, part.start_block_index, part.end_block_index],
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
    let blocks = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(readable_chapter_blocks(&chapter.title, blocks)
        .into_iter()
        .filter(|block| !block.text.trim().is_empty())
        .map(|block| AudioParagraphSource {
            block_index: block.block_index,
            text: block.text,
        })
        .collect())
}

pub fn generated_audio_paragraphs(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
) -> Result<Vec<GeneratedAudioParagraph>> {
    let mut statement = connection.prepare(
        r#"
        SELECT block_index, text_hash, audio_path, duration_seconds
        FROM audio_paragraphs
        WHERE book_id = ?
          AND chapter_index = ?
          AND part_index = ?
          AND voice = ?
        ORDER BY block_index
        "#,
    )?;
    let rows = statement.query_map(params![book_id, chapter_index, part_index, voice], |row| {
        Ok(GeneratedAudioParagraph {
            block_index: row.get(0)?,
            text_hash: row.get(1)?,
            audio_path: row.get(2)?,
            duration_seconds: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn save_part_audio(
    connection: &mut Connection,
    audio: &GeneratedPartAudio,
) -> Result<PartAudioPayload> {
    let timestamp = now_iso();
    let tx = connection.transaction()?;

    tx.execute(
        r#"
        INSERT INTO audio_parts (
            book_id, chapter_index, part_index, voice, audio_path, paragraph_count, duration_seconds, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(book_id, chapter_index, part_index, voice) DO UPDATE SET
            audio_path = excluded.audio_path,
            paragraph_count = excluded.paragraph_count,
            duration_seconds = excluded.duration_seconds,
            updated_at = excluded.updated_at
        "#,
        params![
            audio.book_id,
            audio.chapter_index,
            audio.part_index,
            audio.voice,
            audio.audio_path,
            audio.paragraphs.len() as i64,
            audio.duration_seconds,
            timestamp,
            timestamp
        ],
    )?;

    for paragraph in &audio.paragraphs {
        tx.execute(
            r#"
            INSERT INTO audio_paragraphs (
                book_id, chapter_index, part_index, block_index, voice, text_hash, audio_path,
                duration_seconds, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(book_id, block_index, voice) DO UPDATE SET
                chapter_index = excluded.chapter_index,
                part_index = excluded.part_index,
                text_hash = excluded.text_hash,
                audio_path = excluded.audio_path,
                duration_seconds = excluded.duration_seconds,
                updated_at = excluded.updated_at
            "#,
            params![
                audio.book_id,
                audio.chapter_index,
                audio.part_index,
                paragraph.block_index,
                audio.voice,
                paragraph.text_hash,
                paragraph.audio_path,
                paragraph.duration_seconds,
                timestamp,
                timestamp
            ],
        )?;
    }

    tx.commit()?;
    Ok(PartAudioPayload {
        book_id: audio.book_id,
        chapter_index: audio.chapter_index,
        part_index: audio.part_index,
        voice: audio.voice.clone(),
        audio_path: audio.audio_path.clone(),
        paragraph_count: audio.paragraphs.len() as i64,
        duration_seconds: audio.duration_seconds,
        generated_at: timestamp,
        alignment_available: false,
        alignment_error: None,
    })
}

pub fn get_part_alignment(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
) -> Result<Option<PartAlignmentPayload>> {
    let path: Option<String> = connection
        .query_row(
            r#"
            SELECT alignment_path
            FROM audio_alignments
            WHERE book_id = ?
              AND chapter_index = ?
              AND part_index = ?
              AND voice = ?
              AND alignment_path != ''
            "#,
            params![book_id, chapter_index, part_index, voice],
            |row| row.get(0),
        )
        .optional()?;
    let Some(path) = path else {
        return Ok(None);
    };
    if !Path::new(&path).exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).with_context(|| format!("Unable to read alignment {}", path))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("Invalid alignment JSON {}", path))
        .map(Some)
}

pub fn save_part_alignment(
    connection: &Connection,
    alignment: &PartAlignmentPayload,
    alignment_path: &Path,
) -> Result<()> {
    let timestamp = now_iso();
    connection.execute(
        r#"
        INSERT INTO audio_alignments (
            book_id, chapter_index, part_index, voice, audio_path, alignment_path, token_count,
            duration_seconds, last_error, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '', ?, ?)
        ON CONFLICT(book_id, chapter_index, part_index, voice) DO UPDATE SET
            audio_path = excluded.audio_path,
            alignment_path = excluded.alignment_path,
            token_count = excluded.token_count,
            duration_seconds = excluded.duration_seconds,
            last_error = '',
            updated_at = excluded.updated_at
        "#,
        params![
            alignment.book_id,
            alignment.chapter_index,
            alignment.part_index,
            alignment.voice,
            alignment.audio_path,
            path_to_string(alignment_path.to_path_buf()),
            alignment.tokens.len() as i64,
            alignment.duration_seconds,
            timestamp,
            timestamp
        ],
    )?;
    Ok(())
}

pub fn save_part_alignment_error(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
    audio_path: &str,
    error: &str,
) -> Result<()> {
    let timestamp = now_iso();
    connection.execute(
        r#"
        INSERT INTO audio_alignments (
            book_id, chapter_index, part_index, voice, audio_path, alignment_path, token_count,
            duration_seconds, last_error, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, '', 0, 0, ?, ?, ?)
        ON CONFLICT(book_id, chapter_index, part_index, voice) DO UPDATE SET
            audio_path = excluded.audio_path,
            alignment_path = '',
            token_count = 0,
            duration_seconds = 0,
            last_error = excluded.last_error,
            updated_at = excluded.updated_at
        "#,
        params![
            book_id,
            chapter_index,
            part_index,
            voice,
            audio_path,
            error,
            timestamp,
            timestamp
        ],
    )?;
    Ok(())
}

pub fn delete_part_alignment(
    connection: &Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
) -> Result<()> {
    connection.execute(
        r#"
        DELETE FROM audio_alignments
        WHERE book_id = ?
          AND chapter_index = ?
          AND part_index = ?
          AND voice = ?
        "#,
        params![book_id, chapter_index, part_index, voice],
    )?;
    Ok(())
}
