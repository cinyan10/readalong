import { TimerIcon } from "lucide-react";
import { Progress } from "@/components/ui/progress";
import { clampNumber, formatDuration } from "../reader-utils";

export function AudioNavProgress({
  progress,
}: {
  progress: {
    label: string;
    detail: string;
    percent: number;
    etaSeconds: number | null;
    current: string;
  };
}) {
  const percent = Math.round(clampNumber(progress.percent, 0, 100));
  const eta = progress.etaSeconds ? formatDuration(progress.etaSeconds) : "";
  return (
    <div className="audio-nav-progress" aria-label={`${progress.label}, ${percent}% complete`}>
      <div className="audio-nav-progress-main">
        <span className="audio-nav-label">{progress.label}</span>
        <span className="audio-nav-percent">{percent}%</span>
      </div>
      <Progress className="audio-nav-bar" value={percent} />
      <div className="audio-nav-progress-meta">
        <span>{progress.detail}</span>
        {eta ? (
          <span className="audio-nav-eta">
            <TimerIcon aria-hidden="true" />
            {eta} left
          </span>
        ) : null}
      </div>
      <span className="audio-nav-current" title={progress.current}>
        {progress.current}
      </span>
    </div>
  );
}

