import type { SelectionContextMenuState } from "../reader-types";

export function SelectionContextMenu({
  menu,
  onHighlight,
}: {
  menu: SelectionContextMenuState;
  onHighlight: () => void;
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
    </div>
  );
}
