use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::conflict::CONFLICT_COORDINATOR;
use crate::domain::{
    ConflictAction, CreateSkillFromTemplateInput, ImportSkillMarkdownInput, OperationConflictRequiredEvent,
    OperationFinishedEvent, OperationItemResultEvent, OperationStartedEvent, ValidateImportMarkdownInput,
    ValidateImportMarkdownOutput, ValidateTemplateInput, ValidateTemplateOutput, ValidationIssue, ValidationLevel,
};
use crate::operation_manager::{OperationModeKind, OPERATION_MANAGER};
use crate::paths::resolve_roots;
use crate::skill_slug::{is_safe_slug, slugify};

fn now_iso() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{millis}")
}

fn make_issue(level: ValidationLevel, code: &str, message: &str, field: Option<&str>) -> ValidationIssue {
    ValidationIssue {
        level,
        code: code.to_string(),
        message: message.to_string(),
        field: field.map(|f| f.to_string()),
    }
}

fn has_error(issues: &[ValidationIssue]) -> bool {
    issues
        .iter()
        .any(|issue| matches!(issue.level, ValidationLevel::Error))
}

pub fn validate_template(input: &ValidateTemplateInput) -> ValidateTemplateOutput {
    let mut issues = Vec::new();
    if input.name.trim().is_empty() {
        issues.push(make_issue(
            ValidationLevel::Error,
            "name_required",
            "Skill name is required.",
            Some("name"),
        ));
    }
    if input.instructions.trim().is_empty() {
        issues.push(make_issue(
            ValidationLevel::Error,
            "instructions_required",
            "Main instructions are required.",
            Some("instructions"),
        ));
    }
    if input.slug.trim().is_empty() {
        issues.push(make_issue(
            ValidationLevel::Error,
            "slug_required",
            "Slug is required.",
            Some("slug"),
        ));
    } else if !is_safe_slug(input.slug.trim()) {
        issues.push(make_issue(
            ValidationLevel::Error,
            "slug_invalid",
            "Slug contains invalid characters.",
            Some("slug"),
        ));
    }
    if input.description.trim().is_empty() {
        issues.push(make_issue(
            ValidationLevel::Warning,
            "description_recommended",
            "Short description is recommended.",
            Some("description"),
        ));
    }

    ValidateTemplateOutput {
        can_submit: !has_error(&issues),
        issues,
    }
}

pub fn validate_import_markdown(input: &ValidateImportMarkdownInput) -> ValidateImportMarkdownOutput {
    let mut issues = validate_template(&ValidateTemplateInput {
        name: input.name.clone(),
        slug: input.slug.clone(),
        description: String::new(),
        instructions: input.content.clone(),
    })
    .issues;

    if input.content.trim().is_empty() {
        issues.push(make_issue(
            ValidationLevel::Error,
            "file_empty",
            "File is empty.",
            Some("content"),
        ));
    }

    let lower = input.content.to_lowercase();
    let has_frontmatter = input.content.trim_start().starts_with("---");
    if !has_frontmatter {
        issues.push(make_issue(
            ValidationLevel::Warning,
            "frontmatter_missing",
            "YAML frontmatter is recommended.",
            Some("content"),
        ));
    }
    if !lower.contains("# purpose") && !lower.contains("## purpose") {
        issues.push(make_issue(
            ValidationLevel::Info,
            "purpose_missing",
            "Section 'Purpose' is recommended.",
            Some("content"),
        ));
    }
    if !lower.contains("# examples") && !lower.contains("## examples") {
        issues.push(make_issue(
            ValidationLevel::Info,
            "examples_missing",
            "Section 'Examples' is recommended.",
            Some("content"),
        ));
    }

    ValidateImportMarkdownOutput {
        can_submit: !has_error(&issues),
        issues,
    }
}

fn render_template_markdown(input: &CreateSkillFromTemplateInput) -> String {
    format!(
        "---\nname: {}\ndescription: {}\ntype: component\n---\n\n# {}\n\n## Purpose\n{}\n\n## Instructions\n{}\n",
        input.slug.trim(),
        input.description.trim(),
        input.name.trim(),
        if input.description.trim().is_empty() {
            "Define what this skill should do."
        } else {
            input.description.trim()
        },
        input.instructions.trim()
    )
}

