#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugifies_titles_for_storage() {
        assert_eq!(
            slugify("My Youth Romantic Comedy, Vol. 1"),
            "my-youth-romantic-comedy-vol-1"
        );
        assert_eq!(slugify("!!!"), "book");
    }

    #[test]
    fn groups_split_chapter_files_and_ornamental_dividers_into_parts() {
        let raw_chapters = vec![
            RawChapterSummary {
                title: "4 Komachi Hikigaya is shrewdly scheming.".to_string(),
                source_href: "Text/chapter004.xhtml".to_string(),
                start_block_index: 10,
                end_block_index: 19,
            },
            RawChapterSummary {
                title: "Chapter004 B".to_string(),
                source_href: "Text/chapter004_b.xhtml".to_string(),
                start_block_index: 20,
                end_block_index: 29,
            },
            RawChapterSummary {
                title: "Chapter004 D".to_string(),
                source_href: "Text/chapter004_d.xhtml".to_string(),
                start_block_index: 30,
                end_block_index: 39,
            },
        ];
        let markers = vec![
            BlockMarker {
                block_index: 15,
                kind: "image".to_string(),
                asset_path: Some("../Images/Art_orn.jpg".to_string()),
                consumes_block: true,
            },
            BlockMarker {
                block_index: 19,
                kind: "image".to_string(),
                asset_path: Some("../Images/Art_orn.jpg".to_string()),
                consumes_block: true,
            },
            BlockMarker {
                block_index: 24,
                kind: "image".to_string(),
                asset_path: Some("../Images/Art_orn.jpg".to_string()),
                consumes_block: true,
            },
            BlockMarker {
                block_index: 29,
                kind: "image".to_string(),
                asset_path: Some("../Images/Art_orn.jpg".to_string()),
                consumes_block: true,
            },
        ];

        let chapters = build_reader_chapters(&raw_chapters, &markers).expect("chapters");

        assert_eq!(chapters.len(), 1);
        assert_eq!(
            chapters[0].title,
            "4 Komachi Hikigaya is shrewdly scheming."
        );
        assert_eq!(chapters[0].start_block_index, 10);
        assert_eq!(chapters[0].end_block_index, 39);
        assert_eq!(
            chapters[0]
                .parts
                .iter()
                .map(|part| (
                    part.title.as_str(),
                    part.start_block_index,
                    part.end_block_index
                ))
                .collect::<Vec<_>>(),
            vec![
                ("Part 1", 10, 14),
                ("Part 2", 16, 18),
                ("Part 3", 20, 23),
                ("Part 4", 25, 28),
                ("Part 5", 30, 39),
            ]
        );
    }

    #[test]
    fn removes_repeated_chapter_heading_blocks_from_display_blocks() {
        let blocks = readable_chapter_blocks(
            "4 Komachi Hikigaya is shrewdly scheming.",
            vec![
                ChapterBlock {
                    block_index: 1,
                    kind: "paragraph".to_string(),
                    text: "4 Komachi Hikigaya is shrewdly scheming. It was Sunday. The clear skies provided a brief respite from the rainy season. ".repeat(20),
                    asset_path: None,
                    alt: String::new(),
                    tokens: Vec::new(),
                },
                ChapterBlock {
                    block_index: 2,
                    kind: "paragraph".to_string(),
                    text: "4".to_string(),
                    asset_path: None,
                    alt: String::new(),
                    tokens: Vec::new(),
                },
                ChapterBlock {
                    block_index: 3,
                    kind: "paragraph".to_string(),
                    text: "Komachi Hikigaya is shrewdly scheming.".to_string(),
                    asset_path: None,
                    alt: String::new(),
                    tokens: Vec::new(),
                },
                ChapterBlock {
                    block_index: 4,
                    kind: "paragraph".to_string(),
                    text: "It was Sunday.".to_string(),
                    asset_path: None,
                    alt: String::new(),
                    tokens: Vec::new(),
                },
            ],
        );

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_index, 4);
        assert_eq!(blocks[0].text, "It was Sunday.");
    }

    #[test]
    fn caches_frequency_counts_for_progress_chapters_only() {
        let connection = frequency_test_connection();

        ensure_book_word_frequency_cache(&connection, 1).expect("cache");
        let frequencies = book_word_frequency_map(&connection, 1).expect("frequencies");

        assert_eq!(
            frequencies.get("the").map(|frequency| frequency.count),
            Some(1)
        );
        assert_eq!(
            frequencies.get("wugalpha").map(|frequency| frequency.count),
            Some(4)
        );
        assert_eq!(
            frequencies.get("wugbeta").map(|frequency| frequency.count),
            Some(2)
        );
        assert_eq!(
            frequencies.get("wuggamma").map(|frequency| frequency.count),
            Some(1)
        );
        assert!(!frequencies.contains_key("copyrightonly"));
        assert!(!frequencies.contains_key("headingonly"));
    }

    #[test]
    fn assigns_more_frequent_words_to_earlier_levels() {
        let connection = frequency_test_connection();
        let frequencies = book_word_frequency_map(&connection, 1).expect("frequencies");

        assert_eq!(frequencies["the"].level, CefrLevel::A1);
        assert_eq!(frequencies["wugalpha"].level, CefrLevel::A2);
        assert_eq!(frequencies["wuggamma"].level, CefrLevel::C2);
        assert!(
            frequency_level_rank(frequencies["wugalpha"].level)
                < frequency_level_rank(frequencies["wuggamma"].level)
        );
    }

    #[test]
    fn annotates_chapter_tokens_from_cached_frequency_counts() {
        let connection = frequency_test_connection();
        let chapter = get_chapter(&connection, 1, 1)
            .expect("chapter")
            .expect("chapter");
        let token = chapter.blocks[0]
            .tokens
            .iter()
            .find(|token| token.normalized_text == "wugalpha")
            .expect("wugalpha token");

        assert_eq!(token.frequency_count, Some(4));
        assert_eq!(token.frequency_level, Some(CefrLevel::A2));
    }

    #[test]
    fn reuses_existing_frequency_cache_without_recounting() {
        let connection = frequency_test_connection();
        ensure_book_word_frequency_cache(&connection, 1).expect("cache");
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 1, 5, 'paragraph', 'wugalpha newword', NULL, '')
                "#,
                [],
            )
            .expect("extra block");

        ensure_book_word_frequency_cache(&connection, 1).expect("cached");
        let frequencies = book_word_frequency_map(&connection, 1).expect("frequencies");

        assert_eq!(
            frequencies.get("wugalpha").map(|frequency| frequency.count),
            Some(4)
        );
        assert!(!frequencies.contains_key("newword"));
    }

    #[test]
    fn saves_wordlist_entry_from_reader_token() {
        let connection = frequency_test_connection();

        let entry = save_wordlist_entry(
            &connection,
            1,
            1,
            2,
            2,
            "wugalpha",
            "wugalpha",
            "Fallback context.",
            "",
        )
        .expect("entry");
        let entries = list_wordlist_entries(&connection).expect("entries");

        assert_eq!(entries.len(), 1);
        assert_eq!(entry.root_word, "wugalpha");
        assert_eq!(entry.book_title, "Book");
        assert_eq!(entry.chapter_index, 1);
        assert_eq!(entry.block_index, 2);
        assert_eq!(entry.token_index, 2);
        assert!(entry.context.contains("wugalpha"));
    }

    #[test]
    fn saves_wordlist_entry_when_reader_chapter_differs_from_raw_block() {
        let connection = frequency_test_connection();
        let text = "The girl in the seat beside me hadn’t spoken a word to me today, either. Maybe the reason English education in Japan doesn’t work is because they force you into pairs for compulsory conversation.";
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 1, 6, 'paragraph', ?, NULL, '')
                "#,
                params![text],
            )
            .expect("block");
        let token_index = cefr::tokenize_text(text)
            .iter()
            .position(|token| token.normalized_text == "compulsory")
            .expect("compulsory token");

        let entry = save_wordlist_entry(
            &connection,
            1,
            99,
            6,
            token_index,
            "compulsory",
            "compulsory",
            text,
            "",
        )
        .expect("entry");

        assert_eq!(entry.chapter_index, 99);
        assert_eq!(entry.block_index, 6);
        assert_eq!(entry.root_word, "compulsory");
        assert!(entry.context.contains("compulsory conversation"));
    }

    #[test]
    fn wordlist_reuses_existing_root_entry() {
        let connection = frequency_test_connection();

        let first = save_wordlist_entry(
            &connection,
            1,
            1,
            2,
            2,
            "wugalpha",
            "wugalpha",
            "",
            "",
        )
        .expect("first entry");
        let duplicate = save_wordlist_entry(
            &connection,
            1,
            1,
            3,
            0,
            "wugalpha",
            "wugalpha",
            "",
            "",
        )
        .expect("duplicate entry");
        let entries = list_wordlist_entries(&connection).expect("entries");

        assert_eq!(first.id, duplicate.id);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].block_index, 2);
    }

    #[test]
    fn deletes_wordlist_entry_by_root() {
        let connection = frequency_test_connection();
        save_wordlist_entry(&connection, 1, 1, 2, 2, "wugalpha", "wugalpha", "", "")
            .expect("entry");

        assert!(delete_wordlist_entry(&connection, "wugalpha").expect("delete"));
        assert!(list_wordlist_entries(&connection).expect("entries").is_empty());
    }

    #[test]
    fn toggles_reader_highlight_range() {
        let connection = frequency_test_connection();

        let created = toggle_highlight(&connection, 1, 1, 2, 1, 3, 0, 8, "wugalpha wugalpha")
            .expect("highlight")
            .expect("created");
        let highlights = list_book_highlights(&connection, 1).expect("highlights");

        assert_eq!(highlights.len(), 1);
        assert_eq!(created.text, "wugalpha wugalpha");
        assert_eq!(highlights[0].start_token_index, 1);
        assert_eq!(highlights[0].end_token_index, 3);
        assert_eq!(highlights[0].end_offset, 8);

        let removed = toggle_highlight(&connection, 1, 1, 2, 1, 3, 0, 8, "wugalpha wugalpha")
            .expect("toggle");

        assert!(removed.is_none());
        assert!(list_book_highlights(&connection, 1)
            .expect("highlights")
            .is_empty());
    }

    #[test]
    fn wordlist_lookup_error_keeps_saved_entry() {
        let connection = frequency_test_connection();
        let entry =
            save_wordlist_entry(&connection, 1, 1, 2, 2, "wugalpha", "wugalpha", "", "")
                .expect("entry");

        let updated =
            update_wordlist_entry_lookup(&connection, entry.id, None, "offline").expect("update");
        let entries = list_wordlist_entries(&connection).expect("entries");

        assert_eq!(updated.expect("updated").definition_lookup_error, "offline");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].definition, "");
    }

    #[test]
    fn wordlist_lookup_cache_preserves_ai_enrichment() {
        let connection = frequency_test_connection();
        let entry =
            save_wordlist_entry(&connection, 1, 1, 2, 2, "wugalpha", "wugalpha", "", "")
                .expect("entry");
        let lookup = crate::dictionary::DictionaryLookup {
            word: "wugalpha".to_string(),
            selected_word: "wugalpha".to_string(),
            word_type: "adjective".to_string(),
            cefr_level: "C2".to_string(),
            phonetics: vec!["/wug-alpha/".to_string()],
            audio_url: "https://example.com/wugalpha.mp3".to_string(),
            source_url: "https://example.com/wugalpha".to_string(),
            definitions: vec![crate::dictionary::DictionaryDefinition {
                entry_id: "wugalpha".to_string(),
                word_type: "adjective".to_string(),
                number: 2,
                definition: "required by rule".to_string(),
                examples: vec!["A compulsory task.".to_string()],
                source_url: "https://example.com/wugalpha".to_string(),
            }],
            context_definition: crate::dictionary::DictionaryChoice {
                entry_id: Some("wugalpha".to_string()),
                definition_number: Some(2),
                definition: "required by rule".to_string(),
                examples: vec!["A compulsory task.".to_string()],
                ai_explanation: "The context is about being forced into pairs.".to_string(),
                matched: true,
            },
            simple_meaning: "required".to_string(),
            in_context_meaning: "The conversation exercise is mandatory.".to_string(),
            original_meaning: "From a sense of compulsion.".to_string(),
        };

        let updated = update_wordlist_entry_lookup(&connection, entry.id, Some(&lookup), "")
            .expect("update")
            .expect("updated");

        assert_eq!(updated.simple_meaning, "required");
        assert_eq!(
            updated.in_context_meaning,
            "The conversation exercise is mandatory."
        );
        assert_eq!(updated.original_meaning, "From a sense of compulsion.");
        assert_eq!(
            updated.ai_explanation,
            "The context is about being forced into pairs."
        );
    }

    #[test]
    fn virtual_dividers_split_after_their_previous_paragraph() {
        let parts = build_chapter_parts(
            10,
            20,
            &[BlockMarker {
                block_index: 14,
                kind: "image".to_string(),
                asset_path: Some("../Images/Art_orn.jpg".to_string()),
                consumes_block: false,
            }],
        );

        assert_eq!(
            parts
                .iter()
                .map(|part| (part.start_block_index, part.end_block_index))
                .collect::<Vec<_>>(),
            vec![(10, 14), (15, 20)]
        );
    }

    #[test]
    fn search_book_ignores_empty_and_short_queries() {
        let connection = search_test_connection();

        assert!(search_book(&connection, 1, "").expect("empty").is_empty());
        assert!(search_book(&connection, 1, "a").expect("short").is_empty());
    }

    #[test]
    fn search_book_finds_case_insensitive_paragraph_matches_in_order() {
        let connection = search_test_connection();

        let results = search_book(&connection, 1, "Lantern").expect("results");

        assert_eq!(
            results
                .iter()
                .map(|result| (result.chapter_index, result.block_index))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 3)]
        );
        assert_eq!(results[0].chapter_title, "1 One");
        assert_eq!(results[0].match_count, 2);
        assert!(results[0].snippet.contains("Lantern"));
    }

    #[test]
    fn search_book_ignores_image_blocks() {
        let connection = search_test_connection();

        let results = search_book(&connection, 1, "imageonly").expect("results");

        assert!(results.is_empty());
    }

    #[test]
    fn search_book_caps_results() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash-search-cap', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        connection
            .execute(
                r#"
                INSERT INTO book_chapters (
                    book_id, chapter_index, title, source_href, start_block_index, end_block_index
                ) VALUES (1, 0, '1 One', 'chapter001.xhtml', 0, 120)
                "#,
                [],
            )
            .expect("chapter");
        for block_index in 0..120 {
            connection
                .execute(
                    r#"
                    INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                    VALUES (1, 0, ?, 'paragraph', 'needle text', NULL, '')
                    "#,
                    params![block_index],
                )
                .expect("block");
        }

        let results = search_book(&connection, 1, "needle").expect("results");

        assert_eq!(results.len(), 100);
        assert_eq!(results[99].block_index, 99);
    }

    #[test]
    fn migrates_legacy_reading_progress_columns() {
        let connection = Connection::open_in_memory().expect("connection");
        connection
            .execute_batch(
                r#"
                CREATE TABLE reading_progress (
                    book_id INTEGER PRIMARY KEY,
                    last_read_at TEXT NOT NULL,
                    last_chapter_index INTEGER NOT NULL DEFAULT 0,
                    last_block_index INTEGER NOT NULL DEFAULT 0,
                    progress_percent REAL NOT NULL DEFAULT 0
                );
                "#,
            )
            .expect("legacy schema");

        migrate_reading_progress(&connection).expect("migration");

        let columns = table_columns(&connection, "reading_progress");
        for column in [
            "last_part_index",
            "last_scroll_ratio",
            "last_audio_time_seconds",
            "last_audio_duration_seconds",
            "last_playing_block_index",
            "last_playing_token_index",
        ] {
            assert!(columns.iter().any(|item| item == column), "{column}");
        }
    }

    #[test]
    fn saves_and_reads_rich_progress_with_clamps() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        migrate_reading_progress(&connection).expect("migration");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        connection
            .execute(
                r#"
                INSERT INTO book_chapters (
                    book_id, chapter_index, title, source_href, start_block_index, end_block_index
                ) VALUES (1, 2, 'Chapter', 'chapter.xhtml', 40, 50)
                "#,
                [],
            )
            .expect("chapter");
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 2, 42, 'paragraph', 'Text', NULL, '')
                "#,
                [],
            )
            .expect("block");

        save_progress(
            &connection,
            1,
            2,
            3,
            42,
            1.25,
            Some(12.5),
            Some(10.0),
            Some(41),
            Some(7),
            125.0,
        )
        .expect("save");

        let reader = get_reader(&connection, 1).expect("reader").expect("book");
        assert_eq!(reader.progress.last_chapter_index, 0);
        assert_eq!(reader.progress.last_part_index, 0);
        assert_eq!(reader.progress.last_block_index, 42);
        assert_eq!(reader.progress.last_scroll_ratio, 1.0);
        assert_eq!(reader.progress.last_audio_time_seconds, Some(10.0));
        assert_eq!(reader.progress.last_audio_duration_seconds, Some(10.0));
        assert_eq!(reader.progress.last_playing_block_index, Some(41));
        assert_eq!(reader.progress.last_playing_token_index, Some(7));
        assert_eq!(reader.progress.progress_percent, 100.0);
    }

    #[test]
    fn saves_and_overwrites_word_bookmark() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        connection
            .execute(
                r#"
                INSERT INTO book_chapters (
                    book_id, chapter_index, title, source_href, start_block_index, end_block_index
                ) VALUES (1, 0, '1 Chapter', 'chapter001.xhtml', 40, 50)
                "#,
                [],
            )
            .expect("chapter");
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 0, 42, 'paragraph', 'First bookmarked word', NULL, '')
                "#,
                [],
            )
            .expect("block");

        save_bookmark(
            &connection,
            1,
            0,
            0,
            42,
            1,
            "bookmarked",
            "bookmark",
            1.25,
            125.0,
        )
        .expect("save bookmark");
        save_bookmark(&connection, 1, 0, 0, 42, 2, "word", "word", 0.25, 35.0)
            .expect("overwrite bookmark");

        let bookmark = get_reader(&connection, 1)
            .expect("reader")
            .expect("book")
            .bookmark
            .expect("bookmark");
        assert_eq!(bookmark.chapter_index, 0);
        assert_eq!(bookmark.part_index, 0);
        assert_eq!(bookmark.block_index, 42);
        assert_eq!(bookmark.token_index, 2);
        assert_eq!(bookmark.word, "word");
        assert_eq!(bookmark.root_word, "word");
        assert_eq!(bookmark.scroll_ratio, 0.25);
        assert_eq!(bookmark.progress_percent, 35.0);
    }

    #[test]
    fn normalizes_saved_chapter_from_saved_block_on_read() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        migrate_reading_progress(&connection).expect("migration");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        for (chapter_index, title, source_href, start_block, end_block) in [
            (1, "1 One", "chapter001.xhtml", 1, 10),
            (2, "2 Two", "chapter002.xhtml", 40, 50),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO book_chapters (
                        book_id, chapter_index, title, source_href, start_block_index, end_block_index
                    ) VALUES (1, ?, ?, ?, ?, ?)
                    "#,
                    params![chapter_index, title, source_href, start_block, end_block],
                )
                .expect("chapter");
        }
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 2, 42, 'paragraph', 'Text', NULL, '')
                "#,
                [],
            )
            .expect("block");
        connection
            .execute(
                r#"
                INSERT INTO reading_progress (
                    book_id, last_read_at, last_chapter_index, last_part_index, last_block_index,
                    last_scroll_ratio, progress_percent
                ) VALUES (1, ?, 1, 0, 42, 0.5, 25)
                "#,
                params![timestamp],
            )
            .expect("progress");

        let reader = get_reader(&connection, 1).expect("reader").expect("book");
        assert_eq!(reader.progress.last_chapter_index, 1);
        assert_eq!(reader.progress.last_part_index, 0);
        assert_eq!(reader.progress.last_block_index, 42);
    }

    #[test]
    fn preserves_audio_progress_when_same_part_save_omits_audio() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        migrate_reading_progress(&connection).expect("migration");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        connection
            .execute(
                r#"
                INSERT INTO book_chapters (
                    book_id, chapter_index, title, source_href, start_block_index, end_block_index
                ) VALUES (1, 0, '1 One', 'chapter001.xhtml', 40, 50)
                "#,
                [],
            )
            .expect("chapter");
        connection
            .execute(
                r#"
                INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                VALUES (1, 0, 42, 'paragraph', 'Text', NULL, '')
                "#,
                [],
            )
            .expect("block");

        save_progress(
            &connection,
            1,
            0,
            0,
            42,
            0.1,
            Some(12.0),
            Some(100.0),
            Some(42),
            Some(3),
            10.0,
        )
        .expect("audio save");
        save_progress(&connection, 1, 0, 0, 42, 0.2, None, None, None, None, 10.0)
            .expect("scroll save");

        let reader = get_reader(&connection, 1).expect("reader").expect("book");
        assert_eq!(reader.progress.last_scroll_ratio, 0.2);
        assert_eq!(reader.progress.last_audio_time_seconds, Some(12.0));
        assert_eq!(reader.progress.last_audio_duration_seconds, Some(100.0));
        assert_eq!(reader.progress.last_playing_block_index, Some(42));
        assert_eq!(reader.progress.last_playing_token_index, Some(3));
    }

    #[test]
    fn progress_percent_counts_only_numbered_chapter_characters() {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        migrate_reading_progress(&connection).expect("migration");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");

        for (chapter_index, title, source_href, block_index) in [
            (0, "Copyright", "copyright.xhtml", 0),
            (1, "1 One", "chapter001.xhtml", 1),
            (2, "2 Two", "chapter002.xhtml", 2),
            (3, "BT Bonus track!", "bonus.xhtml", 3),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO book_chapters (
                        book_id, chapter_index, title, source_href, start_block_index, end_block_index
                    ) VALUES (1, ?, ?, ?, ?, ?)
                    "#,
                    params![chapter_index, title, source_href, block_index, block_index],
                )
                .expect("chapter");
            connection
                .execute(
                    r#"
                    INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                    VALUES (1, ?, ?, 'paragraph', ?, NULL, '')
                    "#,
                    params![chapter_index, block_index, "x".repeat(100)],
                )
                .expect("block");
        }

        save_progress(&connection, 1, 2, 0, 2, 0.0, None, None, None, None, 99.0).expect("save");

        let reader = get_reader(&connection, 1).expect("reader").expect("book");
        assert_eq!(reader.total_progress_units, 200);
        assert_eq!(reader.progress.progress_percent, 50.0);

        let books = list_books(&connection).expect("books");
        assert_eq!(books[0].progress_percent, 50.0);
    }

    fn frequency_test_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash-frequency', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        for (chapter_index, title, source_href, start_block_index, end_block_index) in [
            (0, "Copyright", "copyright.xhtml", 0, 0),
            (1, "0 Headingonly", "chapter000.xhtml", 1, 3),
            (2, "Translation Notes", "notes.xhtml", 4, 4),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO book_chapters (
                        book_id, chapter_index, title, source_href, start_block_index, end_block_index
                    ) VALUES (1, ?, ?, ?, ?, ?)
                    "#,
                    params![chapter_index, title, source_href, start_block_index, end_block_index],
                )
                .expect("chapter");
        }
        for (chapter_index, block_index, text) in [
            (0, 0, "copyrightonly copyrightonly"),
            (1, 1, "0 Headingonly"),
            (1, 2, "the wugalpha wugalpha wugalpha wugbeta wuggamma"),
            (1, 3, "wugalpha wugbeta"),
            (2, 4, "notesonly notesonly notesonly"),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                    VALUES (1, ?, ?, 'paragraph', ?, NULL, '')
                    "#,
                    params![chapter_index, block_index, text],
                )
                .expect("block");
        }
        connection
    }

    fn search_test_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("connection");
        connection.execute_batch(SCHEMA).expect("schema");
        let timestamp = now_iso();
        connection
            .execute(
                r#"
                INSERT INTO books (
                    id, slug, title, author, content_hash, original_filename, stored_path,
                    cover_asset_path, created_at, updated_at
                ) VALUES (1, 'book', 'Book', '', 'hash-search', 'book.epub', '/tmp/book.epub', NULL, ?, ?)
                "#,
                params![timestamp, timestamp],
            )
            .expect("book");
        for (chapter_index, title, source_href, start_block_index, end_block_index) in [
            (0, "1 One", "chapter001.xhtml", 0, 2),
            (1, "2 Two", "chapter002.xhtml", 3, 4),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO book_chapters (
                        book_id, chapter_index, title, source_href, start_block_index, end_block_index
                    ) VALUES (1, ?, ?, ?, ?, ?)
                    "#,
                    params![chapter_index, title, source_href, start_block_index, end_block_index],
                )
                .expect("chapter");
        }
        for (chapter_index, block_index, kind, text, asset_path, alt) in [
            (0, 0, "paragraph", "A quiet opening paragraph", None, ""),
            (
                0,
                1,
                "paragraph",
                "Lantern light and another lantern on the wall",
                None,
                "",
            ),
            (0, 2, "image", "", Some("imageonly.png"), "imageonly"),
            (
                1,
                3,
                "paragraph",
                "The final LANTERN waited upstairs",
                None,
                "",
            ),
            (1, 4, "paragraph", "No matching term here", None, ""),
        ] {
            connection
                .execute(
                    r#"
                    INSERT INTO chapter_blocks (book_id, chapter_index, block_index, kind, text, asset_path, alt)
                    VALUES (1, ?, ?, ?, ?, ?, ?)
                    "#,
                    params![chapter_index, block_index, kind, text, asset_path, alt],
                )
                .expect("block");
        }
        connection
    }

    fn frequency_level_rank(level: CefrLevel) -> u8 {
        match level {
            CefrLevel::A1 => 1,
            CefrLevel::A2 => 2,
            CefrLevel::B1 => 3,
            CefrLevel::B2 => 4,
            CefrLevel::C1 => 5,
            CefrLevel::C2 => 6,
        }
    }

    fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info");
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("columns")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("column names")
    }
}
