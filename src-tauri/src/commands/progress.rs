#[tauri::command]
pub fn save_progress(
    book_id: i64,
    chapter_index: i64,
    part_index: Option<i64>,
    block_index: i64,
    scroll_ratio: Option<f64>,
    audio_time_seconds: Option<f64>,
    audio_duration_seconds: Option<f64>,
    last_playing_block_index: Option<i64>,
    last_playing_token_index: Option<i64>,
    progress_percent: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::save_progress(
        &connection,
        book_id,
        chapter_index,
        part_index.unwrap_or(0),
        block_index,
        scroll_ratio.unwrap_or(0.0),
        audio_time_seconds,
        audio_duration_seconds,
        last_playing_block_index,
        last_playing_token_index,
        progress_percent,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_bookmark(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    block_index: i64,
    token_index: i64,
    word: String,
    root_word: String,
    scroll_ratio: f64,
    progress_percent: f64,
    state: State<'_, AppState>,
) -> Result<ReadingBookmark, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::save_bookmark(
        &connection,
        book_id,
        chapter_index,
        part_index,
        block_index,
        token_index,
        &word,
        &root_word,
        scroll_ratio,
        progress_percent,
    )
    .map_err(|error| error.to_string())
}
