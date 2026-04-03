mod commands;
mod models;
mod parser;
mod tm;
mod tb;

use commands::{parser::{parse_file, export_xliff}, tm::*, tb::*, export::*};

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
