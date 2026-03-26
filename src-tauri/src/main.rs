/// Decrypts all items before serialisation to guarantee plaintext JSON output.
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

