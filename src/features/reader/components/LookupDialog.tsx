import { XIcon } from "lucide-react";
import { useRef, type MouseEvent as ReactMouseEvent } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { LookupDialogState } from "../reader-types";
import { clampNumber } from "../reader-utils";

export function LookupDialog({
  lookup,
  onClose,
  onMove,
  onPlayPronunciation,
}: {
  lookup: LookupDialogState;
  onClose: () => void;
  onMove: (x: number, y: number) => void;
  onPlayPronunciation: (audioUrl: string) => void;
}) {
  const result = lookup.result;
  const choice = result?.context_definition;
  const examples = choice?.examples ?? [];
  const displayWord = result?.selected_word || lookup.word;
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const startDrag = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    const target = event.target;
    if (target instanceof Element && target.closest("button")) {
      return;
    }
    event.preventDefault();
    const dialog = dialogRef.current;
    const startX = event.clientX;
    const startY = event.clientY;
    const startLeft = lookup.x;
    const startTop = lookup.y;
    const width = dialog?.offsetWidth ?? 380;
    const height = dialog?.offsetHeight ?? 260;
    const maxLeft = Math.max(8, window.innerWidth - width - 8);
    const maxTop = Math.max(72, window.innerHeight - height - 8);

    const move = (moveEvent: MouseEvent) => {
      onMove(
        clampNumber(startLeft + moveEvent.clientX - startX, 8, maxLeft),
        clampNumber(startTop + moveEvent.clientY - startY, 72, maxTop),
      );
    };
    const stop = () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", stop);
    };

    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", stop);
  };

  return (
    <div
      ref={dialogRef}
      className="lookup-dialog"
      style={{ left: lookup.x, top: lookup.y }}
      role="dialog"
      aria-label={`${displayWord} lookup`}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <div className="lookup-toolbar" onMouseDown={startDrag}>
        <div className="lookup-title">
          <strong>{displayWord}</strong>
          <div className="lookup-badges">
            {result?.cefr_level ? <Badge variant="secondary">{result.cefr_level}</Badge> : null}
            {result?.word_type ? <Badge variant="outline">{result.word_type}</Badge> : null}
          </div>
        </div>
        <Button variant="ghost" size="icon" onClick={onClose} aria-label="Close lookup">
          <XIcon />
        </Button>
      </div>

      {lookup.loading ? (
        <div className="lookup-loading">
          <Skeleton className="h-4 w-3/4" />
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}

      {lookup.error ? <p className="lookup-error">{lookup.error}</p> : null}

      {result ? (
        <div className="lookup-content">
          {result.phonetics.length ? (
            <div className="lookup-pronunciation">
              {result.audio_url ? (
                <button type="button" className="lookup-phonetic-button" onClick={() => onPlayPronunciation(result.audio_url)}>
                  {result.phonetics.join("  ")}
                </button>
              ) : (
                <span>{result.phonetics.join("  ")}</span>
              )}
            </div>
          ) : null}

          <LookupSection title="Simple meaning" body={result.simple_meaning} />
          <LookupSection title="In context" body={result.in_context_meaning} />

          {choice?.matched && choice.definition ? (
            <section className="lookup-section">
              <h3>Oxford definition</h3>
              <p>
                {choice.definition_number ? <b>Definition {choice.definition_number}. </b> : null}
                {choice.definition}
              </p>
              {result.source_url ? (
                <a href={result.source_url} target="_blank" rel="noreferrer">
                  Oxford Learner's Dictionary
                </a>
              ) : null}
            </section>
          ) : null}

          <LookupSection title="Original meaning" body={result.original_meaning} />

          {examples.length ? (
            <section className="lookup-section">
              <h3>Examples</h3>
              <div className="lookup-examples">
                {examples.slice(0, 3).map((example) => (
                  <blockquote key={example}>{example}</blockquote>
                ))}
              </div>
            </section>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function LookupSection({ title, body }: { title: string; body: string }) {
  if (!body.trim()) {
    return null;
  }
  return (
    <section className="lookup-section">
      <h3>{title}</h3>
      <p>{body}</p>
    </section>
  );
}

