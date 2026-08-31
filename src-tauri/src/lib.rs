mod commands;
mod domain;
mod picker;

use std::sync::Mutex;

use commands::diff::DiffStore;
use commands::excel::ExcelStore;
use commands::folder::FolderStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(DiffStore::default()))
        .manage(Mutex::new(FolderStore::default()))
        .manage(Mutex::new(ExcelStore::default()))
        .invoke_handler(tauri::generate_handler![
            commands::diff::compare_texts,
            commands::diff::get_diff_rows,
            commands::file::read_text_file,
            commands::folder::start_folder_compare,
            commands::folder::list_folder_rows,
            commands::folder::list_folder_children,
            commands::folder::folder_enter,
            commands::folder::folder_up,
            commands::folder::cancel_folder_compare,
            commands::folder::read_folder_entry,
            commands::folder::java_available,
            commands::folder::decompile_class,
            commands::excel::compare_excel,
            commands::excel::get_excel_rows,
            picker::pick_compare_root
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
