import { PauseIcon, PlayIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { formatClock } from "../reader-utils";

export function PartAudioPlayer({
  playing,
  currentTime,
  duration,
  onTogglePlay,
  onSeek,
}: {
  playing: boolean;
  currentTime: number;
  duration: number;
  onTogglePlay: () => void;
  onSeek: (time: number) => void;
}) {
  const safeDuration = Math.max(duration, 0);
  const remainingTime = safeDuration > 0 ? Math.max(safeDuration - currentTime, 0) : 0;
  return (
    <div className="reader-audio" aria-label="Audio playback controls">
      <Button className="audio-button" size="icon" onClick={onTogglePlay} aria-label={playing ? "Pause" : "Play"}>
        {playing ? <PauseIcon aria-hidden="true" /> : <PlayIcon aria-hidden="true" />}
      </Button>
      <div className="audio-timeline">
        <span className="audio-time audio-time-current" aria-label={`${formatClock(currentTime)} elapsed`}>
          {formatClock(currentTime)}
        </span>
        <input
          className="audio-slider"
          type="range"
          min={0}
          max={safeDuration}
          step={0.1}
          value={Math.min(currentTime, safeDuration)}
          onChange={(event) => onSeek(Number(event.target.value))}
          aria-label="Audio position"
        />
        <span className="audio-time audio-time-total" aria-label={`${formatClock(duration)} total`}>
          {formatClock(duration)}
        </span>
        <span className="audio-time audio-time-left" aria-label={`${formatClock(remainingTime)} remaining`}>
          ({formatClock(remainingTime)})
        </span>
      </div>
    </div>
  );
}

