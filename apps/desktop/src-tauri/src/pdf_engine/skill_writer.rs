use std::fs;
use std::path::Path;

use super::chunker::chunk_text;
use super::types::{PdfEngineError, SkillWriteInput, SkillWriteOutput};

fn write_supporting_files(dest_dir: &Path, title: &str) -> Result<(), PdfEngineError> {
    fs::write(
        dest_dir.join("glossary.md"),
        format!("# Glossary - {title}\n\n(Generated in a later structured mode.)\n"),
    )
    .map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    fs::write(
        dest_dir.join("patterns.md"),
        format!("# Patterns - {title}\n\n(Generated in a later structured mode.)\n"),
    )
    .map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    fs::write(
        dest_dir.join("cheatsheet.md"),
        format!("# Cheatsheet - {title}\n\n(Generated in a later structured mode.)\n"),
    )
    .map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    Ok(())
}

pub fn write_skill(input: SkillWriteInput) -> Result<SkillWriteOutput, PdfEngineError> {
    fs::create_dir_all(&input.destination_dir).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
    let chapters_dir = input.destination_dir.join("chapters");
    fs::create_dir_all(&chapters_dir).map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;

    let chunks = chunk_text(&input.full_text, input.chunk_options.clone())?;
    if chunks.is_empty() {
        return Err(PdfEngineError::WriteFailed(
            "cannot write skill from empty chunk set".to_string(),
        ));
    }

    let mut index_lines = Vec::new();
    for chunk in &chunks {
        let chapter_file = format!("ch{:02}-chunk.md", chunk.index);
        let chapter_path = chapters_dir.join(&chapter_file);
        let chapter_header = format!(
            "# {} - Chunk {:02}\n**Source**: {}\n**Estimated tokens**: ~{}K\n\n",
            input.title,
            chunk.index,
            input.source_filename,
            (chunk.est_tokens.max(1) / 1000).max(1)
        );
        fs::write(chapter_path, format!("{chapter_header}{}", chunk.text))
            .map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;
        index_lines.push(format!(
            "| [ch{:02}](chapters/{}) | Chunk {:02} | ~{} |",
            chunk.index, chapter_file, chunk.index, chunk.est_tokens
        ));
    }

    write_supporting_files(&input.destination_dir, &input.title)?;

    let skill_md = format!(
        "---\nname: {slug}\ndescription: \"Chunked reference skill generated from \\\"{title}\\\" (PDF). Use when you want grounded answers from this document; open relevant chapter under chapters/.\"\nallowed-tools:\n  - Read\n  - Grep\nargument-hint: [topic keywords or chNN]\n---\n\n# {title}\n**File**: {source_file}\n**Pages/Sections**: {pages} | **Source tokens**: {tokens}\n\n## How to Use This Skill\n\n- Ask without arguments to load this index.\n- Ask for a topic and open the best matching chapter.\n- Ask for a specific chunk: `ch01`, `ch02`, etc.\n\n## Chapter Index (Chunks)\n\n| # | Title | Est. tokens |\n|---|-------|-------------|\n{index}\n\n## Supporting Files\n- [glossary.md](glossary.md)\n- [patterns.md](patterns.md)\n- [cheatsheet.md](cheatsheet.md)\n",
        slug = input.slug,
        title = input.title,
        source_file = input.source_filename,
        pages = input.pages_or_sections,
        tokens = input.estimated_tokens_human,
        index = index_lines.join("\n")
    );

    fs::write(input.destination_dir.join("SKILL.md"), skill_md)
        .map_err(|e| PdfEngineError::WriteFailed(e.to_string()))?;

    Ok(SkillWriteOutput {
        skill_dir: input.destination_dir,
        chapter_count: chunks.len(),
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::pdf_engine::types::ChunkOptions;

    #[test]
    fn writes_expected_files_and_skill_index() {
        let tmp = TempDir::new().expect("tempdir");
        let destination = tmp.path().join("my-skill");

        let output = write_skill(SkillWriteInput {
            slug: "my-skill".to_string(),
            title: "My Book".to_string(),
            source_filename: "my-book.pdf".to_string(),
            pages_or_sections: 10,
            estimated_tokens_human: "~12K".to_string(),
            full_text: "Paragraph one.\n\nParagraph two.\n\nParagraph three.".to_string(),
            destination_dir: destination.clone(),
            chunk_options: ChunkOptions::default(),
        })
        .expect("write skill");

        assert!(output.chapter_count >= 1);
        assert!(destination.join("SKILL.md").exists());
        assert!(destination.join("chapters/ch01-chunk.md").exists());
        assert!(destination.join("glossary.md").exists());
        assert!(destination.join("patterns.md").exists());
        assert!(destination.join("cheatsheet.md").exists());

        let skill_md = std::fs::read_to_string(destination.join("SKILL.md")).expect("read skill md");
        assert!(skill_md.contains("chapters/ch01-chunk.md"));
    }
}

