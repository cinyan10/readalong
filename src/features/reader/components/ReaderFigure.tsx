import { convertFileSrc } from "@tauri-apps/api/core";
import type { ChapterPayload } from "@/types";
import type { ReaderImage } from "../reader-types";
import { blockDomId } from "../reader-utils";

export function ReaderFigure({
  block,
  fallbackAlt,
  onOpen,
}: {
  block: ChapterPayload["blocks"][number];
  fallbackAlt: string;
  onOpen: (image: ReaderImage) => void;
}) {
  if (!block.asset_path) {
    return null;
  }
  const image = {
    src: convertFileSrc(block.asset_path),
    alt: block.alt || fallbackAlt,
  };
  return (
    <figure
      id={blockDomId(block.block_index)}
      className="reader-figure"
      data-reader-block
      data-block-index={block.block_index}
    >
      <button
        className="reader-image-button"
        type="button"
        onClick={() => onOpen(image)}
        aria-label="Open image"
      >
        <img className="reader-image" src={image.src} alt={image.alt} />
      </button>
    </figure>
  );
}

