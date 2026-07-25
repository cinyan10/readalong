fn chapter_summaries(connection: &Connection, book_id: i64) -> Result<Vec<ChapterSummary>> {
    let raw_chapters = raw_chapter_summaries(connection, book_id)?;
    let chapters = build_reader_chapters(
        &raw_chapters,
        &block_markers(connection, book_id, &raw_chapters)?,
    )?;
    with_progress_units(connection, book_id, chapters)
}

fn with_progress_units(
    connection: &Connection,
    book_id: i64,
    mut chapters: Vec<ChapterSummary>,
) -> Result<Vec<ChapterSummary>> {
    let mut progress_start = 0_i64;
    for chapter in &mut chapters {
        chapter.contributes_to_progress = is_progress_chapter_title(&chapter.title);
        if chapter.contributes_to_progress {
            let units: i64 = connection.query_row(
                r#"
                SELECT COALESCE(SUM(length(text)), 0)
                FROM chapter_blocks
                WHERE book_id = ?
                  AND kind = 'paragraph'
                  AND block_index BETWEEN ? AND ?
                "#,
                params![book_id, chapter.start_block_index, chapter.end_block_index],
                |row| row.get(0),
            )?;
            chapter.progress_start_unit = progress_start;
            chapter.progress_units = units.max(0);
            progress_start += chapter.progress_units;
            chapter.progress_end_unit = progress_start;
        } else {
            chapter.progress_start_unit = progress_start;
            chapter.progress_end_unit = progress_start;
            chapter.progress_units = 0;
        }
    }
    Ok(chapters)
}

#[derive(Debug)]
struct RawChapterSummary {
    title: String,
    source_href: String,
    start_block_index: i64,
    end_block_index: i64,
}

#[derive(Debug)]
struct BlockMarker {
    block_index: i64,
    kind: String,
    asset_path: Option<String>,
    consumes_block: bool,
}

