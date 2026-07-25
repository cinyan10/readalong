import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  AudioLinesIcon,
  BookMarkedIcon,
  BookmarkIcon,
  ChevronLeftIcon,
  MenuIcon,
  SearchIcon,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { toast } from "sonner";

import {
  addWordlistEntry,
  deleteWordlistEntry,
  generatePartAudio,
  getChapter,
  getPartAlignment,
  getPartAudio,
  getReader,
  listWordlistEntries,
  lookupWord,
  saveBookmark,
  saveProgress,
  searchBook,
  syncPartAlignment,
  type SaveProgressInput,
} from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import type {
  BookSearchResult,
  ChapterPayload,
  PartAlignmentPayload,
  PartAudioPayload,
  ReaderPayload,
  ReadingBookmark,
  TimedToken,
  WordlistEntry,
} from "@/types";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { AudioNavProgress } from "./components/AudioNavProgress";
import { ChapterContextMenu } from "./components/ChapterContextMenu";
import { ChapterFindBar } from "./components/ChapterFindBar";
import { ImageDialog } from "./components/ImageDialog";
import { LookupDialog } from "./components/LookupDialog";
import { MarkedWordScrollRail } from "./components/MarkedWordScrollRail";
import { PartAudioPlayer } from "./components/PartAudioPlayer";
import { ReaderFigure } from "./components/ReaderFigure";
import { ReaderSkeleton } from "./components/ReaderSkeleton";
import { ReaderTokens } from "./components/ReaderTokens";
import { SearchPanel } from "./components/SearchPanel";
import { WordContextMenu } from "./components/WordContextMenu";
import type {
  ActiveSearchResult,
  AudioGenerationProgress,
  AudioQueueMode,
  AudioQueueState,
  ChapterContextMenuState,
  ChapterFindRange,
  ColorMode,
  LookupDialogState,
  MarkedWordLocation,
  PendingRestore,
  ReaderImage,
  SaveProgressOptions,
  WordContextMenuState,
} from "./reader-types";
import {
  blockDomId,
  chapterPartEffort,
  clampNumber,
  closestChapterFindMatchIndex,
  colorModeStorageKey,
  countWords,
  currentScrollRatio,
  findChapterMatches,
  findLastIndex,
  findSavedPlayingToken,
  formatChapterTitle,
  formatDuration,
  isEditableTarget,
  readingProgressPercent,
  restoreScrollPosition,
  scheduleScrollRestore,
  shouldIgnorePlaybackShortcut,
  storedColorMode,
  timedTokenAtTime,
  timedTokenKey,
} from "./reader-utils";
import {
  dictionaryLookupFromWordlistEntry,
  hasWordlistAiEnrichment,
  isWordlistEntryAtToken,
  upsertWordlistEntry,
  wordlistExactKey,
  wordlistTokenKey,
} from "@/features/wordlist/wordlist-utils";

