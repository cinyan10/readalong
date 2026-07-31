#[derive(Debug, Serialize)]
struct GeneratorRequest {
    voice: String,
    speed: f64,
    part_output_path: String,
    paragraphs: Vec<GeneratorRequestParagraph>,
}

#[derive(Debug, Serialize)]
struct GeneratorRequestParagraph {
    block_index: i64,
    text: String,
    output_path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GeneratorResponse {
    voice: String,
    part_path: String,
    duration_seconds: f64,
    paragraphs: Vec<GeneratorResponseParagraph>,
}

#[derive(Clone, Debug, Deserialize)]
struct GeneratorResponseParagraph {
    block_index: i64,
    path: String,
    duration_seconds: f64,
}

#[derive(Debug, Deserialize)]
struct GeneratorProgressLine {
    event: String,
    stage: String,
    completed: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
struct AlignmentRequest {
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: String,
    audio_path: String,
    duration_seconds: f64,
    paragraphs: Vec<AlignmentRequestParagraph>,
}

#[derive(Debug, Serialize)]
struct AlignmentRequestParagraph {
    block_index: i64,
    text: String,
    audio_path: String,
    offset_seconds: f64,
    duration_seconds: f64,
    tokens: Vec<AlignmentRequestToken>,
}

#[derive(Debug, Serialize)]
struct AlignmentRequestToken {
    token_index: usize,
    block_index: i64,
    text: String,
    normalized_text: String,
}

#[derive(Clone, Debug, Serialize)]
struct AudioGenerationProgress {
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    completed: usize,
    total: usize,
    percent: f64,
    stage: String,
}

fn emit_audio_generation_progress(
    window: &Window,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    completed: usize,
    total: usize,
    percent: f64,
    stage: &str,
) {
    let _ = window.emit(
        "part-audio-progress",
        AudioGenerationProgress {
            book_id,
            chapter_index,
            part_index,
            completed,
            total,
            percent: percent.clamp(0.0, 100.0),
            stage: stage.to_string(),
        },
    );
}

async fn run_and_store_part_alignment(
    state: &State<'_, AppState>,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
    audio_path: &str,
    duration_seconds: f64,
    paragraphs: &[db::AudioParagraphSource],
    generated_paragraphs: &[GeneratedAudioParagraph],
    output_dir: &Path,
) -> Result<PartAlignmentPayload, String> {
    let request_path = output_dir.join("alignment-request.json");
    let response_path = output_dir.join("alignment.json");
    let request = build_alignment_request(
        book_id,
        chapter_index,
        part_index,
        voice,
        audio_path,
        duration_seconds,
        paragraphs,
        generated_paragraphs,
    )?;
    fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    let alignment_request_path = request_path.clone();
    let alignment_response_path = response_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_alignment_worker(&alignment_request_path, &alignment_response_path)
    })
    .await
    .map_err(|error| format!("Alignment task failed: {error}"))??;

    let alignment: PartAlignmentPayload = serde_json::from_slice(
        &fs::read(&response_path)
            .map_err(|error| format!("Unable to read alignment response: {error}"))?,
    )
    .map_err(|error| format!("Invalid alignment response: {error}"))?;

    let connection = state
        .db
        .lock()
        .map_err(|_| "Database lock failed.".to_string())?;
    db::save_part_alignment(&connection, &alignment, &response_path)
        .map_err(|error| error.to_string())?;
    Ok(alignment)
}

fn build_alignment_request(
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
    audio_path: &str,
    duration_seconds: f64,
    paragraphs: &[db::AudioParagraphSource],
    generated_paragraphs: &[GeneratedAudioParagraph],
) -> Result<AlignmentRequest, String> {
    let generated_by_block = generated_paragraphs
        .iter()
        .map(|paragraph| (paragraph.block_index, paragraph))
        .collect::<HashMap<_, _>>();
    let mut offset_seconds = leading_audio_offset_seconds(generated_paragraphs);
    let mut request_paragraphs = Vec::new();

    for paragraph in paragraphs {
        let generated = generated_by_block
            .get(&paragraph.block_index)
            .ok_or_else(|| {
                format!(
                    "Generated audio missing for block {}.",
                    paragraph.block_index
                )
            })?;
        let tokens = cefr::tokenize_text(&paragraph.text)
            .into_iter()
            .enumerate()
            .map(|(token_index, token)| AlignmentRequestToken {
                token_index,
                block_index: paragraph.block_index,
                text: token.text,
                normalized_text: token.normalized_text,
            })
            .collect();
        request_paragraphs.push(AlignmentRequestParagraph {
            block_index: paragraph.block_index,
            text: paragraph.text.clone(),
            audio_path: generated.audio_path.clone(),
            offset_seconds: round_seconds(offset_seconds),
            duration_seconds: generated.duration_seconds,
            tokens,
        });
        offset_seconds += generated.duration_seconds + PARAGRAPH_SILENCE_SECONDS;
    }

    Ok(AlignmentRequest {
        book_id,
        chapter_index,
        part_index,
        voice: voice.to_string(),
        audio_path: audio_path.to_string(),
        duration_seconds,
        paragraphs: request_paragraphs,
    })
}

