import type { ChapterPayload, ReaderPayload, ReadingProgress, TimedToken } from "@/types";
import type { ChapterFindMatch, ColorMode } from "./reader-types";

function initials(title: string) {
  return title
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase())
    .join("");
}

export function blockDomId(blockIndex: number) {
  return `reader-block-${blockIndex}`;
}

export function formatChapterTitle(title: string) {
  return title.replace(/^(\d+)\s+/, "$1 ");
}

export function countWords(text: string) {
  return text.match(/[\p{L}\p{N}]+(?:['’.-][\p{L}\p{N}]+)*/gu)?.length ?? 0;
}

export function chapterPartEffort(chapter: ChapterPayload, startBlockIndex: number, endBlockIndex: number) {
  let characters = 0;
  let paragraphCount = 0;
  for (const block of chapter.blocks) {
    if (block.kind !== "paragraph" || block.block_index < startBlockIndex || block.block_index > endBlockIndex) {
      continue;
    }
    characters += block.text.trim().length;
    paragraphCount += 1;
  }
  return {
    characters: Math.max(1, characters),
    paragraphCount,
  };
}

export function formatClock(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "0:00";
  }
  const whole = Math.floor(seconds);
  const minutes = Math.floor(whole / 60);
  const remainingSeconds = whole % 60;
  return `${minutes}:${String(remainingSeconds).padStart(2, "0")}`;
}

export function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "";
  }
  const rounded = Math.ceil(seconds);
  if (rounded < 60) {
    return `${rounded}s`;
  }
  const minutes = Math.floor(rounded / 60);
  const remainingSeconds = rounded % 60;
  if (minutes < 60) {
    return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
}

export function clampNumber(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.max(min, Math.min(max, value));
}

export function findLastIndex<T>(items: T[], predicate: (item: T) => boolean) {
  for (let index = items.length - 1; index >= 0; index -= 1) {
    if (predicate(items[index])) {
      return index;
    }
  }
  return -1;
}

export function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
}

export function currentScrollRatio() {
  const maxScroll = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
  if (maxScroll <= 0) {
    return 0;
  }
  return clampNumber(window.scrollY / maxScroll, 0, 1);
}

export function scheduleScrollRestore(callback: () => void) {
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(callback);
  });
}

export function restoreScrollPosition(progress: ReadingProgress, chapter: ChapterPayload) {
  scheduleScrollRestore(() => {
    const maxScroll = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
    if (maxScroll > 0 && progress.last_read_at) {
      window.scrollTo({ top: clampNumber(progress.last_scroll_ratio, 0, 1) * maxScroll });
      return;
    }

    const target = chapter.blocks.find((block) => block.block_index >= progress.last_block_index);
    if (target) {
      document.getElementById(blockDomId(target.block_index))?.scrollIntoView({ block: "center" });
    } else {
      window.scrollTo({ top: 0 });
    }
  });
}

export function readingProgressPercent(reader: ReaderPayload, chapter: ChapterPayload | null, blockIndex: number) {
  if (reader.total_progress_units <= 0) {
    return reader.progress.progress_percent;
  }

  const progressChapter = reader.chapters.find(
    (item) => blockIndex >= item.start_block_index && blockIndex <= item.end_block_index,
  );
  if (!progressChapter) {
    return reader.progress.progress_percent;
  }

  let currentUnits = progressChapter.progress_start_unit;
  if (progressChapter.contributes_to_progress && chapter?.chapter_index === progressChapter.chapter_index) {
    currentUnits += chapter.blocks
      .filter((block) => block.kind === "paragraph" && block.block_index < blockIndex)
      .reduce((total, block) => total + block.text.length, 0);
  }

  const percent = (currentUnits / reader.total_progress_units) * 100;
  return Math.min(100, Math.max(0, Math.round(percent * 10) / 10));
}

export function findSavedPlayingToken(tokens: TimedToken[], progress: ReadingProgress) {
  if (progress.last_playing_block_index === null || progress.last_playing_token_index === null) {
    return null;
  }
  return (
    tokens.find(
      (token) =>
        token.block_index === progress.last_playing_block_index &&
        token.token_index === progress.last_playing_token_index,
    ) ?? null
  );
}

export function timedTokenAtTime(tokens: TimedToken[], time: number) {
  return (
    tokens.find((token) => time >= token.start_time && time < token.end_time) ??
    [...tokens].reverse().find((token) => token.start_time <= time) ??
    null
  );
}

export function caseInsensitiveTextRange(text: string, query: string) {
  const needle = query.trim().toLocaleLowerCase();
  if (needle.length < 2) {
    return null;
  }
  const start = text.toLocaleLowerCase().indexOf(needle);
  return start === -1 ? null : { start, end: start + needle.length };
}

export function findChapterMatches(chapter: ChapterPayload | null, query: string) {
  const needle = query.trim().toLocaleLowerCase();
  if (!chapter || !needle) {
    return [];
  }
  const matches: ChapterFindMatch[] = [];
  for (const block of chapter.blocks) {
    if (block.kind !== "paragraph") {
      continue;
    }
    const haystack = block.text.toLocaleLowerCase();
    let start = haystack.indexOf(needle);
    while (start !== -1) {
      matches.push({
        blockIndex: block.block_index,
        start,
        end: start + needle.length,
      });
      start = haystack.indexOf(needle, start + needle.length);
    }
  }
  return matches;
}

export function closestChapterFindMatchIndex(matches: ChapterFindMatch[], fallbackBlockIndex: number | null) {
  if (!matches.length) {
    return 0;
  }
  const viewportTop = 0;
  const viewportBottom = window.innerHeight;
  const viewportCenter = viewportBottom / 2;
  let bestDomIndex = -1;
  let bestDomDistance = Number.POSITIVE_INFINITY;
  let bestFallbackIndex = 0;
  let bestFallbackDistance = Number.POSITIVE_INFINITY;

  matches.forEach((match, index) => {
    const element = document.getElementById(blockDomId(match.blockIndex));
    if (element) {
      const bounds = element.getBoundingClientRect();
      const distance =
        bounds.bottom < viewportTop
          ? viewportTop - bounds.bottom
          : bounds.top > viewportBottom
            ? bounds.top - viewportBottom
            : Math.abs((bounds.top + bounds.bottom) / 2 - viewportCenter);
      if (distance < bestDomDistance) {
        bestDomDistance = distance;
        bestDomIndex = index;
      }
      return;
    }

    if (fallbackBlockIndex !== null) {
      const distance = Math.abs(match.blockIndex - fallbackBlockIndex);
      if (distance < bestFallbackDistance) {
        bestFallbackDistance = distance;
        bestFallbackIndex = index;
      }
    }
  });

  return bestDomIndex === -1 ? bestFallbackIndex : bestDomIndex;
}

export function timedTokenKey(blockIndex: number, tokenIndex: number) {
  return `${blockIndex}:${tokenIndex}`;
}

export function colorModeStorageKey() {
  return "readalong:color-mode";
}

export function storedColorMode(): ColorMode {
  const stored = window.localStorage.getItem(colorModeStorageKey());
  return stored === "cefr" ? "cefr" : "frequency";
}

export function shouldIgnorePlaybackShortcut(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}