export function ReaderView({
  bookId,
  initialChapterIndex,
  onBack,
}: {
  bookId: number;
  initialChapterIndex?: number;
  onBack: () => void;
}) {
  const [reader, setReader] = useState<ReaderPayload | null>(null);
  const [chapter, setChapter] = useState<ChapterPayload | null>(null);
  const [chapterIndex, setChapterIndex] = useState(initialChapterIndex ?? 0);
  const [partIndex, setPartIndex] = useState(0);
  const [tocOpen, setTocOpen] = useState(true);
  const [loading, setLoading] = useState(true);
  const [partAudio, setPartAudio] = useState<PartAudioPayload | null>(null);
  const [loadingAudio, setLoadingAudio] = useState(false);
  const [generatingAudio, setGeneratingAudio] = useState(false);
  const [preparingAudioQueue, setPreparingAudioQueue] = useState(false);
  const [audioQueue, setAudioQueue] = useState<AudioQueueState | null>(null);
  const [partAlignment, setPartAlignment] = useState<PartAlignmentPayload | null>(null);
  const [loadingAlignment, setLoadingAlignment] = useState(false);
  const [syncingAlignment, setSyncingAlignment] = useState(false);
  const [activeTokenKey, setActiveTokenKey] = useState<string | null>(null);
  const [audioProgress, setAudioProgress] = useState<AudioGenerationProgress | null>(null);
  const [audioState, setAudioState] = useState({ currentTime: 0, duration: 0, playing: false });
  const [wordlistEntries, setWordlistEntries] = useState<WordlistEntry[]>([]);
  const [colorMode, setColorMode] = useState<ColorMode>(() => storedColorMode());
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<BookSearchResult[]>([]);
  const [loadingSearch, setLoadingSearch] = useState(false);
  const [activeSearchResult, setActiveSearchResult] = useState<ActiveSearchResult | null>(null);
  const [chapterFindOpen, setChapterFindOpen] = useState(false);
  const [chapterFindQuery, setChapterFindQuery] = useState("");
  const [activeChapterFindIndex, setActiveChapterFindIndex] = useState(0);
  const [chapterFindScrollRequest, setChapterFindScrollRequest] = useState(0);
  const [dialogImage, setDialogImage] = useState<ReaderImage | null>(null);
  const [wordContextMenu, setWordContextMenu] = useState<WordContextMenuState | null>(null);
  const [chapterContextMenu, setChapterContextMenu] = useState<ChapterContextMenuState | null>(null);
  const [lookupDialog, setLookupDialog] = useState<LookupDialogState | null>(null);
  const [markedWordLocations, setMarkedWordLocations] = useState<MarkedWordLocation[]>([]);
  const [imageZoom, setImageZoom] = useState(1);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const wordPreviewAudioRef = useRef<HTMLAudioElement | null>(null);
  const dictionaryAudioRef = useRef<HTMLAudioElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const chapterFindInputRef = useRef<HTMLInputElement | null>(null);
  const visibleBlockRef = useRef<number | null>(null);
  const saveTimerRef = useRef<number | null>(null);
  const saveQueuedPayloadRef = useRef<SaveProgressInput | null>(null);
  const saveFlushScheduledRef = useRef(false);
  const saveInFlightRef = useRef(false);
  const audioSaveTimerRef = useRef<number | null>(null);
  const lastAudioSaveAtRef = useRef(0);
  const pendingRestoreRef = useRef<PendingRestore | null>(null);
  const pendingAudioResumeTimeRef = useRef<number | null>(null);
  const audioLookupPendingRef = useRef(false);
  const alignmentLookupPendingRef = useRef(false);
  const pendingPartBlockRef = useRef<number | null>(null);
  const tokenRefs = useRef<Record<string, HTMLElement | null>>({});
  const activeTimedTokenRef = useRef<TimedToken | null>(null);
  const wordPreviewEndTimeRef = useRef<number | null>(null);
  const lastAutoScrollTokenRef = useRef<string | null>(null);
  const lastSelectionSeekKeyRef = useRef("");
  const lastContextMenuAtRef = useRef(0);
  const lastChapterFindRevealKeyRef = useRef("");
  const searchRequestRef = useRef(0);
  const lookupRequestRef = useRef(0);
  const audioQueueRunRef = useRef(0);
  const selectedPartRef = useRef({ chapterIndex, partIndex });

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    pendingRestoreRef.current = null;
    void getReader(bookId)
      .then((payload) => {
        if (cancelled) {
          return;
        }
        setReader(payload);
        const shouldResume = initialChapterIndex === undefined;
        const savedBookmark = shouldResume ? payload.bookmark : null;
        const savedProgress = shouldResume ? payload.progress : null;
        pendingRestoreRef.current = savedBookmark
          ? { kind: "bookmark", bookmark: savedBookmark }
          : savedProgress
            ? { kind: "progress", progress: savedProgress }
            : null;
        const savedChapterIndex = savedBookmark?.chapter_index ?? savedProgress?.last_chapter_index ?? initialChapterIndex ?? 0;
        const nextChapterIndex = payload.chapters.some((item) => item.chapter_index === savedChapterIndex) ? savedChapterIndex : 0;
        const nextChapter = payload.chapters.find((item) => item.chapter_index === nextChapterIndex);
        const savedPart = savedBookmark
          ? nextChapter?.parts.find((part) => part.part_index === savedBookmark.part_index)
          : savedProgress
            ? nextChapter?.parts.find((part) => part.part_index === savedProgress.last_part_index)
            : null;
        const blockPart = savedBookmark
          ? nextChapter?.parts.find(
              (part) =>
                savedBookmark.block_index >= part.start_block_index &&
                savedBookmark.block_index <= part.end_block_index,
            )
          : savedProgress
          ? nextChapter?.parts.find(
              (part) =>
                savedProgress.last_block_index >= part.start_block_index &&
                savedProgress.last_block_index <= part.end_block_index,
            )
          : null;
        const nextPartIndex = savedPart?.part_index ?? blockPart?.part_index ?? 0;
        setChapterIndex(nextChapterIndex);
        setPartIndex(nextPartIndex);
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to open book.")))
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [bookId, initialChapterIndex]);

  useEffect(() => {
    if (!reader) {
      setChapter(null);
      return;
    }
    let cancelled = false;
    setChapter(null);
    void getChapter(bookId, chapterIndex)
      .then((payload) => {
        if (!cancelled) {
          setChapter(payload);
        }
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to load chapter.")));
    return () => {
      cancelled = true;
    };
  }, [bookId, chapterIndex, reader]);

  useEffect(() => {
    let cancelled = false;
    void listWordlistEntries()
      .then((entries) => {
        if (!cancelled) {
          setWordlistEntries(entries);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setWordlistEntries([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [bookId]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void listen<WordlistEntry>("wordlist_entry_enriched", (event) => {
      setWordlistEntries((current) => upsertWordlistEntry(current, event.payload));
    }).then((unsubscribe) => {
      if (cancelled) {
        unsubscribe();
      } else {
        unlisten = unsubscribe;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    window.localStorage.setItem(colorModeStorageKey(), colorMode);
  }, [colorMode]);

  const activeChapter = useMemo(
    () => reader?.chapters.find((item) => item.chapter_index === chapterIndex),
    [chapterIndex, reader],
  );
  const activePart = useMemo(
    () => activeChapter?.parts.find((item) => item.part_index === partIndex),
    [activeChapter, partIndex],
  );
  const currentQueueItem = audioQueue?.items[audioQueue.currentIndex] ?? null;
  const bookmark = reader?.bookmark ?? null;
  const bookmarkedTokenKey = bookmark ? timedTokenKey(bookmark.block_index, bookmark.token_index) : null;
  const wordlistRoots = useMemo(
    () => new Set(wordlistEntries.map((entry) => entry.root_word)),
    [wordlistEntries],
  );
  const wordlistExactKeys = useMemo(
    () => new Set(wordlistEntries.map((entry) => wordlistExactKey(entry))),
    [wordlistEntries],
  );
  const visibleBlocks = useMemo(() => {
    if (!chapter) {
      return [];
    }
    if (!activePart || !activeChapter || activeChapter.parts.length <= 1) {
      return chapter.blocks;
    }
    return chapter.blocks.filter(
      (block) => block.block_index >= activePart.start_block_index && block.block_index <= activePart.end_block_index,
    );
  }, [activeChapter, activePart, chapter]);
  const partWordCount = useMemo(
    () => visibleBlocks.reduce((total, block) => total + countWords(block.text), 0),
    [visibleBlocks],
  );
  const partParagraphCount = useMemo(
    () => visibleBlocks.filter((block) => block.kind === "paragraph").length,
    [visibleBlocks],
  );
  useEffect(() => {
    selectedPartRef.current = { chapterIndex, partIndex };
  }, [chapterIndex, partIndex]);
  const timedTokensByKey = useMemo(() => {
    const tokens = new Map<string, TimedToken>();
    for (const token of partAlignment?.tokens ?? []) {
      tokens.set(timedTokenKey(token.block_index, token.token_index), token);
    }
    return tokens;
  }, [partAlignment]);
  const chapterFindMatches = useMemo(
    () => findChapterMatches(chapter, chapterFindQuery),
    [chapter, chapterFindQuery],
  );
  const activeChapterFindMatch = chapterFindOpen ? chapterFindMatches[activeChapterFindIndex] ?? null : null;
  const chapterFindRangesByBlock = useMemo(() => {
    if (!chapterFindOpen) {
      return new Map<number, ChapterFindRange[]>();
    }
    const ranges = new Map<number, ChapterFindRange[]>();
    chapterFindMatches.forEach((match, index) => {
      const blockRanges = ranges.get(match.blockIndex) ?? [];
      blockRanges.push({
        start: match.start,
        end: match.end,
        active: index === activeChapterFindIndex,
      });
      ranges.set(match.blockIndex, blockRanges);
    });
    return ranges;
  }, [activeChapterFindIndex, chapterFindMatches, chapterFindOpen]);
  const trimmedSearchQuery = searchQuery.trim();

  useEffect(() => {
    if (!reader || !chapter || !visibleBlocks.length || !wordlistExactKeys.size) {
      setMarkedWordLocations([]);
      return;
    }

    let cancelled = false;
    let animationFrame = 0;

    const measureMarkedWords = () => {
      const maxScroll = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
      if (maxScroll <= 0) {
        setMarkedWordLocations([]);
        return;
      }

      const nextLocations: MarkedWordLocation[] = [];
      for (const block of visibleBlocks) {
        block.tokens.forEach((token, tokenIndex) => {
          const exactKey = wordlistTokenKey(bookId, chapter.chapter_index, block.block_index, tokenIndex);
          if (!wordlistExactKeys.has(exactKey)) {
            return;
          }

          const key = timedTokenKey(block.block_index, tokenIndex);
          const node = tokenRefs.current[key];
          if (!node) {
            return;
          }

          const top = node.getBoundingClientRect().top + window.scrollY;
          nextLocations.push({
            key,
            word: token.text,
            blockIndex: block.block_index,
            tokenIndex,
            ratio: clampNumber(top / maxScroll, 0, 1),
          });
        });
      }

      if (!cancelled) {
        setMarkedWordLocations(nextLocations.sort((left, right) => left.ratio - right.ratio));
      }
    };

    const scheduleMeasure = () => {
      window.cancelAnimationFrame(animationFrame);
      animationFrame = window.requestAnimationFrame(measureMarkedWords);
    };

    scheduleScrollRestore(scheduleMeasure);
    window.addEventListener("resize", scheduleMeasure);
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(animationFrame);
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [bookId, chapter, reader, visibleBlocks, wordlistExactKeys]);

  useEffect(() => {
    if (!searchOpen) {
      return;
    }
    window.requestAnimationFrame(() => searchInputRef.current?.focus({ preventScroll: true }));
  }, [searchOpen]);

  useEffect(() => {
    if (!chapterFindOpen) {
      return;
    }
    window.requestAnimationFrame(() => {
      chapterFindInputRef.current?.focus({ preventScroll: true });
      chapterFindInputRef.current?.select();
    });
  }, [chapterFindOpen]);

  useEffect(() => {
    if (!chapterFindMatches.length) {
      if (activeChapterFindIndex !== 0) {
        setActiveChapterFindIndex(0);
      }
      return;
    }
    if (activeChapterFindIndex >= chapterFindMatches.length) {
      setActiveChapterFindIndex(0);
    }
  }, [activeChapterFindIndex, chapterFindMatches.length]);

  useEffect(() => {
    lastChapterFindRevealKeyRef.current = "";
  }, [chapter?.chapter_index, chapterFindQuery]);

  useEffect(() => {
    if (!chapterFindOpen || !chapterFindMatches.length) {
      return;
    }
    const closestIndex = closestChapterFindMatchIndex(chapterFindMatches, visibleBlockRef.current);
    setActiveChapterFindIndex((current) => (current === closestIndex ? current : closestIndex));
  }, [chapter?.chapter_index, chapterFindMatches, chapterFindOpen]);

  const flushQueuedProgress = useCallback(() => {
    if (saveInFlightRef.current) {
      return;
    }
    const payload = saveQueuedPayloadRef.current;
    if (!payload) {
      return;
    }

    saveQueuedPayloadRef.current = null;
    saveInFlightRef.current = true;
    void saveProgress(payload)
      .catch((error) => {
        toast.error(errorMessage(error, "Failed to save progress."));
      })
      .finally(() => {
        saveInFlightRef.current = false;
        if (saveQueuedPayloadRef.current && !saveFlushScheduledRef.current) {
          saveFlushScheduledRef.current = true;
          window.setTimeout(() => {
            saveFlushScheduledRef.current = false;
            flushQueuedProgress();
          }, 0);
        }
      });
  }, []);

  const queueProgressSave = useCallback(
    (payload: SaveProgressInput) => {
      saveQueuedPayloadRef.current = payload;
      if (saveInFlightRef.current || saveFlushScheduledRef.current) {
        return;
      }
      saveFlushScheduledRef.current = true;
      window.setTimeout(() => {
        saveFlushScheduledRef.current = false;
        flushQueuedProgress();
      }, 0);
    },
    [flushQueuedProgress],
  );

  useEffect(() => {
    const requestId = searchRequestRef.current + 1;
    searchRequestRef.current = requestId;
    if (!searchOpen || !reader || trimmedSearchQuery.length < 2) {
      setSearchResults([]);
      setLoadingSearch(false);
      return;
    }

    setLoadingSearch(true);
    const timeout = window.setTimeout(() => {
      void searchBook(bookId, trimmedSearchQuery)
        .then((results) => {
          if (searchRequestRef.current === requestId) {
            setSearchResults(results);
          }
        })
        .catch((error) => {
          if (searchRequestRef.current === requestId) {
            toast.error(errorMessage(error, "Search failed."));
            setSearchResults([]);
          }
        })
        .finally(() => {
          if (searchRequestRef.current === requestId) {
            setLoadingSearch(false);
          }
        });
    }, 250);

    return () => window.clearTimeout(timeout);
  }, [bookId, reader, searchOpen, trimmedSearchQuery]);

  const saveCurrentProgress = useCallback(
    (options: SaveProgressOptions = {}) => {
      if (!reader || !activePart || pendingRestoreRef.current || pendingAudioResumeTimeRef.current !== null) {
        return;
      }
      const audio = audioRef.current;
      const blockIndex = options.blockIndex ?? visibleBlockRef.current ?? activePart.start_block_index;
      const progressChapter = reader.chapters.find(
        (chapter) => blockIndex >= chapter.start_block_index && blockIndex <= chapter.end_block_index,
      );
      const progressPart = progressChapter?.parts.find(
        (part) => blockIndex >= part.start_block_index && blockIndex <= part.end_block_index,
      );
      const progressPercent = readingProgressPercent(reader, chapter, blockIndex);
      const audioDuration =
        options.audioDurationSeconds ??
        (partAudio && audio && Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : null);
      const audioTime =
        options.audioTimeSeconds ??
        (partAudio && audio && Number.isFinite(audio.currentTime) ? audio.currentTime : null);
      const lastPlayingToken = partAlignment?.tokens.length
        ? options.lastPlayingToken === undefined
          ? activeTimedTokenRef.current
          : options.lastPlayingToken
        : null;
      const payload = {
        bookId,
        chapterIndex: progressChapter?.chapter_index ?? chapterIndex,
        partIndex: progressPart?.part_index ?? activePart.part_index,
        blockIndex,
        scrollRatio: currentScrollRatio(),
        progressPercent,
        audioTimeSeconds: audioTime,
        audioDurationSeconds: audioDuration,
        lastPlayingBlockIndex: lastPlayingToken?.block_index ?? null,
        lastPlayingTokenIndex: lastPlayingToken?.token_index ?? null,
      };

      if (options.immediate) {
        if (saveTimerRef.current) {
          window.clearTimeout(saveTimerRef.current);
          saveTimerRef.current = null;
        }
        queueProgressSave(payload);
        return;
      }

      if (saveTimerRef.current) {
        window.clearTimeout(saveTimerRef.current);
      }
      saveTimerRef.current = window.setTimeout(() => {
        saveTimerRef.current = null;
        queueProgressSave(payload);
      }, 900);
    },
    [activePart, bookId, chapter, chapterIndex, partAlignment, partAudio, queueProgressSave, reader],
  );

  useEffect(() => {
    tokenRefs.current = {};
    lastAutoScrollTokenRef.current = null;
    lastSelectionSeekKeyRef.current = "";
    activeTimedTokenRef.current = null;
    setActiveTokenKey(null);
    setWordContextMenu(null);
    setLookupDialog(null);
  }, [chapterIndex, partIndex]);

  useEffect(() => {
    if (!reader || !chapter) {
      return;
    }
    const blocks = Array.from(document.querySelectorAll<HTMLElement>("[data-reader-block]"));
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
        if (!visible) {
          return;
        }
        const blockIndex = Number((visible.target as HTMLElement).dataset.blockIndex);
        if (!Number.isFinite(blockIndex) || blockIndex === visibleBlockRef.current) {
          return;
        }
        visibleBlockRef.current = blockIndex;
        saveCurrentProgress({ blockIndex });
      },
      { rootMargin: "-35% 0px -50% 0px", threshold: [0.2, 0.6, 1] },
    );
    blocks.forEach((block) => observer.observe(block));
    return () => observer.disconnect();
  }, [chapter, reader, saveCurrentProgress]);

  useEffect(() => {
    const handleScroll = () => saveCurrentProgress();
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, [saveCurrentProgress]);

  useEffect(() => {
    if (!activePart && !currentQueueItem) {
      return;
    }
    let disposed = false;
    let unlisten: (() => void) | null = null;
    void listen<AudioGenerationProgress>("part-audio-progress", (event) => {
      const progress = event.payload;
      if (progress.book_id !== bookId) {
        return;
      }
      const matchesQueue =
        currentQueueItem &&
        progress.chapter_index === currentQueueItem.chapterIndex &&
        progress.part_index === currentQueueItem.partIndex;
      const matchesActivePart =
        !currentQueueItem &&
        activePart &&
        progress.chapter_index === chapterIndex &&
        progress.part_index === activePart.part_index;
      if (!matchesQueue && !matchesActivePart) {
        return;
      }
      setAudioProgress(progress);
    }).then((cleanup) => {
      if (disposed) {
        cleanup();
      } else {
        unlisten = cleanup;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [activePart, bookId, chapterIndex, currentQueueItem]);

  useEffect(() => {
    const audio = audioRef.current;
    audio?.pause();
    audio?.removeAttribute("src");
    audio?.load();
    setPartAudio(null);
    setAudioProgress(null);
    setAudioState({ currentTime: 0, duration: 0, playing: false });
    audioLookupPendingRef.current = false;

    if (!reader || !chapter || !activePart) {
      return;
    }

    let cancelled = false;
    audioLookupPendingRef.current = true;
    setLoadingAudio(true);
    void getPartAudio(bookId, chapterIndex, activePart.part_index)
      .then((payload) => {
        if (!cancelled) {
          setPartAudio(payload);
        }
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to check part audio.")))
      .finally(() => {
        if (!cancelled) {
          audioLookupPendingRef.current = false;
          setLoadingAudio(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activePart, bookId, chapter, chapterIndex, reader]);

  useEffect(() => {
    const audio = audioRef.current;
    wordPreviewAudioRef.current?.pause();
    wordPreviewEndTimeRef.current = null;
    if (!audio) {
      return;
    }
    if (!partAudio) {
      audio.pause();
      audio.removeAttribute("src");
      audio.load();
      setAudioState({ currentTime: 0, duration: 0, playing: false });
      return;
    }

    audio.pause();
    audio.src = convertFileSrc(partAudio.audio_path);
    audio.load();
    setAudioState({ currentTime: 0, duration: 0, playing: false });
  }, [partAudio]);

  useEffect(() => {
    setPartAlignment(null);
    setActiveTokenKey(null);
    lastAutoScrollTokenRef.current = null;
    alignmentLookupPendingRef.current = false;
    if (!partAudio?.alignment_available || !activePart) {
      return;
    }

    let cancelled = false;
    alignmentLookupPendingRef.current = true;
    setLoadingAlignment(true);
    void getPartAlignment(bookId, chapterIndex, activePart.part_index)
      .then((payload) => {
        if (!cancelled) {
          setPartAlignment(payload);
        }
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to load word sync.")))
      .finally(() => {
        if (!cancelled) {
          alignmentLookupPendingRef.current = false;
          setLoadingAlignment(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activePart, bookId, chapterIndex, partAudio]);

  const seekPartAudioToToken = useCallback(
    (blockIndex: number, tokenIndex: number) => {
      const audio = audioRef.current;
      if (!audio || !partAudio) {
        return false;
      }

      const key = timedTokenKey(blockIndex, tokenIndex);
      const timedToken = timedTokensByKey.get(key);
      const duration =
        Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : partAudio.duration_seconds;
      let targetTime: number | null = null;

      if (timedToken) {
        targetTime = timedToken.start_time;
        activeTimedTokenRef.current = timedToken;
      } else if (duration > 0) {
        let wordCount = 0;
        let targetWordIndex: number | null = null;
        let previousTimedWord: { wordIndex: number; token: TimedToken } | null = null;
        let nextTimedWord: { wordIndex: number; token: TimedToken } | null = null;

        for (const block of visibleBlocks) {
          if (block.kind !== "paragraph") {
            continue;
          }
          for (let index = 0; index < block.tokens.length; index += 1) {
            const token = block.tokens[index];
            if (!token.normalized_text) {
              continue;
            }
            const blockToken = timedTokensByKey.get(timedTokenKey(block.block_index, index));
            if (block.block_index === blockIndex && index === tokenIndex) {
              targetWordIndex = wordCount;
            } else if (blockToken && targetWordIndex === null) {
              previousTimedWord = { wordIndex: wordCount, token: blockToken };
            } else if (blockToken && targetWordIndex !== null && nextTimedWord === null) {
              nextTimedWord = { wordIndex: wordCount, token: blockToken };
            }
            wordCount += 1;
          }
        }

        if (targetWordIndex !== null && wordCount > 0) {
          if (previousTimedWord && nextTimedWord && nextTimedWord.token.start_time >= previousTimedWord.token.end_time) {
            const gapWords = nextTimedWord.wordIndex - previousTimedWord.wordIndex;
            const gapRatio = gapWords <= 0 ? 0 : (targetWordIndex - previousTimedWord.wordIndex) / gapWords;
            targetTime =
              previousTimedWord.token.end_time +
              (nextTimedWord.token.start_time - previousTimedWord.token.end_time) * gapRatio;
          } else {
            const ratio = wordCount === 1 ? 0 : targetWordIndex / (wordCount - 1);
            targetTime = duration * ratio;
          }
        }
      }

      if (targetTime === null) {
        return false;
      }

      wordPreviewEndTimeRef.current = null;
      wordPreviewAudioRef.current?.pause();
      audio.currentTime = clampNumber(targetTime, 0, duration > 0 ? duration : targetTime);
      setAudioState((current) => ({ ...current, currentTime: audio.currentTime }));
      setActiveTokenKey(key);
      return true;
    },
    [partAudio, timedTokensByKey, visibleBlocks],
  );

  useEffect(() => {
    const pending = pendingRestoreRef.current;
    if (!pending || !reader || !chapter || !activePart) {
      return;
    }

    const finishRestore = () => {
      pendingRestoreRef.current = null;
    };
    if (pending.kind === "bookmark") {
      if (loadingAudio || audioLookupPendingRef.current) {
        return;
      }
      if (partAudio?.alignment_available && (loadingAlignment || alignmentLookupPendingRef.current)) {
        return;
      }
      const key = timedTokenKey(pending.bookmark.block_index, pending.bookmark.token_index);
      const tokenElement = tokenRefs.current[key];
      visibleBlockRef.current = pending.bookmark.block_index;
      activeTimedTokenRef.current = null;
      setActiveTokenKey(key);
      seekPartAudioToToken(pending.bookmark.block_index, pending.bookmark.token_index);
      if (tokenElement) {
        scheduleScrollRestore(() => {
          tokenElement.scrollIntoView({ block: "center" });
        });
      } else {
        const target = chapter.blocks.find((block) => block.block_index >= pending.bookmark.block_index);
        scheduleScrollRestore(() => {
          if (target) {
            document.getElementById(blockDomId(target.block_index))?.scrollIntoView({ block: "center" });
          } else {
            window.scrollTo({ top: 0 });
          }
        });
      }
      finishRestore();
      return;
    }
    const progress = pending.progress;

    if (partAudio) {
      const audio = audioRef.current;
      const hasSavedAudioTime = progress.last_audio_time_seconds !== null;
      if (hasSavedAudioTime) {
        if (!audio || (audio.readyState < 1 && audioState.duration <= 0)) {
          return;
        }
        const duration =
          audioState.duration ||
          (Number.isFinite(audio.duration) ? audio.duration : 0) ||
          progress.last_audio_duration_seconds ||
          progress.last_audio_time_seconds ||
          0;
        const resumeTime = clampNumber(progress.last_audio_time_seconds ?? 0, 0, duration);
        pendingAudioResumeTimeRef.current = resumeTime;
        audio.pause();
        audio.currentTime = resumeTime;
        setAudioState({
          currentTime: resumeTime,
          duration,
          playing: false,
        });
        window.setTimeout(() => {
          if (pendingAudioResumeTimeRef.current !== resumeTime) {
            return;
          }
          if (Math.abs(audio.currentTime - resumeTime) > 0.25) {
            audio.pause();
            audio.currentTime = resumeTime;
            setAudioState({
              currentTime: resumeTime,
              duration,
              playing: false,
            });
          }
          pendingAudioResumeTimeRef.current = null;
        }, 500);
      }

      if (partAudio.alignment_available && (loadingAlignment || alignmentLookupPendingRef.current)) {
        return;
      }

      if (partAlignment?.tokens.length) {
        const savedToken = findSavedPlayingToken(partAlignment.tokens, progress);
        const timeToken =
          progress.last_audio_time_seconds === null ? null : timedTokenAtTime(partAlignment.tokens, progress.last_audio_time_seconds);
        const targetToken = savedToken ?? timeToken;
        if (targetToken) {
          const key = timedTokenKey(targetToken.block_index, targetToken.token_index);
          activeTimedTokenRef.current = targetToken;
          visibleBlockRef.current = targetToken.block_index;
          setActiveTokenKey(key);
          scheduleScrollRestore(() => {
            tokenRefs.current[key]?.scrollIntoView({ block: "center" });
          });
          finishRestore();
          return;
        }
      }

      restoreScrollPosition(progress, chapter);
      finishRestore();
      return;
    }

    if (loadingAudio || audioLookupPendingRef.current) {
      return;
    }

    restoreScrollPosition(progress, chapter);
    finishRestore();
  }, [activePart, audioState.duration, chapter, loadingAlignment, loadingAudio, partAlignment, partAudio, reader, seekPartAudioToToken]);

  useEffect(() => {
    if (!chapter || pendingRestoreRef.current) {
      return;
    }
    const pendingPartBlock = pendingPartBlockRef.current;
    pendingPartBlockRef.current = null;
    if (pendingPartBlock === null) {
      return;
    }
    const target = chapter.blocks.find((block) => block.block_index >= pendingPartBlock);
    if (!target) {
      window.scrollTo({ top: 0 });
      return;
    }
    visibleBlockRef.current = target.block_index;
    scheduleScrollRestore(() => {
      document.getElementById(blockDomId(target.block_index))?.scrollIntoView({ block: "center" });
    });
  }, [chapter]);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) {
      return;
    }
    const syncAudioState = () => {
      setAudioState({
        currentTime: audio.currentTime,
        duration: Number.isFinite(audio.duration) ? audio.duration : 0,
        playing: !audio.paused,
      });
    };
    audio.addEventListener("loadedmetadata", syncAudioState);
    audio.addEventListener("timeupdate", syncAudioState);
    audio.addEventListener("play", syncAudioState);
    audio.addEventListener("pause", syncAudioState);
    audio.addEventListener("ended", syncAudioState);
    return () => {
      audio.removeEventListener("loadedmetadata", syncAudioState);
      audio.removeEventListener("timeupdate", syncAudioState);
      audio.removeEventListener("play", syncAudioState);
      audio.removeEventListener("pause", syncAudioState);
      audio.removeEventListener("ended", syncAudioState);
    };
  }, []);

  useEffect(() => {
    const audio = audioRef.current;
    if (!audio || !partAudio) {
      return;
    }

    const savePlaybackPosition = (immediate: boolean) => {
      saveCurrentProgress({
        immediate,
        audioTimeSeconds: audio.currentTime,
        audioDurationSeconds: Number.isFinite(audio.duration) ? audio.duration : null,
        lastPlayingToken: activeTimedTokenRef.current,
      });
    };
    const scheduleImmediatePlaybackPosition = () => {
      if (audioSaveTimerRef.current) {
        window.clearTimeout(audioSaveTimerRef.current);
      }
      audioSaveTimerRef.current = window.setTimeout(() => {
        audioSaveTimerRef.current = null;
        savePlaybackPosition(true);
      }, 150);
    };
    const saveThrottledPlaybackPosition = () => {
      const now = Date.now();
      if (now - lastAudioSaveAtRef.current < 2000) {
        return;
      }
      lastAudioSaveAtRef.current = now;
      savePlaybackPosition(false);
    };
    audio.addEventListener("timeupdate", saveThrottledPlaybackPosition);
    audio.addEventListener("pause", scheduleImmediatePlaybackPosition);
    audio.addEventListener("seeked", scheduleImmediatePlaybackPosition);
    return () => {
      if (audioSaveTimerRef.current) {
        window.clearTimeout(audioSaveTimerRef.current);
        audioSaveTimerRef.current = null;
      }
      savePlaybackPosition(true);
      audio.removeEventListener("timeupdate", saveThrottledPlaybackPosition);
      audio.removeEventListener("pause", scheduleImmediatePlaybackPosition);
      audio.removeEventListener("seeked", scheduleImmediatePlaybackPosition);
    };
  }, [partAudio, saveCurrentProgress]);

  useEffect(() => {
    const audio = wordPreviewAudioRef.current;
    if (!audio) {
      return;
    }
    const stopAtEnd = () => {
      if (wordPreviewEndTimeRef.current !== null && audio.currentTime >= wordPreviewEndTimeRef.current) {
        wordPreviewEndTimeRef.current = null;
        audio.pause();
      }
    };
    const clearEnd = () => {
      wordPreviewEndTimeRef.current = null;
    };
    audio.addEventListener("timeupdate", stopAtEnd);
    audio.addEventListener("ended", clearEnd);
    return () => {
      audio.removeEventListener("timeupdate", stopAtEnd);
      audio.removeEventListener("ended", clearEnd);
    };
  }, []);

  useEffect(() => {
    const saveBeforeUnload = () => saveCurrentProgress({ immediate: true });
    window.addEventListener("beforeunload", saveBeforeUnload);
    return () => {
      saveBeforeUnload();
      window.removeEventListener("beforeunload", saveBeforeUnload);
      if (saveTimerRef.current) {
        window.clearTimeout(saveTimerRef.current);
        saveTimerRef.current = null;
      }
    };
  }, [saveCurrentProgress]);

  const selectChapter = useCallback((nextChapterIndex: number, nextPartIndex = 0, startBlockIndex?: number) => {
    pendingPartBlockRef.current = startBlockIndex ?? null;
    setPartIndex(nextPartIndex);
    setChapterIndex(nextChapterIndex);
    if (nextChapterIndex === chapterIndex && startBlockIndex !== undefined) {
      window.requestAnimationFrame(() => {
        document.getElementById(blockDomId(startBlockIndex))?.scrollIntoView({ block: "center" });
      });
    }
  }, [chapterIndex]);

  const restoreBookmark = useCallback(
    (savedBookmark: ReadingBookmark) => {
      const key = timedTokenKey(savedBookmark.block_index, savedBookmark.token_index);
      const tokenElement = tokenRefs.current[key];
      visibleBlockRef.current = savedBookmark.block_index;
      activeTimedTokenRef.current = null;
      setActiveTokenKey(key);
      seekPartAudioToToken(savedBookmark.block_index, savedBookmark.token_index);
      if (tokenElement) {
        scheduleScrollRestore(() => {
          tokenElement.scrollIntoView({ block: "center" });
        });
        return;
      }

      const target = chapter?.blocks.find((block) => block.block_index >= savedBookmark.block_index);
      scheduleScrollRestore(() => {
        if (target) {
          document.getElementById(blockDomId(target.block_index))?.scrollIntoView({ block: "center" });
        } else {
          window.scrollTo({ top: 0 });
        }
      });
    },
    [chapter, seekPartAudioToToken],
  );

  const jumpToBookmark = useCallback(() => {
    if (!bookmark) {
      return;
    }
    if (chapterIndex === bookmark.chapter_index && partIndex === bookmark.part_index && chapter) {
      restoreBookmark(bookmark);
      return;
    }
    pendingRestoreRef.current = { kind: "bookmark", bookmark };
    pendingPartBlockRef.current = bookmark.block_index;
    setPartIndex(bookmark.part_index);
    setChapterIndex(bookmark.chapter_index);
  }, [bookmark, chapter, chapterIndex, partIndex, restoreBookmark]);

  const jumpToMarkedWord = useCallback((location: MarkedWordLocation) => {
    const tokenElement = tokenRefs.current[location.key];
    visibleBlockRef.current = location.blockIndex;
    setActiveTokenKey(location.key);
    if (tokenElement) {
      tokenElement.scrollIntoView({ block: "center" });
      return;
    }
    document.getElementById(blockDomId(location.blockIndex))?.scrollIntoView({ block: "center" });
  }, []);

  const moveMarkedWord = useCallback(
    (direction: 1 | -1) => {
      if (!markedWordLocations.length) {
        return;
      }

      const activeIndex = activeTokenKey
        ? markedWordLocations.findIndex((location) => location.key === activeTokenKey)
        : -1;
      if (activeIndex !== -1) {
        const nextIndex = (activeIndex + direction + markedWordLocations.length) % markedWordLocations.length;
        jumpToMarkedWord(markedWordLocations[nextIndex]);
        return;
      }

      const scrollRatio = currentScrollRatio();
      const nextIndex =
        direction === 1
          ? markedWordLocations.findIndex((location) => location.ratio > scrollRatio)
          : findLastIndex(markedWordLocations, (location) => location.ratio < scrollRatio);
      const fallbackIndex = direction === 1 ? 0 : markedWordLocations.length - 1;
      jumpToMarkedWord(markedWordLocations[nextIndex === -1 ? fallbackIndex : nextIndex]);
    },
    [activeTokenKey, jumpToMarkedWord, markedWordLocations],
  );

  useEffect(() => {
    const handleMarkedWordShortcut = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || isEditableTarget(event.target)) {
        return;
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        moveMarkedWord(1);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        moveMarkedWord(-1);
      }
    };

    window.addEventListener("keydown", handleMarkedWordShortcut);
    return () => window.removeEventListener("keydown", handleMarkedWordShortcut);
  }, [moveMarkedWord]);

  const saveCurrentBookmark = useCallback(() => {
    if (!reader || !chapter || !activeTokenKey) {
      return;
    }
    const [blockPart, tokenPart] = activeTokenKey.split(":");
    const blockIndex = Number(blockPart);
    const tokenIndex = Number(tokenPart);
    if (!Number.isFinite(blockIndex) || !Number.isFinite(tokenIndex)) {
      return;
    }
    const block = chapter.blocks.find((item) => item.block_index === blockIndex);
    const token = block?.tokens[tokenIndex];
    if (!block || !token || !token.normalized_text) {
      toast.error("Pick a word first.");
      return;
    }

    void saveBookmark({
      bookId,
      chapterIndex: chapter.chapter_index,
      partIndex: activePart?.part_index ?? partIndex,
      blockIndex,
      tokenIndex,
      word: token.text,
      rootWord: token.root_text || token.normalized_text,
      scrollRatio: currentScrollRatio(),
      progressPercent: readingProgressPercent(reader, chapter, blockIndex),
    })
      .then((saved) => {
        setReader((current) => (current ? { ...current, bookmark: saved } : current));
        toast.success("Bookmark saved.");
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to save bookmark.")));
  }, [activePart?.part_index, activeTokenKey, bookId, chapter, partIndex, reader]);

  const toggleSearch = useCallback(() => {
    const nextOpen = !searchOpen;
    setSearchOpen(nextOpen);
    if (nextOpen) {
      setTocOpen(true);
    }
  }, [searchOpen]);

  const selectSearchResult = useCallback(
    (result: BookSearchResult) => {
      const resultChapter = reader?.chapters.find((item) => item.chapter_index === result.chapter_index);
      const resultPart = resultChapter?.parts.find(
        (part) => result.block_index >= part.start_block_index && result.block_index <= part.end_block_index,
      );
      setActiveSearchResult({ blockIndex: result.block_index, query: trimmedSearchQuery });
      selectChapter(result.chapter_index, resultPart?.part_index ?? 0, result.block_index);
    },
    [reader, selectChapter, trimmedSearchQuery],
  );

  useEffect(() => {
    if (!activeSearchResult || !chapter) {
      return;
    }
    if (!visibleBlocks.some((block) => block.block_index === activeSearchResult.blockIndex)) {
      return;
    }
    visibleBlockRef.current = activeSearchResult.blockIndex;
    scheduleScrollRestore(() => {
      document.getElementById(blockDomId(activeSearchResult.blockIndex))?.scrollIntoView({ block: "center" });
    });
  }, [activeSearchResult, chapter, visibleBlocks]);

  const openChapterFind = useCallback(() => {
    setChapterFindOpen(true);
    setActiveSearchResult(null);
    window.requestAnimationFrame(() => {
      chapterFindInputRef.current?.focus({ preventScroll: true });
      chapterFindInputRef.current?.select();
    });
  }, []);

  const closeChapterFind = useCallback(() => {
    setChapterFindOpen(false);
  }, []);

  const requestChapterFindScroll = useCallback(() => {
    setChapterFindScrollRequest((current) => current + 1);
  }, []);

  const moveChapterFind = useCallback(
    (direction: 1 | -1) => {
      if (!chapterFindMatches.length) {
        return;
      }
      setActiveChapterFindIndex((current) => (current + direction + chapterFindMatches.length) % chapterFindMatches.length);
      requestChapterFindScroll();
    },
    [chapterFindMatches.length, requestChapterFindScroll],
  );

  const confirmChapterFind = useCallback(() => {
    if (!chapterFindMatches.length) {
      return;
    }
    const revealKey = `${chapter?.chapter_index ?? ""}:${chapterFindQuery}:${activeChapterFindIndex}`;
    if (lastChapterFindRevealKeyRef.current === revealKey) {
      const nextIndex = (activeChapterFindIndex + 1) % chapterFindMatches.length;
      lastChapterFindRevealKeyRef.current = `${chapter?.chapter_index ?? ""}:${chapterFindQuery}:${nextIndex}`;
      setActiveChapterFindIndex(nextIndex);
      requestChapterFindScroll();
      return;
    }
    lastChapterFindRevealKeyRef.current = revealKey;
    requestChapterFindScroll();
  }, [activeChapterFindIndex, chapter?.chapter_index, chapterFindMatches.length, chapterFindQuery, requestChapterFindScroll]);

  useEffect(() => {
    if (!chapterFindScrollRequest || !chapterFindOpen || !activeChapterFindMatch || !chapter || !activeChapter) {
      return;
    }
    const targetPart = activeChapter.parts.find(
      (part) =>
        activeChapterFindMatch.blockIndex >= part.start_block_index &&
        activeChapterFindMatch.blockIndex <= part.end_block_index,
    );
    if (targetPart && targetPart.part_index !== partIndex) {
      setPartIndex(targetPart.part_index);
      return;
    }
    if (!visibleBlocks.some((block) => block.block_index === activeChapterFindMatch.blockIndex)) {
      return;
    }
    visibleBlockRef.current = activeChapterFindMatch.blockIndex;
    scheduleScrollRestore(() => {
      document.getElementById(blockDomId(activeChapterFindMatch.blockIndex))?.scrollIntoView({ block: "center" });
    });
  }, [activeChapter, activeChapterFindMatch, chapter, chapterFindOpen, chapterFindScrollRequest, partIndex, visibleBlocks]);

  const generateCurrentPartAudio = useCallback(
    async (regenerate: boolean) => {
      if (!activePart) {
        return;
      }
      setAudioProgress({
        book_id: bookId,
        chapter_index: chapterIndex,
        part_index: activePart.part_index,
        completed: 0,
        total: partParagraphCount,
        percent: 0,
        stage: "queued",
      });
      setGeneratingAudio(true);
      try {
        const payload = await generatePartAudio(bookId, chapterIndex, activePart.part_index, regenerate);
        setPartAudio(payload);
        if (payload.alignment_error) {
          toast.warning(regenerate ? "Audio regenerated, word sync failed." : "Audio generated, word sync failed.", {
            description: payload.alignment_error,
          });
        } else {
          toast.success(payload.alignment_available ? "Audio and word sync ready." : regenerate ? "Audio regenerated." : "Audio generated.");
        }
      } catch (error) {
        toast.error(errorMessage(error, "Audio generation failed."));
      } finally {
        setGeneratingAudio(false);
        setAudioProgress(null);
      }
    },
    [activePart, bookId, chapterIndex, partParagraphCount],
  );

  const startAudioQueue = useCallback(
    async (startChapterIndex: number, mode: AudioQueueMode) => {
      if (!reader || generatingAudio) {
        return;
      }
      setChapterContextMenu(null);
      const runId = audioQueueRunRef.current + 1;
      audioQueueRunRef.current = runId;
      setGeneratingAudio(true);
      setPreparingAudioQueue(true);
      setAudioProgress(null);
      setAudioQueue(null);
      try {
        const targetChapters =
          mode === "chapter"
            ? reader.chapters.filter((item) => item.chapter_index === startChapterIndex)
            : reader.chapters.filter((item) => item.chapter_index >= startChapterIndex);
        const chapterPayloads = await Promise.all(
          targetChapters.map(async (summary) => ({
            summary,
            payload: await getChapter(bookId, summary.chapter_index),
          })),
        );
        if (audioQueueRunRef.current !== runId) {
          return;
        }
        const items = chapterPayloads.flatMap(({ summary, payload }) =>
          summary.parts.map((part) => {
            const effort = chapterPartEffort(payload, part.start_block_index, part.end_block_index);
            return {
              chapterIndex: summary.chapter_index,
              chapterTitle: summary.title,
              partIndex: part.part_index,
              partTitle: part.title,
              paragraphCount: effort.paragraphCount,
              effort: effort.characters,
            };
          }),
        );
        const totalEffort = items.reduce((total, item) => total + item.effort, 0);
        if (!items.length || totalEffort <= 0) {
          toast.info("No readable parts found for audio generation.");
          return;
        }
        setPreparingAudioQueue(false);
        setAudioQueue({
          mode,
          items,
          currentIndex: 0,
          completedParts: 0,
          completedEffort: 0,
          totalEffort,
          startedAt: Date.now(),
        });

        let completedEffort = 0;
        for (let index = 0; index < items.length; index += 1) {
          if (audioQueueRunRef.current !== runId) {
            return;
          }
          const item = items[index];
          setAudioProgress({
            book_id: bookId,
            chapter_index: item.chapterIndex,
            part_index: item.partIndex,
            completed: 0,
            total: item.paragraphCount,
            percent: 0,
            stage: "queued",
          });
          setAudioQueue((current) =>
            current && audioQueueRunRef.current === runId
              ? {
                  ...current,
                  currentIndex: index,
                  completedParts: index,
                  completedEffort,
                }
              : current,
          );
          const payload = await generatePartAudio(bookId, item.chapterIndex, item.partIndex, false);
          const selectedPart = selectedPartRef.current;
          if (payload.chapter_index === selectedPart.chapterIndex && payload.part_index === selectedPart.partIndex) {
            setPartAudio(payload);
          }
          completedEffort += item.effort;
          setAudioQueue((current) =>
            current && audioQueueRunRef.current === runId
              ? {
                  ...current,
                  completedParts: index + 1,
                  completedEffort,
                }
              : current,
          );
        }
        toast.success("Audio queue complete.", {
          description: `${items.length} ${items.length === 1 ? "part" : "parts"} generated.`,
        });
      } catch (error) {
        toast.error(errorMessage(error, "Audio queue failed."));
      } finally {
        if (audioQueueRunRef.current === runId) {
          setGeneratingAudio(false);
          setPreparingAudioQueue(false);
          setAudioQueue(null);
          setAudioProgress(null);
        }
      }
    },
    [bookId, generatingAudio, reader],
  );

  const stopWordPreview = useCallback(() => {
    wordPreviewEndTimeRef.current = null;
    wordPreviewAudioRef.current?.pause();
  }, []);

  const syncCurrentPartAlignment = useCallback(
    async (regenerate: boolean) => {
      if (!activePart || !partAudio) {
        return;
      }
      setSyncingAlignment(true);
      try {
        const payload = await syncPartAlignment(bookId, chapterIndex, activePart.part_index, regenerate);
        setPartAlignment(payload);
        setPartAudio((current) =>
          current &&
          current.book_id === bookId &&
          current.chapter_index === chapterIndex &&
          current.part_index === activePart.part_index
            ? { ...current, alignment_available: true, alignment_error: null }
            : current,
        );
        toast.success(regenerate ? "Word sync refreshed." : "Word sync ready.");
      } catch (error) {
        const message = errorMessage(error, "Word sync failed.");
        setPartAudio((current) =>
          current &&
          current.book_id === bookId &&
          current.chapter_index === chapterIndex &&
          current.part_index === activePart.part_index
            ? { ...current, alignment_available: false, alignment_error: message }
            : current,
        );
        toast.error(message);
      } finally {
        setSyncingAlignment(false);
      }
    },
    [activePart, bookId, chapterIndex, partAudio],
  );

  const toggleAudioPlay = useCallback(() => {
    const audio = audioRef.current;
    if (!audio || !partAudio) {
      return;
    }
    if (audio.paused) {
      stopWordPreview();
      void audio.play().catch((error) => toast.error(errorMessage(error, "Failed to play audio.")));
    } else {
      audio.pause();
    }
  }, [partAudio, stopWordPreview]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && !event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        if (chapterFindOpen) {
          closeChapterFind();
        } else {
          openChapterFind();
        }
        return;
      }
      if (event.code !== "Space" || event.repeat || shouldIgnorePlaybackShortcut(event.target)) {
        return;
      }
      event.preventDefault();
      toggleAudioPlay();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [chapterFindOpen, closeChapterFind, openChapterFind, toggleAudioPlay]);

  useEffect(() => {
    if (!partAlignment?.tokens.length) {
      activeTimedTokenRef.current = null;
      setActiveTokenKey(null);
      return;
    }
    const activeTokenIndex = partAlignment.tokens.findIndex(
      (token) => audioState.currentTime >= token.start_time && audioState.currentTime < token.end_time,
    );
    const activeToken =
      activeTokenIndex >= 0
        ? partAlignment.tokens[activeTokenIndex]
        : [...partAlignment.tokens].reverse().find((token) => token.start_time <= audioState.currentTime);
    activeTimedTokenRef.current = activeToken ?? null;
    const key = activeToken ? timedTokenKey(activeToken.block_index, activeToken.token_index) : null;
    setActiveTokenKey((current) => (current === key ? current : key));
    if (audioState.playing && activeToken) {
      saveCurrentProgress({
        audioTimeSeconds: audioState.currentTime,
        audioDurationSeconds: audioState.duration,
        blockIndex: activeToken.block_index,
        lastPlayingToken: activeToken,
      });
    }

    if (!audioState.playing || !key || lastAutoScrollTokenRef.current === key) {
      return;
    }

    const element = tokenRefs.current[key];
    if (!element) {
      return;
    }
    const bounds = element.getBoundingClientRect();
    const player = document.querySelector<HTMLElement>(".reader-audio");
    const headerInset = 88;
    const bottomInset = (player?.offsetHeight ?? 0) + 48;
    const visibleBottom = window.innerHeight - bottomInset;
    if (bounds.bottom > visibleBottom || bounds.top < headerInset) {
      lastAutoScrollTokenRef.current = key;
      element.scrollIntoView({ block: "start", behavior: "smooth" });
    }
  }, [audioState.currentTime, audioState.duration, audioState.playing, partAlignment, saveCurrentProgress]);

  const toggleContextMenuWordlist = useCallback(() => {
    if (!wordContextMenu) {
      return;
    }
    const menu = wordContextMenu;
    setWordContextMenu(null);
    if (wordlistRoots.has(menu.rootWord)) {
      void deleteWordlistEntry(menu.rootWord)
        .then(() => {
          setWordlistEntries((current) => current.filter((entry) => entry.root_word !== menu.rootWord));
          toast.success("Removed from word list.");
        })
        .catch((error) => toast.error(errorMessage(error, "Failed to remove word.")));
      return;
    }
    void addWordlistEntry({
      bookId,
      chapterIndex: menu.chapterIndex,
      blockIndex: menu.blockIndex,
      tokenIndex: menu.tokenIndex,
      word: menu.word,
      rootWord: menu.rootWord,
      context: menu.context,
      cefrLevel: menu.cefrLevel,
    })
      .then((entry) => {
        setWordlistEntries((current) => upsertWordlistEntry(current, entry));
        toast.success("Added to word list.", {
          description: entry.definition ? undefined : "Looking up the definition in the background.",
        });
      })
      .catch((error) => toast.error(errorMessage(error, "Failed to add word.")));
  }, [bookId, wordContextMenu, wordlistRoots]);

  useEffect(() => {
    if (!wordContextMenu) {
      return;
    }
    const close = () => setWordContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    document.addEventListener("click", close);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("click", close);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [wordContextMenu]);

  useEffect(() => {
    if (!chapterContextMenu) {
      return;
    }
    const close = () => setChapterContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    document.addEventListener("click", close);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("click", close);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [chapterContextMenu]);

  useEffect(() => {
    if (!lookupDialog) {
      return;
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setLookupDialog(null);
      }
    };
    const closeOnOutsidePointer = (event: MouseEvent) => {
      const target = event.target;
      if (target instanceof Element && target.closest(".lookup-dialog")) {
        return;
      }
      setLookupDialog(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    document.addEventListener("mousedown", closeOnOutsidePointer);
    return () => {
      window.removeEventListener("keydown", closeOnEscape);
      document.removeEventListener("mousedown", closeOnOutsidePointer);
    };
  }, [lookupDialog]);

  const openWordContextMenu = useCallback(
    (
      token: ChapterPayload["blocks"][number]["tokens"][number],
      blockText: string,
      blockIndex: number,
      tokenIndex: number,
      target: HTMLElement,
      clientX: number,
      clientY: number,
    ) => {
      if (!token.normalized_text) {
        return;
      }
      lastContextMenuAtRef.current = Date.now();
      const bounds = target.getBoundingClientRect();
      const menuWidth = 176;
      const menuHeight = 92;
      const dialogWidth = 380;
      setWordContextMenu({
        word: token.text,
        rootWord: token.root_text || token.normalized_text,
        context: blockText,
        cefrLevel: token.cefr_level || "",
        chapterIndex,
        blockIndex,
        tokenIndex,
        x: clampNumber(clientX, 12, Math.max(12, window.innerWidth - menuWidth - 12)),
        y: clampNumber(clientY, 72, Math.max(72, window.innerHeight - menuHeight - 12)),
        lookupX: clampNumber(bounds.left + bounds.width / 2, 12, Math.max(12, window.innerWidth - dialogWidth - 12)),
        lookupY: clampNumber(bounds.bottom + 8, 72, Math.max(72, window.innerHeight - 120)),
      });
    },
    [chapterIndex],
  );

  const openChapterContextMenu = useCallback(
    (targetChapterIndex: number, title: string, event: ReactMouseEvent<HTMLElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const menuWidth = 220;
      const menuHeight = 96;
      setWordContextMenu(null);
      setChapterContextMenu({
        chapterIndex: targetChapterIndex,
        title,
        x: clampNumber(event.clientX, 12, Math.max(12, window.innerWidth - menuWidth - 12)),
        y: clampNumber(event.clientY, 72, Math.max(72, window.innerHeight - menuHeight - 12)),
      });
    },
    [],
  );

  const lookupContextMenuWord = useCallback(() => {
    if (!wordContextMenu) {
      return;
    }
    const requestId = lookupRequestRef.current + 1;
    lookupRequestRef.current = requestId;
    const menu = wordContextMenu;
    const cachedEntry = wordlistEntries.find(
      (entry) =>
        entry.definition &&
        hasWordlistAiEnrichment(entry) &&
        isWordlistEntryAtToken(entry, bookId, menu.chapterIndex, menu.blockIndex, menu.tokenIndex),
    );
    setWordContextMenu(null);
    if (cachedEntry) {
      setLookupDialog({
        word: menu.word,
        x: menu.lookupX,
        y: menu.lookupY,
        loading: false,
        error: null,
        result: dictionaryLookupFromWordlistEntry(cachedEntry, menu.word),
      });
      return;
    }
    setLookupDialog({
      word: menu.word,
      x: menu.lookupX,
      y: menu.lookupY,
      loading: true,
      error: null,
      result: null,
    });
    dictionaryAudioRef.current?.pause();
    void lookupWord(menu.word, `${menu.word}\n\n${menu.context}`, menu.cefrLevel, menu.rootWord)
      .then((result) => {
        if (lookupRequestRef.current !== requestId) {
          return;
        }
        setLookupDialog({
          word: menu.word,
          x: menu.lookupX,
          y: menu.lookupY,
          loading: false,
          error: null,
          result,
        });
      })
      .catch((error) => {
        if (lookupRequestRef.current !== requestId) {
          return;
        }
        setLookupDialog({
          word: menu.word,
          x: menu.lookupX,
          y: menu.lookupY,
          loading: false,
          error: errorMessage(error, "Lookup failed."),
          result: null,
        });
      });
  }, [bookId, wordContextMenu, wordlistEntries]);

  const openImage = useCallback((image: ReaderImage) => {
    setImageZoom(1);
    setDialogImage(image);
  }, []);

  const closeImage = useCallback(() => {
    setDialogImage(null);
    setImageZoom(1);
  }, []);

  useEffect(() => {
    if (!dialogImage) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeImage();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [closeImage, dialogImage]);

  const seekAudio = useCallback((time: number) => {
    const audio = audioRef.current;
    if (!audio) {
      return;
    }
    stopWordPreview();
    audio.currentTime = Math.max(0, Math.min(time, Number.isFinite(audio.duration) ? audio.duration : time));
    setAudioState((current) => ({ ...current, currentTime: audio.currentTime }));
  }, [stopWordPreview]);

  const previewTimedToken = useCallback(
    (blockIndex: number, tokenIndex: number) => {
      const audio = audioRef.current;
      if (!audio || !partAudio) {
        return;
      }
      const key = timedTokenKey(blockIndex, tokenIndex);
      const token = timedTokensByKey.get(key);
      if (!token) {
        return;
      }
      stopWordPreview();
      audio.pause();
      audio.currentTime = Math.max(0, token.start_time);
      setAudioState((current) => ({ ...current, currentTime: audio.currentTime }));
      setActiveTokenKey(key);

      const preview = wordPreviewAudioRef.current;
      if (!preview) {
        return;
      }
      preview.pause();
      preview.src = audio.src || convertFileSrc(partAudio.audio_path);
      wordPreviewEndTimeRef.current = Math.max(token.end_time, token.start_time + 0.15);
      preview.currentTime = token.start_time;
      void preview.play().catch((error) => toast.error(errorMessage(error, "Failed to play word preview.")));
    },
    [partAudio, stopWordPreview, timedTokensByKey],
  );

  const seekToRelativeToken = useCallback(
    (blockIndex: number, tokenIndex: number) => {
      const key = timedTokenKey(blockIndex, tokenIndex);
      const audio = audioRef.current;
      const duration =
        audio && partAudio
          ? Number.isFinite(audio.duration) && audio.duration > 0
            ? audio.duration
            : partAudio.duration_seconds
          : 0;
      if (!audio || !partAudio || duration <= 0) {
        visibleBlockRef.current = blockIndex;
        setActiveTokenKey(key);
        scheduleScrollRestore(() => {
          tokenRefs.current[key]?.scrollIntoView({ block: "center" });
        });
        return;
      }

      let wordCount = 0;
      let targetWordIndex: number | null = null;
      let previousTimedWord: { wordIndex: number; token: TimedToken } | null = null;
      let nextTimedWord: { wordIndex: number; token: TimedToken } | null = null;
      for (const block of visibleBlocks) {
        if (block.kind !== "paragraph") {
          continue;
        }
        for (let index = 0; index < block.tokens.length; index += 1) {
          const token = block.tokens[index];
          if (!token.normalized_text) {
            continue;
          }
          const timedToken = timedTokensByKey.get(timedTokenKey(block.block_index, index));
          if (block.block_index === blockIndex && index === tokenIndex) {
            targetWordIndex = wordCount;
          } else if (timedToken && targetWordIndex === null) {
            previousTimedWord = { wordIndex: wordCount, token: timedToken };
          } else if (timedToken && targetWordIndex !== null && nextTimedWord === null) {
            nextTimedWord = { wordIndex: wordCount, token: timedToken };
          }
          wordCount += 1;
        }
      }
      if (targetWordIndex === null || wordCount === 0) {
        return;
      }

      let targetTime: number;
      if (previousTimedWord && nextTimedWord && nextTimedWord.token.start_time >= previousTimedWord.token.end_time) {
        const gapWords = nextTimedWord.wordIndex - previousTimedWord.wordIndex;
        const gapRatio = gapWords <= 0 ? 0 : (targetWordIndex - previousTimedWord.wordIndex) / gapWords;
        targetTime = previousTimedWord.token.end_time + (nextTimedWord.token.start_time - previousTimedWord.token.end_time) * gapRatio;
      } else {
        const ratio = wordCount === 1 ? 0 : targetWordIndex / (wordCount - 1);
        targetTime = duration * ratio;
      }

      stopWordPreview();
      audio.currentTime = clampNumber(targetTime, 0, duration);
      setAudioState((current) => ({ ...current, currentTime: audio.currentTime }));
      setActiveTokenKey(key);
    },
    [partAudio, stopWordPreview, timedTokensByKey, visibleBlocks],
  );

  const playFromToken = useCallback(
    (blockIndex: number, tokenIndex: number) => {
      const audio = audioRef.current;
      if (!audio || !partAudio) {
        return;
      }
      const key = timedTokenKey(blockIndex, tokenIndex);
      const token = timedTokensByKey.get(key);
      if (token) {
        stopWordPreview();
        audio.currentTime = Math.max(0, token.start_time);
        setAudioState((current) => ({ ...current, currentTime: audio.currentTime }));
        setActiveTokenKey(key);
      } else {
        seekToRelativeToken(blockIndex, tokenIndex);
      }
      void audio.play().catch((error) => toast.error(errorMessage(error, "Failed to play audio.")));
    },
    [partAudio, seekToRelativeToken, stopWordPreview, timedTokensByKey],
  );

  const previewToken = useCallback(
    (blockIndex: number, tokenIndex: number) => {
      const key = timedTokenKey(blockIndex, tokenIndex);
      if (timedTokensByKey.has(key)) {
        previewTimedToken(blockIndex, tokenIndex);
        return;
      }
      const audio = audioRef.current;
      if (audio) {
        audio.pause();
      }
      seekToRelativeToken(blockIndex, tokenIndex);
    },
    [previewTimedToken, seekToRelativeToken, timedTokensByKey],
  );

  useEffect(() => {
    const seekSelectedWord = (event: KeyboardEvent | MouseEvent) => {
      let mouseTokenElement: HTMLElement | null = null;
      if (event instanceof MouseEvent) {
        if (event.button !== 0) {
          return;
        }
        const target = event.target;
        mouseTokenElement = target instanceof Element ? target.closest<HTMLElement>("[data-timed-token-key]") : null;
        if (!mouseTokenElement) {
          lastSelectionSeekKeyRef.current = "";
          return;
        }
      }
      if (Date.now() - lastContextMenuAtRef.current < 500) {
        return;
      }
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed || selection.rangeCount !== 1) {
        lastSelectionSeekKeyRef.current = "";
        return;
      }
      const text = selection.toString().trim();
      if (!text || /\s/.test(text)) {
        lastSelectionSeekKeyRef.current = "";
        return;
      }
      const selectedNode = selection.anchorNode;
      const selectedElement =
        selectedNode instanceof HTMLElement ? selectedNode : selectedNode?.parentElement ?? null;
      const tokenElement = mouseTokenElement ?? selectedElement?.closest<HTMLElement>("[data-timed-token-key]");
      if (!tokenElement || tokenElement.textContent?.trim() !== text) {
        return;
      }
      const blockIndex = Number(tokenElement.dataset.blockIndex);
      const tokenIndex = Number(tokenElement.dataset.tokenIndex);
      const key = tokenElement.dataset.timedTokenKey ?? "";
      if (!Number.isFinite(blockIndex) || !Number.isFinite(tokenIndex)) {
        return;
      }
      const seekKey = `${key}:${text}`;
      if (lastSelectionSeekKeyRef.current === seekKey) {
        return;
      }
      lastSelectionSeekKeyRef.current = seekKey;
      setActiveTokenKey(key);
    };

    document.addEventListener("mouseup", seekSelectedWord);
    document.addEventListener("keyup", seekSelectedWord);
    return () => {
      document.removeEventListener("mouseup", seekSelectedWord);
      document.removeEventListener("keyup", seekSelectedWord);
    };
  }, []);

  const audioGenerationPercent = Math.round(audioProgress?.percent ?? 0);
  const queuePartPercent =
    currentQueueItem &&
    audioProgress?.chapter_index === currentQueueItem.chapterIndex &&
    audioProgress.part_index === currentQueueItem.partIndex
      ? clampNumber(audioProgress.percent, 0, 100)
      : 0;
  const audioQueuePercent = audioQueue
    ? clampNumber(
        ((audioQueue.completedEffort + (currentQueueItem?.effort ?? 0) * (queuePartPercent / 100)) /
          audioQueue.totalEffort) *
          100,
        0,
        100,
      )
    : 0;
  const audioQueueEtaSeconds =
    audioQueue && audioQueuePercent >= 1
      ? ((Date.now() - audioQueue.startedAt) / 1000) * ((100 - audioQueuePercent) / audioQueuePercent)
      : null;
  const navAudioProgress = audioQueue
    ? {
        label: audioQueue.mode === "chapter" ? "Chapter audio" : "Book audio",
        detail: `${audioQueue.completedParts}/${audioQueue.items.length} parts generated`,
        percent: audioQueuePercent,
        etaSeconds: audioQueueEtaSeconds,
        current: currentQueueItem
          ? `${formatChapterTitle(currentQueueItem.chapterTitle)}${audioQueue.items.length > 1 ? `, ${currentQueueItem.partTitle}` : ""}`
          : "Finishing queue",
      }
    : preparingAudioQueue
      ? {
          label: "Audio queue",
          detail: "Preparing parts",
          percent: 0,
          etaSeconds: null,
          current: "Measuring chapter text",
        }
      : generatingAudio
        ? {
            label: "Part audio",
            detail: "0/1 parts generated",
            percent: audioGenerationPercent,
            etaSeconds: null,
            current: activePart?.title ?? "Current part",
          }
        : null;
  const audioGenerationStatus =
    audioQueue
      ? `Generating audio, ${Math.round(audioQueuePercent)}%, ${audioQueue.completedParts} of ${audioQueue.items.length} parts complete`
      : audioProgress && audioProgress.total > 0
      ? `Generating audio, ${audioGenerationPercent}%, ${audioProgress.completed} of ${audioProgress.total} paragraphs complete`
      : "Generating audio";

  return (
    <TooltipProvider>
      <main className="min-h-screen bg-reader text-foreground">
        <header className="sticky top-0 z-20 border-b bg-reader/95 backdrop-blur">
          <div className="grid h-16 grid-cols-[auto_1fr_auto] items-center gap-3 px-4">
            <div className="flex items-center gap-2">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant="ghost" size="icon" onClick={onBack} aria-label="Back to library">
                    <ChevronLeftIcon />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Library</TooltipContent>
              </Tooltip>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  const nextOpen = !tocOpen;
                  setTocOpen(nextOpen);
                  if (!nextOpen) {
                    setSearchOpen(false);
                  }
                }}
                aria-label="Toggle chapters"
              >
                <MenuIcon />
              </Button>
            </div>
            <div className="min-w-0">
              <h1 className="truncate text-sm font-semibold">{reader?.title || "Opening book"}</h1>
              <p className="truncate text-xs text-muted-foreground">{activeChapter?.title || chapter?.title || "Chapter"}</p>
            </div>
            <div className="flex items-center justify-end gap-2">
              {navAudioProgress ? <AudioNavProgress progress={navAudioProgress} /> : null}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button variant={searchOpen ? "secondary" : "ghost"} size="icon" onClick={toggleSearch} aria-label="Search book">
                    <SearchIcon />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Search</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant={bookmark && bookmarkedTokenKey === activeTokenKey ? "secondary" : "ghost"}
                    size="icon"
                    onClick={saveCurrentBookmark}
                    disabled={!activeTokenKey}
                    aria-label="Save bookmark"
                  >
                    <BookmarkIcon />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{activeTokenKey ? "Save selected word" : "Select a word first"}</TooltipContent>
              </Tooltip>
              {bookmark ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button variant="ghost" size="icon" onClick={jumpToBookmark} aria-label={`Go to ${bookmark.word}`}>
                      <BookMarkedIcon />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{`Go to ${bookmark.word}`}</TooltipContent>
                </Tooltip>
              ) : null}
              <Badge variant="secondary">{reader ? `${Math.round(reader.progress.progress_percent)}%` : "..."}</Badge>
            </div>
          </div>
        </header>

        <div className={cn("reader-shell", tocOpen && "has-toc")}>
          {tocOpen ? (
            <aside className="toc-panel">
              <ScrollArea className="h-[calc(100vh-4rem)]">
                <div className="flex flex-col gap-2 p-4">
                  {searchOpen ? (
                    <SearchPanel
                      inputRef={searchInputRef}
                      query={searchQuery}
                      results={searchResults}
                      loading={loadingSearch}
                      activeBlockIndex={activeSearchResult?.blockIndex ?? null}
                      onQueryChange={(query) => {
                        setSearchQuery(query);
                        setActiveSearchResult(null);
                      }}
                      onSelect={selectSearchResult}
                      onClose={() => {
                        setSearchOpen(false);
                        setActiveSearchResult(null);
                      }}
                    />
                  ) : null}
                  <p className="px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">Chapters</p>
                  {reader?.chapters.map((item) => (
                    <div key={item.chapter_index} className="toc-group">
                      <button
                        className={cn("toc-item", item.chapter_index === chapterIndex && "active")}
                        type="button"
                        onClick={() => selectChapter(item.chapter_index, 0, item.start_block_index)}
                        onContextMenu={(event) => openChapterContextMenu(item.chapter_index, item.title, event)}
                      >
                        <span className="toc-title">{formatChapterTitle(item.title)}</span>
                        {item.parts.length > 1 ? <span className="toc-count">{item.parts.length}</span> : null}
                      </button>
                      {item.chapter_index === chapterIndex && item.parts.length > 1 ? (
                        <div className="toc-parts" aria-label={`${item.title} parts`}>
                          {item.parts.map((part) => (
                            <button
                              key={part.part_index}
                              className={cn("toc-part", part.part_index === partIndex && "active")}
                              type="button"
                              onClick={() => selectChapter(item.chapter_index, part.part_index, part.start_block_index)}
                            >
                              {part.title}
                            </button>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </aside>
          ) : null}

          <article className="reader-page">
            {loading || !reader || !chapter ? (
              <ReaderSkeleton />
            ) : (
              <>
                <div className="reader-heading">
                  <h2>{chapter.title}</h2>
                  <div className="part-stats" aria-label="Part statistics">
                    {activePart && activeChapter && activeChapter.parts.length > 1 ? <span className="part-label">{activePart.title}</span> : null}
                    <span>{partWordCount.toLocaleString()} words</span>
                    <div className="color-mode-toggle" aria-label="Word color mode">
                      {(["frequency", "cefr"] as const).map((mode) => (
                        <button
                          key={mode}
                          className={cn(colorMode === mode && "active")}
                          type="button"
                          onClick={() => setColorMode(mode)}
                          aria-pressed={colorMode === mode}
                        >
                          {mode === "frequency" ? "Frequency" : "CEFR"}
                        </button>
                      ))}
                    </div>
                    {partAudio ? (
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <Button size="sm" disabled={generatingAudio || loadingAudio || syncingAlignment} aria-label={generatingAudio ? audioGenerationStatus : undefined}>
                            <AudioLinesIcon data-icon="inline-start" />
                            {generatingAudio ? "Generating audio" : "Regenerate audio"}
                          </Button>
                        </AlertDialogTrigger>
                        <AlertDialogContent>
                          <AlertDialogHeader>
                            <AlertDialogTitle>Regenerate audio?</AlertDialogTitle>
                            <AlertDialogDescription>
                              The current generated audio for this part will be replaced.
                            </AlertDialogDescription>
                          </AlertDialogHeader>
                          <AlertDialogFooter>
                            <AlertDialogCancel>Cancel</AlertDialogCancel>
                            <AlertDialogAction onClick={() => void generateCurrentPartAudio(true)}>
                              Regenerate
                            </AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    ) : (
                      <Button
                        size="sm"
                        disabled={generatingAudio || loadingAudio || !activePart}
                        onClick={() => void generateCurrentPartAudio(false)}
                        aria-label={generatingAudio ? audioGenerationStatus : undefined}
                      >
                        <AudioLinesIcon data-icon="inline-start" />
                        {generatingAudio ? "Generating audio" : "Generate audio"}
                      </Button>
                    )}
                    {partAudio ? (
                      <Button
                        size="sm"
                        variant={partAlignment ? "secondary" : "default"}
                        disabled={syncingAlignment || generatingAudio || loadingAlignment}
                        onClick={() => void syncCurrentPartAlignment(Boolean(partAlignment))}
                      >
                        <AudioLinesIcon data-icon="inline-start" />
                        {syncingAlignment ? "Syncing words" : partAlignment ? "Resync words" : "Sync words"}
                      </Button>
                    ) : null}
                    {loadingAlignment ? <span>Loading word sync</span> : null}
                    {!loadingAlignment && !partAlignment && partAudio?.alignment_error ? <span>Word sync unavailable</span> : null}
                  </div>
                </div>
                {chapterFindOpen ? (
                  <ChapterFindBar
                    inputRef={chapterFindInputRef}
                    query={chapterFindQuery}
                    activeIndex={chapterFindMatches.length ? activeChapterFindIndex : -1}
                    matchCount={chapterFindMatches.length}
                    onQueryChange={setChapterFindQuery}
                    onPrevious={() => moveChapterFind(-1)}
                    onNext={() => moveChapterFind(1)}
                    onConfirm={confirmChapterFind}
                    onClose={closeChapterFind}
                  />
                ) : null}
                <Separator />
                <div className="reader-text">
                  {visibleBlocks.map((block) =>
                    block.kind === "paragraph" ? (
                      <p
                        key={block.block_index}
                        id={blockDomId(block.block_index)}
                        data-reader-block
                        data-block-index={block.block_index}
                      >
                        <ReaderTokens
                          bookId={bookId}
                          block={block}
                          chapterIndex={chapterIndex}
                          colorMode={colorMode}
                          activeTokenKey={activeTokenKey}
                          bookmarkedTokenKey={bookmarkedTokenKey}
                          activeSearchResult={activeSearchResult}
                          chapterFindRanges={chapterFindRangesByBlock.get(block.block_index) ?? []}
                          wordlistRoots={wordlistRoots}
                          wordlistExactKeys={wordlistExactKeys}
                          timedTokensByKey={timedTokensByKey}
                          onPlayToken={playFromToken}
                          onPreviewToken={previewToken}
                          onOpenWordContextMenu={openWordContextMenu}
                          onTokenRef={(tokenKey, node) => {
                            tokenRefs.current[tokenKey] = node;
                          }}
                        />
                      </p>
                    ) : (
                      <ReaderFigure
                        key={block.block_index}
                        block={block}
                        fallbackAlt={chapter.title}
                        onOpen={openImage}
                      />
                    ),
                  )}
                </div>
              </>
            )}
          </article>
        </div>
        <MarkedWordScrollRail
          locations={markedWordLocations}
          activeKey={activeTokenKey}
          onSelect={jumpToMarkedWord}
        />
        <audio ref={audioRef} preload="metadata" className="hidden" />
        <audio ref={wordPreviewAudioRef} preload="metadata" className="hidden" />
        <audio ref={dictionaryAudioRef} preload="metadata" className="hidden" />
        {wordContextMenu ? (
          <WordContextMenu
            menu={wordContextMenu}
            saved={wordlistRoots.has(wordContextMenu.rootWord)}
            onLookup={lookupContextMenuWord}
            onToggleWordlist={toggleContextMenuWordlist}
          />
        ) : null}
        {chapterContextMenu ? (
          <ChapterContextMenu
            menu={chapterContextMenu}
            disabled={generatingAudio}
            onGenerateChapter={() => void startAudioQueue(chapterContextMenu.chapterIndex, "chapter")}
            onGenerateFromChapter={() => void startAudioQueue(chapterContextMenu.chapterIndex, "from-chapter")}
          />
        ) : null}
        {lookupDialog ? (
          <LookupDialog
            lookup={lookupDialog}
            onClose={() => setLookupDialog(null)}
            onMove={(x, y) => setLookupDialog((current) => (current ? { ...current, x, y } : current))}
            onPlayPronunciation={(audioUrl) => {
              const audio = dictionaryAudioRef.current;
              if (!audio) {
                return;
              }
              audio.pause();
              audio.src = audioUrl;
              void audio.play().catch(() => undefined);
            }}
          />
        ) : null}
        {dialogImage ? (
          <ImageDialog
            image={dialogImage}
            zoom={imageZoom}
            onZoomChange={setImageZoom}
            onClose={closeImage}
          />
        ) : null}
        {partAudio ? (
          <PartAudioPlayer
            playing={audioState.playing}
            currentTime={audioState.currentTime}
            duration={audioState.duration}
            onTogglePlay={toggleAudioPlay}
            onSeek={seekAudio}
          />
        ) : null}
      </main>
    </TooltipProvider>
  );
}
