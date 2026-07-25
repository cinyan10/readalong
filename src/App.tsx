import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { importBooks, listBooks } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type { BookSummary } from "@/types";
import { TooltipProvider } from "@/components/ui/tooltip";
import { LibraryView } from "@/features/library/LibraryView";
import { ReaderView } from "@/features/reader/ReaderView";
import { WordlistView } from "@/features/wordlist/WordlistView";

type ViewState =
  | { kind: "library" }
  | { kind: "wordlist" }
  | { kind: "reader"; bookId: number; chapterIndex?: number };

function App() {
  const [view, setView] = useState<ViewState>({ kind: "library" });
  const [books, setBooks] = useState<BookSummary[]>([]);
  const [loadingBooks, setLoadingBooks] = useState(true);
  const [importing, setImporting] = useState(false);

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

  return (
    <TooltipProvider>
      <LibraryView
        books={books}
        loading={loadingBooks}
        importing={importing}
        onImport={() => void handleImport()}
        onOpenWordlist={() => setView({ kind: "wordlist" })}
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
