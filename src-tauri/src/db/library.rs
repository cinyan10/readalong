pub fn list_books(connection: &Connection) -> Result<Vec<BookSummary>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            b.id,
            b.title,
            b.author,
            b.cover_asset_path,
            COALESCE(rp.progress_percent, 0.0) AS progress_percent,
            rp.last_read_at,
            rp.last_block_index,
            b.created_at,
            b.updated_at
        FROM books b
        LEFT JOIN reading_progress rp ON rp.book_id = b.id
        ORDER BY COALESCE(rp.last_read_at, b.updated_at) DESC, b.title COLLATE NOCASE ASC
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            BookSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                cover_asset_path: row.get(3)?,
                progress_percent: row.get(4)?,
                last_read_at: row.get(5)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            },
            row.get::<_, Option<i64>>(6)?,
        ))
    })?;
    let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(mut book, last_block_index)| {
            if let Some(block_index) = last_block_index {
                if let Some(percent) = progress_percent_for_block(connection, book.id, block_index)?
                {
                    book.progress_percent = percent;
                }
            }
            Ok(book)
        })
        .collect()
}

pub fn import_book(
    connection: &mut Connection,
    data_dir: &Path,
    source_path: &Path,
) -> Result<ImportOutcome> {
    if !source_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("epub"))
    {
        return Err(anyhow!("Only EPUB files can be imported in phase 1."));
    }

    let bytes = fs::read(source_path)
        .with_context(|| format!("Unable to read {}", source_path.display()))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let existing: Option<i64> = connection
        .query_row(
            "SELECT id FROM books WHERE content_hash = ?",
            params![hash],
            |row| row.get(0),
        )
        .optional()?;
    if existing.is_some() {
        return Ok(ImportOutcome::Skipped);
    }

    let extracted = epub::read_epub(source_path)
        .with_context(|| format!("Unable to parse {}", source_path.display()))?;
    if extracted.chapters.is_empty() {
        return Err(anyhow!("No readable chapters were found."));
    }

    let books_dir = data_dir.join("books");
    let assets_dir = data_dir.join("assets").join(&hash);
    fs::create_dir_all(&books_dir)?;
    fs::create_dir_all(&assets_dir)?;

    let stored_path = books_dir.join(format!("{hash}.epub"));
    fs::write(&stored_path, bytes)?;

    let cover_asset_path = if let Some(cover) = extracted.cover {
        let extension = Path::new(&cover.path)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("img");
        let path = assets_dir.join(format!("cover.{extension}"));
        fs::write(&path, cover.bytes)?;
        Some(path_to_string(path))
    } else {
        None
    };

    let timestamp = now_iso();
    let tx = connection.transaction()?;
    tx.execute(
        r#"
        INSERT INTO books (
            slug, title, author, content_hash, original_filename, stored_path, cover_asset_path, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            slugify(&extracted.title),
            extracted.title,
            extracted.author,
            hash,
            source_path.file_name().and_then(|value| value.to_str()).unwrap_or("book.epub"),
            path_to_string(stored_path),
            cover_asset_path,
            timestamp,
            timestamp
        ],
    )?;
    let book_id = tx.last_insert_rowid();

    let mut block_index = 0_i64;
    for (chapter_index, chapter) in extracted.chapters.iter().enumerate() {
        let start_block_index = block_index;
        for block in &chapter.blocks {
            let asset_path = if matches!(block.kind, ExtractedBlockKind::Image) {
                block
                    .asset_path
                    .as_deref()
                    .and_then(|path| {
                        materialize_epub_asset(source_path, &assets_dir, path)
                            .ok()
                            .flatten()
                    })
                    .or_else(|| block.asset_path.clone())
            } else {
                None
            };
            tx.execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    book_id,
                    chapter_index as i64,
                    block_index,
                    match block.kind {
                        ExtractedBlockKind::Paragraph => "paragraph",
                        ExtractedBlockKind::Image => "image",
                    },
                    block.text,
                    asset_path,
                    block.alt.clone()
                ],
            )?;
            block_index += 1;
        }
        tx.execute(
            r#"
            INSERT INTO book_chapters (
                book_id, chapter_index, title, source_href, start_block_index, end_block_index
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            params![
                book_id,
                chapter_index as i64,
                chapter.title,
                chapter.source_href,
                start_block_index,
                block_index.saturating_sub(1)
            ],
        )?;
    }
    ensure_book_word_frequency_cache(&tx, book_id)?;
    tx.commit()?;

    Ok(ImportOutcome::Imported)
}
