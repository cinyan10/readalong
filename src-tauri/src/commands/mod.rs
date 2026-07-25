use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{Emitter, State, Window};

use crate::cefr;
use crate::db::{self, GeneratedAudioParagraph, GeneratedPartAudio, ImportOutcome};
use crate::models::{
    BookSummary, ChapterPayload, ImportFailure, ImportSummary, PartAlignmentPayload,
    PartAudioPayload, ReaderPayload, ReadingBookmark, WordlistEntry,
};
use crate::AppState;

const DEFAULT_AUDIO_VOICE: &str = "bf_emma";
const DEFAULT_AUDIO_SPEED: f64 = 0.95;
const PARAGRAPH_SILENCE_SECONDS: f64 = 0.22;
const TITLE_AUDIO_BLOCK_BASE: i64 = -1_000_000_000_000;

include!("library.rs");
include!("wordlist.rs");
include!("audio.rs");
include!("progress.rs");
include!("audio_worker.rs");
