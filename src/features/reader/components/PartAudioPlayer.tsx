import { PauseIcon, PlayIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatClock } from "../reader-utils";

const CONTROLS_HIDE_DELAY = 2_500;
const CONTROLS_REVEAL_ZONE = 96;

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
  const [controlsVisible, setControlsVisible] = useState(true);
  const controlsRef = useRef<HTMLDivElement | null>(null);
  const hideTimerRef = useRef<number | null>(null);

  const clearHideTimer = useCallback(() => {
    if (hideTimerRef.current !== null) {
      window.clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  }, []);

  const scheduleHide = useCallback(() => {
    clearHideTimer();
    if (!playing) {
      return;
    }
    hideTimerRef.current = window.setTimeout(() => {
      if (!controlsRef.current?.contains(document.activeElement)) {
        setControlsVisible(false);
      }
    }, CONTROLS_HIDE_DELAY);
  }, [clearHideTimer, playing]);

  const revealControls = useCallback(() => {
    setControlsVisible(true);
    scheduleHide();
  }, [scheduleHide]);

  useEffect(() => {
    if (!playing) {
      clearHideTimer();
      setControlsVisible(true);
      return;
    }
    scheduleHide();
    return clearHideTimer;
  }, [clearHideTimer, playing, scheduleHide]);

  useEffect(() => {
    const revealNearBottom = (event: PointerEvent) => {
      if (window.innerHeight - event.clientY <= CONTROLS_REVEAL_ZONE) {
        revealControls();
      }
    };
    document.addEventListener("pointermove", revealNearBottom, { passive: true });
    return () => {
      document.removeEventListener("pointermove", revealNearBottom);
    };
  }, [revealControls]);

  return (
    <div
      ref={controlsRef}
      className={cn("reader-audio", !controlsVisible && "is-hidden")}
      aria-label="Audio playback controls"
      onFocusCapture={revealControls}
      onPointerMove={revealControls}
      onPointerLeave={scheduleHide}
    >
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
