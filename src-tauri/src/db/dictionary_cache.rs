pub fn get_oxford_cache(connection: &Connection, lemma: &str) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT payload FROM dictionary_oxford_cache WHERE lemma = ?",
            params![lemma],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub fn save_oxford_cache(connection: &Connection, lemma: &str, payload: &str) -> Result<()> {
    let timestamp = Utc::now().to_rfc3339();
    connection.execute(
        r#"
        INSERT INTO dictionary_oxford_cache (lemma, payload, created_at, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(lemma) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at
        "#,
        params![lemma, payload, timestamp, timestamp],
    )?;
    Ok(())
}

pub fn get_context_cache(
    connection: &Connection,
    lemma: &str,
    context_key: &str,
) -> Result<Option<String>> {
    connection
        .query_row(
            "SELECT payload FROM dictionary_context_cache WHERE lemma = ? AND context_key = ?",
            params![lemma, context_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

pub fn save_context_cache(
    connection: &Connection,
    lemma: &str,
    context_key: &str,
    payload: &str,
) -> Result<()> {
    let timestamp = Utc::now().to_rfc3339();
    connection.execute(
        r#"
        INSERT INTO dictionary_context_cache (lemma, context_key, payload, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(lemma, context_key) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at
        "#,
        params![lemma, context_key, payload, timestamp, timestamp],
    )?;
    Ok(())
}
