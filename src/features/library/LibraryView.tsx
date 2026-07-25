import { convertFileSrc } from "@tauri-apps/api/core";
import { BookMarkedIcon, BookOpenTextIcon, ImportIcon, LibraryIcon } from "lucide-react";

import type { BookSummary } from "@/types";
import { Button } from "@/components/ui/button";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";

export function LibraryView({
  books,
  loading,
  importing,
  onImport,
  onOpenWordlist,
  onOpenBook,
}: {
  books: BookSummary[];
  loading: boolean;
  importing: boolean;
  onImport: () => void;
  onOpenWordlist: () => void;
  onOpenBook: (book: BookSummary) => void;
}) {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-16 w-full max-w-7xl items-center justify-between px-6">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
              <BookOpenTextIcon aria-hidden="true" />
            </div>
            <div>
              <h1 className="text-base font-semibold">Readalong</h1>
              <p className="text-xs text-muted-foreground">{books.length ? `${books.length} books` : "Local reader"}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="secondary" onClick={onOpenWordlist}>
              <BookMarkedIcon data-icon="inline-start" />
              Word list
            </Button>
            <Button onClick={onImport} disabled={importing}>
              <ImportIcon data-icon="inline-start" />
              {importing ? "Importing" : "Import"}
            </Button>
          </div>
        </div>
      </header>

      <section className="mx-auto flex w-full max-w-7xl flex-col gap-8 px-6 py-8">
        {loading ? (
          <LibrarySkeleton />
        ) : books.length ? (
          <div className="library-grid">
            {books.map((book) => (
              <BookTile key={book.id} book={book} onOpen={() => onOpenBook(book)} />
            ))}
          </div>
        ) : (
          <Empty className="rounded-md border border-dashed bg-muted/20">
            <EmptyHeader>
              <EmptyTitle>Your shelf is empty</EmptyTitle>
              <EmptyDescription>Import an EPUB and start reading locally without audio setup.</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button onClick={onImport} disabled={importing}>
                <ImportIcon data-icon="inline-start" />
                {importing ? "Importing" : "Import EPUB"}
              </Button>
            </EmptyContent>
          </Empty>
        )}
      </section>
    </main>
  );
}

function BookTile({ book, onOpen }: { book: BookSummary; onOpen: () => void }) {
  const coverSrc = book.cover_asset_path ? convertFileSrc(book.cover_asset_path) : null;
  const progress = Math.round(book.progress_percent);
  return (
    <article className="book-tile">
      <button className="book-open" type="button" onClick={onOpen} aria-label={`Open ${book.title}`}>
        <div className="cover-frame">
          {coverSrc ? (
            <img src={coverSrc} alt="" className="cover-image" />
          ) : (
            <div className="cover-fallback">
              <LibraryIcon aria-hidden="true" />
              <span>{initials(book.title)}</span>
            </div>
          )}
        </div>
        <span className="cover-title" aria-hidden="true">
          {book.title}
        </span>
      </button>
      <div className="book-progress" aria-label={`${progress}% read`}>
        <ProgressRing percent={book.progress_percent} />
        <span>{progress}%</span>
      </div>
    </article>
  );
}

function ProgressRing({ percent }: { percent: number }) {
  const radius = 18;
  const circumference = 2 * Math.PI * radius;
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const offset = circumference * (1 - clampedPercent / 100);

  return (
    <svg className="progress-ring" viewBox="0 0 48 48" aria-hidden="true">
      <circle className="progress-ring-track" cx="24" cy="24" r={radius} />
      <circle
        className="progress-ring-fill"
        cx="24"
        cy="24"
        r={radius}
        style={{ strokeDasharray: circumference, strokeDashoffset: offset }}
      />
    </svg>
  );
}

function LibrarySkeleton() {
  return (
    <div className="library-grid">
      {Array.from({ length: 6 }).map((_, index) => (
        <div key={index} className="book-tile">
          <Skeleton className="shelf-skeleton-cover" />
          <Skeleton className="shelf-skeleton-progress" />
        </div>
      ))}
    </div>
  );
}


function initials(title: string) {
  return title
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase())
    .join("");
}
