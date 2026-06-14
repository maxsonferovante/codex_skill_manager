use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfExtractionMode {
    Technical,
    Text,
}

#[derive(Debug, Clone)]
pub struct PdfFileTask {
    pub file_name: String,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
    pub est_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct ChunkOptions {
    pub target_chunk_tokens: usize,
    pub max_chunk_tokens: usize,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            target_chunk_tokens: 1400,
            max_chunk_tokens: 2000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillWriteInput {
    pub slug: String,
    pub title: String,
    pub source_filename: String,
    pub pages_or_sections: usize,
    pub estimated_tokens_human: String,
    pub full_text: String,
    pub destination_dir: PathBuf,
    pub chunk_options: ChunkOptions,
}

#[derive(Debug, Clone)]
pub struct SkillWriteOutput {
    pub skill_dir: PathBuf,
    pub chapter_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PdfEngineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("scan failed: {0}")]
    ScanFailed(String),
    #[error("write failed: {0}")]
    WriteFailed(String),
    #[error("chunking failed: {0}")]
    ChunkingFailed(String),
}

