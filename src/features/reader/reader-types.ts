import type { ReadingBookmark, ReadingProgress, TimedToken } from "@/types";

export type AudioGenerationProgress = {
  book_id: number;
  chapter_index: number;
  part_index: number;
  completed: number;
  total: number;
  percent: number;
  stage: string;
};

export type AudioQueueMode = "chapter" | "from-chapter";

export type AudioQueueItem = {
  chapterIndex: number;
  chapterTitle: string;
  partIndex: number;
  partTitle: string;
  paragraphCount: number;
  effort: number;
};

export type AudioQueueState = {
  mode: AudioQueueMode;
  items: AudioQueueItem[];
  currentIndex: number;
  completedParts: number;
  completedEffort: number;
  totalEffort: number;
  startedAt: number;
};

export type ReaderImage = {
  src: string;
  alt: string;
};

export type PendingRestore =
  | { kind: "progress"; progress: ReadingProgress }
  | { kind: "bookmark"; bookmark: ReadingBookmark };

export type ColorMode = "frequency" | "cefr";

export type SaveProgressOptions = {
  immediate?: boolean;
  blockIndex?: number;
  audioTimeSeconds?: number | null;
  audioDurationSeconds?: number | null;
  lastPlayingToken?: TimedToken | null;
};

export type ActiveSearchResult = {
  blockIndex: number;
  query: string;
};

export type ChapterFindMatch = {
  blockIndex: number;
  start: number;
  end: number;
};

export type ChapterFindRange = {
  start: number;
  end: number;
  active: boolean;
};

export type MarkedWordLocation = {
  key: string;
  word: string;
  blockIndex: number;
  tokenIndex: number;
  ratio: number;
};

export type WordContextMenuState = {
  word: string;
  rootWord: string;
  context: string;
  cefrLevel: string;
  chapterIndex: number;
  blockIndex: number;
  tokenIndex: number;
  lookupX: number;
  lookupY: number;
  x: number;
  y: number;
};

export type HighlightRangeInput = {
  chapterIndex: number;
  blockIndex: number;
  startTokenIndex: number;
  endTokenIndex: number;
  startOffset: number;
  endOffset: number;
  text: string;
};

export type SelectionContextMenuState = HighlightRangeInput & {
  x: number;
  y: number;
};

export type ChapterContextMenuState = {
  chapterIndex: number;
  title: string;
  x: number;
  y: number;
};

export type LookupDialogState = {
  word: string;
  x: number;
  y: number;
  loading: boolean;
  error: string | null;
  result: import("@/types").DictionaryLookup | null;
};
