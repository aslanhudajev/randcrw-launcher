//! randcrw launcher backend. The page is a thin view; everything that touches files,
//! processes, dialogs or the network lives here.

mod commands;
pub mod contract;
pub mod extractor;
pub mod github;
pub mod launch;
pub mod paths;
pub mod settings;
pub mod versions;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let default_root = commands::default_root_or_fallback(app.handle());
            let env_root = std::env::var_os(paths::ROOT_ENV).map(Into::into);
            app.manage(commands::Launcher::init(default_root, env_root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::game_status,
            commands::pick_iso,
            commands::start_extract,
            commands::start_verify,
            commands::cancel_job,
            commands::uninstall_game,
            commands::launch_game,
            commands::open_folder,
            commands::open_url,
            commands::pick_folder,
            commands::move_data_root,
            commands::set_ntsc_only,
            commands::list_versions,
            commands::add_dev_version,
            commands::validate_version,
            commands::remove_dev_version,
            commands::set_active_version,
            commands::official_releases,
            commands::set_official_config,
            commands::download_official,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the randcrw launcher");
}
