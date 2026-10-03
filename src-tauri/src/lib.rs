mod archive;
mod book;
mod cache;
mod commands;
mod image_pipeline;
mod library;
mod sorting;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            let library = library::Library::open(&directory).map_err(std::io::Error::other)?;
            app.manage(library);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_path,
            commands::render_page,
            commands::open_library_book,
            library::list_library,
            library::import_book,
            library::set_favorite,
            library::save_reading_progress,
            library::library_cover,
            library::remove_library_books,
            library::favorite_library_books,
            library::relink_library_book,
            library::export_library_backup,
            library::import_book_result,
            library::edit_library_book,
            library::set_reading_status,
            library::list_tags,
            library::create_tag,
            library::rename_tag,
            library::delete_tag,
            library::assign_book_tag,
            library::restore_library_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
