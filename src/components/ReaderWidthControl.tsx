import { RotateCcwIcon } from "lucide-react";
import { useId } from "react";
import { Button } from "@/components/ui/button";
import {
  DEFAULT_READER_WIDTH,
  MAX_READER_WIDTH,
  MIN_READER_WIDTH,
  READER_WIDTH_STEP,
  useSettings,
} from "@/lib/settings";

export function ReaderWidthControl() {
  const { readerWidth, setReaderWidth } = useSettings();
  const inputId = useId();

  return (
    <div className="flex w-full max-w-md items-center gap-3">
      <label htmlFor={inputId} className="shrink-0 text-xs font-medium text-muted-foreground">Line length</label>
      <input
        id={inputId}
        className="h-6 min-w-20 flex-1 cursor-pointer accent-primary"
        type="range"
        min={MIN_READER_WIDTH}
        max={MAX_READER_WIDTH}
        step={READER_WIDTH_STEP}
        value={readerWidth}
        aria-valuetext={`Approximately ${readerWidth} characters per line`}
        onChange={(event) => setReaderWidth(Number(event.target.value))}
      />
      <Button
        variant="ghost"
        size="sm"
        className="min-w-24 shrink-0 tabular-nums"
        aria-label={`Line length approximately ${readerWidth} characters. Reset to ${DEFAULT_READER_WIDTH} characters`}
        title="Reset line length"
        onClick={() => setReaderWidth(DEFAULT_READER_WIDTH)}
      >
        {readerWidth} ch
        <RotateCcwIcon data-icon="inline-end" />
      </Button>
    </div>
  );
}