fn ensure_destination_with_conflict(
    app: &AppHandle,
    operation_id: &str,
    _mode: &str,
    slug: &str,
    initial_dst: &Path,
) -> Result<Option<PathBuf>, String> {
    let mut dst = initial_dst.to_path_buf();
    if !dst.exists() {
        return Ok(Some(dst));
    }

    let _ = app.emit(
        "operation:conflict_required",
        OperationConflictRequiredEvent {
            operation_id: operation_id.to_string(),
            slug: slug.to_string(),
            src: "-".to_string(),
            dst: dst.display().to_string(),
            allow_apply_to_all: true,
        },
    );
    let decision = CONFLICT_COORDINATOR.wait_for_conflict_decision(operation_id, slug);
    match decision.action {
        ConflictAction::Skip => Ok(None),
        ConflictAction::Rename => {
            let renamed_slug = format!("{slug}-{}", now_iso());
            dst = dst
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(renamed_slug);
            Ok(Some(dst))
        }
        ConflictAction::Overwrite => {
            fs::remove_dir_all(&dst).map_err(|e| format!("overwrite remove failed: {e}"))?;
            Ok(Some(dst))
        }
    }
}

fn write_skill_dir(destination: &Path, content: &str) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    fs::write(destination.join("SKILL.md"), content).map_err(|e| e.to_string())?;
    Ok(())
}

fn run_single_creation(
    app: AppHandle,
    operation_id: String,
    mode: &'static str,
    slug: String,
    content: String,
) {
    let _ = app.emit(
        "operation:started",
        OperationStartedEvent {
            operation_id: operation_id.clone(),
            mode: mode.to_string(),
            total: 1,
            started_at: now_iso(),
        },
    );

    let roots = resolve_roots();
    let initial_dst = PathBuf::from(roots.enabled_root).join(&slug);

    let mut ok = 0usize;
    let mut error = 0usize;
    let mut skipped = 0usize;

    match ensure_destination_with_conflict(&app, &operation_id, mode, &slug, &initial_dst) {
        Ok(Some(dst)) => match write_skill_dir(&dst, &content) {
            Ok(_) => {
                ok = 1;
                let _ = app.emit(
                    "operation:item_result",
                    OperationItemResultEvent {
                        operation_id: operation_id.clone(),
                        mode: mode.to_string(),
                        item: slug.clone(),
                        src: "-".to_string(),
                        dst: dst.display().to_string(),
                        status: "success".to_string(),
                        slug: slug.clone(),
                        error: None,
                        at: now_iso(),
                    },
                );
            }
            Err(err_msg) => {
                error = 1;
                let _ = app.emit(
                    "operation:item_result",
                    OperationItemResultEvent {
                        operation_id: operation_id.clone(),
                        mode: mode.to_string(),
                        item: slug.clone(),
                        src: "-".to_string(),
                        dst: initial_dst.display().to_string(),
                        status: "failed".to_string(),
                        slug: slug.clone(),
                        error: Some(err_msg),
                        at: now_iso(),
                    },
                );
            }
        },
        Ok(None) => {
            skipped = 1;
            let _ = app.emit(
                "operation:item_result",
                OperationItemResultEvent {
                    operation_id: operation_id.clone(),
                    mode: mode.to_string(),
                    item: slug.clone(),
                    src: "-".to_string(),
                    dst: initial_dst.display().to_string(),
                    status: "skipped".to_string(),
                    slug: slug.clone(),
                    error: None,
                    at: now_iso(),
                },
            );
        }
        Err(err_msg) => {
            error = 1;
            let _ = app.emit(
                "operation:item_result",
                OperationItemResultEvent {
                    operation_id: operation_id.clone(),
                    mode: mode.to_string(),
                    item: slug.clone(),
                    src: "-".to_string(),
                    dst: initial_dst.display().to_string(),
                    status: "failed".to_string(),
                    slug: slug.clone(),
                    error: Some(err_msg),
                    at: now_iso(),
                },
            );
        }
    }

    let _ = app.emit(
        "operation:finished",
        OperationFinishedEvent {
            operation_id: operation_id.clone(),
            mode: mode.to_string(),
            total: 1,
            attempted: 1,
            ok,
            error,
            skipped,
            cancelled: false,
            finished_at: now_iso(),
        },
    );
    CONFLICT_COORDINATOR.finish_operation(&operation_id);
    OPERATION_MANAGER.finish_if_matches(&operation_id);
}

