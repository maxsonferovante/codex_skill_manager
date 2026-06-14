pub mod chunker;
pub mod extractor;
pub mod metadata;
pub mod scan;
pub mod skill_writer;
pub mod slug;
pub mod types;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::domain::{
    CancelPdfConversionOutput, OperationFinishedEvent, OperationItemLogEvent, OperationItemResultEvent,
    OperationStartedEvent, PdfExtractionModeInput, PdfFileLogEvent, PdfFileProgressEvent, PdfFileResultEvent,
    PdfFinishedEvent, PdfStartedEvent, StartPdfConversionInput, StartPdfConversionOutput,
};
use crate::operation_manager::{OperationModeKind, OPERATION_MANAGER};
use crate::paths::resolve_roots;

use self::extractor::extract_pdf;
use self::metadata::{build_metadata, write_extraction_artifacts};
use self::scan::scan_pdfs;
use self::skill_writer::write_skill;
use self::slug::{resolve_unique_skill_dir, slugify};
use self::types::{ChunkOptions, PdfExtractionMode, PdfFileTask, SkillWriteInput};

struct PdfOperationState {
    operation_id: String,
    cancel: Arc<AtomicBool>,
}

static PDF_OPERATION: Lazy<Mutex<Option<PdfOperationState>>> = Lazy::new(|| Mutex::new(None));

fn map_mode(mode: PdfExtractionModeInput) -> PdfExtractionMode {
    match mode {
        PdfExtractionModeInput::Technical => PdfExtractionMode::Technical,
        PdfExtractionModeInput::Text => PdfExtractionMode::Text,
    }
}

fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{millis}")
}

fn with_selected_files(tasks: Vec<PdfFileTask>, selected_files: Option<Vec<String>>) -> Vec<PdfFileTask> {
    let Some(selected) = selected_files else {
        return tasks;
    };
    if selected.is_empty() {
        return tasks;
    }
    tasks.into_iter()
        .filter(|task| {
            selected.iter().any(|selected_item| {
                selected_item == &task.file_name || selected_item == &task.absolute_path.display().to_string()
            })
        })
        .collect()
}

fn is_cancelled(cancel: &Arc<AtomicBool>) -> bool {
    cancel.load(Ordering::SeqCst)
}

fn clear_operation(operation_id: &str) {
    if let Ok(mut guard) = PDF_OPERATION.lock() {
        if let Some(active) = guard.as_ref() {
            if active.operation_id == operation_id {
                *guard = None;
            }
        }
    }
    OPERATION_MANAGER.finish_if_matches(operation_id);
}

fn emit_file_log(app: &AppHandle, operation_id: &str, file: &str, message: &str) {
    let _ = app.emit(
        "pdf:file_log",
        PdfFileLogEvent {
            operation_id: operation_id.to_string(),
            file: file.to_string(),
            message: message.to_string(),
        },
    );
}

