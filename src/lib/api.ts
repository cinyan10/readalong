import { invoke } from "@tauri-apps/api/core";

import type {
  BookSearchResult,
  BookSummary,
  ChapterPayload,
  DictionaryLookup,
  ImportSummary,
  PartAlignmentPayload,
  PartAudioPayload,
  ReaderHighlight,
  ReaderPayload,
  ReadingBookmark,
  WordlistEntry,
} from "@/types";

export function listBooks() {
  return invoke<BookSummary[]>("list_books");
}

export function importBooks(paths: string[]) {
  return invoke<ImportSummary>("import_books", { paths });
}

export function getReader(bookId: number) {
  return invoke<ReaderPayload>("get_reader", { bookId });
}

export type SaveBookmarkInput = {
  bookId: number;
  chapterIndex: number;
  partIndex: number;
  blockIndex: number;
  tokenIndex: number;
  word: string;
  rootWord: string;
  scrollRatio: number;
  progressPercent: number;
};

export function saveBookmark(input: SaveBookmarkInput) {
  return invoke<ReadingBookmark>("save_bookmark", {
    bookId: input.bookId,
    chapterIndex: input.chapterIndex,
    partIndex: input.partIndex,
    blockIndex: input.blockIndex,
    tokenIndex: input.tokenIndex,
    word: input.word,
    rootWord: input.rootWord,
    scrollRatio: input.scrollRatio,
    progressPercent: input.progressPercent,
  });
}

export function getChapter(bookId: number, chapterIndex: number) {
  return invoke<ChapterPayload>("get_chapter", { bookId, chapterIndex });
}

export function searchBook(bookId: number, query: string) {
  return invoke<BookSearchResult[]>("search_book", { bookId, query });
}

export function lookupWord(word: string, context: string, cefrLevel: string, rootWord: string) {
  return invoke<DictionaryLookup>("lookup_word", {
    word,
    context,
    cefrLevel,
    rootWord,
  });
}

export function listWordlistEntries() {
  return invoke<WordlistEntry[]>("list_wordlist_entries");
}

export function listBookWordlistEntries(bookId: number) {
  return invoke<WordlistEntry[]>("list_book_wordlist_entries", { bookId });
}

export type AddWordlistEntryInput = {
  bookId: number;
  chapterIndex: number;
  blockIndex: number;
  tokenIndex: number;
  word: string;
  rootWord: string;
  context: string;
  cefrLevel: string;
};

export function addWordlistEntry(input: AddWordlistEntryInput) {
  return invoke<WordlistEntry>("add_wordlist_entry", {
    bookId: input.bookId,
    chapterIndex: input.chapterIndex,
    blockIndex: input.blockIndex,
    tokenIndex: input.tokenIndex,
    word: input.word,
    rootWord: input.rootWord,
    context: input.context,
    cefrLevel: input.cefrLevel,
  });
}

export function deleteWordlistEntry(rootWord: string) {
  return invoke<boolean>("delete_wordlist_entry", { rootWord });
}

export function listBookHighlights(bookId: number) {
  return invoke<ReaderHighlight[]>("list_book_highlights", { bookId });
}

export type ToggleHighlightInput = {
  bookId: number;
  chapterIndex: number;
  blockIndex: number;
  startTokenIndex: number;
  endTokenIndex: number;
  startOffset: number;
  endOffset: number;
  text: string;
};

export function toggleHighlight(input: ToggleHighlightInput) {
  return invoke<ReaderHighlight | null>("toggle_highlight", {
    bookId: input.bookId,
    chapterIndex: input.chapterIndex,
    blockIndex: input.blockIndex,
    startTokenIndex: input.startTokenIndex,
    endTokenIndex: input.endTokenIndex,
    startOffset: input.startOffset,
    endOffset: input.endOffset,
    text: input.text,
  });
}

export function getPartAudio(bookId: number, chapterIndex: number, partIndex: number) {
  return invoke<PartAudioPayload | null>("get_part_audio", { bookId, chapterIndex, partIndex });
}

export function getPartAlignment(bookId: number, chapterIndex: number, partIndex: number) {
  return invoke<PartAlignmentPayload | null>("get_part_alignment", { bookId, chapterIndex, partIndex });
}

export function generatePartAudio(bookId: number, chapterIndex: number, partIndex: number, regenerate: boolean) {
  return invoke<PartAudioPayload>("generate_part_audio", {
    bookId,
    chapterIndex,
    partIndex,
    regenerate,
  });
}

export function syncPartAlignment(bookId: number, chapterIndex: number, partIndex: number, regenerate: boolean) {
  return invoke<PartAlignmentPayload>("sync_part_alignment", {
    bookId,
    chapterIndex,
    partIndex,
    regenerate,
  });
}

export type SaveProgressInput = {
  bookId: number;
  chapterIndex: number;
  partIndex: number;
  blockIndex: number;
  scrollRatio: number;
  progressPercent: number;
  audioTimeSeconds?: number | null;
  audioDurationSeconds?: number | null;
  lastPlayingBlockIndex?: number | null;
  lastPlayingTokenIndex?: number | null;
};

export function saveProgress(input: SaveProgressInput) {
  return invoke<void>("save_progress", {
    bookId: input.bookId,
    chapterIndex: input.chapterIndex,
    partIndex: input.partIndex,
    blockIndex: input.blockIndex,
    scrollRatio: input.scrollRatio,
    progressPercent: input.progressPercent,
    audioTimeSeconds: input.audioTimeSeconds ?? null,
    audioDurationSeconds: input.audioDurationSeconds ?? null,
    lastPlayingBlockIndex: input.lastPlayingBlockIndex ?? null,
    lastPlayingTokenIndex: input.lastPlayingTokenIndex ?? null,
  });
}
