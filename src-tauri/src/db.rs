use chrono::Local;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardItem {
    pub id: i64,
    pub content_type: String,
    pub content_hash: String,
    pub content_preview: String,
    pub content_full: Option<String>,
    pub thumbnail_path: Option<String>,
    pub source_app: Option<String>,
    pub created_at: i64,
    pub is_pinned: bool,
    pub is_template: bool,
    pub use_count: i64,
    pub tags: Option<String>,
}

pub struct Database {
    conn: Connection,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

impl Database {
    pub fn new() -> Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clipboard-manager");
        
        fs::create_dir_all(&data_dir)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        
        let db_path = data_dir.join("clipboard.db");
        let conn = Connection::open(db_path)?;
        
        let db = Self { conn, data_dir };
        db.init_schema()?;
        
        Ok(db)
    }
    
    fn init_schema(&self) -> Result<()> {
        // Create base tables
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL,
                content_hash TEXT NOT NULL UNIQUE,
                content_preview TEXT,
                content_full BLOB,
                thumbnail_path TEXT,
                source_app TEXT,
                created_at INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                use_count INTEGER NOT NULL DEFAULT 0
            );
            
            CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_items(created_at);
            CREATE INDEX IF NOT EXISTS idx_search ON clipboard_items(content_preview);
            CREATE INDEX IF NOT EXISTS idx_pinned ON clipboard_items(is_pinned);
            
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            );
            "
        )?;
        
        // Migrations — safe to re-run, errors ignored
        let _ = self.conn.execute("ALTER TABLE clipboard_items ADD COLUMN tags TEXT", []);
        let _ = self.conn.execute("ALTER TABLE clipboard_items ADD COLUMN is_template INTEGER NOT NULL DEFAULT 0", []);
        let _ = self.conn.execute("CREATE INDEX IF NOT EXISTS idx_tags ON clipboard_items(tags)", []);
        let _ = self.conn.execute("CREATE INDEX IF NOT EXISTS idx_template ON clipboard_items(is_template)", []);
        
        Ok(())
    }
    
    pub fn save_item(
        &self,
        content_type: &str,
        content: &str,
        source_app: Option<&str>,
        encrypted_content: Option<&str>,
        encrypted_preview: Option<&str>,
    ) -> Result<i64> {
        // Check for duplicates by hash
        let hash = Self::compute_hash(content);
        
        let existing: Result<i64> = self.conn.query_row(
            "SELECT id FROM clipboard_items WHERE content_hash = ?1",
            params![&hash],
            |row| row.get(0),
        );
        
        if existing.is_ok() {
            // Update timestamp for existing item
            self.conn.execute(
                "UPDATE clipboard_items SET created_at = ?1 WHERE content_hash = ?2",
                params![Local::now().timestamp(), &hash],
            )?;
            return Ok(existing.unwrap());
        }
        
        // Preview: first 200 chars or image indicator (plaintext, used as fallback)
        let plain_preview = if content_type == "text" {
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            }
        } else if content_type == "image" {
            "[Image]".to_string()
        } else {
            "[File]".to_string()
        };
        
        let preview_to_save = encrypted_preview.unwrap_or(&plain_preview);
        let content_to_save = encrypted_content.unwrap_or(content);
        
        self.conn.execute(
            "INSERT INTO clipboard_items 
             (content_type, content_hash, content_preview, content_full, source_app, created_at, is_pinned, use_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)",
            params![
                content_type,
                &hash,
                preview_to_save,
                content_to_save,
                source_app,
                Local::now().timestamp()
            ],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    pub fn get_history(&self, limit: i64) -> Result<Vec<ClipboardItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content_type, content_hash,
                    CASE WHEN content_type = 'image' THEN content_full ELSE content_preview END as content_preview,
                    content_full,
                    thumbnail_path, source_app, created_at, is_pinned, is_template, use_count, tags
             FROM clipboard_items
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT ?1"
        )?;
        
        let items = stmt.query_map(params![limit], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                content_hash: row.get(2)?,
                content_preview: row.get(3)?,
                content_full: row.get(4)?,
                thumbnail_path: row.get(5)?,
                source_app: row.get(6)?,
                created_at: row.get(7)?,
                is_pinned: row.get::<_, i64>(8)? != 0,
                is_template: row.get::<_, i64>(9)? != 0,
                use_count: row.get(10)?,
                tags: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
        
        Ok(items)
    }
    
    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<ClipboardItem>> {
        let search_term = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, content_type, content_hash,
                    CASE WHEN content_type = 'image' THEN content_full ELSE content_preview END as content_preview,
                    content_full,
                    thumbnail_path, source_app, created_at, is_pinned, is_template, use_count, tags
             FROM clipboard_items
             WHERE content_preview LIKE ?1 OR tags LIKE ?1
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT ?2"
        )?;
        
        let items = stmt.query_map(params![search_term, limit], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                content_hash: row.get(2)?,
                content_preview: row.get(3)?,
                content_full: row.get(4)?,
                thumbnail_path: row.get(5)?,
                source_app: row.get(6)?,
                created_at: row.get(7)?,
                is_pinned: row.get::<_, i64>(8)? != 0,
                is_template: row.get::<_, i64>(9)? != 0,
                use_count: row.get(10)?,
                tags: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
        
        Ok(items)
    }
    
    pub fn get_content(&self, id: i64) -> Result<String> {
        self.conn.query_row(
            "SELECT content_full FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
    }
    
    pub fn get_item_type(&self, id: i64) -> Result<String> {
        self.conn.query_row(
            "SELECT content_type FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
    }
    
    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
            params![if pinned { 1 } else { 0 }, id],
        )?;
        Ok(())
    }
    
    pub fn delete_item(&self, id: i64) -> Result<()> {
        // Clean up thumbnail if exists
        if let Ok(thumb) = self.conn.query_row::<String, _, _>(
            "SELECT thumbnail_path FROM clipboard_items WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ) {
            if !thumb.is_empty() {
                let _ = fs::remove_file(&thumb);
            }
        }
        
        self.conn.execute(
            "DELETE FROM clipboard_items WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }
    
    pub fn increment_use_count(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET use_count = use_count + 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn mark_as_template(&self, id: i64, is_template: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET is_template = ?1 WHERE id = ?2",
            params![is_template as i64, id],
        )?;
        Ok(())
    }
    
    pub fn clear_history(&self, keep_pinned: bool) -> Result<()> {
        if keep_pinned {
            self.conn.execute(
                "DELETE FROM clipboard_items WHERE is_pinned = 0",
                [],
            )?;
        } else {
            self.conn.execute("DELETE FROM clipboard_items", [])?;
        }
        Ok(())
    }
    
    fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    /// Delete non-pinned non-template items of a given type older than `days` days.
    /// If days == 0 the cleanup is skipped.
    pub fn cleanup_by_type(&self, content_type: &str, days: i64) -> Result<usize> {
        if days <= 0 {
            return Ok(0);
        }
        let cutoff = Local::now().timestamp() - (days * 24 * 60 * 60);
        let deleted = self.conn.execute(
            "DELETE FROM clipboard_items
             WHERE content_type = ?1
               AND created_at < ?2
               AND is_pinned = 0
               AND is_template = 0",
            params![content_type, cutoff],
        )?;
        Ok(deleted)
    }

    pub fn cleanup_old_items(&self, days: i64) -> Result<usize> {
        let cutoff = Local::now().timestamp() - (days * 24 * 60 * 60);
        let count = self.conn.execute(
            "DELETE FROM clipboard_items WHERE created_at < ?1 AND is_pinned = 0",
            params![cutoff],
        )?;
        Ok(count)
    }
    
    pub fn get_settings(&self) -> Result<crate::settings::Settings> {
        crate::settings::Settings::load(&self.conn)
    }
    
    pub fn save_settings(&self, settings: &crate::settings::Settings) -> Result<()> {
        settings.save(&self.conn)
    }
    
    pub fn get_statistics(&self) -> Result<serde_json::Value> {
        let total_items: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;
        
        let pinned_items: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE is_pinned = 1",
            [],
            |row| row.get(0),
        )?;
        
        let text_items: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'text'",
            [],
            |row| row.get(0),
        )?;
        
        let image_items: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clipboard_items WHERE content_type = 'image'",
            [],
            |row| row.get(0),
        )?;
        
        let most_used = self.get_history(5)?;
        
        Ok(serde_json::json!({
            "total_items": total_items,
            "pinned_items": pinned_items,
            "text_items": text_items,
            "image_items": image_items,
            "most_used": most_used,
        }))
    }
    
    pub fn update_tags(&self, id: i64, tags: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE clipboard_items SET tags = ?1 WHERE id = ?2",
            params![tags, id],
        )?;
        Ok(())
    }
    
    pub fn save_encryption_config(&self, password_hash: &str, salt: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('encryption_password_hash', ?1)",
            params![password_hash],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('encryption_salt', ?1)",
            params![salt],
        )?;
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('encryption_enabled', 'true')",
            [],
        )?;
        Ok(())
    }
    
    pub fn get_password_hash(&self) -> Result<String> {
        self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'encryption_password_hash'",
            [],
            |row| row.get(0),
        ).map_err(|_| rusqlite::Error::QueryReturnedNoRows)
    }
    
    pub fn get_encryption_salt(&self) -> Result<String, String> {
        self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'encryption_salt'",
            [],
            |row| row.get(0),
        ).map_err(|_| "Encryption not configured".to_string())
    }
    
    pub fn has_encryption_setup(&self) -> bool {
        self.conn.query_row(
            "SELECT value FROM settings WHERE key = 'encryption_enabled'",
            [],
            |row| row.get::<_, String>(0),
        ).is_ok()
    }
}
