fn is_redundant_chapter_text(chapter_title: &str, text: &str) -> bool {
    let normalized_title = normalize_inline(chapter_title);
    let normalized_text = normalize_inline(text);
    if normalized_text == normalized_title {
        return true;
    }
    if let Some((number, title_without_number)) = normalized_title.split_once(' ') {
        if number.chars().all(|character| character.is_ascii_digit())
            && (normalized_text == number || normalized_text == title_without_number)
        {
            return true;
        }
    }
    normalized_text.len() > normalized_title.len() + 500
        && normalized_text.starts_with(&normalized_title)
}

fn normalize_inline(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn chapter_group_key(title: &str, source_href: &str) -> Option<String> {
    let stem = source_stem(source_href).to_lowercase();
    if matches!(stem.as_str(), "toc" | "newslettersignup") {
        return None;
    }
    if stem.starts_with("insert") {
        return Some("insert".to_string());
    }
    if let Some(base) = chapter_number_stem(&stem) {
        return Some(base);
    }
    title.split_whitespace().next().and_then(|first| {
        if first.chars().all(|character| character.is_ascii_digit()) {
            Some(first.to_string())
        } else {
            Some(stem)
        }
    })
}

fn chapter_group_title(title: &str, source_href: &str) -> String {
    let stem = source_stem(source_href).to_lowercase();
    if stem.starts_with("insert") {
        "Insert".to_string()
    } else {
        title.to_string()
    }
}

fn is_progress_chapter_title(title: &str) -> bool {
    title
        .split_whitespace()
        .next()
        .is_some_and(|first| first.chars().all(|character| character.is_ascii_digit()))
}

fn chapter_number_stem(stem: &str) -> Option<String> {
    let rest = stem.strip_prefix("chapter")?;
    let digits = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let suffix = &rest[digits.len()..];
    if suffix.is_empty()
        || (suffix.len() == 2
            && suffix.starts_with('_')
            && suffix
                .chars()
                .nth(1)
                .is_some_and(|value| value.is_ascii_lowercase()))
    {
        Some(format!("chapter{digits}"))
    } else {
        None
    }
}

fn source_stem(source_href: &str) -> String {
    source_href
        .rsplit('/')
        .next()
        .and_then(|value| value.rsplit_once('.').map(|(stem, _)| stem).or(Some(value)))
        .unwrap_or_default()
        .to_string()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "book".to_string()
    } else {
        slug
    }
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn resolve_chapter_image_assets(
    connection: &Connection,
    book_id: i64,
    stored_path: &Path,
    content_hash: &str,
    blocks: &mut [ChapterBlock],
) -> Result<()> {
    let Some(data_dir) = stored_path.parent().and_then(Path::parent) else {
        return Ok(());
    };
    let assets_dir = data_dir.join("assets").join(content_hash);

    for block in blocks {
        if block.kind != "image" {
            continue;
        }
        let Some(asset_path) = block.asset_path.clone() else {
            continue;
        };
        if Path::new(&asset_path).exists() {
            continue;
        }
        if let Some(local_path) = materialize_epub_asset(stored_path, &assets_dir, &asset_path)? {
            connection.execute(
                "UPDATE chapter_blocks SET asset_path = ? WHERE book_id = ? AND block_index = ?",
                params![local_path, book_id, block.block_index],
            )?;
            block.asset_path = Some(local_path);
        }
    }

    Ok(())
}

fn materialize_epub_asset(
    epub_path: &Path,
    assets_dir: &Path,
    asset_path: &str,
) -> Result<Option<String>> {
    if asset_path.trim().is_empty() || Path::new(asset_path).exists() {
        return Ok(Some(asset_path.to_string()));
    }

    let bytes = match epub::read_asset_bytes(epub_path, asset_path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let local_path = assets_dir
        .join("images")
        .join(normalized_relative_path(asset_path));
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&local_path, bytes)?;
    Ok(Some(path_to_string(local_path)))
}

fn normalized_relative_path(path: &str) -> PathBuf {
    let mut relative = PathBuf::new();
    for part in path.replace('\\', "/").split('/') {
        match part {
            "" | "." | ".." => {}
            value => relative.push(value),
        }
    }
    if relative.as_os_str().is_empty() {
        PathBuf::from("image")
    } else {
        relative
    }
}
