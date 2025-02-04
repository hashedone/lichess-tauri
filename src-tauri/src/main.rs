#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use engine_directory::Engine;
use login::start_oauth_flow;
use serde_json::{json, Value};
use std::thread;
use sysinfo::{CpuExt, System, SystemExt};
use tauri::{AppHandle, Manager, State, Window};

use crate::db::Db;

mod engine_directory;
mod lichess;
mod login;

pub mod db;
pub mod schema;
pub mod utils;

#[tauri::command]
fn get_all_settings(db: State<Db>) -> Value {
    let settings = db.get_all_settings();

    let mut json = json!({});
    for setting in settings {
        json[setting.key] = json!(setting.value);
    }

    json
}

#[tauri::command]
fn update_setting(db: State<Db>, key: &str, value: &str) {
    db.update_setting(key, value);
}

#[tauri::command]
fn add_engine(db: State<Db>, engine_id: &str, binary_location: &str) {
    db.add_engine(engine_id, binary_location);
}

#[tauri::command]
fn delete_engine(db: State<Db>, engine_id: &str) {
    db.delete_engine(engine_id);
}

#[tauri::command]
fn open_path(path: String) {
    utils::open_path(path);
}

#[tauri::command]
fn get_sysinfo() -> Value {
    let mut sys = System::new_all();
    sys.refresh_all();

    json!({
        "total_memory"   : sys.total_memory(),
        "used_memory"    : sys.used_memory(),
        "total_swap"     : sys.total_swap(),
        "used_swap"      : sys.used_swap(),
        "name"           : sys.name(),
        "kernel_version" : sys.kernel_version(),
        "os_version"     : sys.os_version(),
        "long_os_version": sys.long_os_version(),
        "host_name"      : sys.host_name(),
        "distribution_id": sys.distribution_id(),
        "cpus_len"       : sys.cpus().len(),
        "cpu_cpu_usage"  : sys.global_cpu_info().cpu_usage(),
        "cpu_brand"      : sys.global_cpu_info().brand(),
        "cpu_frequency"  : sys.global_cpu_info().frequency(),
        "cpu_vendor_id"  : sys.global_cpu_info().vendor_id(),
        "cpu_name"       : sys.global_cpu_info().name(),
    })
}

#[tauri::command]
fn download_engine_to_folder(app: AppHandle, engine: Engine) -> String {
    engine_directory::install(app.path(), engine)
        .into_os_string()
        .into_string()
        .unwrap()
}

#[tauri::command]
fn get_app_data_dir(app: AppHandle) -> String {
    utils::get_app_data_dir(app.path())
        .into_os_string()
        .into_string()
        .unwrap()
}

#[tauri::command]
fn login_with_lichess(db: State<Db>, window: Window) {
    start_oauth_flow(&db, window);
}

#[tauri::command]
fn logout(db: State<Db>, window: Window) {
    login::logout(&db, window);
}

fn main() {
    pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            add_engine,
            delete_engine,
            download_engine_to_folder,
            get_all_settings,
            get_app_data_dir,
            get_sysinfo,
            login_with_lichess,
            logout,
            update_setting,
            open_path
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db = Db::new(app.path());
            db.establish_connection()
                .run_pending_migrations(MIGRATIONS)
                .unwrap();
            app.manage(db.clone());

            let app_handle = app.handle().clone();

            thread::spawn(move || match lichess::work(&app_handle, db) {
                Ok(_) => println!("Success"),
                Err(e) => println!("Error: {}", e),
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
