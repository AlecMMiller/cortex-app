// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use commands::notes::{
    create_note, get_all_notes, get_last_updated_note, get_note, get_notes_by_title, rename_note,
    update_note,
};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::{embed_migrations, MigrationHarness};
use std::fs::create_dir;
use std::fs::create_dir_all;
use tantivy::IndexWriter;
use tauri::Manager;
mod models;
mod schema;

mod commands;
mod lexical;
mod macros;
mod notes;
mod search;
mod utils;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct PoolWrapper {
    pub pool: SqlitePool,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let path = app
                .path()
                .app_data_dir()
                .expect("This should never be None");

            create_dir_all(&path).unwrap();
            let db_path = path.join("data.db");
            let db_path = db_path.to_str().expect("This should never be None");

            let mut conn = SqliteConnection::establish(db_path)
                .expect("Could not establish connection to database");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("Could not run migrations");

            let manager = ConnectionManager::<SqliteConnection>::new(db_path);
            let pool = Pool::builder()
                .build(manager)
                .expect("Could not create connection pool");
            let pool = PoolWrapper { pool };

            app.manage(pool);

            let tantivy_path = path.join("tantivy");

            let mut needs_reindex = false;
            match create_dir(tantivy_path.clone()) {
                Ok(_) => {
                    needs_reindex = true;
                }
                Err(_) => {}
            };
            let (tantivy_index, needs_reindex) = search::initialize(tantivy_path, needs_reindex)
                .expect("Tantivy should be able to create an index");

            println!("{needs_reindex}");
            app.manage(tantivy_index.clone());

            let index_writer: IndexWriter = tantivy_index
                .writer(30_000_000)
                .expect("Should be able to create an index writer");
            app.manage(std::sync::Mutex::new(index_writer));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_note,
            get_all_notes,
            get_last_updated_note,
            update_note,
            get_notes_by_title,
            create_note,
            rename_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
