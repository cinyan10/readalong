import { convertFileSrc } from "@tauri-apps/api/core";
import { ChevronLeftIcon, GaugeIcon, LoaderCircleIcon, PlayIcon, RotateCcwIcon, Volume2Icon } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { ThemeModeControl } from "@/components/ThemeModeControl";
import { Button } from "@/components/ui/button";
import { generateAudioPreview } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import {
  AUDIO_GENERATION_SPEED_STEP,
  DEFAULT_AUDIO_GENERATION_SPEED,
  MAX_AUDIO_GENERATION_SPEED,
  MIN_AUDIO_GENERATION_SPEED,
  useSettings,
} from "@/lib/settings";
import { cn } from "@/lib/utils";
import type { AudioPreviewPayload } from "@/types";

const speedPresets = [
  { value: 0.75, label: "Relaxed" },
  { value: 0.95, label: "Natural" },
  { value: 1.25, label: "Brisk" },
  { value: 1.5, label: "Fast" },
];

export function SettingsView({ onBack }: { onBack: () => void }) {
  const { audioGenerationSpeed, setAudioGenerationSpeed } = useSettings();
  const [preview, setPreview] = useState<AudioPreviewPayload | null>(null);
  const [generatingPreview, setGeneratingPreview] = useState(false);
  const percentage = Math.round(audioGenerationSpeed * 100);

  useEffect(() => {
    setPreview(null);
  }, [audioGenerationSpeed]);

  const generatePreview = async () => {
    setGeneratingPreview(true);
    try {
      setPreview(await generateAudioPreview(audioGenerationSpeed));
    } catch (error) {
      toast.error(errorMessage(error, "Audio preview generation failed."));
    } finally {
      setGeneratingPreview(false);
    }
  };

  return (
    <main className="min-h-screen bg-background text-foreground">
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto grid h-16 w-full max-w-7xl grid-cols-[auto_1fr_auto] items-center gap-3 px-6">
          <Button variant="ghost" size="icon" onClick={onBack} aria-label="Back to library">
            <ChevronLeftIcon />
          </Button>
          <div className="min-w-0">
            <h1 className="truncate text-base font-semibold">Settings</h1>
            <p className="truncate text-xs text-muted-foreground">Reading and audio preferences</p>
          </div>
          <ThemeModeControl />
        </div>
      </header>

      <section className="mx-auto w-full max-w-3xl px-6 py-10 sm:py-14">
        <div className="mb-8 flex items-start gap-4">
          <div className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
            <Volume2Icon className="size-5" aria-hidden="true" />
          </div>
          <div>
            <h2 className="m-0 text-xl font-semibold tracking-tight">Audio</h2>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">
              Choose how quickly newly generated narration should be spoken.
            </p>
          </div>
        </div>

        <div className="border-y">
          <div className="flex items-start justify-between gap-6 py-6">
            <div className="min-w-0">
              <h3 className="m-0 text-sm font-semibold">Generation speed</h3>
              <p className="mt-1 max-w-md text-sm leading-6 text-muted-foreground">
                Applies to new audio. Existing narration stays unchanged until regenerated.
              </p>
            </div>
            <div className="shrink-0 text-right" aria-live="polite">
              <strong className="block text-2xl font-semibold tabular-nums tracking-tight text-primary">
                {audioGenerationSpeed.toFixed(2).replace(/0$/, "")}×
              </strong>
              <span className="text-xs tabular-nums text-muted-foreground">{percentage}% of normal</span>
            </div>
          </div>

          <div className="border-t pb-7 pt-6">
            <label className="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3">
              <span className="sr-only">Narration speed</span>
              <GaugeIcon className="size-4 text-muted-foreground" aria-hidden="true" />
              <input
                className="h-6 w-full cursor-pointer accent-primary"
                type="range"
                min={MIN_AUDIO_GENERATION_SPEED}
                max={MAX_AUDIO_GENERATION_SPEED}
                step={AUDIO_GENERATION_SPEED_STEP}
                value={audioGenerationSpeed}
                onChange={(event) => setAudioGenerationSpeed(Number(event.target.value))}
              />
            </label>
            <div className="ml-7 mt-1 flex justify-between text-xs tabular-nums text-muted-foreground" aria-hidden="true">
              <span>0.5×</span>
              <span>2×</span>
            </div>

            <div className="mt-6 grid grid-cols-2 gap-2 sm:grid-cols-4" aria-label="Narration speed presets">
              {speedPresets.map((preset) => {
                const selected = Math.abs(audioGenerationSpeed - preset.value) < 0.001;
                return (
                  <button
                    key={preset.value}
                    type="button"
                    className={cn(
                      "flex min-h-16 flex-col items-start justify-center rounded-lg border px-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                      selected
                        ? "border-primary/50 bg-primary/10 text-foreground"
                        : "border-transparent bg-secondary text-secondary-foreground hover:bg-secondary/70",
                    )}
                    aria-pressed={selected}
                    onClick={() => setAudioGenerationSpeed(preset.value)}
                  >
                    <span className="text-xs text-muted-foreground">{preset.label}</span>
                    <strong className="mt-0.5 text-sm font-semibold tabular-nums">{preset.value}×</strong>
                  </button>
                );
              })}
            </div>
          </div>

          <div className="border-t py-6">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div className="max-w-md">
                <h3 className="m-0 text-sm font-semibold">Voice preview</h3>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">
                  Generate this sample at {audioGenerationSpeed.toFixed(2).replace(/0$/, "")}× to hear the selected pace.
                </p>
                <blockquote className="mb-0 mt-3 border-l-2 border-primary/40 pl-3 text-sm italic leading-6 text-foreground/80">
                  “A quiet morning is the perfect time to open a book and discover somewhere new.”
                </blockquote>
              </div>
              <Button
                className="shrink-0"
                variant={preview ? "secondary" : "default"}
                size="sm"
                disabled={generatingPreview}
                onClick={() => void generatePreview()}
              >
                {generatingPreview ? (
                  <LoaderCircleIcon className="animate-spin" data-icon="inline-start" />
                ) : (
                  <PlayIcon data-icon="inline-start" />
                )}
                {generatingPreview ? "Generating" : preview ? "Regenerate" : "Generate preview"}
              </Button>
            </div>

            {preview ? (
              <div className="mt-5 rounded-lg bg-secondary/70 p-3">
                <audio
                  className="block h-10 w-full"
                  controls
                  src={convertFileSrc(preview.audio_path)}
                  aria-label={`Narration preview at ${preview.speed} times speed`}
                />
              </div>
            ) : null}
          </div>
        </div>

        <div className="flex flex-col items-start justify-between gap-3 pt-4 sm:flex-row sm:items-center">
          <p className="m-0 text-xs text-muted-foreground">Saved automatically on this device.</p>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setAudioGenerationSpeed(DEFAULT_AUDIO_GENERATION_SPEED)}
            disabled={Math.abs(audioGenerationSpeed - DEFAULT_AUDIO_GENERATION_SPEED) < 0.001}
          >
            <RotateCcwIcon data-icon="inline-start" />
            Reset to default
          </Button>
        </div>
      </section>
    </main>
  );
}
