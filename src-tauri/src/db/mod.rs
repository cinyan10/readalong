use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::cefr;
use crate::cefr::{CefrLevel, ReaderToken};
use crate::epub;
use crate::epub::ExtractedBlockKind;
use crate::models::{
    BookSearchResult, BookSummary, ChapterBlock, ChapterPartSummary, ChapterPayload,
    ChapterSummary, PartAlignmentPayload, PartAudioPayload, ReaderHighlight, ReaderPayload,
    ReadingBookmark, ReadingProgress, WordlistEntry,
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    author TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL UNIQUE,
    original_filename TEXT NOT NULL,
    stored_path TEXT NOT NULL UNIQUE,
    cover_asset_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS book_chapters (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    title TEXT NOT NULL,
    source_href TEXT NOT NULL DEFAULT '',
    start_block_index INTEGER NOT NULL,
    end_block_index INTEGER NOT NULL,
    PRIMARY KEY(book_id, chapter_index)
);

CREATE TABLE IF NOT EXISTS chapter_blocks (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    kind TEXT NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    asset_path TEXT,
    alt TEXT NOT NULL DEFAULT '',
    PRIMARY KEY(book_id, block_index)
);

CREATE TABLE IF NOT EXISTS reading_progress (
    book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
    last_read_at TEXT NOT NULL,
    last_chapter_index INTEGER NOT NULL DEFAULT 0,
    last_part_index INTEGER NOT NULL DEFAULT 0,
    last_block_index INTEGER NOT NULL DEFAULT 0,
    last_scroll_ratio REAL NOT NULL DEFAULT 0,
    last_audio_time_seconds REAL,
    last_audio_duration_seconds REAL,
    last_playing_block_index INTEGER,
    last_playing_token_index INTEGER,
    progress_percent REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS bookmarks (
    book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    chapter_index INTEGER NOT NULL DEFAULT 0,
    part_index INTEGER NOT NULL DEFAULT 0,
    block_index INTEGER NOT NULL DEFAULT 0,
    token_index INTEGER NOT NULL DEFAULT 0,
    word TEXT NOT NULL DEFAULT '',
    root_word TEXT NOT NULL DEFAULT '',
    scroll_ratio REAL NOT NULL DEFAULT 0,
    progress_percent REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS audio_paragraphs (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    part_index INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    voice TEXT NOT NULL,
    text_hash TEXT NOT NULL,
    audio_path TEXT NOT NULL,
    duration_seconds REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(book_id, block_index, voice)
);

CREATE TABLE IF NOT EXISTS audio_parts (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    part_index INTEGER NOT NULL,
    voice TEXT NOT NULL,
    audio_path TEXT NOT NULL,
    paragraph_count INTEGER NOT NULL DEFAULT 0,
    duration_seconds REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(book_id, chapter_index, part_index, voice)
);

CREATE TABLE IF NOT EXISTS audio_alignments (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    part_index INTEGER NOT NULL,
    voice TEXT NOT NULL,
    audio_path TEXT NOT NULL,
    alignment_path TEXT NOT NULL DEFAULT '',
    token_count INTEGER NOT NULL DEFAULT 0,
    duration_seconds REAL NOT NULL DEFAULT 0,
    last_error TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(book_id, chapter_index, part_index, voice)
);

CREATE TABLE IF NOT EXISTS book_word_frequencies (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    word_key TEXT NOT NULL,
    frequency_count INTEGER NOT NULL,
    frequency_level TEXT NOT NULL,
    PRIMARY KEY(book_id, word_key)
);

CREATE TABLE IF NOT EXISTS book_word_frequency_cache (
    book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
    generated_at TEXT NOT NULL,
    algorithm_version INTEGER NOT NULL DEFAULT 6
);

CREATE TABLE IF NOT EXISTS wordlist_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    book_title TEXT NOT NULL DEFAULT '',
    chapter_index INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    token_index INTEGER NOT NULL,
    root_word TEXT NOT NULL,
    original_word TEXT NOT NULL,
    word_type TEXT NOT NULL DEFAULT '',
    cefr_level TEXT NOT NULL DEFAULT '',
    definition_number INTEGER,
    definition TEXT NOT NULL DEFAULT '',
    definition_examples TEXT NOT NULL DEFAULT '[]',
    definition_phonetics TEXT NOT NULL DEFAULT '[]',
    definition_audio_url TEXT NOT NULL DEFAULT '',
    definition_source_url TEXT NOT NULL DEFAULT '',
    definition_lookup_error TEXT NOT NULL DEFAULT '',
    simple_meaning TEXT NOT NULL DEFAULT '',
    in_context_meaning TEXT NOT NULL DEFAULT '',
    original_meaning TEXT NOT NULL DEFAULT '',
    ai_explanation TEXT NOT NULL DEFAULT '',
    context TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(root_word)
);

CREATE TABLE IF NOT EXISTS reader_highlights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    start_token_index INTEGER NOT NULL,
    end_token_index INTEGER NOT NULL,
    start_offset INTEGER NOT NULL DEFAULT 0,
    end_offset INTEGER NOT NULL DEFAULT 0,
    text TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(book_id, chapter_index, block_index, start_token_index, end_token_index, start_offset, end_offset)
);

CREATE INDEX IF NOT EXISTS idx_books_title ON books(title);
CREATE INDEX IF NOT EXISTS idx_book_chapters_book ON book_chapters(book_id, chapter_index);
CREATE INDEX IF NOT EXISTS idx_chapter_blocks_book ON chapter_blocks(book_id, chapter_index, block_index);
CREATE INDEX IF NOT EXISTS idx_reading_progress_last_read ON reading_progress(last_read_at DESC);
CREATE INDEX IF NOT EXISTS idx_audio_paragraphs_part ON audio_paragraphs(book_id, chapter_index, part_index, voice);
CREATE INDEX IF NOT EXISTS idx_book_word_frequencies_book ON book_word_frequencies(book_id);
CREATE INDEX IF NOT EXISTS idx_wordlist_entries_book ON wordlist_entries(book_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_wordlist_entries_root ON wordlist_entries(root_word);
CREATE INDEX IF NOT EXISTS idx_reader_highlights_book_chapter ON reader_highlights(book_id, chapter_index, block_index);
"#;

pub enum ImportOutcome {
    Imported,
    Skipped,
}

const WORD_FREQUENCY_ALGORITHM_VERSION: i64 = 6;

#[derive(Debug)]
pub struct AudioParagraphSource {
    pub block_index: i64,
    pub text: String,
}

#[derive(Debug)]
pub struct GeneratedAudioParagraph {
    pub block_index: i64,
    pub text_hash: String,
    pub audio_path: String,
    pub duration_seconds: f64,
}

#[derive(Debug)]
pub struct GeneratedPartAudio {
    pub book_id: i64,
    pub chapter_index: i64,
    pub part_index: i64,
    pub voice: String,
    pub audio_path: String,
    pub duration_seconds: f64,
    pub paragraphs: Vec<GeneratedAudioParagraph>,
}

pub fn connect(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let connection = Connection::open(path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.execute_batch(SCHEMA)?;
    migrate_reading_progress(&connection)?;
    migrate_word_frequency_cache(&connection)?;
    migrate_wordlist_entries(&connection)?;
    Ok(connection)
}

fn migrate_reading_progress(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(reading_progress)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let has_column = |name: &str| columns.iter().any(|column| column == name);

    let migrations = [
        (
            "last_part_index",
            "ALTER TABLE reading_progress ADD COLUMN last_part_index INTEGER NOT NULL DEFAULT 0",
        ),
        (
            "last_scroll_ratio",
            "ALTER TABLE reading_progress ADD COLUMN last_scroll_ratio REAL NOT NULL DEFAULT 0",
        ),
        (
            "last_audio_time_seconds",
            "ALTER TABLE reading_progress ADD COLUMN last_audio_time_seconds REAL",
        ),
        (
            "last_audio_duration_seconds",
            "ALTER TABLE reading_progress ADD COLUMN last_audio_duration_seconds REAL",
        ),
        (
            "last_playing_block_index",
            "ALTER TABLE reading_progress ADD COLUMN last_playing_block_index INTEGER",
        ),
        (
            "last_playing_token_index",
            "ALTER TABLE reading_progress ADD COLUMN last_playing_token_index INTEGER",
        ),
    ];

    for (column, sql) in migrations {
        if !has_column(column) {
            connection.execute(sql, [])?;
        }
    }

    Ok(())
}

fn migrate_word_frequency_cache(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(book_word_frequency_cache)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|column| column == "algorithm_version") {
        connection.execute(
            "ALTER TABLE book_word_frequency_cache ADD COLUMN algorithm_version INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    Ok(())
}

fn migrate_wordlist_entries(connection: &Connection) -> Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(wordlist_entries)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let has_column = |name: &str| columns.iter().any(|column| column == name);

    let migrations = [
        (
            "simple_meaning",
            "ALTER TABLE wordlist_entries ADD COLUMN simple_meaning TEXT NOT NULL DEFAULT ''",
        ),
        (
            "in_context_meaning",
            "ALTER TABLE wordlist_entries ADD COLUMN in_context_meaning TEXT NOT NULL DEFAULT ''",
        ),
        (
            "original_meaning",
            "ALTER TABLE wordlist_entries ADD COLUMN original_meaning TEXT NOT NULL DEFAULT ''",
        ),
        (
            "ai_explanation",
            "ALTER TABLE wordlist_entries ADD COLUMN ai_explanation TEXT NOT NULL DEFAULT ''",
        ),
    ];

    for (column, sql) in migrations {
        if !has_column(column) {
            connection.execute(sql, [])?;
        }
    }

    Ok(())
}

include!("library.rs");
include!("reader.rs");
include!("wordlist.rs");
include!("highlights.rs");
include!("audio.rs");
include!("progress.rs");
include!("reader_structure.rs");
include!("common.rs");
include!("tests.rs");
