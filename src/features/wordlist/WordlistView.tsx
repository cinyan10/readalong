import { listen } from "@tauri-apps/api/event";
import { ChevronLeftIcon, Trash2Icon } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

import { deleteWordlistEntry, listWordlistEntries } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type { WordlistEntry } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { ThemeModeControl } from "@/components/ThemeModeControl";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { formatSavedAt, highlightContextWord, upsertWordlistEntry } from "./wordlist-utils";

export function WordlistView({
  onBack,
  onOpenEntry,
}: {
  onBack: () => void;
  onOpenEntry: (entry: WordlistEntry) => void;
}) {
  const [entries, setEntries] = useState<WordlistEntry[]>([]);
  const [loading, setLoading] = useState(true);

  const refreshEntries = useCallback(async () => {
    setLoading(true);
    try {
      setEntries(await listWordlistEntries());
    } catch (error) {
      toast.error(errorMessage(error, "Failed to load word list."));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshEntries();
  }, [refreshEntries]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void listen<WordlistEntry>("wordlist_entry_enriched", (event) => {
      setEntries((current) => upsertWordlistEntry(current, event.payload));
    }).then((unsubscribe) => {
      if (cancelled) {
        unsubscribe();
      } else {
        unlisten = unsubscribe;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const removeEntry = (entry: WordlistEntry) => {
    void deleteWordlistEntry(entry.root_word)
      .then(() => {
        setEntries((current) => current.filter((item) => item.root_word !== entry.root_word));
        toast.success("Removed from word list.");
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to remove word.")));
  };

  return (
    <main className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto grid h-16 w-full max-w-7xl grid-cols-[auto_1fr_auto] items-center gap-3 px-6">
          <Button variant="ghost" size="icon" onClick={onBack} aria-label="Back to library">
            <ChevronLeftIcon />
          </Button>
          <div className="min-w-0">
            <h1 className="truncate text-base font-semibold">Word list</h1>
            <p className="truncate text-xs text-muted-foreground">
              {entries.length ? `${entries.length} saved word${entries.length === 1 ? "" : "s"}` : "Saved vocabulary"}
            </p>
          </div>
          <ThemeModeControl />
        </div>
      </header>

      <section className="wordlist-page mx-auto w-full max-w-5xl px-6 py-8">
        {loading ? (
          <div className="wordlist-list">
            {Array.from({ length: 4 }).map((_, index) => (
              <Skeleton key={index} className="h-32 rounded-md" />
            ))}
          </div>
        ) : entries.length ? (
          <div className="wordlist-list">
            {entries.map((entry) => (
              <article key={entry.id} className="wordlist-entry">
                <button className="wordlist-entry-main" type="button" onClick={() => onOpenEntry(entry)}>
                  <span className="wordlist-word-row">
                    <strong>{entry.root_word}</strong>
                    <span>{entry.original_word}</span>
                  </span>
                  <span className="wordlist-meta-row">
                    {entry.word_type ? <Badge variant="secondary">{entry.word_type}</Badge> : null}
                    {entry.cefr_level ? <Badge variant="outline">{entry.cefr_level}</Badge> : null}
                    <span>{entry.book_title}</span>
                    <span>{formatSavedAt(entry.created_at)}</span>
                  </span>
                  <span className="wordlist-context">{highlightContextWord(entry.context, entry.original_word)}</span>
                  {entry.definition ? (
                    <span className="wordlist-definition">
                      <span>{entry.simple_meaning || entry.definition}</span>
                      {entry.in_context_meaning ? <span>{entry.in_context_meaning}</span> : null}
                      {entry.definition_examples[0] ? <em>{entry.definition_examples[0]}</em> : null}
                    </span>
                  ) : entry.definition_lookup_error ? (
                    <span className="wordlist-error">{entry.definition_lookup_error}</span>
                  ) : (
                    <span className="wordlist-muted">Looking up definition...</span>
                  )}
                </button>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="icon" onClick={() => removeEntry(entry)} aria-label={`Remove ${entry.root_word}`}>
                      <Trash2Icon />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Remove</TooltipContent>
                </Tooltip>
              </article>
            ))}
          </div>
        ) : (
          <Empty className="rounded-md border border-dashed bg-muted/20">
            <EmptyHeader>
              <EmptyTitle>No saved words yet</EmptyTitle>
              <EmptyDescription>Add words from the reader context menu.</EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </section>
    </main>
  );
}
