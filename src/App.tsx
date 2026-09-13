import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { completeAppExit, generatePartAudio, importBooks, listBooks, listMissingBookAudioParts } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type { BookAudioPart, BookSummary } from "@/types";
import { TooltipProvider } from "@/components/ui/tooltip";
import { LibraryView } from "@/features/library/LibraryView";
import { ReaderView } from "@/features/reader/ReaderView";
import { WordlistView } from "@/features/wordlist/WordlistView";
import { SettingsView } from "@/features/settings/SettingsView";
import { useSettings } from "@/lib/settings";

type ViewState =
  | { kind: "library" }
  | { kind: "settings" }
  | { kind: "wordlist" }
  | { kind: "reader"; bookId: number; chapterIndex?: number };

export type LeaveRequestHandler = () => void;

export type BookAudioQueueStatus = {
  activeBookId: number;
  activeBookTitle: string;
  completedParts: number;
  totalParts: number;
  currentPartPercent: number;
  queuedBookCount: number;
  cancelling: boolean;
};

function App() {
  const { audioGenerationSpeed } = useSettings();
  const [view, setView] = useState<ViewState>({ kind: "library" });
  const [books, setBooks] = useState<BookSummary[]>([]);
  const [loadingBooks, setLoadingBooks] = useState(true);
  const [importing, setImporting] = useState(false);
  const [audioQueueStatus, setAudioQueueStatus] = useState<BookAudioQueueStatus | null>(null);
  const leaveRequestHandlerRef = useRef<LeaveRequestHandler | null>(null);
  const queuedAudioBooksRef = useRef<BookSummary[]>([]);
  const queuedAudioBookIdsRef = useRef(new Set<number>());
  const activeAudioBookIdRef = useRef<number | null>(null);
  const audioQueueRunningRef = useRef(false);
  const cancelAudioQueueRef = useRef(false);

  const registerLeaveRequestHandler = useCallback((handler: LeaveRequestHandler) => {
    leaveRequestHandlerRef.current = handler;
    return () => {
      if (leaveRequestHandlerRef.current === handler) {
        leaveRequestHandlerRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("app-leave-requested", () => {
      const handler = leaveRequestHandlerRef.current;
      if (handler) {
        handler();
        return;
      }
      void completeAppExit();
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<{ book_id: number; percent: number }>("part-audio-progress", (event) => {
      if (event.payload.book_id !== activeAudioBookIdRef.current) {
        return;
      }
      setAudioQueueStatus((current) =>
        current
          ? { ...current, currentPartPercent: Math.max(0, Math.min(100, event.payload.percent)) }
          : current,
      );
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const refreshLibrary = useCallback(async () => {
    setLoadingBooks(true);
    try {
      setBooks(await listBooks());
    } catch (error) {
      toast.error(errorMessage(error, "Failed to load library."));
    } finally {
      setLoadingBooks(false);
    }
  }, []);

  useEffect(() => {
    void refreshLibrary();
  }, [refreshLibrary]);

  const runAudioQueue = useCallback(async () => {
    if (audioQueueRunningRef.current) {
      return;
    }
    audioQueueRunningRef.current = true;
    try {
      while (queuedAudioBooksRef.current.length) {
        const book = queuedAudioBooksRef.current.shift();
        if (!book) {
          continue;
        }
        activeAudioBookIdRef.current = book.id;
        let parts: BookAudioPart[];
        try {
          parts = await listMissingBookAudioParts(book.id);
        } catch (error) {
          toast.error(`Unable to prepare audio for ${book.title}.`, {
            description: errorMessage(error, "Failed to find missing audio parts."),
          });
          queuedAudioBookIdsRef.current.delete(book.id);
          continue;
        }

        if (!parts.length) {
          queuedAudioBookIdsRef.current.delete(book.id);
          continue;
        }

        let completedParts = 0;
        setAudioQueueStatus({
          activeBookId: book.id,
          activeBookTitle: book.title,
          completedParts,
          totalParts: parts.length,
          currentPartPercent: 0,
          queuedBookCount: queuedAudioBooksRef.current.length,
          cancelling: false,
        });

        for (const part of parts) {
          if (cancelAudioQueueRef.current) {
            break;
          }
          setAudioQueueStatus((current) =>
            current
              ? {
                  ...current,
                  completedParts,
                  currentPartPercent: 0,
                  queuedBookCount: queuedAudioBooksRef.current.length,
                }
              : current,
          );
          try {
            await generatePartAudio(book.id, part.chapter_index, part.part_index, false, audioGenerationSpeed);
            completedParts += 1;
            setAudioQueueStatus((current) =>
              current ? { ...current, completedParts, currentPartPercent: 0 } : current,
            );
          } catch (error) {
            toast.error(`Audio generation stopped for ${book.title}.`, {
              description: errorMessage(error, "A book part could not be generated."),
            });
            break;
          }
        }

        queuedAudioBookIdsRef.current.delete(book.id);
        const latestBooks = await listBooks();
        setBooks(latestBooks);
        if (cancelAudioQueueRef.current) {
          break;
        }
      }
    } finally {
      activeAudioBookIdRef.current = null;
      audioQueueRunningRef.current = false;
      const wasCancelled = cancelAudioQueueRef.current;
      cancelAudioQueueRef.current = false;
      setAudioQueueStatus(null);
      if (wasCancelled) {
        toast.info("Audio queue cancelled after the active part.");
      }
      if (queuedAudioBooksRef.current.length) {
        void runAudioQueue();
      }
    }
  }, [audioGenerationSpeed]);

  const enqueueBookAudio = useCallback(
    (book: BookSummary) => {
      if (queuedAudioBookIdsRef.current.has(book.id)) {
        toast.info(`${book.title} is already queued for audio.`);
        return;
      }
      queuedAudioBookIdsRef.current.add(book.id);
      queuedAudioBooksRef.current.push(book);
      setAudioQueueStatus((current) =>
        current
          ? { ...current, queuedBookCount: queuedAudioBooksRef.current.length }
          : {
              activeBookId: book.id,
              activeBookTitle: book.title,
              completedParts: 0,
              totalParts: 0,
              currentPartPercent: 0,
              queuedBookCount: 0,
              cancelling: false,
            },
      );
      void runAudioQueue();
    },
    [runAudioQueue],
  );

  const cancelAudioQueue = useCallback(() => {
    if (!audioQueueRunningRef.current) {
      return;
    }
    queuedAudioBooksRef.current = [];
    queuedAudioBookIdsRef.current.clear();
    if (activeAudioBookIdRef.current !== null) {
      queuedAudioBookIdsRef.current.add(activeAudioBookIdRef.current);
    }
    cancelAudioQueueRef.current = true;
    setAudioQueueStatus((current) =>
      current ? { ...current, queuedBookCount: 0, cancelling: true } : current,
    );
  }, []);

  const handleImport = async () => {
    const selected = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "EPUB books", extensions: ["epub"] }],
    });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    if (!paths.length) {
      return;
    }

    setImporting(true);
    try {
      const summary = await importBooks(paths);
      setBooks(summary.books);
      if (summary.failed.length) {
        toast.error(`${summary.failed.length} import failed`, {
          description: summary.failed[0]?.message,
        });
      } else if (summary.imported) {
        toast.success(`Imported ${summary.imported} book${summary.imported === 1 ? "" : "s"}.`);
      } else {
        toast.info("Those books are already in your library.");
      }
    } catch (error) {
      toast.error(errorMessage(error, "Import failed."));
    } finally {
      setImporting(false);
    }
  };

  if (view.kind === "reader") {
    return (
      <ReaderView
        bookId={view.bookId}
        initialChapterIndex={view.chapterIndex}
        registerLeaveRequestHandler={registerLeaveRequestHandler}
        onBack={async () => {
          setView({ kind: "library" });
          await refreshLibrary();
        }}
      />
    );
  }

  if (view.kind === "wordlist") {
    return (
      <TooltipProvider>
        <WordlistView
          onBack={() => setView({ kind: "library" })}
          onOpenEntry={(entry) =>
            setView({
              kind: "reader",
              bookId: entry.book_id,
              chapterIndex: entry.chapter_index,
            })
          }
        />
      </TooltipProvider>
    );
  }

  if (view.kind === "settings") {
    return (
      <TooltipProvider>
        <SettingsView onBack={() => setView({ kind: "library" })} />
      </TooltipProvider>
    );
  }

  return (
    <TooltipProvider>
      <LibraryView
        books={books}
        loading={loadingBooks}
        importing={importing}
        onImport={() => void handleImport()}
        onOpenWordlist={() => setView({ kind: "wordlist" })}
        onOpenSettings={() => setView({ kind: "settings" })}
        audioQueueStatus={audioQueueStatus}
        onQueueAudio={enqueueBookAudio}
        onCancelAudioQueue={cancelAudioQueue}
        onOpenBook={(book) =>
          setView({
            kind: "reader",
            bookId: book.id,
          })
        }
      />
    </TooltipProvider>
  );
}

export default App;