fn cached_audio_matches_current_format(
    connection: &rusqlite::Connection,
    book_id: i64,
    chapter_index: i64,
    part_index: i64,
    voice: &str,
) -> anyhow::Result<bool> {
    let paragraphs = db::part_audio_paragraphs(connection, book_id, chapter_index, part_index)?;
    let existing =
        db::generated_audio_paragraphs(connection, book_id, chapter_index, part_index, voice)?;
    let existing_by_block = existing
        .iter()
        .map(|paragraph| (paragraph.block_index, paragraph))
        .collect::<HashMap<_, _>>();

    for paragraph in paragraphs {
        let Some(generated) = existing_by_block.get(&paragraph.block_index) else {
            return Ok(false);
        };
        if !Path::new(&generated.audio_path).exists()
            || generated.text_hash != hash_text(&tts_pronunciation_text(&paragraph.text))
        {
            return Ok(false);
        }
    }

    if part_index != 0 {
        return Ok(true);
    }

    let chapter_title =
        db::reader_chapter_title(connection, book_id, chapter_index)?.unwrap_or_default();
    let title = chapter_title.trim();
    if title.is_empty() {
        return Ok(true);
    }
    let title_block_index = title_audio_block_index(chapter_index);
    Ok(existing_by_block
        .get(&title_block_index)
        .is_some_and(|paragraph| {
            Path::new(&paragraph.audio_path).exists()
                && paragraph.text_hash == hash_text(&tts_pronunciation_text(title))
        }))
}

fn leading_audio_offset_seconds(generated_paragraphs: &[GeneratedAudioParagraph]) -> f64 {
    generated_paragraphs
        .first()
        .filter(|paragraph| is_title_audio_block_index(paragraph.block_index))
        .map(|paragraph| paragraph.duration_seconds + PARAGRAPH_SILENCE_SECONDS)
        .unwrap_or(0.0)
}

fn title_audio_block_index(chapter_index: i64) -> i64 {
    TITLE_AUDIO_BLOCK_BASE.saturating_sub(chapter_index.max(0))
}

fn is_title_audio_block_index(block_index: i64) -> bool {
    block_index <= TITLE_AUDIO_BLOCK_BASE
}

