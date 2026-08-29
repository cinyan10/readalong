pub fn start_definition_prefetch_worker(db_path: PathBuf, running: Arc<AtomicBool>) {
    if running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let job = match db::connect(&db_path).and_then(|connection| db::next_definition_prefetch_job(&connection)) {
                Ok(Some(job)) => job,
                Ok(None) | Err(_) => break,
            };
            let claim = db::connect(&db_path).and_then(|connection| {
                db::set_definition_prefetch_job_status(&connection, &job, "running", "")
            });
            if claim.is_err() {
                break;
            }

            let result = prefetch_definition(&db_path, &job).await;
            if let Ok(connection) = db::connect(&db_path) {
                match result {
                    Ok(()) => {
                        let _ = db::set_definition_prefetch_job_status(&connection, &job, "completed", "");
                    }
                    Err(error) => {
                        let _ = db::set_definition_prefetch_job_status(
                            &connection,
                            &job,
                            "failed",
                            &error.to_string(),
                        );
                    }
                }
            }
        }

        running.store(false, Ordering::SeqCst);
        let has_pending = db::connect(&db_path)
            .and_then(|connection| db::has_pending_definition_prefetch_jobs(&connection))
            .unwrap_or(false);
        if has_pending {
            start_definition_prefetch_worker(db_path, running);
        }
    });
}

async fn prefetch_definition(db_path: &Path, job: &db::DefinitionPrefetchJob) -> anyhow::Result<()> {
    let lemma_candidates = crate::dictionary::cache_lemma_candidates(&job.word, &job.root_word);
    let (cached_oxford, cached_context) = {
        let connection = db::connect(db_path)?;
        let mut oxford = None;
        let mut context = None;
        for lemma in &lemma_candidates {
            if context.is_none() {
                context = db::get_context_cache(&connection, lemma, &job.context_key)?;
            }
            if oxford.is_none() {
                oxford = db::get_oxford_cache(&connection, lemma)?;
            }
            if context.is_some() && oxford.is_some() {
                break;
            }
        }
        (oxford, context)
    };

    if cached_context.is_some() {
        return Ok(());
    }

    let requested_lemma = crate::dictionary::normalize_cache_lemma(&job.word, &job.root_word);
    let (lookup, oxford_payload) = crate::dictionary::lookup_word_with_cached_data(
        job.word.clone(),
        job.context.clone(),
        job.cefr_level.clone(),
        job.root_word.clone(),
        cached_oxford,
        None,
    )
    .await?;
    let canonical_lemma = crate::dictionary::normalize_cache_lemma(&lookup.word, "");
    if canonical_lemma.is_empty() {
        return Ok(());
    }

    let connection = db::connect(db_path)?;
    if !oxford_payload.is_empty() {
        db::save_oxford_cache(&connection, &canonical_lemma, &oxford_payload)?;
        if canonical_lemma != requested_lemma && !requested_lemma.is_empty() {
            db::save_oxford_cache(&connection, &requested_lemma, &oxford_payload)?;
        }
    }
    let payload = serde_json::to_string(&lookup)?;
    db::save_context_cache(&connection, &canonical_lemma, &job.context_key, &payload)?;
    if canonical_lemma != requested_lemma && !requested_lemma.is_empty() {
        db::save_context_cache(&connection, &requested_lemma, &job.context_key, &payload)?;
    }
    Ok(())
}
