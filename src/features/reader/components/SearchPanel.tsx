import { SearchIcon, XIcon } from "lucide-react";
import type { RefObject } from "react";
import type { BookSearchResult } from "@/types";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatChapterTitle } from "../reader-utils";

export function SearchPanel({
  inputRef,
  query,
  results,
  loading,
  activeBlockIndex,
  onQueryChange,
  onSelect,
  onClose,
}: {
  inputRef: RefObject<HTMLInputElement>;
  query: string;
  results: BookSearchResult[];
  loading: boolean;
  activeBlockIndex: number | null;
  onQueryChange: (query: string) => void;
  onSelect: (result: BookSearchResult) => void;
  onClose: () => void;
}) {
  const trimmedQuery = query.trim();
  const status = loading
    ? "Searching"
    : trimmedQuery.length < 2
      ? "Type at least 2 characters"
      : `${results.length} result${results.length === 1 ? "" : "s"}`;
  return (
    <section className="search-panel" aria-label="Book search" onKeyDown={(event) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    }}>
      <div className="search-field">
        <SearchIcon aria-hidden="true" />
        <input
          ref={inputRef}
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder="Search book"
          aria-label="Search book"
        />
        {query ? (
          <Button variant="ghost" size="icon" onClick={() => onQueryChange("")} aria-label="Clear search">
            <XIcon />
          </Button>
        ) : null}
      </div>
      <div className="search-status">{status}</div>
      {trimmedQuery.length >= 2 && !loading ? (
        <div className="search-results">
          {results.length ? (
            results.map((result) => (
              <button
                key={`${result.chapter_index}-${result.block_index}`}
                className={cn("search-result", activeBlockIndex === result.block_index && "active")}
                type="button"
                onClick={() => onSelect(result)}
              >
                <span className="search-result-title">{formatChapterTitle(result.chapter_title)}</span>
                <span className="search-result-snippet">
                  <HighlightedSnippet result={result} />
                </span>
                {result.match_count > 1 ? <span className="search-result-count">{result.match_count} matches</span> : null}
              </button>
            ))
          ) : (
            <div className="search-empty">No matches</div>
          )}
        </div>
      ) : null}
    </section>
  );
}

function HighlightedSnippet({ result }: { result: BookSearchResult }) {
  const before = result.snippet.slice(0, result.match_start);
  const match = result.snippet.slice(result.match_start, result.match_end);
  const after = result.snippet.slice(result.match_end);
  return (
    <>
      {before}
      <mark>{match}</mark>
      {after}
    </>
  );
}

