#[tauri::command]
pub fn get_part_audio(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    state: State<'_, AppState>,
) -> Result<Option<PartAudioPayload>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    let audio = db::get_part_audio(
        &connection,
        book_id,
        chapter_index,
        part_index,
        DEFAULT_AUDIO_VOICE,
    )
    .map_err(|error| error.to_string())?;
    Ok(audio.filter(|payload| Path::new(&payload.audio_path).exists()))
}

#[tauri::command]
pub fn get_part_alignment(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    state: State<'_, AppState>,
) -> Result<Option<PartAlignmentPayload>, String> {
    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::get_part_alignment(
        &connection,
        book_id,
        chapter_index,
        part_index,
        DEFAULT_AUDIO_VOICE,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn sync_part_alignment(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    regenerate: bool,
    state: State<'_, AppState>,
) -> Result<PartAlignmentPayload, String> {
    if !regenerate {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        if let Some(alignment) = db::get_part_alignment(
            &connection,
            book_id,
            chapter_index,
            part_index,
            DEFAULT_AUDIO_VOICE,
        )
        .map_err(|error| error.to_string())?
        {
            return Ok(alignment);
        }
    }

    let (audio, paragraphs, generated_paragraphs) = {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        let audio = db::get_part_audio(
            &connection,
            book_id,
            chapter_index,
            part_index,
            DEFAULT_AUDIO_VOICE,
        )
        .map_err(|error| error.to_string())?
        .filter(|payload| Path::new(&payload.audio_path).exists())
        .ok_or_else(|| "Generate audio before syncing words.".to_string())?;
        let paragraphs = db::part_audio_paragraphs(&connection, book_id, chapter_index, part_index)
            .map_err(|error| error.to_string())?;
        let generated_paragraphs = db::generated_audio_paragraphs(
            &connection,
            book_id,
            chapter_index,
            part_index,
            DEFAULT_AUDIO_VOICE,
        )
        .map_err(|error| error.to_string())?;
        (audio, paragraphs, generated_paragraphs)
    };

    let output_dir = Path::new(&audio.audio_path)
        .parent()
        .ok_or_else(|| "Audio path has no parent directory.".to_string())?
        .to_path_buf();
    run_and_store_part_alignment(
        &state,
        book_id,
        chapter_index,
        part_index,
        &audio.voice,
        &audio.audio_path,
        audio.duration_seconds,
        &paragraphs,
        &generated_paragraphs,
        &output_dir,
    )
    .await
}

#[tauri::command]
pub async fn generate_part_audio(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    regenerate: bool,
    state: State<'_, AppState>,
    window: Window,
) -> Result<PartAudioPayload, String> {
    {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        if !regenerate {
            if let Some(audio) = db::get_part_audio(
                &connection,
                book_id,
                chapter_index,
                part_index,
                DEFAULT_AUDIO_VOICE,
            )
            .map_err(|error| error.to_string())?
            .filter(|payload| Path::new(&payload.audio_path).exists())
            {
                if cached_audio_matches_current_format(
                    &connection,
                    book_id,
                    chapter_index,
                    part_index,
                    &audio.voice,
                )
                .map_err(|error| error.to_string())?
                {
                    return Ok(audio);
                }
            }
        }
    }

    let (chapter_title, paragraphs) = {
        let connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        let chapter_title = db::reader_chapter_title(&connection, book_id, chapter_index)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Chapter not found.".to_string())?;
        let paragraphs = db::part_audio_paragraphs(&connection, book_id, chapter_index, part_index)
            .map_err(|error| error.to_string())?;
        (chapter_title, paragraphs)
    };
    if paragraphs.is_empty() {
        return Err("No paragraphs found for this part.".to_string());
    }

    let output_dir = state
        .data_dir
        .join("audio")
        .join(format!("book-{book_id}"))
        .join(format!("chapter-{chapter_index}"))
        .join(format!("part-{part_index}"))
        .join(DEFAULT_AUDIO_VOICE);
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let request_path = output_dir.join("request.json");
    let response_path = output_dir.join("response.json");
    let part_output_path = output_dir.join("part.wav");
    let mut paragraph_hashes = HashMap::new();
    let mut request_paragraphs = Vec::new();
    if part_index == 0 {
        let title = chapter_title.trim();
        if !title.is_empty() {
            let tts_text = tts_pronunciation_text(title);
            let block_index = title_audio_block_index(chapter_index);
            paragraph_hashes.insert(block_index, hash_text(&tts_text));
            request_paragraphs.push(GeneratorRequestParagraph {
                block_index,
                text: tts_text,
                output_path: path_to_string(output_dir.join("chapter-title.wav")),
            });
        }
    }
    request_paragraphs.extend(paragraphs.iter().map(|paragraph| {
        let tts_text = tts_pronunciation_text(&paragraph.text);
        let text_hash = hash_text(&tts_text);
        paragraph_hashes.insert(paragraph.block_index, text_hash);
        GeneratorRequestParagraph {
            block_index: paragraph.block_index,
            text: tts_text,
            output_path: path_to_string(
                output_dir.join(format!("block-{}.wav", paragraph.block_index)),
            ),
        }
    }));

    let request = GeneratorRequest {
        voice: DEFAULT_AUDIO_VOICE.to_string(),
        speed: DEFAULT_AUDIO_SPEED,
        part_output_path: path_to_string(part_output_path),
        paragraphs: request_paragraphs,
    };
    fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    let generator_request_path = request_path.clone();
    let generator_response_path = response_path.clone();
    let generator_window = window.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_kokoro_generator(
            &generator_request_path,
            &generator_response_path,
            move |line| {
                let percent = match line.stage.as_str() {
                    "loading_model" => 2.0,
                    "rendering" => {
                        let rendered = line.completed as f64 / line.total.max(1) as f64;
                        5.0 + rendered * 85.0
                    }
                    "assembling" => 95.0,
                    _ => 0.0,
                };
                emit_audio_generation_progress(
                    &generator_window,
                    book_id,
                    chapter_index,
                    part_index,
                    line.completed,
                    line.total,
                    percent,
                    &line.stage,
                );
            },
        )
    })
    .await
    .map_err(|error| format!("Kokoro generation task failed: {error}"))??;

    let response: GeneratorResponse = serde_json::from_slice(
        &fs::read(&response_path)
            .map_err(|error| format!("Unable to read generator response: {error}"))?,
    )
    .map_err(|error| format!("Invalid generator response: {error}"))?;

    let response_paragraphs = response.paragraphs.clone();
    let generated = GeneratedPartAudio {
        book_id,
        chapter_index,
        part_index,
        voice: response.voice,
        audio_path: response.part_path,
        duration_seconds: response.duration_seconds,
        paragraphs: response
            .paragraphs
            .into_iter()
            .map(|paragraph| GeneratedAudioParagraph {
                block_index: paragraph.block_index,
                text_hash: paragraph_hashes
                    .remove(&paragraph.block_index)
                    .unwrap_or_else(|| "".to_string()),
                audio_path: paragraph.path,
                duration_seconds: paragraph.duration_seconds,
            })
            .collect(),
    };

    let saved = {
        let mut connection = state
            .db
            .lock()
            .map_err(|_| "Database lock failed.".to_string())?;
        let saved =
            db::save_part_audio(&mut connection, &generated).map_err(|error| error.to_string())?;
        db::delete_part_alignment(
            &connection,
            book_id,
            chapter_index,
            part_index,
            DEFAULT_AUDIO_VOICE,
        )
        .map_err(|error| error.to_string())?;
        saved
    };

    let alignment_paragraphs = response_paragraphs
        .iter()
        .map(|paragraph| GeneratedAudioParagraph {
            block_index: paragraph.block_index,
            text_hash: String::new(),
            audio_path: paragraph.path.clone(),
            duration_seconds: paragraph.duration_seconds,
        })
        .collect::<Vec<_>>();
    let saved = match run_and_store_part_alignment(
        &state,
        book_id,
        chapter_index,
        part_index,
        &saved.voice,
        &saved.audio_path,
        saved.duration_seconds,
        &paragraphs,
        &alignment_paragraphs,
        &output_dir,
    )
    .await
    {
        Ok(_) => PartAudioPayload {
            alignment_available: true,
            alignment_error: None,
            ..saved
        },
        Err(error) => {
            let connection = state
                .db
                .lock()
                .map_err(|_| "Database lock failed.".to_string())?;
            db::save_part_alignment_error(
                &connection,
                book_id,
                chapter_index,
                part_index,
                &saved.voice,
                &saved.audio_path,
                &error,
            )
            .map_err(|error| error.to_string())?;
            PartAudioPayload {
                alignment_available: false,
                alignment_error: Some(error),
                ..saved
            }
        }
    };
    emit_audio_generation_progress(
        &window,
        book_id,
        chapter_index,
        part_index,
        saved.paragraph_count.max(0) as usize,
        saved.paragraph_count.max(0) as usize,
        100.0,
        "complete",
    );
    Ok(saved)
}