fn run_pdf_conversion(app: AppHandle, operation_id: String, cancel: Arc<AtomicBool>, input: StartPdfConversionInput) {
    let mode_label = "pdf".to_string();
    let mut success_count = 0usize;
    let mut failure_count = 0usize;
    let mut cancelled = false;

    let directory = PathBuf::from(&input.directory);
    let tasks = match scan_pdfs(&directory) {
        Ok(tasks) => with_selected_files(tasks, input.selected_files.clone()),
        Err(err) => {
            let _ = app.emit(
                "pdf:finished",
                PdfFinishedEvent {
                    operation_id: operation_id.clone(),
                    success_count: 0,
                    failure_count: 1,
                    cancelled: false,
                },
            );
            emit_file_log(&app, &operation_id, "-", &format!("Scan failed: {err}"));
            clear_operation(&operation_id);
            return;
        }
    };
    let total = tasks.len();
    let _ = app.emit(
        "operation:started",
        OperationStartedEvent {
            operation_id: operation_id.clone(),
            mode: mode_label.clone(),
            total,
            started_at: now_iso(),
        },
    );

    let _ = app.emit(
        "pdf:started",
        PdfStartedEvent {
            operation_id: operation_id.clone(),
            total,
        },
    );

    let roots = resolve_roots();
    let skills_root = PathBuf::from(roots.enabled_root);
    let mode = map_mode(input.mode);
    let chunk_options = ChunkOptions {
        target_chunk_tokens: input.target_chunk_tokens.unwrap_or(1400),
        max_chunk_tokens: input.max_chunk_tokens.unwrap_or(2000),
    };

    for (index, task) in tasks.iter().enumerate() {
        if is_cancelled(&cancel) {
            cancelled = true;
            break;
        }
        let _ = app.emit(
            "pdf:file_progress",
            PdfFileProgressEvent {
                operation_id: operation_id.clone(),
                current_index: index + 1,
                total,
                file: task.file_name.clone(),
            },
        );
        let log_file = task.file_name.clone();
        let extraction = extract_pdf(task, mode, |message| emit_file_log(&app, &operation_id, &log_file, message));

        match extraction {
            Ok(output) => {
                let stem = task
                    .absolute_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("skill");
                let slug = slugify(stem);
                let skill_dir = resolve_unique_skill_dir(&skills_root, &slug, input.overwrite);
                if input.overwrite && skill_dir.exists() {
                    let _ = std::fs::remove_dir_all(&skill_dir);
                }
                let metadata = build_metadata(&task.file_name, match mode { PdfExtractionMode::Technical => "technical", PdfExtractionMode::Text => "text" }, &output);
                let write_result = write_skill(SkillWriteInput {
                    slug: slug.clone(),
                    title: stem.to_string(),
                    source_filename: task.file_name.clone(),
                    pages_or_sections: output.pages_estimate,
                    estimated_tokens_human: metadata.estimated_tokens_human.clone(),
                    full_text: output.full_text.clone(),
                    destination_dir: skill_dir.clone(),
                    chunk_options: chunk_options.clone(),
                });
                match write_result {
                    Ok(_) => {
                        let extraction_dir = skill_dir.join(".extraction");
                        let _ = write_extraction_artifacts(&extraction_dir, &metadata, &output.full_text);
                        success_count += 1;
                        let _ = app.emit(
                            "operation:item_result",
                            OperationItemResultEvent {
                                operation_id: operation_id.clone(),
                                mode: mode_label.clone(),
                                item: task.file_name.clone(),
                                src: task.absolute_path.display().to_string(),
                                dst: skill_dir.display().to_string(),
                                status: "success".to_string(),
                                slug: slug.clone(),
                                error: None,
                                at: now_iso(),
                            },
                        );
                        let _ = app.emit(
                            "pdf:file_result",
                            PdfFileResultEvent {
                                operation_id: operation_id.clone(),
                                file: task.file_name.clone(),
                                status: "success".to_string(),
                                output_skill_path: Some(skill_dir.display().to_string()),
                                error: None,
                            },
                        );
                    }
                    Err(err) => {
                        failure_count += 1;
                        let _ = app.emit(
                            "operation:item_result",
                            OperationItemResultEvent {
                                operation_id: operation_id.clone(),
                                mode: mode_label.clone(),
                                item: task.file_name.clone(),
                                src: task.absolute_path.display().to_string(),
                                dst: skill_dir.display().to_string(),
                                status: "failed".to_string(),
                                slug: slug.clone(),
                                error: Some(err.to_string()),
                                at: now_iso(),
                            },
                        );
                        let _ = app.emit(
                            "pdf:file_result",
                            PdfFileResultEvent {
                                operation_id: operation_id.clone(),
                                file: task.file_name.clone(),
                                status: "failed".to_string(),
                                output_skill_path: None,
                                error: Some(err.to_string()),
                            },
                        );
                    }
                }
            }
            Err(err) => {
                failure_count += 1;
                let _ = app.emit(
                    "operation:item_result",
                    OperationItemResultEvent {
                        operation_id: operation_id.clone(),
                        mode: mode_label.clone(),
                        item: task.file_name.clone(),
                        src: task.absolute_path.display().to_string(),
                        dst: "".to_string(),
                        status: "failed".to_string(),
                        slug: slugify(
                            task.absolute_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("skill"),
                        ),
                        error: Some(err.to_string()),
                        at: now_iso(),
                    },
                );
                let _ = app.emit(
                    "pdf:file_result",
                    PdfFileResultEvent {
                        operation_id: operation_id.clone(),
                        file: task.file_name.clone(),
                        status: "failed".to_string(),
                        output_skill_path: None,
                        error: Some(err.to_string()),
                    },
                );
            }
        }
    }

    let _ = app.emit(
        "pdf:finished",
        PdfFinishedEvent {
            operation_id: operation_id.clone(),
            success_count,
            failure_count,
            cancelled,
        },
    );
    let _ = app.emit(
        "operation:item_log",
        OperationItemLogEvent {
            operation_id: operation_id.clone(),
            mode: mode_label.clone(),
            item: "-".to_string(),
            level: "info".to_string(),
            message: format!(
                "Finished: OK={} FAIL={} CANCELLED={}",
                success_count, failure_count, cancelled
            ),
            at: now_iso(),
        },
    );
    let _ = app.emit(
        "operation:finished",
        OperationFinishedEvent {
            operation_id: operation_id.clone(),
            mode: mode_label,
            total,
            attempted: success_count + failure_count,
            ok: success_count,
            error: failure_count,
            skipped: 0,
            cancelled,
            finished_at: now_iso(),
        },
    );
    clear_operation(&operation_id);
}

pub fn start_pdf_conversion(app: AppHandle, input: StartPdfConversionInput) -> Result<StartPdfConversionOutput, String> {
    let operation_id = Uuid::new_v4().to_string();
    let tasks = with_selected_files(
        scan_pdfs(PathBuf::from(&input.directory).as_path()).map_err(|e| e.to_string())?,
        input.selected_files.clone(),
    );
    let planned_total = tasks.len();

    let cancel = Arc::new(AtomicBool::new(false));
    if !OPERATION_MANAGER.try_start(&operation_id, OperationModeKind::PdfConversion) {
        return Err("another operation is already running".to_string());
    }
    {
        let mut guard = PDF_OPERATION
            .lock()
            .map_err(|_| "failed to lock pdf operation state".to_string())?;
        if guard.is_some() {
            OPERATION_MANAGER.finish_if_matches(&operation_id);
            return Err("another pdf conversion is already running".to_string());
        }
        *guard = Some(PdfOperationState {
            operation_id: operation_id.clone(),
            cancel: cancel.clone(),
        });
    }

    let app_for_thread = app.clone();
    let operation_id_for_thread = operation_id.clone();
    thread::spawn(move || {
        run_pdf_conversion(app_for_thread, operation_id_for_thread, cancel, input);
    });

    Ok(StartPdfConversionOutput {
        operation_id,
        planned_total,
    })
}

pub fn cancel_pdf_conversion(operation_id: &str) -> CancelPdfConversionOutput {
    let mut accepted = false;
    if let Ok(guard) = PDF_OPERATION.lock() {
        if let Some(active) = guard.as_ref() {
            if active.operation_id == operation_id {
                active.cancel.store(true, Ordering::SeqCst);
                accepted = true;
            }
        }
    }
    CancelPdfConversionOutput { accepted }
}
