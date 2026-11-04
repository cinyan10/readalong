import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export const DEFAULT_AUDIO_GENERATION_SPEED = 0.95;
export const MIN_AUDIO_GENERATION_SPEED = 0.5;
export const MAX_AUDIO_GENERATION_SPEED = 2;
export const AUDIO_GENERATION_SPEED_STEP = 0.05;
export const DEFAULT_READER_WIDTH = 65;
export const MIN_READER_WIDTH = 50;
export const MAX_READER_WIDTH = 75;
export const READER_WIDTH_STEP = 1;

const audioSpeedStorageKey = "readalong:audio-generation-speed";
const readerWidthStorageKey = "readalong:reader-line-length";

type SettingsContextValue = {
  audioGenerationSpeed: number;
  setAudioGenerationSpeed: (speed: number) => void;
  readerWidth: number;
  setReaderWidth: (width: number) => void;
};

const SettingsContext = createContext<SettingsContextValue | null>(null);

function normalizeAudioSpeed(speed: number) {
  const clamped = Math.min(MAX_AUDIO_GENERATION_SPEED, Math.max(MIN_AUDIO_GENERATION_SPEED, speed));
  return Number((Math.round(clamped / AUDIO_GENERATION_SPEED_STEP) * AUDIO_GENERATION_SPEED_STEP).toFixed(2));
}

function storedAudioSpeed() {
  const stored = Number(window.localStorage.getItem(audioSpeedStorageKey));
  return Number.isFinite(stored) && stored > 0
    ? normalizeAudioSpeed(stored)
    : DEFAULT_AUDIO_GENERATION_SPEED;
}

function normalizeReaderWidth(width: number) {
  return Number.isFinite(width)
    ? Math.min(MAX_READER_WIDTH, Math.max(MIN_READER_WIDTH, Math.round(width / READER_WIDTH_STEP) * READER_WIDTH_STEP))
    : DEFAULT_READER_WIDTH;
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [audioGenerationSpeed, setAudioGenerationSpeedState] = useState(storedAudioSpeed);
  const [readerWidth, setReaderWidthState] = useState(() => {
    const stored = window.localStorage.getItem(readerWidthStorageKey);
    return stored === null ? DEFAULT_READER_WIDTH : normalizeReaderWidth(Number(stored));
  });
  const setReaderWidth = useCallback((width: number) => setReaderWidthState(normalizeReaderWidth(width)), []);

  useEffect(() => {
    window.localStorage.setItem(readerWidthStorageKey, String(readerWidth));
    document.documentElement.style.setProperty("--reader-width", `${readerWidth}ch`);
    window.dispatchEvent(new Event("resize"));
  }, [readerWidth]);

  const value = useMemo(
    () => ({
      audioGenerationSpeed,
      readerWidth,
      setReaderWidth,
      setAudioGenerationSpeed: (speed: number) => {
        const normalized = normalizeAudioSpeed(speed);
        window.localStorage.setItem(audioSpeedStorageKey, String(normalized));
        setAudioGenerationSpeedState(normalized);
      },
    }),
    [audioGenerationSpeed, readerWidth, setReaderWidth],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings() {
  const value = useContext(SettingsContext);
  if (!value) {
    throw new Error("useSettings must be used within SettingsProvider.");
  }
  return value;
}
