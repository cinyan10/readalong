import { ChevronDownIcon, ChevronUpIcon, SearchIcon, XIcon } from "lucide-react";
import type { RefObject } from "react";
import { Button } from "@/components/ui/button";

export function ChapterFindBar({
  inputRef,
  query,
  activeIndex,
  matchCount,
  onQueryChange,
  onPrevious,
  onNext,
  onConfirm,
  onClose,
}: {
  inputRef: RefObject<HTMLInputElement>;
  query: string;
  activeIndex: number;
  matchCount: number;
  onQueryChange: (query: string) => void;
  onPrevious: () => void;
  onNext: () => void;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const counter = query.trim() ? `${activeIndex >= 0 ? activeIndex + 1 : 0}/${matchCount}` : "0/0";
  return (
    <section
      className="chapter-find-bar"
      aria-label="Find in current chapter"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          event.stopPropagation();
          if (event.shiftKey) {
            onPrevious();
          } else {
            onConfirm();
          }
        }
      }}
    >
      <SearchIcon aria-hidden="true" />
      <input
        ref={inputRef}
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        placeholder="Find in chapter"
        aria-label="Find in current chapter"
      />
      <span className="chapter-find-count" aria-label={`${matchCount} matches`}>
        {counter}
      </span>
      <Button variant="ghost" size="icon" onClick={onPrevious} disabled={!matchCount} aria-label="Previous match">
        <ChevronUpIcon />
      </Button>
      <Button variant="ghost" size="icon" onClick={onNext} disabled={!matchCount} aria-label="Next match">
        <ChevronDownIcon />
      </Button>
      <Button variant="ghost" size="icon" onClick={onClose} aria-label="Close find">
        <XIcon />
      </Button>
    </section>
  );
}

