import type { WordContextMenuState } from "../reader-types";

export function WordContextMenu({
  menu,
  saved,
  onLookup,
  onToggleWordlist,
}: {
  menu: WordContextMenuState;
  saved: boolean;
  onLookup: () => void;
  onToggleWordlist: () => void;
}) {
  return (
    <div
      className="reader-context-menu"
      style={{ left: menu.x, top: menu.y }}
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <button type="button" onClick={onLookup}>
        Look up word
      </button>
      <button type="button" onClick={onToggleWordlist}>
        {saved ? "Remove from word list" : "Add to word list"}
      </button>
    </div>
  );
}

