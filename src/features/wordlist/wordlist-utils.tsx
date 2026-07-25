import type { DictionaryLookup, WordlistEntry } from "@/types";

export function wordlistExactKey(entry: WordlistEntry) {
  return wordlistTokenKey(entry.book_id, entry.chapter_index, entry.block_index, entry.token_index);
}

export function wordlistTokenKey(bookId: number, chapterIndex: number, blockIndex: number, tokenIndex: number) {
  return `${bookId}:${chapterIndex}:${blockIndex}:${tokenIndex}`;
}

export function isWordlistEntryAtToken(entry: WordlistEntry, bookId: number, chapterIndex: number, blockIndex: number, tokenIndex: number) {
  return wordlistExactKey(entry) === wordlistTokenKey(bookId, chapterIndex, blockIndex, tokenIndex);
}

export function upsertWordlistEntry(entries: WordlistEntry[], entry: WordlistEntry) {
  const index = entries.findIndex((item) => item.id === entry.id || item.root_word === entry.root_word);
  if (index === -1) {
    return [entry, ...entries];
  }
  const next = [...entries];
  next[index] = entry;
  return next;
}

export function dictionaryLookupFromWordlistEntry(entry: WordlistEntry, selectedWord: string): DictionaryLookup {
  const definitionNumber = entry.definition_number ?? 1;
  const definition = entry.definition;
  return {
    word: entry.root_word,
    selected_word: selectedWord,
    word_type: entry.word_type,
    cefr_level: entry.cefr_level,
    phonetics: entry.definition_phonetics,
    audio_url: entry.definition_audio_url,
    source_url: entry.definition_source_url,
    definitions: definition
      ? [
          {
            entry_id: entry.root_word,
            word_type: entry.word_type,
            number: definitionNumber,
            definition,
            examples: entry.definition_examples,
            source_url: entry.definition_source_url,
          },
        ]
      : [],
    context_definition: {
      entry_id: definition ? entry.root_word : null,
      definition_number: definition ? definitionNumber : null,
      definition,
      examples: entry.definition_examples,
      ai_explanation: entry.ai_explanation,
      matched: Boolean(definition),
    },
    simple_meaning: entry.simple_meaning || definition,
    in_context_meaning: entry.in_context_meaning,
    original_meaning: entry.original_meaning,
  };
}

export function hasWordlistAiEnrichment(entry: WordlistEntry) {
  return Boolean(entry.simple_meaning.trim() || entry.in_context_meaning.trim());
}

export function highlightContextWord(context: string, word: string) {
  const trimmedWord = word.trim();
  if (!context || !trimmedWord) {
    return context;
  }
  const start = context.toLocaleLowerCase().indexOf(trimmedWord.toLocaleLowerCase());
  if (start === -1) {
    return context;
  }
  const end = start + trimmedWord.length;
  return (
    <>
      {context.slice(0, start)}
      <mark>{context.slice(start, end)}</mark>
      {context.slice(end)}
    </>
  );
}

export function formatSavedAt(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}
