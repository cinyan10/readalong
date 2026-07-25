import { XIcon, ZoomInIcon, ZoomOutIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { ReaderImage } from "../reader-types";

export function ImageDialog({
  image,
  zoom,
  onZoomChange,
  onClose,
}: {
  image: ReaderImage;
  zoom: number;
  onZoomChange: (zoom: number) => void;
  onClose: () => void;
}) {
  const setZoom = (value: number) => onZoomChange(Math.max(0.5, Math.min(3, value)));
  return (
    <div className="image-dialog-backdrop" onClick={onClose} role="presentation">
      <dialog className="image-dialog" open onClick={(event) => event.stopPropagation()}>
        <div className="image-dialog-toolbar" aria-label="Image controls">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="secondary" size="icon" onClick={() => setZoom(zoom - 0.25)} aria-label="Zoom out">
                <ZoomOutIcon />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Zoom out</TooltipContent>
          </Tooltip>
          <input
            className="image-zoom-slider"
            type="range"
            min={0.5}
            max={3}
            step={0.05}
            value={zoom}
            onChange={(event) => setZoom(Number(event.target.value))}
            aria-label="Image zoom"
          />
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="secondary" size="icon" onClick={() => setZoom(zoom + 0.25)} aria-label="Zoom in">
                <ZoomInIcon />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Zoom in</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="secondary" size="icon" onClick={onClose} aria-label="Close image">
                <XIcon />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Close</TooltipContent>
          </Tooltip>
        </div>
        <div className="image-dialog-viewport">
          <img
            className="image-dialog-image"
            src={image.src}
            alt={image.alt}
            style={{ transform: `scale(${zoom})` }}
          />
        </div>
      </dialog>
    </div>
  );
}

