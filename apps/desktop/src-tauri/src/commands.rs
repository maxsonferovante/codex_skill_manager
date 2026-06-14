use std::thread;

use tauri::AppHandle;
use uuid::Uuid;

use crate::conflict::CONFLICT_COORDINATOR;
use crate::domain::{
    CancelOperationInput, CancelOperationOutput, ListSkillsInput, ListSkillsOutput, ResolveConflictInput,
    ResolveConflictOutput, RootsInfo, StartOperationInput, StartOperationOutput, OpenSkillsFolderOutput,
    PickMarkdownFileOutput, LoadMarkdownFileInput, LoadMarkdownFileOutput, ValidateTemplateInput,
    ValidateTemplateOutput, CreateSkillFromTemplateInput, ValidateImportMarkdownInput,
    ValidateImportMarkdownOutput, ImportSkillMarkdownInput,
};
use crate::executor::run_operation;
use crate::operation_manager::{OperationModeKind, OPERATION_MANAGER};
use crate::list::list_skills as list_skills_impl;
use crate::paths::resolve_roots;
use crate::planner::build_plan;
use crate::platform::open_skills_folder as open_skills_folder_impl;
use crate::pdf_engine::{cancel_pdf_conversion as cancel_pdf_conversion_impl, scan::scan_pdfs, start_pdf_conversion as start_pdf_conversion_impl};
use crate::creation::{validate_template as validate_template_impl, validate_import_markdown as validate_import_markdown_impl, start_template_creation, start_import_markdown_creation};
use std::path::PathBuf;

#[tauri::command]
pub fn get_roots() -> RootsInfo {
    resolve_roots()
}

#[tauri::command]
pub fn list_skills(input: ListSkillsInput) -> ListSkillsOutput {
    list_skills_impl(input)
}

#[tauri::command]
pub fn start_operation(app: AppHandle, input: StartOperationInput) -> Result<StartOperationOutput, String> {
    let operation_id = Uuid::new_v4().to_string();
    if !OPERATION_MANAGER.try_start(&operation_id, OperationModeKind::SkillsMove) {
        return Err("another operation is already running".to_string());
    }
    if !CONFLICT_COORDINATOR.start_operation(&operation_id) {
        OPERATION_MANAGER.finish_if_matches(&operation_id);
        return Err("another operation is already running".to_string());
    }
    let plan = build_plan(&input);
    let planned_total = plan.len();
    let app_for_thread = app.clone();
    let operation_id_for_thread = operation_id.clone();
    thread::spawn(move || {
        run_operation(app_for_thread, operation_id_for_thread, plan);
    });
    Ok(StartOperationOutput {
        operation_id,
        planned_total,
    })
}

#[tauri::command]
pub fn cancel_operation(input: CancelOperationInput) -> CancelOperationOutput {
    CancelOperationOutput {
        accepted: CONFLICT_COORDINATOR.request_cancel(&input.operation_id),
    }
}

#[tauri::command]
pub fn resolve_conflict(input: ResolveConflictInput) -> ResolveConflictOutput {
    ResolveConflictOutput {
        accepted: CONFLICT_COORDINATOR.resolve_conflict(input),
    }
}

#[tauri::command]
pub fn open_skills_folder() -> OpenSkillsFolderOutput {
    open_skills_folder_impl()
}

#[tauri::command]
pub fn scan_pdfs_in_directory(input: crate::domain::ScanPdfsInput) -> Result<crate::domain::ScanPdfsOutput, String> {
    let tasks = scan_pdfs(PathBuf::from(input.directory).as_path()).map_err(|e| e.to_string())?;
    Ok(crate::domain::ScanPdfsOutput {
        files: tasks.into_iter().map(|t| t.file_name).collect(),
    })
}

#[tauri::command]
pub fn start_pdf_conversion(
    app: AppHandle,
    input: crate::domain::StartPdfConversionInput,
) -> Result<crate::domain::StartPdfConversionOutput, String> {
    if OPERATION_MANAGER.get_active().is_some() {
        return Err("another operation is already running".to_string());
    }
    start_pdf_conversion_impl(app, input)
}

#[tauri::command]
pub fn cancel_pdf_conversion(input: crate::domain::CancelPdfConversionInput) -> crate::domain::CancelPdfConversionOutput {
    cancel_pdf_conversion_impl(&input.operation_id)
}

#[tauri::command]
pub fn pick_pdf_files() -> crate::domain::PickPdfFilesOutput {
    let paths = rfd::FileDialog::new()
        .add_filter("PDF", &["pdf"])
        .set_title("Select PDF files")
        .pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|path| path.display().to_string())
        .collect();
    crate::domain::PickPdfFilesOutput { paths }
}

#[tauri::command]
pub fn pick_markdown_file() -> PickMarkdownFileOutput {
    let path = rfd::FileDialog::new()
        .add_filter("Markdown", &["md"])
        .set_title("Select Markdown file")
        .pick_file()
        .map(|path| path.display().to_string());
    PickMarkdownFileOutput { path }
}

#[tauri::command]
pub fn load_markdown_file(input: LoadMarkdownFileInput) -> Result<LoadMarkdownFileOutput, String> {
    let content = std::fs::read_to_string(&input.path).map_err(|err| err.to_string())?;
    Ok(LoadMarkdownFileOutput {
        path: input.path,
        content,
    })
}

#[tauri::command]
pub fn validate_template(input: ValidateTemplateInput) -> ValidateTemplateOutput {
    validate_template_impl(&input)
}

#[tauri::command]
pub fn create_skill_from_template(
    app: AppHandle,
    input: CreateSkillFromTemplateInput,
) -> Result<StartOperationOutput, String> {
    let operation_id = start_template_creation(app, input)?;
    Ok(StartOperationOutput {
        operation_id,
        planned_total: 1,
    })
}

#[tauri::command]
pub fn validate_import_markdown(input: ValidateImportMarkdownInput) -> ValidateImportMarkdownOutput {
    validate_import_markdown_impl(&input)
}

#[tauri::command]
pub fn import_skill_markdown(
    app: AppHandle,
    input: ImportSkillMarkdownInput,
) -> Result<StartOperationOutput, String> {
    let operation_id = start_import_markdown_creation(app, input)?;
    Ok(StartOperationOutput {
        operation_id,
        planned_total: 1,
    })
}
