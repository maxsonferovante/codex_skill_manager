use std::fs;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::extractor::ExtractionOutput;
use super::types::PdfEngineError;

const WORDS_PER_TOKEN: f64 = 0.75;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StructureDetection {
    pub chapters_detected: usize,
    pub chapter_headings_sample: Vec<String>,
    pub has_toc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExtractionMetadata {
    pub filename: String,
    pub format: String,
    pub extraction_mode: String,
    pub extraction_method: String,
    pub pages: usize,
    pub words: usize,
    pub estimated_tokens: usize,
    pub estimated_tokens_human: String,
    pub structure: StructureDetection,
}

pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    if words == 0 {
        0
    } else {
        (words as f64 / WORDS_PER_TOKEN) as usize
    }
}

pub fn detect_structure(text: &str) -> StructureDetection {
    let sample = &text[..text.len().min(50_000)];
    let chapter_re = Regex::new(r"(?im)^\s*(chapter\s+\d+|ch\.\s*\d+|\d+\.\s+[A-Z])").expect("regex");
    let toc_re = Regex::new(r"(?im)^\s*(table of contents|contents|índice|sumário)\s*$").expect("regex");

    let headings: Vec<String> = chapter_re
        .find_iter(sample)
        .map(|m| m.as_str().trim().to_string())
        .take(10)
        .collect();

    StructureDetection {
        chapters_detected: chapter_re.find_iter(sample).count(),
        chapter_headings_sample: headings,
        has_toc: toc_re.is_match(&text[..text.len().min(30_000)]),
    }
}

pub fn build_metadata(
    filename: &str,
    extraction_mode: &str,
    output: &ExtractionOutput,
) -> ExtractionMetadata {
    let words = output.full_text.split_whitespace().count();
    let estimated_tokens = estimate_tokens(&output.full_text);
    let estimated_tokens_human = format!("~{}K", (estimated_tokens.max(1) / 1000).max(1));
    ExtractionMetadata {
        filename: filename.to_string(),
        format: "pdf".to_string(),
        extraction_mode: extraction_mode.to_string(),
        extraction_method: output.method_used.clone(),
        pages: output.pages_estimate,
        words,
        estimated_tokens,
        estimated_tokens_human,
        structure: detect_structure(&output.full_text),
    }
}

pub fn write_extraction_artifacts(
    workdir: &Path,
    metadata: &ExtractionMetadata,
    full_text: &str,
) -> Result<(), PdfEngineError> {
    fs::create_dir_all(workdir).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    let metadata_path = workdir.join("metadata.json");
    let text_path = workdir.join("full_text.txt");
    let json = serde_json::to_string_pretty(metadata).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    fs::write(metadata_path, json).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    fs::write(text_path, full_text).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn detects_structure_and_writes_files() {
        let text = "Table of Contents\n\nChapter 1\nAlpha\n\nChapter 2\nBeta";
        let output = ExtractionOutput {
            full_text: text.to_string(),
            method_used: "text-lopdf".to_string(),
            pages_estimate: 2,
        };
        let metadata = build_metadata("book.pdf", "text", &output);
        assert!(metadata.structure.has_toc);
        assert!(metadata.structure.chapters_detected >= 2);

        let tmp = TempDir::new().expect("tempdir");
        write_extraction_artifacts(tmp.path(), &metadata, text).expect("write");
        assert!(tmp.path().join("metadata.json").exists());
        assert!(tmp.path().join("full_text.txt").exists());
    }
}

