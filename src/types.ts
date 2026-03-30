// Clipboard item from backend
export interface ClipboardItem {
  id: number;
  content_type: string;
  content_hash: string;
  content_preview: string;
  content_full: string | null;
  thumbnail_path: string | null;
  source_app: string | null;
  created_at: number;
  is_pinned: boolean;
  is_template: boolean;
  use_count: number;
  tags: string | null;
}

// Settings from backend
export interface Settings {
  history_limit: number;
  auto_cleanup_enabled: boolean;
  auto_cleanup_days: number;
  theme: string;
  custom_hotkey: string;
}

// Statistics from backend
export interface Statistics {
  total_items: number;
  pinned_items: number;
  text_items: number;
  image_items: number;
  most_used: ClipboardItem[];
}
