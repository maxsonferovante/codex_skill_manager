use codex_skill_manager_desktop::commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_roots,
            commands::list_skills,
            commands::start_operation,
            commands::cancel_operation,
            commands::resolve_conflict,
            commands::open_skills_folder,
            commands::scan_pdfs_in_directory,
            commands::start_pdf_conversion,
            commands::cancel_pdf_conversion,
            commands::pick_pdf_files,
            commands::pick_markdown_file,
            commands::load_markdown_file,
            commands::validate_template,
            commands::create_skill_from_template,
            commands::validate_import_markdown,
            commands::import_skill_markdown
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri app");
}
