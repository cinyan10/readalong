import { useEffect, useRef, type ReactNode } from "react";
import type { ChapterPayload, ReaderHighlight, TimedToken } from "@/types";
import { cn } from "@/lib/utils";
import type { ActiveSearchResult, ChapterFindRange, ColorMode } from "../reader-types";
import { caseInsensitiveTextRange, timedTokenKey } from "../reader-utils";
import { wordlistTokenKey } from "@/features/wordlist/wordlist-utils";

export function ReaderTokens({
  bookId,
  block,
  chapterIndex,
  colorMode,
  activeTokenKey,
  bookmarkedTokenKey,
  activeSearchResult,
  chapterFindRanges,
  wordlistRoots,
  wordlistExactKeys,
  highlights,
  timedTokensByKey,
  onPlayToken,
  onPreviewToken,
  onOpenWordContextMenu,
  onTokenAuxAction,
  onTokenRef,
}: {
  bookId: number;
  block: ChapterPayload["blocks"][number];
  chapterIndex: number;
  colorMode: ColorMode;
  activeTokenKey: string | null;
  bookmarkedTokenKey: string | null;
  activeSearchResult: ActiveSearchResult | null;
  chapterFindRanges: ChapterFindRange[];
  wordlistRoots: Set<string>;
  wordlistExactKeys: Set<string>;
  highlights: ReaderHighlight[];
  timedTokensByKey: Map<string, TimedToken>;
  onPlayToken: (blockIndex: number, tokenIndex: number) => void;
  onPreviewToken: (blockIndex: number, tokenIndex: number) => void;
  onOpenWordContextMenu: (
    token: ChapterPayload["blocks"][number]["tokens"][number],
    blockText: string,
    blockIndex: number,
    tokenIndex: number,
    target: HTMLElement,
    clientX: number,
    clientY: number,
  ) => void;
  onTokenAuxAction: (
    action: "highlight" | "wordlist",
    token: ChapterPayload["blocks"][number]["tokens"][number],
    blockText: string,
    blockIndex: number,
    tokenIndex: number,
  ) => void;
  onTokenRef: (tokenKey: string, node: HTMLElement | null) => void;
}) {
  const singleClickTimerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (singleClickTimerRef.current !== null) {
        window.clearTimeout(singleClickTimerRef.current);
      }
    };
  }, []);

  if (!block.tokens.length) {
    return <>{block.text}</>;
  }
  const searchRange =
    activeSearchResult && activeSearchResult.blockIndex === block.block_index
      ? caseInsensitiveTextRange(block.text, activeSearchResult.query)
      : null;
  let tokenOffset = 0;
  return (
    <>
      {block.tokens.map((token, index) => {
        const exactKey = wordlistTokenKey(bookId, chapterIndex, block.block_index, index);
        const syncKey = timedTokenKey(block.block_index, index);
        const hasTiming = timedTokensByKey.has(syncKey);
        const colorLevel = colorMode === "frequency" ? token.frequency_level : token.cefr_level;
        const rootWord = token.root_text || token.normalized_text;
        const isWordlistExact = wordlistExactKeys.has(exactKey);
        const isWordlistRoot = Boolean(rootWord && wordlistRoots.has(rootWord) && !isWordlistExact);
        const isBookmarked = bookmarkedTokenKey === syncKey;
        const tokenStart = tokenOffset;
        const tokenEnd = tokenStart + token.text.length;
        const isBookSearchHit = Boolean(searchRange && tokenEnd > searchRange.start && tokenStart < searchRange.end);
        const isChapterFindHit = chapterFindRanges.some((range) => tokenEnd > range.start && tokenStart < range.end);
        const isActiveChapterFindHit = chapterFindRanges.some(
          (range) => range.active && tokenEnd > range.start && tokenStart < range.end,
        );
        tokenOffset = tokenEnd;
        return (
          <span
            key={`${block.block_index}-${index}`}
            ref={(node) => onTokenRef(syncKey, node)}
            className={cn(
              "reader-token",
              colorLevel && `level-${colorLevel.toLowerCase()}`,
              token.normalized_text && "clickable",
              isWordlistRoot && "wordlisted-root",
              isWordlistExact && "marked",
              isBookmarked && "bookmarked",
              hasTiming && "synced",
              activeTokenKey === syncKey && "active",
              (isBookSearchHit || isChapterFindHit) && "search-hit",
              isActiveChapterFindHit && "find-hit-active",
            )}
            data-cefr-level={token.cefr_level || undefined}
            data-frequency-level={token.frequency_level || undefined}
            data-frequency-count={token.frequency_count || undefined}
            data-root-text={token.root_text || undefined}
            data-block-index={block.block_index}
            data-token-index={index}
            data-timed-token-key={syncKey}
            onClick={() => {
              if (!token.normalized_text) {
                return;
              }
              if (singleClickTimerRef.current !== null) {
                window.clearTimeout(singleClickTimerRef.current);
              }
              singleClickTimerRef.current = window.setTimeout(() => {
                singleClickTimerRef.current = null;
                onPlayToken(block.block_index, index);
              }, 220);
            }}
            onDoubleClick={() => {
              if (singleClickTimerRef.current !== null) {
                window.clearTimeout(singleClickTimerRef.current);
                singleClickTimerRef.current = null;
              }
              if (token.normalized_text) {
                onPreviewToken(block.block_index, index);
              }
            }}
            onMouseUp={(event) => {
              if (event.button !== 3 && event.button !== 4) {
                return;
              }
              if (event.button === 3 && !token.normalized_text) {
                return;
              }
              event.preventDefault();
              event.stopPropagation();
              onTokenAuxAction(event.button === 4 ? "highlight" : "wordlist", token, block.text, block.block_index, index);
            }}
            onContextMenu={(event) => {
              event.preventDefault();
              event.stopPropagation();
              if (token.normalized_text) {
                onOpenWordContextMenu(token, block.text, block.block_index, index, event.currentTarget, event.clientX, event.clientY);
              }
            }}
          >
            {renderHighlightedTokenText(token.text, index, highlights)}
          </span>
        );
      })}
    </>
  );
}

function renderHighlightedTokenText(text: string, tokenIndex: number, highlights: ReaderHighlight[]) {
  const ranges = highlights
    .filter((highlight) => tokenIndex >= highlight.start_token_index && tokenIndex <= highlight.end_token_index)
    .map((highlight) => ({
      start: tokenIndex === highlight.start_token_index ? highlight.start_offset : 0,
      end: tokenIndex === highlight.end_token_index ? highlight.end_offset : text.length,
    }))
    .map((range) => ({
      start: Math.max(0, Math.min(text.length, range.start)),
      end: Math.max(0, Math.min(text.length, range.end)),
    }))
    .filter((range) => range.end > range.start)
    .sort((left, right) => left.start - right.start || left.end - right.end);

  if (!ranges.length) {
    return text;
  }

  const merged: { start: number; end: number }[] = [];
  for (const range of ranges) {
    const last = merged.at(-1);
    if (last && range.start <= last.end) {
      last.end = Math.max(last.end, range.end);
    } else {
      merged.push({ ...range });
    }
  }

  const parts: ReactNode[] = [];
  let cursor = 0;
  merged.forEach((range, index) => {
    if (range.start > cursor) {
      parts.push(text.slice(cursor, range.start));
    }
    parts.push(
      <span key={`${range.start}-${range.end}-${index}`} className="reader-highlight">
        {text.slice(range.start, range.end)}
      </span>,
    );
    cursor = range.end;
  });
  if (cursor < text.length) {
    parts.push(text.slice(cursor));
  }
  return parts;
}
