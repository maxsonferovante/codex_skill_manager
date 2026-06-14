use super::types::{Chunk, ChunkOptions, PdfEngineError};

const WORDS_PER_TOKEN: f64 = 0.75;

fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    if words == 0 {
        0
    } else {
        (words as f64 / WORDS_PER_TOKEN) as usize
    }
}

pub fn chunk_text(input: &str, options: ChunkOptions) -> Result<Vec<Chunk>, PdfEngineError> {
    if input.trim().is_empty() {
        return Ok(vec![]);
    }

    let target = options.target_chunk_tokens.max(200);
    let max_tokens = options.max_chunk_tokens.max(target);

    let mut units: Vec<String> = if input.contains('\u{000C}') {
        input
            .split('\u{000C}')
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    } else {
        let paragraphs: Vec<String> = input
            .split("\n\n")
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if paragraphs.len() > 1 {
            paragraphs
        } else {
            input
                .lines()
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        }
    };

    if units.is_empty() {
        return Ok(vec![]);
    }

    let mut chunks: Vec<Chunk> = Vec::new();
    let mut buffer: Vec<String> = Vec::new();
    let mut buffer_tokens = 0usize;

    let flush = |chunks: &mut Vec<Chunk>, buffer: &mut Vec<String>, buffer_tokens: &mut usize| {
        if buffer.is_empty() {
            return;
        }
        let text = format!("{}\n", buffer.join("\n\n").trim());
        let est = estimate_tokens(&text);
        chunks.push(Chunk {
            index: chunks.len() + 1,
            text,
            est_tokens: est,
        });
        buffer.clear();
        *buffer_tokens = 0;
    };

    for unit in units.drain(..) {
        let unit_tokens = estimate_tokens(&unit);

        if unit_tokens > max_tokens {
            flush(&mut chunks, &mut buffer, &mut buffer_tokens);
            let chars_per_token = (unit.len() / unit_tokens.max(1)).max(1);
            let step = chars_per_token * max_tokens;
            let mut start = 0usize;
            while start < unit.len() {
                let end = (start + step).min(unit.len());
                let piece = unit[start..end].trim();
                if !piece.is_empty() {
                    let text = format!("{piece}\n");
                    chunks.push(Chunk {
                        index: chunks.len() + 1,
                        est_tokens: estimate_tokens(&text),
                        text,
                    });
                }
                start = end;
            }
            continue;
        }

        if !buffer.is_empty() && buffer_tokens + unit_tokens > target {
            flush(&mut chunks, &mut buffer, &mut buffer_tokens);
        }

        buffer.push(unit);
        buffer_tokens += unit_tokens;

        if buffer_tokens >= max_tokens {
            flush(&mut chunks, &mut buffer, &mut buffer_tokens);
        }
    }

    flush(&mut chunks, &mut buffer, &mut buffer_tokens);
    if chunks.is_empty() {
        return Err(PdfEngineError::ChunkingFailed(
            "no chunks produced from non-empty input".to_string(),
        ));
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_page_boundaries_when_available() {
        let text = "page one\u{000C}page two\u{000C}page three";
        let chunks = chunk_text(text, ChunkOptions::default()).expect("chunks");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("page one"));
        assert!(chunks[0].text.contains("page three"));
    }

    #[test]
    fn splits_large_unit() {
        let mut unit = String::new();
        for _ in 0..5000 {
            unit.push_str("token ");
        }
        let chunks = chunk_text(
            &unit,
            ChunkOptions {
                target_chunk_tokens: 300,
                max_chunk_tokens: 500,
            },
        )
        .expect("chunks");
        assert!(chunks.len() > 1);
    }
}
