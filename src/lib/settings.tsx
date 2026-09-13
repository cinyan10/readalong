import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

export const DEFAULT_AUDIO_GENERATION_SPEED = 0.95;
export const MIN_AUDIO_GENERATION_SPEED = 0.5;
export const MAX_AUDIO_GENERATION_SPEED = 2;
export const AUDIO_GENERATION_SPEED_STEP = 0.05;

const audioSpeedStorageKey = "readalong:audio-generation-speed";

type SettingsContextValue = {
  audioGenerationSpeed: number;
  setAudioGenerationSpeed: (speed: number) => void;
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

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [audioGenerationSpeed, setAudioGenerationSpeedState] = useState(storedAudioSpeed);

  const value = useMemo(
    () => ({
      audioGenerationSpeed,
      setAudioGenerationSpeed: (speed: number) => {
        const normalized = normalizeAudioSpeed(speed);
        window.localStorage.setItem(audioSpeedStorageKey, String(normalized));
        setAudioGenerationSpeedState(normalized);
      },
    }),
    [audioGenerationSpeed],
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
