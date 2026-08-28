#[tauri::command]
pub fn list_wordlist_entries(state: State<'_, AppState>) -> Result<Vec<WordlistEntry>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::list_wordlist_entries(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_book_wordlist_entries(
    book_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<WordlistEntry>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::list_book_wordlist_entries(&connection, book_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn add_wordlist_entry(
    book_id: i64,
    chapter_index: i64,
    block_index: i64,
    token_index: usize,
    word: String,
    root_word: String,
    context: String,
    cefr_level: String,
    state: State<'_, AppState>,
    window: Window,
) -> Result<WordlistEntry, String> {
    let entry = {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        db::save_wordlist_entry(
            &connection,
            book_id,
            chapter_index,
            block_index,
            token_index,
            &word,
            &root_word,
            &context,
            &cefr_level,
        )
        .map_err(|error| error.to_string())?
    };
    if wordlist_entry_needs_enrichment(&entry) {
        let db_path = state.data_dir.join("readalong.sqlite3");
        spawn_wordlist_enrichment(db_path, window, entry.id);
    }
    Ok(entry)
}

#[tauri::command]
pub fn delete_wordlist_entry(root_word: String, state: State<'_, AppState>) -> Result<bool, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::delete_wordlist_entry(&connection, &root_word).map_err(|error| error.to_string())
}

fn spawn_wordlist_enrichment(db_path: PathBuf, window: Window, entry_id: i64) {
    tauri::async_runtime::spawn(async move {
        let entry = {
            let Ok(connection) = db::connect(&db_path) else {
                return;
            };
            match db::wordlist_entry_for_lookup(&connection, entry_id) {
                Ok(Some(entry)) if wordlist_entry_needs_enrichment(&entry) => entry,
                _ => return,
            }
        };
        let context = format!("{}\n\n{}", entry.root_word, entry.context);
        let lemma = crate::dictionary::normalize_cache_lemma(&entry.root_word, &entry.root_word);
        let context_key = crate::dictionary::context_cache_key(&context);
        let (cached_oxford, cached_context) = match db::connect(&db_path) {
            Ok(connection) => (
                db::get_oxford_cache(&connection, &lemma).ok().flatten(),
                db::get_context_cache(&connection, &lemma, &context_key).ok().flatten(),
            ),
            Err(_) => (None, None),
        };
        let lookup = crate::dictionary::lookup_word_with_cached_data(
            entry.root_word.clone(),
            context,
            entry.cefr_level.clone(),
            entry.root_word.clone(),
            cached_oxford,
            cached_context,
        )
        .await;
        let updated = {
            let Ok(connection) = db::connect(&db_path) else {
                return;
            };
            match lookup {
                Ok((result, oxford_payload)) => {
                    let canonical = crate::dictionary::normalize_cache_lemma(&result.word, "");
                    if !oxford_payload.is_empty() && !canonical.is_empty() {
                        let _ = db::save_oxford_cache(&connection, &canonical, &oxford_payload);
                        if canonical != lemma {
                            let _ = db::save_oxford_cache(&connection, &lemma, &oxford_payload);
                        }
                    }
                    if !context_key.is_empty() && !canonical.is_empty() {
                        if let Ok(payload) = serde_json::to_string(&result) {
                            let _ = db::save_context_cache(&connection, &canonical, &context_key, &payload);
                            if canonical != lemma {
                                let _ = db::save_context_cache(&connection, &lemma, &context_key, &payload);
                            }
                        }
                    }
                    db::update_wordlist_entry_lookup(&connection, entry_id, Some(&result), "")
                }
                Err(error) => db::update_wordlist_entry_lookup(&connection, entry_id, None, &error.to_string()),
            }
        };
        if let Ok(Some(entry)) = updated {
            let _ = window.emit("wordlist_entry_enriched", entry);
        }
    });
}

fn wordlist_entry_needs_enrichment(entry: &WordlistEntry) -> bool {
    entry.definition.is_empty()
        || (entry.simple_meaning.trim().is_empty() && entry.in_context_meaning.trim().is_empty())
}
