use crate::db::Database;
use crate::encryption::SharedEncryption;
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

/// Encodes captured image bytes as a base64 data-URI for inline HTML preview.
pub struct ClipboardWatcher {
    clipboard: Clipboard,
}

impl ClipboardWatcher {
    pub fn new() -> Result<Self, arboard::Error> {
        let clipboard = Clipboard::new()?;
        Ok(Self { clipboard })
    }
    
    pub fn set_content(&mut self, content: &str, content_type: &str) -> Result<(), arboard::Error> {
        if content_type == "text" {
            self.clipboard.set_text(content)
        } else if content_type == "image" {
            // Decode base64 image and set to clipboard
            if let Ok(image_bytes) = general_purpose::STANDARD.decode(content) {
                if let Ok(img) = image::load_from_memory(&image_bytes) {
                    let rgba = img.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    let image_data = ImageData {
                        width: width as usize,
                        height: height as usize,
                        bytes: rgba.into_raw().into(),
                    };
                    return self.clipboard.set_image(image_data);
                }
            }
            Err(arboard::Error::ConversionFailure)
        } else {
            self.clipboard.set_text(content)
        }
    }
    
    #[allow(dead_code)]
    pub fn get_text(&mut self) -> Result<String, arboard::Error> {
        self.clipboard.get_text()
    }
}

pub async fn watch_clipboard(db: Arc<Mutex<Database>>, encryption: SharedEncryption, app: AppHandle) {
    let mut last_text = String::new();
    let mut last_image_hash = String::new();
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialize clipboard: {}", e);
            return;
        }
    };
    
    loop {
        let mut changed = false;
        
        // Check for images first (higher priority)
        if let Ok(image) = clipboard.get_image() {
            let rgba_image = DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(
                    image.width as u32,
                    image.height as u32,
                    image.bytes.to_vec(),
                )
                .unwrap_or_else(|| image::RgbaImage::new(1, 1)),
            );
            
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            if rgba_image.write_to(&mut cursor, ImageFormat::Png).is_ok() {
                let image_base64 = general_purpose::STANDARD.encode(&buffer);
                let image_hash = format!("{:x}", md5::compute(&buffer));
                
                if image_hash != last_image_hash {
                    last_image_hash = image_hash;
                    last_text.clear();
                    
                    let enc_full = {
                        let enc = encryption.lock().unwrap();
                        if enc.is_enabled() { enc.encrypt(&image_base64).ok() } else { None }
                    };
                    
                    let db = db.lock().await;
                    if let Err(e) = db.save_item("image", &image_base64, None, enc_full.as_deref(), None) {
                        eprintln!("Failed to save image: {}", e);
                    } else {
                        changed = true;
                    }
                }
            }
        }
        // Check for text if no image
        else if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() && text != last_text {
                last_text = text.clone();
                last_image_hash.clear();
                
                let (enc_full, enc_preview) = {
                    let enc = encryption.lock().unwrap();
                    if enc.is_enabled() {
                        let preview = if text.len() > 200 {
                            format!("{}...", &text[..200])
                        } else {
                            text.clone()
                        };
                        (enc.encrypt(&text).ok(), enc.encrypt(&preview).ok())
                    } else {
                        (None, None)
                    }
                };
                
                let db = db.lock().await;
                if let Err(e) = db.save_item("text", &text, None, enc_full.as_deref(), enc_preview.as_deref()) {
                    eprintln!("Failed to save clipboard item: {}", e);
                } else {
                    changed = true;
                }
            }
        }
        
        if changed {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.emit::<()>("clipboard-changed", ()) {
                    eprintln!("Failed to emit clipboard-changed event: {}", e);
                }
            }
        }
        
        sleep(Duration::from_millis(500)).await;
    }
}