pub fn start_template_creation(app: AppHandle, input: CreateSkillFromTemplateInput) -> Result<String, String> {
    let normalized_slug = slugify(input.slug.trim());
    let validation = validate_template(&ValidateTemplateInput {
        name: input.name.clone(),
        slug: normalized_slug.clone(),
        description: input.description.clone(),
        instructions: input.instructions.clone(),
    });
    if !validation.can_submit {
        return Err("Template validation failed.".to_string());
    }

    let operation_id = Uuid::new_v4().to_string();
    if !OPERATION_MANAGER.try_start(&operation_id, OperationModeKind::SkillCreation) {
        return Err("another operation is already running".to_string());
    }
    if !CONFLICT_COORDINATOR.start_operation(&operation_id) {
        OPERATION_MANAGER.finish_if_matches(&operation_id);
        return Err("another operation is already running".to_string());
    }

    let content = render_template_markdown(&CreateSkillFromTemplateInput {
        slug: normalized_slug.clone(),
        ..input
    });
    let app_for_thread = app.clone();
    let operation_for_thread = operation_id.clone();
    thread::spawn(move || {
        run_single_creation(app_for_thread, operation_for_thread, "template", normalized_slug, content);
    });

    Ok(operation_id)
}

pub fn start_import_markdown_creation(app: AppHandle, input: ImportSkillMarkdownInput) -> Result<String, String> {
    let normalized_slug = slugify(input.slug.trim());
    let validation = validate_import_markdown(&ValidateImportMarkdownInput {
        name: input.name.clone(),
        slug: normalized_slug.clone(),
        content: input.content.clone(),
    });
    if !validation.can_submit {
        return Err("Import validation failed.".to_string());
    }

    let operation_id = Uuid::new_v4().to_string();
    if !OPERATION_MANAGER.try_start(&operation_id, OperationModeKind::SkillCreation) {
        return Err("another operation is already running".to_string());
    }
    if !CONFLICT_COORDINATOR.start_operation(&operation_id) {
        OPERATION_MANAGER.finish_if_matches(&operation_id);
        return Err("another operation is already running".to_string());
    }

    let app_for_thread = app.clone();
    let operation_for_thread = operation_id.clone();
    let content = input.content;
    thread::spawn(move || {
        run_single_creation(
            app_for_thread,
            operation_for_thread,
            "import-md",
            normalized_slug,
            content,
        );
    });

    Ok(operation_id)
}

#[cfg(test)]
mod tests {
    use super::{validate_import_markdown, validate_template};
    use crate::domain::{ValidateImportMarkdownInput, ValidateTemplateInput};

    #[test]
    fn template_validation_blocks_name_and_instructions() {
        let out = validate_template(&ValidateTemplateInput {
            name: String::new(),
            slug: "my-skill".to_string(),
            description: String::new(),
            instructions: String::new(),
        });
        assert!(!out.can_submit);
        assert!(out.issues.len() >= 2);
    }

    #[test]
    fn import_validation_allows_warnings_but_blocks_empty() {
        let err = validate_import_markdown(&ValidateImportMarkdownInput {
            name: "A".to_string(),
            slug: "a".to_string(),
            content: String::new(),
        });
        assert!(!err.can_submit);

        let warn = validate_import_markdown(&ValidateImportMarkdownInput {
            name: "A".to_string(),
            slug: "a".to_string(),
            content: "# title\nhello".to_string(),
        });
        assert!(warn.can_submit);
    }
}