fn raw_chapter_summaries(connection: &Connection, book_id: i64) -> Result<Vec<RawChapterSummary>> {
    let mut statement = connection.prepare(
        r#"
        SELECT title, source_href, start_block_index, end_block_index
        FROM book_chapters
        WHERE book_id = ?
        ORDER BY chapter_index
        "#,
    )?;
    let rows = statement.query_map(params![book_id], |row| {
        Ok(RawChapterSummary {
            title: row.get(0)?,
            source_href: row.get(1)?,
            start_block_index: row.get(2)?,
            end_block_index: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn block_markers(
    connection: &Connection,
    book_id: i64,
    raw_chapters: &[RawChapterSummary],
) -> Result<Vec<BlockMarker>> {
    let mut statement = connection.prepare(
        r#"
        SELECT block_index, kind, asset_path
        FROM chapter_blocks
        WHERE book_id = ?
        ORDER BY block_index
        "#,
    )?;
    let rows = statement.query_map(params![book_id], |row| {
        Ok(BlockMarker {
            block_index: row.get(0)?,
            kind: row.get(1)?,
            asset_path: row.get(2)?,
            consumes_block: true,
        })
    })?;
    let markers = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if markers.iter().any(is_divider_marker) {
        return Ok(markers);
    }

    let stored_path = connection
        .query_row(
            "SELECT stored_path FROM books WHERE id = ?",
            params![book_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(stored_path) = stored_path else {
        return Ok(markers);
    };
    let derived = derive_divider_markers(Path::new(&stored_path), raw_chapters);
    if derived.is_empty() {
        Ok(markers)
    } else {
        Ok(derived)
    }
}

fn derive_divider_markers(
    source_path: &Path,
    raw_chapters: &[RawChapterSummary],
) -> Vec<BlockMarker> {
    let mut markers = Vec::new();
    for chapter in raw_chapters {
        let Ok(blocks) = epub::read_chapter_blocks(source_path, &chapter.source_href) else {
            continue;
        };
        let mut block_index = chapter.start_block_index;
        for block in blocks {
            match block.kind {
                ExtractedBlockKind::Paragraph => block_index += 1,
                ExtractedBlockKind::Image => {
                    if block.asset_path.as_deref().is_some_and(is_divider_path) {
                        markers.push(BlockMarker {
                            block_index,
                            kind: "image".to_string(),
                            asset_path: block.asset_path,
                            consumes_block: false,
                        });
                    }
                }
            }
        }
    }
    markers
}

fn build_reader_chapters(
    raw_chapters: &[RawChapterSummary],
    markers: &[BlockMarker],
) -> Result<Vec<ChapterSummary>> {
    let mut groups: Vec<Vec<&RawChapterSummary>> = Vec::new();
    let mut previous_key = String::new();

    for chapter in raw_chapters {
        let Some(key) = chapter_group_key(&chapter.title, &chapter.source_href) else {
            continue;
        };
        if groups.is_empty() || previous_key != key {
            previous_key = key;
            groups.push(vec![chapter]);
        } else if let Some(group) = groups.last_mut() {
            group.push(chapter);
        }
    }

    Ok(groups
        .into_iter()
        .enumerate()
        .filter_map(|(index, group)| {
            let first = group.first()?;
            let last = group.last()?;
            let start_block_index = first.start_block_index;
            let end_block_index = last.end_block_index;
            Some(ChapterSummary {
                chapter_index: index as i64,
                title: chapter_group_title(&first.title, &first.source_href),
                start_block_index,
                end_block_index,
                progress_start_unit: 0,
                progress_end_unit: 0,
                progress_units: 0,
                contributes_to_progress: false,
                parts: build_chapter_parts(start_block_index, end_block_index, markers),
            })
        })
        .collect())
}

fn build_chapter_parts(
    start_block_index: i64,
    end_block_index: i64,
    markers: &[BlockMarker],
) -> Vec<ChapterPartSummary> {
    let mut parts = Vec::new();
    let mut current_start = start_block_index;
    let mut splits = markers
        .iter()
        .filter(|marker| is_divider_marker(marker))
        .map(|marker| (marker.block_index, marker.consumes_block))
        .filter(|(block_index, _)| {
            *block_index > start_block_index && *block_index <= end_block_index
        })
        .collect::<Vec<_>>();
    splits.sort_unstable();
    splits.dedup();

    for (split_block_index, consumes_block) in splits {
        if split_block_index > current_start {
            parts.push(ChapterPartSummary {
                part_index: parts.len() as i64,
                title: format!("Part {}", parts.len() + 1),
                start_block_index: current_start,
                end_block_index: if consumes_block {
                    split_block_index - 1
                } else {
                    split_block_index
                },
            });
        }
        current_start = split_block_index + 1;
    }
    if end_block_index >= current_start {
        parts.push(ChapterPartSummary {
            part_index: parts.len() as i64,
            title: format!("Part {}", parts.len() + 1),
            start_block_index: current_start,
            end_block_index,
        })
    }
    parts
}

fn is_divider_marker(marker: &BlockMarker) -> bool {
    marker.kind == "image" && marker.asset_path.as_deref().is_some_and(is_divider_path)
}

fn is_divider_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    path.contains("Art_orn") || lower.contains("orn")
}

fn readable_chapter_blocks(chapter_title: &str, blocks: Vec<ChapterBlock>) -> Vec<ChapterBlock> {
    blocks
        .into_iter()
        .filter(|block| {
            block.kind != "paragraph" || !is_redundant_chapter_text(chapter_title, &block.text)
        })
        .collect()
}

fn normalized_search_query(query: &str) -> Option<String> {
    let query = query.trim();
    (query.chars().count() >= 2).then(|| query.to_string())
}

fn case_insensitive_match_range(text: &str, query: &str) -> Option<(usize, usize)> {
    let query_chars = query
        .chars()
        .flat_map(|character| character.to_lowercase())
        .collect::<Vec<_>>();
    if query_chars.is_empty() {
        return None;
    }
    let text_chars = text.char_indices().collect::<Vec<_>>();
    for start in 0..text_chars.len() {
        let mut matched = 0_usize;
        for end in start..text_chars.len() {
            for character in text_chars[end].1.to_lowercase() {
                if query_chars.get(matched) != Some(&character) {
                    matched = 0;
                    break;
                }
                matched += 1;
                if matched == query_chars.len() {
                    let start_byte = text_chars[start].0;
                    let end_byte = text_chars
                        .get(end + 1)
                        .map(|(index, _)| *index)
                        .unwrap_or(text.len());
                    return Some((start_byte, end_byte));
                }
            }
            if matched == 0 {
                break;
            }
        }
    }
    None
}

fn count_case_insensitive_matches(text: &str, query: &str) -> usize {
    let mut count = 0_usize;
    let mut remaining = text;
    while let Some((_, end)) = case_insensitive_match_range(remaining, query) {
        count += 1;
        remaining = &remaining[end..];
    }
    count
}

fn search_snippet(text: &str, match_start: usize, match_end: usize) -> (String, usize, usize) {
    const BEFORE: usize = 56;
    const AFTER: usize = 76;

    let match_start_char = text[..match_start].chars().count();
    let match_end_char = text[..match_end].chars().count();
    let total_chars = text.chars().count();
    let start_char = match_start_char.saturating_sub(BEFORE);
    let end_char = (match_end_char + AFTER).min(total_chars);
    let prefix = if start_char > 0 { "..." } else { "" };
    let suffix = if end_char < total_chars { "..." } else { "" };
    let body = text
        .chars()
        .skip(start_char)
        .take(end_char.saturating_sub(start_char))
        .collect::<String>();
    let snippet = format!("{prefix}{body}{suffix}");
    let match_offset = prefix.len() + match_start_char.saturating_sub(start_char);
    let snippet_match_start = match_offset;
    let snippet_match_end = snippet_match_start + match_end_char.saturating_sub(match_start_char);
    (snippet, snippet_match_start, snippet_match_end)
}

#[derive(Clone, Copy, Debug)]
struct WordFrequency {
    count: usize,
    level: CefrLevel,
}

fn with_reader_tokens(
    mut block: ChapterBlock,
    frequencies: &HashMap<String, WordFrequency>,
) -> ChapterBlock {
    if block.kind == "paragraph" {
        block.tokens = cefr::tokenize_text(&block.text);
        for token in &mut block.tokens {
            let Some(key) = canonical_frequency_key(token) else {
                continue;
            };
            if let Some(frequency) = frequencies.get(&key) {
                token.frequency_level = Some(frequency.level);
                token.frequency_count = Some(frequency.count);
            }
        }
    }
    block
}

fn book_word_frequency_map(
    connection: &Connection,
    book_id: i64,
) -> Result<HashMap<String, WordFrequency>> {
    ensure_book_word_frequency_cache(connection, book_id)?;
    let mut statement = connection.prepare(
        r#"
        SELECT word_key, frequency_count, frequency_level
        FROM book_word_frequencies
        WHERE book_id = ?
        "#,
    )?;
    let rows = statement.query_map(params![book_id], |row| {
        let level_text: String = row.get(2)?;
        Ok((
            row.get::<_, String>(0)?,
            WordFrequency {
                count: row.get::<_, i64>(1)?.max(0) as usize,
                level: cefr_level_from_storage(&level_text).unwrap_or(CefrLevel::C2),
            },
        ))
    })?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(Into::into)
}

fn ensure_book_word_frequency_cache(connection: &Connection, book_id: i64) -> Result<()> {
    let cache_version: Option<i64> = connection
        .query_row(
            "SELECT algorithm_version FROM book_word_frequency_cache WHERE book_id = ?",
            params![book_id],
            |row| row.get(0),
        )
        .optional()?;
    if cache_version.is_some_and(|version| version >= WORD_FREQUENCY_ALGORITHM_VERSION) {
        return Ok(());
    }

    let entries = build_book_word_frequency_entries(connection, book_id)?;
    connection.execute(
        "DELETE FROM book_word_frequencies WHERE book_id = ?",
        params![book_id],
    )?;
    for (word_key, frequency) in entries {
        connection.execute(
            r#"
            INSERT OR REPLACE INTO book_word_frequencies (
                book_id, word_key, frequency_count, frequency_level
            ) VALUES (?, ?, ?, ?)
            "#,
            params![
                book_id,
                word_key,
                frequency.count as i64,
                cefr_level_to_storage(frequency.level)
            ],
        )?;
    }
    connection.execute(
        r#"
        INSERT OR REPLACE INTO book_word_frequency_cache (book_id, generated_at, algorithm_version)
        VALUES (?, ?, ?)
        "#,
        params![book_id, now_iso(), WORD_FREQUENCY_ALGORITHM_VERSION],
    )?;
    Ok(())
}

fn build_book_word_frequency_entries(
    connection: &Connection,
    book_id: i64,
) -> Result<Vec<(String, WordFrequency)>> {
    let raw_chapters = raw_chapter_summaries(connection, book_id)?;
    let chapters = build_reader_chapters(
        &raw_chapters,
        &block_markers(connection, book_id, &raw_chapters)?,
    )?;
    let mut counts: HashMap<String, usize> = HashMap::new();

    for chapter in chapters
        .into_iter()
        .filter(|chapter| is_progress_chapter_title(&chapter.title))
    {
        let mut statement = connection.prepare(
            r#"
            SELECT block_index, kind, text, asset_path, alt
            FROM chapter_blocks
            WHERE book_id = ?
              AND kind = 'paragraph'
              AND block_index BETWEEN ? AND ?
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
        let blocks = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        for block in readable_chapter_blocks(&chapter.title, blocks) {
            for token in cefr::tokenize_text(&block.text) {
                if let Some(key) = canonical_frequency_key(&token) {
                    *counts.entry(key).or_default() += 1;
                }
            }
        }
    }

    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_word, left_count), (right_word, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_word.cmp(right_word))
    });
    let total = ranked.len();
    if total == 0 {
        return Ok(Vec::new());
    }
    let non_a1_max_count = ranked
        .iter()
        .filter(|(word_key, _)| !cefr::is_oxford_3000_a1_word(word_key))
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1);

    let mut entries = Vec::with_capacity(total);
    let mut index = 0;
    while index < ranked.len() {
        let count = ranked[index].1;
        let mut end = index + 1;
        while end < ranked.len() && ranked[end].1 == count {
            end += 1;
        }
        for (word_key, _) in &ranked[index..end] {
            let level = if cefr::is_oxford_3000_a1_word(word_key) {
                CefrLevel::A1
            } else {
                frequency_level_for_non_a1_count(count, non_a1_max_count)
            };
            entries.push((word_key.clone(), WordFrequency { count, level }));
        }
        index = end;
    }
    Ok(entries)
}

fn token_frequency_key(token: &ReaderToken) -> Option<&str> {
    if !token.normalized_text.is_empty() {
        Some(&token.normalized_text)
    } else {
        None
    }
}

fn canonical_frequency_key(token: &ReaderToken) -> Option<String> {
    token_frequency_key(token).and_then(cefr::frequency_key)
}

fn frequency_level_for_non_a1_count(count: usize, max_count: usize) -> CefrLevel {
    if max_count <= 1 {
        return CefrLevel::C2;
    }
    // Zipfian word frequencies have a long tail, so compare non-A1 counts in log space.
    let score = (count.max(1) as f64).ln() / (max_count as f64).ln();
    if score >= 4.0 / 5.0 {
        CefrLevel::A2
    } else if score >= 3.0 / 5.0 {
        CefrLevel::B1
    } else if score >= 2.0 / 5.0 {
        CefrLevel::B2
    } else if score >= 1.0 / 5.0 {
        CefrLevel::C1
    } else {
        CefrLevel::C2
    }
}

fn cefr_level_to_storage(level: CefrLevel) -> &'static str {
    match level {
        CefrLevel::A1 => "A1",
        CefrLevel::A2 => "A2",
        CefrLevel::B1 => "B1",
        CefrLevel::B2 => "B2",
        CefrLevel::C1 => "C1",
        CefrLevel::C2 => "C2",
    }
}

fn cefr_level_from_storage(value: &str) -> Option<CefrLevel> {
    match value {
        "A1" => Some(CefrLevel::A1),
        "A2" => Some(CefrLevel::A2),
        "B1" => Some(CefrLevel::B1),
        "B2" => Some(CefrLevel::B2),
        "C1" => Some(CefrLevel::C1),
        "C2" => Some(CefrLevel::C2),
        _ => None,
    }
}
