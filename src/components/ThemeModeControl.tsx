import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";

import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useTheme, type ThemeMode } from "@/lib/theme";

const themeOptions: { mode: ThemeMode; label: string; shortLabel: string; icon: typeof MonitorIcon }[] = [
  { mode: "system", label: "Use system theme", shortLabel: "System", icon: MonitorIcon },
  { mode: "light", label: "Use light theme", shortLabel: "Light", icon: SunIcon },
  { mode: "dark", label: "Use dark theme", shortLabel: "Dark", icon: MoonIcon },
];

export function ThemeModeControl() {
  const { mode, resolvedTheme, setMode } = useTheme();
  const [menuPosition, setMenuPosition] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const activeOption = themeOptions.find((option) => option.mode === mode) ?? themeOptions[0];
  const ActiveIcon = resolvedTheme === "dark" ? MoonIcon : SunIcon;
  const themeLabel =
    mode === "system" ? `System (${resolvedTheme === "dark" ? "Dark" : "Light"})` : activeOption.shortLabel;

  useEffect(() => {
    if (!menuPosition) {
      return;
    }

    const closeMenu = (event: PointerEvent) => {
      if (menuRef.current?.contains(event.target as Node)) {
        return;
      }
      setMenuPosition(null);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuPosition(null);
      }
    };

    window.addEventListener("pointerdown", closeMenu);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeMenu);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [menuPosition]);

  const toggleMode = () => {
    setMode(resolvedTheme === "dark" ? "light" : "dark");
    setMenuPosition(null);
  };

  const openMenu = (event: ReactMouseEvent<HTMLButtonElement>) => {
    event.preventDefault();
    setMenuPosition({
      x: Math.min(event.clientX, window.innerWidth - 150),
      y: Math.min(event.clientY, window.innerHeight - 136),
    });
  };

  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="theme-mode-button"
            onClick={toggleMode}
            onContextMenu={openMenu}
            aria-label={`Theme: ${themeLabel}`}
            aria-haspopup="menu"
            aria-expanded={Boolean(menuPosition)}
          >
            <ActiveIcon />
          </Button>
        </TooltipTrigger>
        <TooltipContent>{`Theme: ${themeLabel}`}</TooltipContent>
      </Tooltip>
      {menuPosition ? (
        <div
          ref={menuRef}
          className="theme-mode-menu"
          role="menu"
          aria-label="Theme preference"
          style={{ left: menuPosition.x, top: menuPosition.y }}
        >
          {themeOptions.map((option) => {
            const Icon = option.icon;
            const active = mode === option.mode;
            return (
              <button
                key={option.mode}
                type="button"
                role="menuitemradio"
                aria-checked={active}
                className={active ? "active" : undefined}
                onClick={() => {
                  setMode(option.mode);
                  setMenuPosition(null);
                }}
              >
                <Icon />
                <span>{option.shortLabel}</span>
              </button>
            );
          })}
        </div>
      ) : null}
    </>
  );
}
