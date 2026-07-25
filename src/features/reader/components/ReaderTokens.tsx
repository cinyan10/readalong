import type { ChapterPayload, TimedToken } from "@/types";
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
  timedTokensByKey,
  onSeekToken,
  onSeekRelativeToken,
  onOpenWordContextMenu,
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
  timedTokensByKey: Map<string, TimedToken>;
  onSeekToken: (blockIndex: number, tokenIndex: number) => void;
  onSeekRelativeToken: (blockIndex: number, tokenIndex: number) => void;
  onOpenWordContextMenu: (
    token: ChapterPayload["blocks"][number]["tokens"][number],
    blockText: string,
    blockIndex: number,
    tokenIndex: number,
    target: HTMLElement,
    clientX: number,
    clientY: number,
  ) => void;
  onTokenRef: (tokenKey: string, node: HTMLElement | null) => void;
}) {
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
              if (hasTiming) {
                onSeekToken(block.block_index, index);
              } else if (token.normalized_text) {
                onSeekRelativeToken(block.block_index, index);
              }
            }}
            onContextMenu={(event) => {
              event.preventDefault();
              event.stopPropagation();
              if (token.normalized_text) {
                onOpenWordContextMenu(token, block.text, block.block_index, index, event.currentTarget, event.clientX, event.clientY);
              }
            }}
          >
            {token.text}
          </span>
        );
      })}
    </>
  );
}
