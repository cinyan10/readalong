#[tauri::command]
pub fn list_books(state: State<'_, AppState>) -> Result<Vec<BookSummary>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    let mut books = db::list_books(&connection).map_err(|error| error.to_string())?;
    for book in &mut books {
        populate_book_audio_availability(&connection, book).map_err(|error| error.to_string())?;
    }
    Ok(books)
}

fn populate_book_audio_availability(
    connection: &rusqlite::Connection,
    book: &mut BookSummary,
) -> anyhow::Result<()> {
    let parts = readable_book_audio_parts(connection, book.id)?;
    let total_parts = parts.len() as i64;
    let mut generated_parts = 0_i64;
    for part in parts {
        if part_audio_is_available(connection, book.id, &part)? {
            generated_parts += 1;
        }
    }

    book.audio_generated_parts = generated_parts;
    book.audio_total_parts = total_parts;
    book.audio_percent = if total_parts == 0 {
        0.0
    } else {
        generated_parts as f64 / total_parts as f64 * 100.0
    };
    Ok(())
}

#[tauri::command]
pub fn list_missing_book_audio_parts(
    book_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<BookAudioPart>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    let parts = readable_book_audio_parts(&connection, book_id).map_err(|error| error.to_string())?;
    parts
        .into_iter()
        .filter_map(|part| match part_audio_is_available(&connection, book_id, &part) {
            Ok(true) => None,
            Ok(false) => Some(Ok(part)),
            Err(error) => Some(Err(error.to_string())),
        })
        .collect()
}

fn readable_book_audio_parts(
    connection: &rusqlite::Connection,
    book_id: i64,
) -> anyhow::Result<Vec<BookAudioPart>> {
    let Some(reader) = db::get_reader(connection, book_id)? else {
        return Ok(Vec::new());
    };
    let mut parts = Vec::new();
    for chapter in reader.chapters {
        for part in chapter.parts {
            if !db::part_audio_paragraphs(connection, book_id, chapter.chapter_index, part.part_index)?.is_empty() {
                parts.push(BookAudioPart {
                    chapter_index: chapter.chapter_index,
                    part_index: part.part_index,
                });
            }
        }
    }
    Ok(parts)
}

fn part_audio_is_available(
    connection: &rusqlite::Connection,
    book_id: i64,
    part: &BookAudioPart,
) -> anyhow::Result<bool> {
    Ok(db::get_part_audio(
        connection,
        book_id,
        part.chapter_index,
        part.part_index,
        DEFAULT_AUDIO_VOICE,
    )?
    .is_some_and(|audio| Path::new(&audio.audio_path).exists()))
}

#[tauri::command]
pub fn import_books(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ImportSummary, String> {
    let mut imported = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();
    let mut connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;

    for path in paths {
        match db::import_book(&mut connection, &state.data_dir, Path::new(&path)) {
            Ok(ImportOutcome::Imported) => imported += 1,
            Ok(ImportOutcome::Skipped) => skipped += 1,
            Err(error) => failed.push(ImportFailure {
                path,
                message: error.to_string(),
            }),
        }
    }

    let mut books = db::list_books(&connection).map_err(|error| error.to_string())?;
    for book in &mut books {
        populate_book_audio_availability(&connection, book).map_err(|error| error.to_string())?;
    }
    Ok(ImportSummary {
        imported,
        skipped,
        failed,
        books,
    })
}

#[tauri::command]
pub fn get_reader(book_id: i64, state: State<'_, AppState>) -> Result<ReaderPayload, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::get_reader(&connection, book_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Book not found.".to_string())
}

#[tauri::command]
pub fn get_chapter(
    book_id: i64,
    chapter_index: i64,
    state: State<'_, AppState>,
) -> Result<ChapterPayload, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::get_chapter(&connection, book_id, chapter_index)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Chapter not found.".to_string())
}

#[tauri::command]
pub fn search_book(
    book_id: i64,
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::BookSearchResult>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::search_book(&connection, book_id, &query).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn lookup_word(
    word: String,
    context: String,
    cefr_level: String,
    root_word: String,
    refresh: bool,
    state: State<'_, AppState>,
) -> Result<crate::dictionary::DictionaryLookup, String> {
    let requested_lemma = crate::dictionary::normalize_cache_lemma(&word, &root_word);
    let lemma_candidates = crate::dictionary::cache_lemma_candidates(&word, &root_word);
    let context_key = crate::dictionary::context_cache_key(&context);
    let (cached_oxford, cached_context) = {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        let mut context = None;
        let mut oxford = None;
        for lemma in &lemma_candidates {
            if context.is_none() && !refresh && !context_key.is_empty() {
                context = db::get_context_cache(&connection, lemma, &context_key)
                    .map_err(|error| error.to_string())?;
            }
            if oxford.is_none() {
                oxford = db::get_oxford_cache(&connection, lemma)
                    .map_err(|error| error.to_string())?;
            }
            if (context.is_some() || refresh || context_key.is_empty()) && oxford.is_some() {
                break;
            }
        }
        (oxford, context)
    };

    let (lookup, oxford_payload) = crate::dictionary::lookup_word_with_cached_data(
        word,
        context,
        cefr_level,
        root_word,
        cached_oxford,
        cached_context,
    )
    .await
    .map_err(|error| error.to_string())?;

    let canonical_lemma = crate::dictionary::normalize_cache_lemma(&lookup.word, "");
    if !oxford_payload.is_empty() && !canonical_lemma.is_empty() {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        db::save_oxford_cache(&connection, &canonical_lemma, &oxford_payload)
            .map_err(|error| error.to_string())?;
        if canonical_lemma != requested_lemma && !requested_lemma.is_empty() {
            db::save_oxford_cache(&connection, &requested_lemma, &oxford_payload)
                .map_err(|error| error.to_string())?;
        }
        if !context_key.is_empty() {
            let payload = serde_json::to_string(&lookup).map_err(|error| error.to_string())?;
            db::save_context_cache(&connection, &canonical_lemma, &context_key, &payload)
                .map_err(|error| error.to_string())?;
            if canonical_lemma != requested_lemma && !requested_lemma.is_empty() {
                db::save_context_cache(&connection, &requested_lemma, &context_key, &payload)
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(lookup)
}

#[tauri::command]
pub async fn lookup_word_at(
    book_id: i64,
    block_index: i64,
    token_index: usize,
    refresh: bool,
    state: State<'_, AppState>,
) -> Result<crate::dictionary::DictionaryLookup, String> {
    let input = {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        db::dictionary_context_for_token(&connection, book_id, block_index, token_index)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Select one English word to look up.".to_string())?
    };
    lookup_word(
        input.word,
        input.context,
        input.cefr_level,
        input.root_word,
        refresh,
        state,
    )
    .await
}
