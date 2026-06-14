use std::fs;
use std::path::Path;

use super::types::{PdfEngineError, PdfFileTask};

pub fn scan_pdfs(directory: &Path) -> Result<Vec<PdfFileTask>, PdfEngineError> {
    if !directory.exists() || !directory.is_dir() {
        return Err(PdfEngineError::InvalidInput(format!(
            "not a directory: {}",
            directory.display()
        )));
    }

    let entries = fs::read_dir(directory).map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
    let mut tasks = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| PdfEngineError::ScanFailed(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_pdf = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        if !is_pdf {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        tasks.push(PdfFileTask {
            file_name,
            absolute_path: path,
        });
    }

    tasks.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scans_only_pdfs_sorted() {
        let tmp = TempDir::new().expect("tempdir");
        std::fs::write(tmp.path().join("b.pdf"), "x").expect("write");
        std::fs::write(tmp.path().join("a.PDF"), "x").expect("write");
        std::fs::write(tmp.path().join("note.txt"), "x").expect("write");
        std::fs::create_dir(tmp.path().join("nested")).expect("mkdir");

        let items = scan_pdfs(tmp.path()).expect("scan");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].file_name, "a.PDF");
        assert_eq!(items[1].file_name, "b.pdf");
    }
}

