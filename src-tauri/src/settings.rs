use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub history_limit: i64,
    pub auto_cleanup_enabled: bool,
    pub auto_cleanup_days: i64,
    pub cleanup_text_days: i64,
    pub cleanup_image_days: i64,
    pub theme: String,
    pub custom_hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            history_limit: 1000,
            auto_cleanup_enabled: false,
            auto_cleanup_days: 30,
            cleanup_text_days: 0,
            cleanup_image_days: 7,
            theme: "dark".to_string(),
            custom_hotkey: "CmdOrCtrl+Shift+V".to_string(),
        }
    }
}

impl Settings {
    pub fn load(conn: &Connection) -> Result<Self> {
        let mut settings = Settings::default();
        
        // Load each setting from database
        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'history_limit'",
            [],
            |row| row.get(0),
        ) {
            settings.history_limit = value.parse().unwrap_or(1000);
        }
        
        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'auto_cleanup_enabled'",
            [],
            |row| row.get(0),
        ) {
            settings.auto_cleanup_enabled = value == "true";
        }
        
        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'auto_cleanup_days'",
            [],
            |row| row.get(0),
        ) {
            settings.auto_cleanup_days = value.parse().unwrap_or(30);
        }

        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'cleanup_text_days'",
            [],
            |row| row.get(0),
        ) {
            settings.cleanup_text_days = value.parse().unwrap_or(0);
        }

        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'cleanup_image_days'",
            [],
            |row| row.get(0),
        ) {
            settings.cleanup_image_days = value.parse().unwrap_or(7);
        }
        
        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'theme'",
            [],
            |row| row.get(0),
        ) {
            settings.theme = value;
        }
        
        if let Ok(value) = conn.query_row::<String, _, _>(
            "SELECT value FROM settings WHERE key = 'custom_hotkey'",
            [],
            |row| row.get(0),
        ) {
            settings.custom_hotkey = value;
        }
        
        Ok(settings)
    }
    
    pub fn save(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('history_limit', ?1)",
            params![self.history_limit.to_string()],
        )?;
        
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('auto_cleanup_enabled', ?1)",
            params![self.auto_cleanup_enabled.to_string()],
        )?;
        
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('auto_cleanup_days', ?1)",
            params![self.auto_cleanup_days.to_string()],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('cleanup_text_days', ?1)",
            params![self.cleanup_text_days.to_string()],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('cleanup_image_days', ?1)",
            params![self.cleanup_image_days.to_string()],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme', ?1)",
            params![&self.theme],
        )?;
        
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('custom_hotkey', ?1)",
            params![&self.custom_hotkey],
        )?;
        
        Ok(())
    }
}
