//! Coral — Tauri 2 backend: IPC command registration and app bootstrap.

pub mod cli;
mod commands;
pub mod core;
pub mod utils;

#[cfg(test)]
mod ipc_tests;

use tauri::{Builder, Runtime};
use utils::jobs::JobRegistry;
use utils::logbus::{self, Level, LogBus};

/// State, startup hook and IPC commands, shared by the real app and the IPC tests.
fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
        .manage(LogBus::default())
        .manage(JobRegistry::default())
        .manage(commands::watch_cmd::WatchRegistry::default())
        .manage(commands::CoralState::default())
        .setup(|app| {
            logbus::emit(
                app.handle(),
                Level::Info,
                "app",
                format!("Coral v{} started ({} {})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_cmd::app_info,
            commands::app_cmd::log_history,
            commands::app_cmd::ui_log,
            commands::app_cmd::cancel_job,
            commands::app_cmd::open_url,
            commands::app_cmd::reveal_in_explorer,
            commands::app_cmd::get_settings,
            commands::app_cmd::set_settings,
            commands::project_cmd::load_project,
            commands::project_cmd::save_project,
            commands::project_cmd::sources_for_paths,
            commands::project_cmd::scan_sources,
            commands::project_cmd::pack_preview,
            commands::project_cmd::pack_and_export,
            commands::project_cmd::sprite_thumbnail,
            commands::project_cmd::preview_png,
            commands::project_cmd::image_png,
            commands::tools_cmd::slice_sheet,
            commands::tools_cmd::apply_slice,
            commands::tools_cmd::build_flipbook,
            commands::tools_cmd::palette_extract,
            commands::tools_cmd::palette_save,
            commands::tools_cmd::palette_load,
            commands::tools_cmd::palette_remap,
            commands::tools_cmd::palette_swap,
            commands::tools_cmd::upscale_nearest,
            commands::tools_cmd::alpha_bleed,
            commands::tools_cmd::normal_map,
            commands::tools_cmd::normal_preview,
            commands::watch_cmd::watch_start,
            commands::watch_cmd::watch_stop,
            commands::watch_cmd::watch_list,
        ])
}

pub fn run() {
    configure(
        Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init()),
    )
    .run(tauri::generate_context!())
    .expect("error while running Coral");
}
