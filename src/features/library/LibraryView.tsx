import { convertFileSrc } from "@tauri-apps/api/core";
import { AudioLinesIcon, BookMarkedIcon, BookOpenTextIcon, ImportIcon, LibraryIcon, SettingsIcon, XIcon } from "lucide-react";
import { useEffect, useState, type MouseEvent as ReactMouseEvent } from "react";

import type { BookSummary } from "@/types";
import type { BookAudioQueueStatus } from "@/App";
import { Button } from "@/components/ui/button";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { ThemeModeControl } from "@/components/ThemeModeControl";

export function LibraryView({
  books,
  loading,
  importing,
  onImport,
  onOpenWordlist,
  onOpenSettings,
  onOpenBook,
  audioQueueStatus,
  onQueueAudio,
  onCancelAudioQueue,
}: {
  books: BookSummary[];
  loading: boolean;
  importing: boolean;
  onImport: () => void;
  onOpenWordlist: () => void;
  onOpenSettings: () => void;
  onOpenBook: (book: BookSummary) => void;
  audioQueueStatus: BookAudioQueueStatus | null;
  onQueueAudio: (book: BookSummary) => void;
  onCancelAudioQueue: () => void;
}) {
  const [contextMenu, setContextMenu] = useState<{ book: BookSummary; x: number; y: number } | null>(null);

  useEffect(() => {
    const dismiss = () => setContextMenu(null);
    window.addEventListener("mousedown", dismiss);
    window.addEventListener("blur", dismiss);
    return () => {
      window.removeEventListener("mousedown", dismiss);
      window.removeEventListener("blur", dismiss);
    };
  }, []);

  useEffect(() => {
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setContextMenu(null);
      }
    };
    window.addEventListener("keydown", dismissOnEscape);
    return () => window.removeEventListener("keydown", dismissOnEscape);
  }, []);

  const openContextMenu = (event: ReactMouseEvent, book: BookSummary) => {
    event.preventDefault();
    const width = 188;
    const height = 48;
    setContextMenu({
      book,
      x: Math.min(event.clientX, window.innerWidth - width - 12),
      y: Math.min(event.clientY, window.innerHeight - height - 12),
    });
  };

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
            <ThemeModeControl />
            <Button variant="ghost" size="icon" onClick={onOpenSettings} aria-label="Open settings" title="Settings">
              <SettingsIcon />
            </Button>
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
        {audioQueueStatus ? (
          <div className="audio-queue-status" aria-live="polite">
            <AudioLinesIcon aria-hidden="true" />
            <div className="min-w-0 flex-1">
              <p>{audioQueueStatus.cancelling ? "Stopping audio queue" : "Generating book audio"}</p>
              <span className="truncate">
                {audioQueueStatus.activeBookTitle}
                {audioQueueStatus.totalParts
                  ? ` · ${audioQueueStatus.completedParts}/${audioQueueStatus.totalParts} parts`
                  : " · Preparing parts"}
              </span>
            </div>
            {audioQueueStatus.queuedBookCount ? (
              <span className="audio-queue-pending">+{audioQueueStatus.queuedBookCount} queued</span>
            ) : null}
            <Button
              variant="ghost"
              size="icon"
              onClick={onCancelAudioQueue}
              disabled={audioQueueStatus.cancelling}
              aria-label="Cancel audio queue"
              title="Cancel audio queue"
            >
              <XIcon />
            </Button>
          </div>
        ) : null}
        {loading ? (
          <LibrarySkeleton />
        ) : books.length ? (
          <div className="library-grid">
            {books.map((book) => (
              <BookTile
                key={book.id}
                book={book}
                audioQueueStatus={audioQueueStatus}
                onOpen={() => onOpenBook(book)}
                onContextMenu={(event) => openContextMenu(event, book)}
              />
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
      {contextMenu ? (
        <div
          className="library-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => {
              onQueueAudio(contextMenu.book);
              setContextMenu(null);
            }}
          >
            <AudioLinesIcon aria-hidden="true" />
            Generate missing audio
          </button>
        </div>
      ) : null}
    </main>
  );
}

function BookTile({
  book,
  audioQueueStatus,
  onOpen,
  onContextMenu,
}: {
  book: BookSummary;
  audioQueueStatus: BookAudioQueueStatus | null;
  onOpen: () => void;
  onContextMenu: (event: ReactMouseEvent) => void;
}) {
  const coverSrc = book.cover_asset_path ? convertFileSrc(book.cover_asset_path) : null;
  const progress = Math.round(book.progress_percent);
  const isGenerating = audioQueueStatus?.activeBookId === book.id;
  const liveAudioPercent = isGenerating && book.audio_total_parts > 0 && audioQueueStatus.totalParts > 0
    ? ((book.audio_generated_parts + audioQueueStatus.completedParts + audioQueueStatus.currentPartPercent / 100) / book.audio_total_parts) * 100
    : book.audio_percent;
  const audioPercent = Math.max(0, Math.min(100, liveAudioPercent));
  return (
    <article className="book-tile" onContextMenu={onContextMenu}>
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
      <div className="book-metrics">
        <div className="book-progress" aria-label={`${progress}% read`}>
          <ProgressRing percent={book.progress_percent} />
          <span>Read {progress}%</span>
        </div>
        <div className="book-progress book-audio-progress" aria-label={audioLabel(book, audioPercent, isGenerating)}>
          <ProgressRing percent={audioPercent} tone="audio" />
          <span>{isGenerating ? "Generating" : audioLabel(book, audioPercent, false)}</span>
        </div>
      </div>
    </article>
  );
}

function ProgressRing({ percent, tone }: { percent: number; tone?: "audio" }) {
  const radius = 18;
  const circumference = 2 * Math.PI * radius;
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const offset = circumference * (1 - clampedPercent / 100);

  return (
    <svg className={`progress-ring${tone ? ` progress-ring-${tone}` : ""}`} viewBox="0 0 48 48" aria-hidden="true">
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

function audioLabel(book: BookSummary, percent: number, isGenerating: boolean) {
  if (isGenerating) {
    return "Generating audio";
  }
  if (!book.audio_total_parts || percent <= 0) {
    return "No audio";
  }
  if (percent >= 100) {
    return "Ready";
  }
  return `Partial ${Math.round(percent)}%`;
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
