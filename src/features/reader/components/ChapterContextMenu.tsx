import type { ChapterContextMenuState } from "../reader-types";

export function ChapterContextMenu({
  menu,
  disabled,
  onGenerateChapter,
  onGenerateFromChapter,
}: {
  menu: ChapterContextMenuState;
  disabled: boolean;
  onGenerateChapter: () => void;
  onGenerateFromChapter: () => void;
}) {
  return (
    <div
      className="reader-context-menu chapter-context-menu"
      style={{ left: menu.x, top: menu.y }}
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
      aria-label={`Audio actions for ${menu.title}`}
    >
      <button type="button" disabled={disabled} onClick={onGenerateChapter}>
        Generate chapter audio
      </button>
      <button type="button" disabled={disabled} onClick={onGenerateFromChapter}>
        Generate from this chapter
      </button>
    </div>
  );
}

