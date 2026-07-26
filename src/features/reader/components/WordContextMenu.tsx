import type { WordContextMenuState } from "../reader-types";

export function WordContextMenu({
  menu,
  saved,
  onHighlight,
  onLookup,
  onToggleWordlist,
}: {
  menu: WordContextMenuState;
  saved: boolean;
  onHighlight: () => void;
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
      <button type="button" onClick={onHighlight}>
        Highlight
      </button>
      <button type="button" onClick={onLookup}>
        Look up
      </button>
      <button type="button" onClick={onToggleWordlist}>
        {saved ? "Remove from wordlist" : "Save to wordlist"}
      </button>
    </div>
  );
}
