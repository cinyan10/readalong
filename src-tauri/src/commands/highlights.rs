#[tauri::command]
pub fn list_book_highlights(
    book_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<ReaderHighlight>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::list_book_highlights(&connection, book_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_highlight(
    book_id: i64,
    chapter_index: i64,
    block_index: i64,
    start_token_index: usize,
    end_token_index: usize,
    start_offset: usize,
    end_offset: usize,
    text: String,
    state: State<'_, AppState>,
) -> Result<Option<ReaderHighlight>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::toggle_highlight(
        &connection,
        book_id,
        chapter_index,
        block_index,
        start_token_index,
        end_token_index,
        start_offset,
        end_offset,
        &text,
    )
    .map_err(|error| error.to_string())
}
