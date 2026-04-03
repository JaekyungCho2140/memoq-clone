mod commands;
mod models;
mod parser;
mod tb;
mod tm;

use commands::{
    export::{export_file, save_segment},
    parser::{export_xliff, parse_file},
    tb::*,
    tm::*,
};

pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
