import { cn } from "@/lib/utils";
import type { MarkedWordLocation } from "../reader-types";

export function MarkedWordScrollRail({
  locations,
  activeKey,
  onSelect,
}: {
  locations: MarkedWordLocation[];
  activeKey: string | null;
  onSelect: (location: MarkedWordLocation) => void;
}) {
  if (!locations.length) {
    return null;
  }

  return (
    <nav className="marked-word-rail" aria-label="Marked word locations">
      {locations.map((location, index) => (
        <button
          key={`${location.key}-${index}`}
          className={cn("marked-word-marker", activeKey === location.key && "active")}
          type="button"
          style={{ top: `${location.ratio * 100}%` }}
          onClick={() => onSelect(location)}
          aria-label={`Go to marked word ${location.word}`}
          title={location.word}
        />
      ))}
    </nav>
  );
}

