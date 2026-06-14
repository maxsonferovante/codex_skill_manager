use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::conflict::CONFLICT_COORDINATOR;
use crate::domain::{
    ConflictAction, OperationCancelRequestedEvent, OperationConflictRequiredEvent, OperationFinishedEvent,
    OperationItemLogEvent, OperationItemResultEvent, OperationStartedEvent,
};
use crate::operation_manager::OPERATION_MANAGER;
use crate::planner::MovePlanItem;

fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{millis}")
}

fn timestamp_suffix() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn unique_timestamp_path(dst: &PathBuf) -> PathBuf {
    let slug = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "skill".to_string());
    let parent = dst
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut candidate = parent.join(format!("{slug}-{}", timestamp_suffix()));
    let mut counter = 2usize;
    while candidate.exists() {
        candidate = parent.join(format!("{slug}-{}-{counter}", timestamp_suffix()));
        counter += 1;
    }
    candidate
}

pub fn run_operation(app: AppHandle, operation_id: String, plan: Vec<MovePlanItem>) {
    let mode = "skills_move".to_string();
    let total = plan.len();
    let _ = app.emit(
        "operation:started",
        OperationStartedEvent {
            operation_id: operation_id.clone(),
            mode: mode.clone(),
            total,
            started_at: now_iso(),
        },
    );

    let mut attempted = 0usize;
    let mut ok = 0usize;
    let mut error = 0usize;
    let mut skipped = 0usize;
    let mut cancelled = false;

    for item in plan {
        if CONFLICT_COORDINATOR.is_cancel_requested(&operation_id) {
            cancelled = true;
            let _ = app.emit(
                "operation:cancel_requested",
                OperationCancelRequestedEvent {
                    operation_id: operation_id.clone(),
                    at: now_iso(),
                },
            );
            break;
        }

        attempted += 1;
        let mut dst = item.dst.clone();
        let src = item.src.clone();
        let slug = item.slug.clone();

        if !src.exists() {
            error += 1;
            let _ = app.emit(
                "operation:item_result",
                OperationItemResultEvent {
                    operation_id: operation_id.clone(),
                    mode: mode.clone(),
                    item: slug.clone(),
                    src: src.display().to_string(),
                    dst: dst.display().to_string(),
                    status: "error".to_string(),
                    slug,
                    error: Some("source path does not exist".to_string()),
                    at: now_iso(),
                },
            );
            continue;
        }

        if let Some(parent) = dst.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if dst.exists() {
            let _ = app.emit(
                "operation:conflict_required",
                OperationConflictRequiredEvent {
                    operation_id: operation_id.clone(),
                    slug: item.slug.clone(),
                    src: src.display().to_string(),
                    dst: dst.display().to_string(),
                    allow_apply_to_all: true,
                },
            );

            let decision = CONFLICT_COORDINATOR.wait_for_conflict_decision(&operation_id, &item.slug);
            match decision.action {
                ConflictAction::Skip => {
                    skipped += 1;
                    let _ = app.emit(
                        "operation:item_result",
                        OperationItemResultEvent {
                            operation_id: operation_id.clone(),
                            mode: mode.clone(),
                            item: slug.clone(),
                            src: src.display().to_string(),
                            dst: dst.display().to_string(),
                            status: "skipped".to_string(),
                            slug,
                            error: None,
                            at: now_iso(),
                        },
                    );
                    continue;
                }
                ConflictAction::Rename => {
                    dst = unique_timestamp_path(&dst);
                }
                ConflictAction::Overwrite => {
                    if let Err(exc) = fs::remove_dir_all(&dst) {
                        error += 1;
                        let _ = app.emit(
                            "operation:item_result",
                            OperationItemResultEvent {
                                operation_id: operation_id.clone(),
                                mode: mode.clone(),
                                item: slug.clone(),
                                src: src.display().to_string(),
                                dst: dst.display().to_string(),
                                status: "error".to_string(),
                                slug,
                                error: Some(format!("overwrite remove failed: {exc}")),
                                at: now_iso(),
                            },
                        );
                        continue;
                    }
                }
            }
        }

        match fs::rename(&src, &dst) {
            Ok(_) => {
                ok += 1;
                let _ = app.emit(
                    "operation:item_result",
                    OperationItemResultEvent {
                        operation_id: operation_id.clone(),
                        mode: mode.clone(),
                        item: slug.clone(),
                        src: src.display().to_string(),
                        dst: dst.display().to_string(),
                        status: "moved".to_string(),
                        slug,
                        error: None,
                        at: now_iso(),
                    },
                );
            }
            Err(exc) => {
                error += 1;
                let _ = app.emit(
                    "operation:item_result",
                    OperationItemResultEvent {
                        operation_id: operation_id.clone(),
                        mode: mode.clone(),
                        item: slug.clone(),
                        src: src.display().to_string(),
                        dst: dst.display().to_string(),
                        status: "error".to_string(),
                        slug,
                        error: Some(exc.to_string()),
                        at: now_iso(),
                    },
                );
            }
        }
    }

    let _ = app.emit(
        "operation:finished",
        OperationFinishedEvent {
            operation_id: operation_id.clone(),
            mode: mode.clone(),
            total,
            attempted,
            ok,
            error,
            skipped,
            cancelled,
            finished_at: now_iso(),
        },
    );
    let _ = app.emit(
        "operation:item_log",
        OperationItemLogEvent {
            operation_id: operation_id.clone(),
            mode,
            item: "-".to_string(),
            level: "info".to_string(),
            message: format!("Done: OK={ok} ERROR={error} SKIPPED={skipped} TOTAL={total}."),
            at: now_iso(),
        },
    );
    CONFLICT_COORDINATOR.finish_operation(&operation_id);
    OPERATION_MANAGER.finish_if_matches(&operation_id);
}
