mod archive;
mod book;
mod cache;
mod commands;
mod image_pipeline;
mod library;
mod media_source;
mod sorting;

use commands::AppState;
use tauri::Manager;

#[cfg(windows)]
fn set_windows_icons(window: &tauri::WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    use windows_sys::Win32::{
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            LoadImageW, SendMessageW, ICON_BIG, IMAGE_ICON, LR_SHARED, WM_SETICON,
        },
    };
    let icon = image::load_from_memory(include_bytes!("../icons/title-bar.png"))?.to_rgba8();
    let (width, height) = icon.dimensions();
    let hwnd = window.hwnd()?;
    // tauri-build embeds the bundle ICO as resource 32512. Shared handles are
    // owned by Windows and must not be destroyed by the application.
    let taskbar_icon = unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        if module.is_null() {
            return Err(std::io::Error::last_os_error().into());
        }
        LoadImageW(
            module,
            32512usize as *const u16,
            IMAGE_ICON,
            128,
            128,
            LR_SHARED,
        )
    };
    if taskbar_icon.is_null() {
        return Err(std::io::Error::last_os_error().into());
    }
    // Pin the green ICON_BIG before replacing ICON_SMALL, which would otherwise
    // also be used as the taskbar fallback.
    unsafe {
        SendMessageW(
            hwnd.0.cast(),
            WM_SETICON,
            ICON_BIG as usize,
            taskbar_icon as isize,
        );
    }
    window.set_icon(tauri::image::Image::new_owned(
        icon.into_raw(),
        width,
        height,
    ))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .manage(media_source::ScanState::default())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            let library = library::Library::open(&directory).map_err(std::io::Error::other)?;
            app.manage(library);
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = set_windows_icons(&window) {
                    eprintln!("Failed to set window icons: {error}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_path,
            commands::render_page,
            commands::open_library_book,
            commands::open_bookmark,
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
            library::open_library_video,
            library::show_library_source_location,
            media_source::scan_library_sources,
            media_source::cancel_library_scan,
            library::list_library_roots,
            library::remember_library_roots,
            library::forget_library_root,
            library::replace_library_cover,
            library::edit_library_book,
            library::set_reading_status,
            library::list_tags,
            library::create_tag,
            library::rename_tag,
            library::delete_tag,
            library::assign_book_tag,
            library::assign_book_series,
            library::list_bookmarks,
            library::save_bookmark,
            library::delete_bookmark,
            library::preview_library_backup,
            library::get_backup_settings,
            library::get_backup_directory,
            library::open_backup_directory,
            library::set_backup_directory,
            library::set_backup_settings,
            library::run_automatic_backup,
            library::restore_library_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
