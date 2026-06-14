use std::path::Path;

use lopdf::Document;

use super::types::{PdfEngineError, PdfExtractionMode, PdfFileTask};

#[derive(Debug, Clone)]
pub struct ExtractionOutput {
    pub full_text: String,
    pub method_used: String,
    pub pages_estimate: usize,
}

fn sorted_pages(document: &Document) -> Vec<u32> {
    let mut pages: Vec<u32> = document.get_pages().keys().copied().collect();
    pages.sort_unstable();
    pages
}

fn extract_text_mode(path: &Path) -> Result<ExtractionOutput, PdfEngineError> {
    let doc = Document::load(path).map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
    let pages = sorted_pages(&doc);
    let page_count = pages.len();
    let text = doc
        .extract_text(&pages)
        .map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
    if text.trim().is_empty() {
        return Err(PdfEngineError::ScanFailed(
            "text extraction produced empty output".to_string(),
        ));
    }
    Ok(ExtractionOutput {
        full_text: text,
        method_used: "text-lopdf".to_string(),
        pages_estimate: page_count,
    })
}

fn extract_technical_mode(path: &Path) -> Result<ExtractionOutput, PdfEngineError> {
    let doc = Document::load(path).map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
    let pages = sorted_pages(&doc);
    let page_count = pages.len();
    let mut page_texts: Vec<String> = Vec::with_capacity(page_count);
    for page in pages {
        let text = doc
            .extract_text(&[page])
            .map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
        page_texts.push(text.trim().to_string());
    }
    let merged = page_texts.join("\u{000C}\n");
    if merged.trim().is_empty() {
        return Err(PdfEngineError::ScanFailed(
            "technical extraction produced empty output".to_string(),
        ));
    }
    Ok(ExtractionOutput {
        full_text: merged,
        method_used: "technical-lopdf-page-join".to_string(),
        pages_estimate: page_count,
    })
}

pub fn extract_pdf(
    task: &PdfFileTask,
    mode: PdfExtractionMode,
    mut on_log: impl FnMut(&str),
) -> Result<ExtractionOutput, PdfEngineError> {
    match mode {
        PdfExtractionMode::Text => {
            on_log("Trying text extractor (lopdf)...");
            extract_text_mode(&task.absolute_path)
        }
        PdfExtractionMode::Technical => {
            on_log("Trying technical extractor (lopdf page-aware)...");
            match extract_technical_mode(&task.absolute_path) {
                Ok(out) => Ok(out),
                Err(err) => {
                    on_log(&format!(
                        "Technical extractor failed: {err}. Falling back to text extractor."
                    ));
                    let mut out = extract_text_mode(&task.absolute_path)?;
                    out.method_used = format!("fallback:{}", out.method_used);
                    Ok(out)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn returns_error_for_missing_pdf() {
        let task = PdfFileTask {
            file_name: "missing.pdf".to_string(),
            absolute_path: PathBuf::from("/definitely/missing/file.pdf"),
        };
        let result = extract_pdf(&task, PdfExtractionMode::Text, |_| {});
        assert!(result.is_err());
    }
}

