// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod db;
mod settings;
mod encryption;

use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tokio::sync::Mutex;

// App state
struct AppState {
    db: Arc<Mutex<db::Database>>,
    clipboard: Arc<Mutex<clipboard::ClipboardWatcher>>,
    encryption: encryption::SharedEncryption,
}

#[tauri::command]
async fn get_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<db::ClipboardItem>, String> {
    let db = state.db.lock().await;
    let mut items = db.get_history(limit.unwrap_or(100)).map_err(|e| e.to_string())?;
    let enc = state.encryption.lock().unwrap();
    if enc.is_enabled() {
        for item in &mut items {
            if let Ok(decrypted) = enc.decrypt(&item.content_preview) {
                item.content_preview = decrypted;
            }
        }
    }
    Ok(items)
}

#[tauri::command]
async fn search_items(
    state: State<'_, AppState>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<db::ClipboardItem>, String> {
    let db = state.db.lock().await;
    let enc = state.encryption.lock().unwrap();
    if enc.is_enabled() {
        let mut all_items = db.get_history(10000).map_err(|e| e.to_string())?;
        let query_lower = query.to_lowercase();
        let lim = limit.unwrap_or(50) as usize;
        let mut results = Vec::new();
        for mut item in all_items.drain(..) {
            if let Ok(decrypted) = enc.decrypt(&item.content_preview) {
                item.content_preview = decrypted;
            }
            let matches_content = item.content_preview.to_lowercase().contains(&query_lower);
            let matches_tags = item.tags.as_deref().unwrap_or("").to_lowercase().contains(&query_lower);
            if matches_content || matches_tags {
                results.push(item);
                if results.len() >= lim { break; }
            }
        }
        Ok(results)
    } else {
        db.search(&query, limit.unwrap_or(50)).map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn get_item_content(
    state: State<'_, AppState>,
    id: i64,
) -> Result<String, String> {
    let db = state.db.lock().await;
    let content = db.get_content(id).map_err(|e| e.to_string())?;
    let enc = state.encryption.lock().unwrap();
    if enc.is_enabled() {
        Ok(enc.decrypt(&content).unwrap_or(content))
    } else {
        Ok(content)
    }
}

#[tauri::command]
async fn copy_to_clipboard(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let content = db.get_content(id).map_err(|e| e.to_string())?;
    let item_type = db.get_item_type(id).map_err(|e| e.to_string())?;
    db.increment_use_count(id).map_err(|e| e.to_string())?;
    drop(db);

    let decrypted = {
        let enc = state.encryption.lock().unwrap();
        if enc.is_enabled() { enc.decrypt(&content).unwrap_or(content) } else { content }
    };

    let mut clipboard = state.clipboard.lock().await;
    clipboard.set_content(&decrypted, &item_type)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn pin_item(
    state: State<'_, AppState>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.set_pinned(id, pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_as_template(
    state: State<'_, AppState>,
    id: i64,
    template: bool,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.mark_as_template(id, template)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_item(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.delete_item(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history(
    state: State<'_, AppState>,
    keep_pinned: bool,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.clear_history(keep_pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn show_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.show().unwrap();
        window.set_focus().unwrap();
        window.center().unwrap();
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().unwrap();
    }
}

#[tauri::command]
async fn start_clipboard_watcher(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.clone();
    let encryption = state.encryption.clone();
    let app_handle = app.clone();
    
    tauri::async_runtime::spawn(async move {
        clipboard::watch_clipboard(db, encryption, app_handle).await;
    });
    
    Ok(())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<settings::Settings, String> {
    let db = state.db.lock().await;
    db.get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_auto_cleanup(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().await;
    let s = db.get_settings().map_err(|e| e.to_string())?;

    if s.auto_cleanup_enabled && s.auto_cleanup_days > 0 {
        db.cleanup_old_items(s.auto_cleanup_days).map_err(|e| e.to_string())?;
    }
    db.cleanup_by_type("text", s.cleanup_text_days).map_err(|e| e.to_string())?;
    db.cleanup_by_type("image", s.cleanup_image_days).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn update_settings(
    state: State<'_, AppState>,
    settings: settings::Settings,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_history(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().await;
    let mut items = db.get_history(10000).map_err(|e| e.to_string())?;
    let enc = state.encryption.lock().unwrap();
    if enc.is_enabled() {
        for item in &mut items {
            if let Ok(dec) = enc.decrypt(&item.content_preview) {
                item.content_preview = dec;
            }
            if let Some(ref full) = item.content_full.clone() {
                if let Ok(dec) = enc.decrypt(full) {
                    item.content_full = Some(dec);
                }
            }
        }
    }
    serde_json::to_string(&items).map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_history_file(state: State<'_, AppState>) -> Result<String, String> {
    let data = export_history(state).await?;
    
    let downloads = dirs::download_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
    
    let filename = format!(
        "clipboard-history-{}.json",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let path = downloads.join(&filename);
    
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn import_history(
    state: State<'_, AppState>,
    data: String,
) -> Result<(), String> {
    let items: Vec<db::ClipboardItem> = serde_json::from_str(&data)
        .map_err(|e| e.to_string())?;
    
    let db = state.db.lock().await;
    let enc = state.encryption.lock().unwrap();
    for item in items {
        let plaintext = item.content_full
            .as_deref()
            .unwrap_or(&item.content_preview);
        let (enc_full, enc_prev) = if enc.is_enabled() {
            let preview = if plaintext.len() > 200 {
                format!("{}...", &plaintext[..200])
            } else {
                plaintext.to_string()
            };
            (enc.encrypt(plaintext).ok(), enc.encrypt(&preview).ok())
        } else {
            (None, None)
        };
        db.save_item(&item.content_type, plaintext, None, enc_full.as_deref(), enc_prev.as_deref())
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_statistics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().await;
    db.get_statistics().map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_item_tags(
    state: State<'_, AppState>,
    id: i64,
    tags: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    db.update_tags(id, &tags).map_err(|e| e.to_string())
}

#[tauri::command]
async fn unlock_encryption(
    state: State<'_, AppState>,
    password: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let salt = db.get_encryption_salt()?;
    
    let mut encryption = state.encryption.lock().unwrap();
    encryption.unlock(&password, &salt)?;
    
    Ok(())
}

#[tauri::command]
async fn lock_encryption(state: State<'_, AppState>) -> Result<(), String> {
    let mut encryption = state.encryption.lock().unwrap();
    encryption.lock();
    Ok(())
}

#[tauri::command]
async fn is_encryption_enabled(state: State<'_, AppState>) -> Result<bool, String> {
    let encryption = state.encryption.lock().unwrap();
    Ok(encryption.is_enabled())
}

#[tauri::command]
async fn setup_encryption(
    state: State<'_, AppState>,
    password: String,
) -> Result<(), String> {
    let password_hash = encryption::EncryptionManager::hash_password(&password)?;
    let salt = encryption::EncryptionManager::generate_salt();
    
    let db = state.db.lock().await;
    db.save_encryption_config(&password_hash, &salt).map_err(|e| e.to_string())?;
    
    let mut encryption = state.encryption.lock().unwrap();
    encryption.unlock(&password, &salt)?;
    
    Ok(())
}

#[tauri::command]
async fn verify_master_password(
    state: State<'_, AppState>,
    password: String,
) -> Result<bool, String> {
    let db = state.db.lock().await;
    let password_hash = db.get_password_hash().map_err(|e| e.to_string())?;
    
    match encryption::EncryptionManager::verify_password(&password, &password_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
async fn has_encryption_setup(state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.lock().await;
    Ok(db.has_encryption_setup())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize database
            let db = db::Database::new().expect("Failed to create database");
            let db = Arc::new(Mutex::new(db));
            
            // Initialize clipboard watcher
            let clipboard = clipboard::ClipboardWatcher::new()
                .expect("Failed to create clipboard watcher");
            let clipboard = Arc::new(Mutex::new(clipboard));
            
            // Initialize encryption
            let encryption = encryption::create_shared();
            
            app.manage(AppState { db, clipboard, encryption });
            
            // Register global shortcut
            let app_handle_for_shortcut = app.handle().clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts(["CmdOrCtrl+Shift+V"])?
                    .with_handler(move |_app, _shortcut, event| {
                        if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                            let _ = show_window(app_handle_for_shortcut.clone());
                        }
                    })
                    .build(),
            )?;
            
            // Create tray icon menu
            let show_item = MenuItem::with_id(app, "show", "Show Clipboard Manager", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&show_item, &settings_item, &quit_item])?;
            
            let app_handle_for_tray = app.handle().clone();
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            show_window(app_handle_for_tray.clone());
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.eval("window.location.href = 'settings.html'");
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.center();
                            }
                        }
                    }
                })
                .build(app)?;
            
            // Run auto-cleanup on startup
            {
                let state = app.state::<AppState>();
                let db = state.db.clone();
                tauri::async_runtime::spawn(async move {
                    let db = db.lock().await;
                    if let Ok(s) = db.get_settings() {
                        if s.auto_cleanup_enabled && s.auto_cleanup_days > 0 {
                            let _ = db.cleanup_old_items(s.auto_cleanup_days);
                        }
                        let _ = db.cleanup_by_type("text", s.cleanup_text_days);
                        let _ = db.cleanup_by_type("image", s.cleanup_image_days);
                    }
                });
            }

            // Start clipboard watcher
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let db = state.db.clone();
            let encryption = state.encryption.clone();
            tauri::async_runtime::spawn(async move {
                clipboard::watch_clipboard(db, encryption, app_handle).await;
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            search_items,
            get_item_content,
            copy_to_clipboard,
            pin_item,
            mark_as_template,
            delete_item,
            clear_history,
            show_window,
            hide_window,
            start_clipboard_watcher,
            get_settings,
            update_settings,
            run_auto_cleanup,
            export_history,
            export_history_file,
            import_history,
            get_statistics,
            update_item_tags,
            unlock_encryption,
            lock_encryption,
            is_encryption_enabled,
            setup_encryption,
            verify_master_password,
            has_encryption_setup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
