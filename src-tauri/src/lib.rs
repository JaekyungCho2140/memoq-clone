pub mod commands;
pub mod error;
pub mod livedocs;
pub mod models;
pub mod mt;
pub mod parser;
pub mod plugin;
pub mod qa;
pub mod tb;
pub mod tm;

use commands::{
    export::{export_file, save_segment},
    livedocs::{
        livedocs_add_document, livedocs_create_library, livedocs_list_libraries, livedocs_search,
    },
    mt::{mt_get_providers, mt_save_api_key, mt_translate},
    parser::{export_xliff, parse_file},
    plugin::{
        plugin_list, plugin_mt_translate, plugin_qa_check, plugin_scan, plugin_set_enabled,
        PluginState,
    },
    project::{
        add_file_to_project, get_project_stats, get_recent_projects, load_project,
        remove_file_from_project, save_project,
    },
    qa::run_qa_check,
    tb::*,
    tm::*,
};

pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(PluginState(std::sync::Mutex::new(
            crate::plugin::PluginRegistry::new(),
        )))
        .invoke_handler(tauri::generate_handler![
            parse_file,
            export_xliff,
            export_file,
            save_segment,
            tm_create,
            tm_add,
            tm_search,
            tb_create,
            tb_add,
            tb_lookup,
            run_qa_check,
            mt_translate,
            mt_save_api_key,
            mt_get_providers,
            add_file_to_project,
            remove_file_from_project,
            save_project,
            load_project,
            get_project_stats,
            get_recent_projects,
            livedocs_create_library,
            livedocs_add_document,
            livedocs_list_libraries,
            livedocs_search,
            plugin_list,
            plugin_scan,
            plugin_set_enabled,
            plugin_mt_translate,
            plugin_qa_check,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