fn run_alignment_worker(request_path: &Path, response_path: &Path) -> Result<(), String> {
    let repo_root = repo_root();
    let python_path = worker_python_path(&repo_root);
    let script_path = std::env::var_os("READALONG_ALIGN_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("scripts").join("align_part_audio.py"));
    let model = std::env::var("READALONG_ALIGN_MODEL").unwrap_or_else(|_| "small.en".to_string());

    if !python_path.exists() {
        return Err(format!(
            "Python environment not found at {}. Create .venv or set READALONG_PYTHON.",
            python_path.display()
        ));
    }
    if !script_path.exists() {
        return Err(format!(
            "Alignment script not found at {}. Set READALONG_ALIGN_SCRIPT if it lives elsewhere.",
            script_path.display()
        ));
    }

    let output = Command::new(&python_path)
        .arg(&script_path)
        .arg("--request")
        .arg(request_path)
        .arg("--response")
        .arg(response_path)
        .arg("--model")
        .arg(model)
        .output()
        .map_err(|error| format!("Unable to start alignment worker: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if stderr.is_empty() { stdout } else { stderr };
    Err(format!("Alignment failed: {details}"))
}

fn run_kokoro_generator<F>(
    request_path: &Path,
    response_path: &Path,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(GeneratorProgressLine),
{
    let repo_root = repo_root();
    let python_path = worker_python_path(&repo_root);
    let script_path = std::env::var_os("READALONG_KOKORO_SCRIPT")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("scripts").join("kokoro_generate_part.py"));

    if !python_path.exists() {
        return Err(format!(
            "Python environment not found at {}. Create .venv or set READALONG_PYTHON.",
            python_path.display()
        ));
    }
    if !script_path.exists() {
        return Err(format!(
            "Kokoro generator script not found at {}. Set READALONG_KOKORO_SCRIPT if it lives elsewhere.",
            script_path.display()
        ));
    }

    let mut child = Command::new(&python_path)
        .arg(&script_path)
        .arg("--request")
        .arg(request_path)
        .arg("--response")
        .arg(response_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start Kokoro generator: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Unable to read Kokoro generator stdout.".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Unable to read Kokoro generator stderr.".to_string())?;
    let stderr_reader = thread::spawn(move || {
        let mut details = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = reader.read_to_string(&mut details);
        details
    });

    let mut stdout_details = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line =
            line.map_err(|error| format!("Unable to read Kokoro generator output: {error}"))?;
        if let Ok(progress) = serde_json::from_str::<GeneratorProgressLine>(&line) {
            if progress.event == "progress" {
                on_progress(progress);
                continue;
            }
        }
        stdout_details.push(line);
    }

    let status = child
        .wait()
        .map_err(|error| format!("Unable to wait for Kokoro generator: {error}"))?;
    let stderr = stderr_reader.join().unwrap_or_default();

    if status.success() {
        return Ok(());
    }

    let details = if stderr.trim().is_empty() {
        stdout_details.join("\n")
    } else {
        stderr.trim().to_string()
    };
    Err(format!("Kokoro generation failed: {details}"))
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap_or(&manifest_dir).to_path_buf()
}

fn worker_python_path(repo_root: &Path) -> PathBuf {
    std::env::var_os("READALONG_PYTHON")
        .or_else(|| std::env::var_os("READALONG_KOKORO_PYTHON"))
        .map(PathBuf::from)
        .unwrap_or_else(|| default_python_path(repo_root))
}

fn default_python_path(repo_root: &Path) -> PathBuf {
    if cfg!(windows) {
        repo_root.join(".venv").join("Scripts").join("python.exe")
    } else {
        repo_root.join(".venv").join("bin").join("python")
    }
}

fn hash_text(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn tts_pronunciation_text(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if let Some((replacement, consumed)) = stutter_pronunciation(&chars, index) {
            out.push_str(&replacement);
            index += consumed;
            continue;
        }

        if let Some((replacement, consumed)) = vocalization_pronunciation(&chars, index) {
            out.push_str(&replacement);
            index += consumed;
            continue;
        }

        out.push(chars[index]);
        index += 1;
    }

    out
}

fn stutter_pronunciation(chars: &[char], index: usize) -> Option<(String, usize)> {
    if !is_stutter_boundary(chars.get(index.wrapping_sub(1)).copied(), index) {
        return None;
    }

    let mut fragments = Vec::new();
    let mut word_start = index;
    loop {
        let mut word_end = word_start;
        while chars
            .get(word_end)
            .is_some_and(|character| character.is_ascii_alphabetic())
        {
            word_end += 1;
        }
        if word_end == word_start || chars.get(word_end) != Some(&'-') {
            break;
        }
        fragments.push(chars[word_start..word_end].iter().collect::<String>());
        word_start = word_end + 1;
    }

    if fragments.is_empty() {
        return None;
    }

    let mut word_end = word_start;
    while chars
        .get(word_end)
        .is_some_and(|character| character.is_ascii_alphabetic())
    {
        word_end += 1;
    }
    if word_end == word_start {
        return None;
    }

    let word = chars[word_start..word_end].iter().collect::<String>();
    if fragments.iter().any(|fragment| {
        !word
            .get(..fragment.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(fragment))
            || (fragment.len() > 1 && fragment.len() >= word.len())
    }) {
        return None;
    }

    Some((word, word_end - index))
}

fn vocalization_pronunciation(chars: &[char], index: usize) -> Option<(String, usize)> {
    if !is_stutter_boundary(chars.get(index.wrapping_sub(1)).copied(), index) {
        return None;
    }

    let mut word_start = index;
    let mut uppercase_like = chars[index].is_ascii_uppercase();
    if index + 2 < chars.len()
        && chars[index].is_ascii_alphabetic()
        && chars[index + 1] == '-'
        && chars[index].eq_ignore_ascii_case(&chars[index + 2])
    {
        word_start = index + 2;
        uppercase_like = chars[index].is_ascii_uppercase();
    }

    let mut word_end = word_start;
    while chars
        .get(word_end)
        .is_some_and(|character| character.is_ascii_alphabetic())
    {
        word_end += 1;
    }
    if word_end == word_start {
        return None;
    }
    if chars
        .get(word_end)
        .is_some_and(|character| character.is_ascii_alphabetic())
    {
        return None;
    }

    let word = chars[word_start..word_end]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    let replacement = match word.as_str() {
        "um" | "umm" | "ummm" => "umm",
        "uh" | "uhh" | "uhhh" => "uhh",
        "ah" | "ahh" | "ahhh" => "ahh",
        "ngh" | "nghh" | "nghhh" | "nghhhh" => "ungh",
        "hngh" | "hnghh" | "hnghhh" | "hnghhhh" => "hungh",
        "unngh" | "unnngh" | "unnnngh" => "ungh",
        _ => return None,
    };

    let consumed = word_end - index;
    Some((capitalize_like(replacement, uppercase_like), consumed))
}

fn capitalize_like(value: &str, uppercase: bool) -> String {
    if !uppercase {
        return value.to_string();
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
}

fn is_stutter_boundary(previous: Option<char>, index: usize) -> bool {
    index == 0 || previous.is_none_or(|character| !character.is_ascii_alphabetic())
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn round_seconds(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_audio_segment_offsets_body_alignment() {
        let paragraphs = vec![
            GeneratedAudioParagraph {
                block_index: title_audio_block_index(4),
                text_hash: String::new(),
                audio_path: String::new(),
                duration_seconds: 1.5,
            },
            GeneratedAudioParagraph {
                block_index: 42,
                text_hash: String::new(),
                audio_path: String::new(),
                duration_seconds: 3.0,
            },
        ];

        assert_eq!(leading_audio_offset_seconds(&paragraphs), 1.72);
    }

    #[test]
    fn body_audio_without_title_starts_alignment_at_zero() {
        let paragraphs = vec![GeneratedAudioParagraph {
            block_index: 42,
            text_hash: String::new(),
            audio_path: String::new(),
            duration_seconds: 3.0,
        }];

        assert_eq!(leading_audio_offset_seconds(&paragraphs), 0.0);
    }

    #[test]
    fn tts_text_removes_stuttered_prefixes() {
        assert_eq!(
            tts_pronunciation_text("L-look at this."),
            "look at this."
        );
        assert_eq!(
            tts_pronunciation_text("\"W-wait,\" she said."),
            "\"wait,\" she said."
        );
        assert_eq!(tts_pronunciation_text("I s-said no."), "I said no.");
        assert_eq!(tts_pronunciation_text("Wh-whoa…"), "whoa…");
        assert_eq!(tts_pronunciation_text("H-Hachiman"), "Hachiman");
        assert_eq!(tts_pronunciation_text("S-s-s-sorry!"), "sorry!");
        assert_eq!(tts_pronunciation_text("I-I'll go."), "I'll go.");
    }

    #[test]
    fn tts_text_keeps_non_stutter_hyphen_words() {
        assert_eq!(
            tts_pronunciation_text("A-team and X-ray."),
            "A-team and X-ray."
        );
        assert_eq!(tts_pronunciation_text("well-worn words"), "well-worn words");
        assert_eq!(tts_pronunciation_text("Ha-ha-ha!"), "Ha-ha-ha!");
        assert_eq!(tts_pronunciation_text("Heh-heh"), "Heh-heh");
    }

    #[test]
    fn tts_text_phoneticizes_short_vocalizations() {
        assert_eq!(tts_pronunciation_text("Um…uh…"), "Umm…uhh…");
        assert_eq!(tts_pronunciation_text("U-um… I have no idea."), "um… I have no idea.");
        assert_eq!(tts_pronunciation_text("“Uh…”"), "“Uhh…”");
        assert_eq!(tts_pronunciation_text("Ngh, alas!"), "Ungh, alas!");
        assert_eq!(tts_pronunciation_text("Hngh! This is my chance!"), "Hungh! This is my chance!");
        assert_eq!(tts_pronunciation_text("but…unnngh"), "but…ungh");
        assert_eq!(tts_pronunciation_text("Ah…ah-ha-ha"), "Ahh…ahh-ha-ha");
    }

    #[test]
    fn tts_text_keeps_vocalization_strings_inside_words() {
        assert_eq!(tts_pronunciation_text("human and unhinged"), "human and unhinged");
        assert_eq!(tts_pronunciation_text("Yuigahama"), "Yuigahama");
    }
}
